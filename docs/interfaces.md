# Interfaces

Dingir Exchange exposes two primary interfaces: a **gRPC API** for high-performance trading operations and a **REST API** for queries and management.

---

## gRPC API

Defined in `proto/exchange/matchengine.proto`. The service is generated via `tonic-build` and served by `matchengine` on port **50051**.

### Service: `Matchengine`

#### Read Operations (Non-Mutating)

These execute under `RwLock::read()` and do not pass through the Controller dispatch queue.

| RPC | Request | Response | Description |
|-----|---------|----------|-------------|
| `AssetList` | `AssetListRequest` | `AssetListResponse` | List all configured assets |
| `BalanceQuery` | `BalanceQueryRequest` | `BalanceQueryResponse` | Query user balances per asset |
| `OrderQuery` | `OrderQueryRequest` | `OrderQueryResponse` | Query user's active orders in a market |
| `OrderBookDepth` | `OrderBookDepthRequest` | `OrderBookDepthResponse` | Market depth (grouped by price) |
| `OrderDetail` | `OrderDetailRequest` | `OrderInfo` | Get details of a specific order |
| `MarketList` | `MarketListRequest` | `MarketListResponse` | List all markets |
| `MarketSummary` | `MarketSummaryRequest` | `MarketSummaryResponse` | 24h summary per market |

#### Write Operations (State-Mutating)

These are serialized through the single-threaded Controller dispatch queue.

| RPC | Request | Response | Description |
|-----|---------|----------|-------------|
| `RegisterUser` | `UserInfo` | `UserInfo` | Register a new user with L1/L2 credentials |
| `BalanceUpdate` | `BalanceUpdateRequest` | `BalanceUpdateResponse` | Deposit/withdraw/force-update balance |
| `OrderPut` | `OrderPutRequest` | `OrderInfo` | Submit a single order (limit or market) |
| `BatchOrderPut` | `BatchOrderPutRequest` | `BatchOrderPutResponse` | Submit up to **40** orders in one RPC |
| `OrderCancel` | `OrderCancelRequest` | `OrderInfo` | Cancel a specific order |
| `OrderCancelAll` | `OrderCancelAllRequest` | `OrderCancelAllResponse` | Cancel all orders for a user in a market |
| `Transfer` | `TransferRequest` | `TransferResponse` | Internal transfer between users |
| `ReloadMarkets` | `ReloadMarketsRequest` | `SimpleSuccessResponse` | Reload market config from DB |

#### Debug Operations (Development Only)

Available only in debug builds (`cfg(debug_assertions)`).

| RPC | Description |
|-----|-------------|
| `DebugDump` | Dump in-memory state to DB |
| `DebugReset` | Clear state and replay from `operation_log` |
| `DebugReload` | Reload state from DB slices |

### Key Message Types

#### `OrderPutRequest`

```protobuf
message OrderPutRequest {
  uint32 user_id = 1;
  string market = 2;
  OrderSide side = 3;      // BID or ASK
  OrderType type_ = 4;     // LIMIT or MARKET
  string amount = 5;       // Decimal string
  string price = 6;        // Decimal string (0 for market)
  string taker_fee = 7;    // Decimal string
  string maker_fee = 8;    // Decimal string
  string quote_limit = 9;  // Max quote for market bid
  bool post_only = 10;     // Cancel if would match immediately
  bytes signature = 11;    // EdDSA signature (optional)
}
```

#### `BatchOrderPutRequest`

```protobuf
message BatchOrderPutRequest {
  string market = 1;
  bool auto_fills = 2;          // Auto-fill remaining as new orders
  repeated OrderPutRequest orders = 3;  // Max 40
}
```

Batch order put is the recommended way for high-frequency submission. It amortizes gRPC round-trip overhead across up to 40 orders, yielding ~10× throughput improvement over single `OrderPut` calls.

#### `OrderInfo` (Response)

```protobuf
message OrderInfo {
  uint64 id = 1;
  string market = 2;
  OrderType type_ = 3;
  OrderSide side = 4;
  string amount = 5;
  string price = 6;
  string taker_fee = 7;
  string maker_fee = 8;
  string remain = 9;
  string finished_base = 10;
  string finished_quote = 11;
  string finished_fee = 12;
  // ... timestamps, status
}
```

All numeric fields are transmitted as **decimal strings** to preserve arbitrary precision. The client and server both use `rust_decimal::Decimal` for parsing.

### Google API Annotations

Many RPCs include `google.api.http` annotations, enabling HTTP/JSON transcoding (e.g. via grpc-gateway or Envoy):

```protobuf
rpc OrderPut(OrderPutRequest) returns (OrderInfo) {
  option (google.api.http) = {
    post: "/api/order"
    body: "*"
  };
}
```

---

## REST API

Served by `restapi` binary on port **50053** using Actix-web 4 with Paperclip (OpenAPI 2.0 auto-generation).

Base path: `/api/exchange/panel`

### Public Market Data

| Method | Path | Description |
|--------|------|-------------|
| GET | `/ping` | Health check |
| GET | `/recenttrades/{market}` | Recent trades |
| GET | `/ordertrades/{market}/{order_id}` | Trades for a specific order |
| GET | `/ticker_{ticker_inv}/{market}` | 24h ticker data |

### TradingView Integration

| Method | Path | Description |
|--------|------|-------------|
| GET | `/tradingview/time` | Server unix timestamp |
| GET | `/tradingview/config` | Chart configuration |
| GET | `/tradingview/symbols` | Symbol info |
| GET | `/tradingview/search` | Symbol search |
| GET | `/tradingview/history` | OHLCV history (K-line) |

The `/tradingview/history` endpoint queries `market_trade` from TimescaleDB and returns data in [TradingView UDF](https://www.tradingview.com/charting-library-docs/latest/connecting_data/UDF/) format.

### User Data

| Method | Path | Description |
|--------|------|-------------|
| GET | `/user/{l1addr_or_l2pubkey}` | Lookup user by address |
| GET | `/closedorders/{market}/{user_id}` | User's closed orders |
| GET | `/internal_txs/{user_id}` | User's internal transfers |

### Management (Optional)

Enabled when `manage_endpoint` is configured in REST API settings. Proxies to `matchengine` gRPC.

| Method | Path | Description |
|--------|------|-------------|
| POST | `/manage/market/reload` | Reload markets from DB |
| POST | `/manage/market/tradepairs` | Add a new trading pair |
| POST | `/manage/market/assets` | Add a new asset |

### OpenAPI Spec

The full OpenAPI 2.0 JSON spec is auto-served at:

```
GET /api/spec
```

---

## Protocol Buffer Compilation

Proto files are compiled automatically during `cargo build` via `build.rs`:

```rust
tonic_build::configure()
    .build_server(true)
    .build_client(true)
    .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
    .compile_protos(
        &["proto/exchange/matchengine.proto", "proto/rollup/rollup.proto"],
        &["proto", "proto/third_party/googleapis"],
    )?;
```

Generated code lands in `target/` and is included via `include!(concat!(env!("OUT_DIR"), "/..."))` in `src/rpc/mod.rs`.
