# Architecture

## Design Philosophy

Dingir Exchange follows the **Redis/Viabtc** design philosophy:

> **Keep the hot path single-threaded and in-memory.** All complexity (networking, persistence, concurrency) is pushed to the edges so the core matching loop runs without locks, atomics, or cache contention.

This yields predictable latency and simplifies reasoning about correctness – there is only one thread mutating the order book and balances at any time.

## System Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                           Clients                                    │
│  (JS SDK / CLI / Trading Bots / REST Frontends)                     │
└──────────────┬────────────────────────────┬─────────────────────────┘
               │ gRPC                        │ HTTP
               ▼                             ▼
┌─────────────────────────┐      ┌─────────────────────────┐
│    GrpcHandler          │      │    REST API Server      │
│    (Tokio / Tonic)      │      │    (Actix-web 4)        │
│    Port: 50051          │      │    Port: 50053          │
└───────────┬─────────────┘      └───────────┬─────────────┘
            │                                │
            │ read: RwLock::read()           │ read: DB query
            │ write: mpsc::send()            │ write: gRPC call to matchengine
            ▼                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        Controller (Single Thread)                    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌───────────┐  │
│  │   Markets   │  │   Balance   │  │    Users    │  │ Sequencer │  │
│  │  (BTreeMap) │  │   Manager   │  │   Manager   │  │           │  │
│  └──────┬──────┘  └──────┬──────┘  └─────────────┘  └───────────┘  │
│         │                │                                          │
│         └────────────────┴──────────────────┐                       │
│                                             ▼                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                        Persistor                             │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │   │
│  │  │   Kafka     │  │     DB      │  │   Operation Log     │  │   │
│  │  │  Producer   │  │   Writer    │  │   (replayable)      │  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

## Component Deep-Dive

### GrpcHandler & Task Dispatcher

`GrpcHandler` (`src/matchengine/server.rs`) is the gRPC service implementation. It enforces a strict **read/write separation**:

* **Read operations** (`asset_list`, `balance_query`, `order_query`, `order_book_depth`, `market_list`, `market_summary`) acquire `stub.read().await` and return immediately. Multiple reads can execute concurrently.
* **Write operations** (`order_put`, `batch_order_put`, `order_cancel`, `balance_update`, `transfer`, `register_user`) are boxed into closures and sent through `task_dispatcher` (a `tokio::mpsc::channel(1024)`). A background task serializes all write operations by dequeuing closures and running them against `&mut Controller`.

This pattern eliminates lock contention in the hot path while preserving serializability.

Key parameters:
* `task_dispatcher` channel size: **1024** (was 16 – increased to handle burst traffic)
* `max_concurrent_streams`: **10,000** (HTTP/2 stream limit to avoid client-side queuing)

### Controller

The `Controller` (`src/matchengine/controller.rs`) is the central state machine. It owns:

* `markets: HashMap<String, Market>` – all trading pairs
* `balance_manager: BalanceManager` – all user balances
* `user_manager: UserManager` – user registration and signature verification
* `sequencer: Sequencer` – monotonic ID generation for orders and trades
* `persistor` – composite persistence layer

All state mutations happen here, in a single thread. There are no locks, no atomics, no async `.await` during matching – just plain synchronous Rust code.

### Market & Order Book

`Market` (`src/matchengine/market/mod.rs`) is the heart of the system.

#### Data Structures

```rust
pub struct Market {
    pub asks: BTreeMap<MarketKeyAsk, OrderRc>,   // ascending price
    pub bids: BTreeMap<MarketKeyBid, OrderRc>,   // descending price
    pub orders: BTreeMap<u64, OrderRc>,          // order_id → order
    pub users: BTreeMap<u32, BTreeMap<u64, OrderRc>>, // user_id → orders
    // ... precision, min_amount, etc.
}
```

`MarketKeyAsk` and `MarketKeyBid` are tuples of `(price, timestamp, order_id)` that implement `Ord` to give **price-time priority**:

* Asks sort ascending → lowest price first
* Bids sort descending → highest price first
* Within the same price, earlier timestamp wins

`OrderRc` is `Rc<RefCell<Order>>` (interior mutability), allowing the matching loop to mutate orders while iterating the `BTreeMap`.

#### Matching Algorithm

1. Validate order (precision, balance, market order constraints)
2. Create `Order` struct with `sequencer.next_order_id()`
3. Emit `PUT` event to persistor
4. Iterate counter-side order book (asks or bids):
   * Price cross? If limit order price doesn't cross, stop.
   * Self-trade? If `disable_self_trade` and same user, cancel.
   * Post-only? If matches immediately, cancel.
   * Calculate trade amount = `min(ask.remain, bid.remain)`
   * Apply market-order quote limit for bid side
   * Update order remains, finished amounts, fees
   * Update balances (4 balance updates per trade: bid base+, ask base-, ask quote+, bid quote-)
   * Emit trade to persistor
   * If maker order fully filled, remove from book
5. Handle taker order remainder:
   * Fully filled → `FINISH`
   * Partially filled limit → `insert_order_into_orderbook`
   * Market order → `FINISH`
   * Cancelled (self-trade/post-only) → `FINISH`

All balance updates use `BalanceUpdateController` which tracks both the in-memory balance and emits history records.

### Balance Manager

`BalanceManager` (`src/matchengine/asset/`) tracks per-user, per-asset balances with two buckets:

* **AVAILABLE** – free to trade or withdraw
* **FREEZE** – locked by open orders

Operations: `add`, `sub`, `frozen`, `unfrozen`. All are synchronous and called from the single Controller thread.

### Persistence Layer

The persistor (`src/matchengine/persist/`) implements a **three-layer durability model**:

#### Layer 1: Kafka Message Stream

Every state change (order put, trade, balance update) is serialized to JSON and pushed to Kafka topics:

| Topic | Content |
|-------|---------|
| `orders` | Order state changes (PUT, UPDATE, FINISH) |
| `trades` | Executed trades |
| `balances` | Balance history entries |
| `deposits` | Deposit events |
| `withdraws` | Withdraw events |
| `internaltransfer` | Internal transfers |
| `registeruser` | User registrations |
| `unifyevents` | Unified event stream |

Producer architecture (`src/message/producer.rs`):
* `BaseProducer` (not `FutureProducer`) for minimal latency
* Dedicated OS thread running `producer.poll()` loop
* `crossbeam_channel::bounded(100_000)` for backpressure
* Backpressure threshold: queue length ≥ capacity − 10,000

#### Layer 2: Database Historical Storage

The `persistor` binary consumes Kafka messages and writes to PostgreSQL/TimescaleDB:

* `order_history` – final order states
* `user_trade` – per-user trade view
* `balance_history` – balance change log
* `market_trade` – market-level trade stream (used for K-lines)
* `account` – user accounts
* `internal_tx` – internal transfers

`DatabaseWriter` (`src/storage/database.rs`) batches INSERTs asynchronously:
* Batch size: **20,000** rows (was 5,000)
* Channel capacity: **10,000**
* Spawn limit: **8** concurrent INSERT tasks
* Backpressure (`is_block`): triggered when pending > 2× capability_limit

#### Layer 3: Operation Log (Replay)

Every RPC call that mutates state is appended to `operation_log`:

```rust
pub struct OperationLog {
    pub id: i64,
    pub time: TimestampDbType,
    pub method: String,   // e.g. "order_put"
    pub params: String,   // JSON-serialized request
}
```

This enables **state recovery** via `debug_reset`: clear in-memory state → replay all operation log entries from DB → reconstruct markets, balances, and orders.

For performance-critical deployments, `FastOperationLogWriter` writes to a JSONL file instead of DB INSERT, using a dedicated thread + `BufWriter` + crossbeam channel.

#### Fork-and-Save Snapshots

Inspired by Redis RDB, `fork_and_make_slice` (called periodically, default every 60s) serializes the current order book and balance state to DB tables `order_slice` and `balance_slice`. This provides a checkpoint for faster recovery than full operation-log replay.

### REST API Server

`restapi` (`src/bin/restapi.rs`, port **50053**) is an Actix-web server providing:

* **Public market data** – recent trades, order book depth, ticker, TradingView K-line history
* **User data** – closed orders, trade history, internal transfers
* **User lookup** – L1 address / L2 pubkey → user info
* **Management** (optional) – add assets, add trading pairs, reload markets (requires `manage_endpoint` config)

Read queries hit PostgreSQL directly. Write operations (management) proxy via gRPC to `matchengine`.

OpenAPI spec is auto-generated via `paperclip` and served at `/api/spec`.
