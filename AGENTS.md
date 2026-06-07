# Agent/Developer Quick Reference

This file contains the essential context needed to navigate and modify the Dingir Exchange codebase efficiently.

## Project Structure Map

| Directory | Responsibility |
|-----------|---------------|
| `src/matchengine/` | Core matching engine: gRPC server, Controller, order book (`Market`), balance management |
| `src/matchengine/server.rs` | `GrpcHandler` – gRPC request routing, task dispatcher, read/write separation |
| `src/matchengine/controller.rs` | `Controller` – orchestrates markets, balances, users; all write ops run here |
| `src/matchengine/market/` | `Market` – in-memory order book using `BTreeMap<price+time, Order>` |
| `src/matchengine/asset/` | `BalanceManager` – per-user per-asset available/frozen balance tracking |
| `src/matchengine/persist/` | State persistence: migrations, DB init, fork-and-save snapshots |
| `src/message/` | Kafka producer/consumer, message types (`OrderMessage`, `Trade`, `BalanceMessage`) |
| `src/storage/` | Database models (`models.rs`), `DatabaseWriter` (batch async INSERT), SQL helpers |
| `src/restapi/` | Actix-web HTTP server: market data, user history, TradingView K-lines, management |
| `src/types.rs` | Core enums and structs: `OrderSide`, `OrderType`, `OrderEventType`, `MarketRole` |
| `proto/` | gRPC `.proto` definitions (compiled via `tonic-build` in `build.rs`) |
| `src/bin/` | Executable binaries: `matchengine`, `restapi`, `persistor`, `perftest` |

## Critical Architecture Conventions

### Single-Threaded Controller

All **write operations** (order put, cancel, balance update, transfer) **must** go through the single `Controller` thread:

1. `GrpcHandler` receives a gRPC write request.
2. It wraps the closure in `ControllerDispatch` and sends it via `task_dispatcher` (a `tokio::mpsc` channel).
3. A background task dequeues the closure, acquires `&mut Controller`, executes it, and returns the result via `oneshot`.

**Never** call `Controller` write methods directly from async handlers – this will cause data races.

Read operations (balance query, order query, market depth) acquire `RwLock::read()` directly.

### Order Book Data Structure

The order book uses `BTreeMap` keyed by `(price, timestamp, order_id)` to achieve price-time priority:

* **Asks** – ascending price (best ask at the top)
* **Bids** – descending price (best bid at the top)
* Each `Order` is wrapped in `RefCell<Order>` (`OrderRc`) for interior mutability during matching

### Persistence: Three Layers

1. **Kafka** – Real-time event streaming (orders, trades, balances). Decouples engine from downstream consumers.
2. **PostgreSQL/TimescaleDB** – Historical data (order history, trade history, balance history). Written by the `persistor` binary consuming Kafka.
3. **Operation Log** – Every state-changing RPC is logged to `operation_log` table (or `FastOperationLogWriter` file). Used by `debug_reset` to replay and recover state.

### Message Flow

```
Controller → Persistor → Kafka Producer (crossbeam_channel + BaseProducer thread)
                                   ↓
                              Kafka Topics
                                   ↓
                           persistor binary (Consumer)
                                   ↓
                            PostgreSQL/TimescaleDB
```

## Adding a New Feature

Typical workflow for adding a new RPC or market behavior:

1. **Define the contract** – Add message types to `proto/exchange/matchengine.proto`.
2. **Regenerate code** – `cargo build` triggers `tonic-build` in `build.rs`.
3. **Implement in Controller** – Add the business logic to `src/matchengine/controller.rs`.
4. **Wire to gRPC** – Add the handler in `src/matchengine/server.rs`:
   * Read-only → acquire `stub.read().await` directly.
   * Write → use `ControllerDispatch::new(...)` + `task_dispatcher.send(...)`.
5. **Add persistence** (if needed) – Update `Persistor`/`MessageManager` in `src/message/` and `src/matchengine/persist/`.
6. **Add tests** – Unit tests in relevant module; integration test in `examples/js/tests/`.

## Running Tests

```bash
# Unit tests
cargo test

# Performance test (use Rust perftest; JS clients are ~100× slower)
cargo run --bin perftest -- --mode order-put --workers 4 --requests 10000

# Sustained pair-trade load test (10 min)
cargo run --release --bin perftest -- \
  --mode pair-trade --workers 4 --duration-secs 600 --requests 10000000

# Integration test (requires running services)
cd examples/js && npx ts-node tests/trade.ts
```

## Build Notes

* **Edition 2024** – Requires Rust 1.90.0+.
* **tonic-build** – Proto compilation happens automatically during `cargo build`. Ensure `protoc` is available.
* **librdkafka** – The `rdkafka` crate links against the system C library. Install via `brew install librdkafka` (macOS) or `apt install librdkafka-dev` (Ubuntu).
* **sqlx** – Uses `runtime-tokio` with `tls-rustls`. No `sqlx-data.json` is committed; relies on compile-time `DATABASE_URL` for query checking.
* **Features** – `emit_state_diff` (default) includes verbose state diff in trades for debugging.

### macOS Startup Advisory Lock Workaround

On macOS, `sqlx::migrate::Migrator::run` on a `Pool` can hold a PostgreSQL advisory lock indefinitely, causing the second service (persistor) to hang waiting for the lock.

**Solution:** Start `matchengine` first (it uses a single `PgConnection` for migration), wait for it to complete migrations, then start `persistor`.

Alternatively, `src/bin/persistor.rs` has been updated to use a dedicated `PgConnection` for migration before creating the `Pool`.

## Important Constants

| Constant | Location | Value | Meaning |
|----------|----------|-------|---------|
| `MAX_BATCH_ORDER_NUM` | `server.rs` | 40 | Max orders per `BatchOrderPut` |
| `user_order_num_limit` | `default.yaml` | 2000 | Max open orders per user |
| `INSERT_LIMIT` | `database.rs` | 20,000 | DB batch insert size |
| `CHANNEL_LIMIT` | `database.rs` | 10,000 | DB writer channel capacity |
| `task_dispatcher` | `server.rs` | 1,024 | Controller dispatch queue size |
| `max_concurrent_streams` | `matchengine.rs` | 10,000 | HTTP/2 concurrent streams |
