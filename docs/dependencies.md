# External Dependencies: Why These Technologies?

This document explains the rationale behind each major external dependency and the trade-offs considered.

---

## PostgreSQL + TimescaleDB

### What We Use It For

* **Operational data** – `operation_log`, user accounts, asset config
* **Historical data** – order history, trade history, balance history
* **Time-series analytics** – K-line (OHLCV) generation via `market_trade` hypertable
* **State snapshots** – `order_slice`, `balance_slice` for fork-and-save checkpoints

### Why PostgreSQL

PostgreSQL provides ACID transactions, rich indexing, and mature ecosystem. For an exchange, data integrity is non-negotiable – every trade and balance update must be durably recorded.

### Why TimescaleDB

TimescaleDB is a PostgreSQL extension that adds **hypertables** – automatic time-based partitioning optimized for time-series workloads:

* **K-line queries** (`market_trade` aggregated by time buckets) are 10-100× faster with continuous aggregates than plain PostgreSQL.
* **Retention policies** automatically drop old raw trade data while keeping aggregated K-lines.
* **It is still PostgreSQL** – we get the full SQL feature set, not a specialized time-series-only database.

### Why Not a Dedicated Time-Series DB?

Options like InfluxDB or ClickHouse offer higher raw ingestion throughput, but:
* They require a separate operational database for relational data (users, assets, config).
* Complex joins between time-series and relational data become cross-system ETL.
* TimescaleDB lets us keep everything in one system with one connection pool (`sqlx::Pool`).

### Optimization: UNLOGGED Tables

The `operation_log` table (write-heavy, replay-only) is created as `UNLOGGED` to skip WAL overhead, doubling INSERT throughput at the cost of non-durability across crashes. For deployments requiring full durability, switch to a regular logged table.

---

## gRPC + Tonic

### What We Use It For

All trading operations (order put, cancel, balance update) and internal communication (REST API → matchengine) use gRPC.

### Why gRPC

* **Binary efficiency** – Protobuf serialization is ~5-10× smaller and faster than JSON.
* **HTTP/2 multiplexing** – A single TCP connection handles thousands of concurrent streams. We set `max_concurrent_streams = 10,000` to avoid head-of-line blocking.
* **Schema evolution** – Proto files provide a versioned, language-neutral contract.
* **Streaming support** – Although not currently used for market data push, the foundation exists for server-side streaming of order/trade events.

### Why Tonic

[Tonic](https://github.com/hyperium/tonic) is the de-facto gRPC framework for Rust:

* Built on Tokio and Hyper – zero-cost async/await.
* `tonic-build` generates Rust code from `.proto` at compile time.
* Full client/server support with interceptors and middleware hooks.

### Trade-off: Not REST for Trading

REST/JSON is simpler to debug with `curl`, but:
* JSON parsing of decimal strings is slower and error-prone.
* HTTP/1.1 requires connection pools or suffers from head-of-line blocking.
* No built-in request/response schema validation.

We provide REST only for **read-only queries** and management, where latency is less critical.

---

## Apache Kafka

### What We Use It For

Kafka acts as the **event bus** between the matching engine and downstream systems:

* `matchengine` produces order/trade/balance events to Kafka topics.
* `persistor` binary consumes these topics and writes to PostgreSQL.
* Future consumers (notification service, analytics, indexers) can read from Kafka without impacting the engine.

### Why Kafka

* **Decoupling** – The matching engine never waits for DB writes. It pushes to Kafka and returns immediately.
* **Durability** – Kafka replicates events across brokers before acknowledging.
* **Replayability** – New consumers can start from `earliest` offset to rebuild state.
* **Backpressure handling** – If the DB persistor slows down, Kafka absorbs the backlog instead of blocking the engine.

### Producer Design

We use `rdkafka`'s `BaseProducer` (not `FutureProducer`) with a dedicated thread:

```rust
// Dedicated thread runs producer.poll() loop
crossbeam_channel::bounded(100_000)  // backpressure channel
```

This minimizes latency – the engine thread only does `try_send()` on a lock-free channel. The producer thread handles the actual network I/O and delivery callbacks.

### Why Not a Simpler Queue?

Options like Redis Streams or RabbitMQ:
* Redis Streams is simpler but requires running Redis as additional infrastructure and lacks Kafka's replication model.
* RabbitMQ is great for task queues but less optimized for high-throughput log-style streaming.

Kafka's log-centric design matches our use case perfectly: append-only, partitionable, high-throughput event streams.

---

## Rust Ecosystem

### `tokio` – Async Runtime

Tokio 1.40 provides the async runtime for gRPC (Tonic), HTTP (Actix-web), and DB (sqlx). We use the **multi-thread scheduler** for I/O-bound binaries (`matchengine`, `restapi`) but the **matching logic itself is synchronous** – it runs inside a single task on the Controller thread.

### `sqlx` – Compile-Time Checked SQL

sqlx verifies SQL queries against a live database at compile time (`cargo sqlx prepare` or `DATABASE_URL` env). This catches schema mismatches early without a heavy ORM. It also supports connection pooling, migrations, and `rust_decimal` types out of the box.

### `rust_decimal` – Financial Precision

Floating-point arithmetic is unacceptable for financial systems. `rust_decimal` provides 128-bit fixed-point decimal arithmetic with configurable precision. All prices, amounts, and balances use `Decimal` throughout the codebase.

### `crossbeam-channel` – Lock-Free Queues

Used for the Kafka producer channel and `FastOperationLogWriter`. Crossbeam channels are faster than Tokio's async channels for thread-to-thread communication where the receiver runs on a dedicated OS thread (not a Tokio task).

### `actix-web` + `paperclip` – REST API

Actix-web 4 is one of the fastest HTTP frameworks available. Paperclip generates OpenAPI specs from handler attributes (`#[api_v2_operation]`), keeping documentation in sync with code.

### `rdkafka` – Kafka Client

Bindings to the mature C library `librdkafka`. Provides both producer and consumer APIs. The `cmake-build` feature compiles librdkafka from source, reducing system dependency issues.

### `tonic-build` – Proto Code Generation

Compiles `.proto` files to Rust during `cargo build`. No manual code generation step required.

---

## Summary Table

| Technology | Role | Key Alternative | Why Chosen |
|------------|------|-----------------|------------|
| PostgreSQL + TimescaleDB | Primary database | MySQL, ClickHouse | ACID + time-series in one system |
| gRPC + Tonic | Trading API | REST/JSON, WebSocket | Binary efficiency, HTTP/2 multiplexing |
| Apache Kafka | Event bus | Redis Streams, RabbitMQ | Durability, replayability, decoupling |
| Tokio | Async runtime | async-std | Ecosystem maturity, Tonic/Actix integration |
| sqlx | Database access | Diesel, SeaORM | Compile-time SQL checking, no ORM overhead |
| rust_decimal | Numeric type | f64, bigdecimal | Fixed-point precision, zero-allocation ops |
| crossbeam-channel | Inter-thread queues | Tokio mpsc | Faster for OS-thread receivers |
| Actix-web + Paperclip | REST API | Axum, Rocket | Performance + auto OpenAPI gen |
| rdkafka | Kafka client | kafka-rust (pure Rust) | Mature C library, production-proven |
