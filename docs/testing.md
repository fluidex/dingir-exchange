# Testing

Dingir Exchange uses a three-tier testing strategy: unit tests, integration tests, and performance benchmarks.

---

## Unit Tests

Run with:

```bash
cargo test
```

Unit tests are embedded within source modules under `#[cfg(test)]`. Key test coverage:

### Order Book & Matching (`src/matchengine/market/mod.rs`)

* `test_multi_orders` – 100 random limit orders between two users, verifies balance consistency and order state.
* `test_market_taker_is_bid` – Market order matching against existing limit orders, verifying fee calculation and balance updates.
* Additional tests cover: partial fills, self-trade cancellation (when enabled), post-only rejection, precision validation.

### Balance Manager (`src/matchengine/asset/`)

* Tests for `add`, `sub`, `frozen`, `unfrozen` operations.
* Verifies that `available + freeze = total` invariant holds after sequences of operations.

### Database Writer (`src/storage/database.rs`)

* `ProgTracingStack` tests – verifies the progress tracking logic for concurrent batch INSERT coordination.

### SQL Helpers (`src/storage/sqlxextend.rs`)

* Tests for dynamic SQL generation (bind parameter count, table name resolution).

---

## Integration Tests

Located in `examples/js/`. These are TypeScript tests that exercise the full stack (matchengine + Kafka + DB) via gRPC and REST.

### Prerequisites

```bash
# Start all services
docker-compose --file "./orchestra/docker/docker-compose.yaml" up --detach
make startall

# Install JS dependencies
cd examples/js && npm install
```

### Key Test Scripts

| Script | What It Tests |
|--------|--------------|
| `tests/trade.ts` | End-to-end order put, matching, balance verification |
| `tests/benchmark.ts` | Throughput benchmark via gRPC |
| `tests/print_orders.ts` | Query and print active orders |
| `tests/stress.ts` | Stress test with concurrent clients |
| `tests/debug_reset.ts` | State recovery via `debug_reset` RPC |

### Running

```bash
cd examples/js
npx ts-node tests/trade.ts
```

---

## Performance Tests

The Rust-native performance tool (`src/bin/perftest.rs`) provides reproducible, low-overhead benchmarks.

### Running

```bash
cargo run --bin perftest -- [OPTIONS]
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `--endpoint` | gRPC endpoint | `http://127.0.0.1:50051` |
| `--mode` | Test mode: `order-put`, `pair-trade`, `batch`, `mixed` | `order-put` |
| `--workers` | Concurrent client workers | 4 |
| `--requests` | Total requests across all workers | 10,000 |
| `--duration-secs` | Test duration in seconds (overrides request count if reached first) | 10 |
| `--batch-size` | Orders per batch (batch mode only) | 20 |

### Modes

#### `order-put`

Each worker independently submits single `OrderPut` requests. Measures raw gRPC throughput of the matching engine.

#### `pair-trade`

Workers alternate bid/ask orders between two users to guarantee matching. Measures matching throughput (both order ingestion and execution).

> **Note on sustainable running:** The `pair-trade` mode periodically `cancelAll`s open orders (every 50 requests) and swaps the buy/sell roles of the two users every 50 requests. This prevents one user from running out of balance during long-duration tests. Use `--duration-secs` for sustained load tests.

#### `batch`

Workers submit `BatchOrderPut` with configurable batch size. This is the most efficient mode for bulk ingestion.

#### `mixed`

Combines single orders and batches to simulate realistic traffic patterns.

### Benchmark Results

Environment: Apple Silicon (M-series), local PostgreSQL 17, Kafka via Docker Compose.

| Mode | Workers | Requests/s | Success Rate | Effective Orders/s |
|------|---------|-----------|--------------|-------------------|
| `order-put` | 4 | ~40,000 | ~97.6% | ~40,000 |
| `pair-trade` | 4 | ~35,000 | ~100% | ~35,000 |
| `batch` (size=20) | 4 | ~200,000 batches/s | ~100% | ~4,000,000 |
| `batch` (size=40) | 4 | ~120,000 batches/s | ~100% | ~4,800,000 |

*Note: Higher batch sizes amortize gRPC round-trip overhead but increase per-RPC processing time.*

### What the Numbers Mean

* **order-put ~40K req/s** – The Controller dispatch queue (1,024) and HTTP/2 stream limit (10,000) are no longer bottlenecks. The limiting factor is now the single-threaded matching loop (BTreeMap iteration + balance updates).
* **batch ~4M orders/s** – At batch size 20, the per-request overhead dominates. The matching loop processes 20 orders in roughly the same wall time as 1 single order.
* **~97.6% success in order-put** – The 2.4% failures are primarily Kafka producer backpressure (`is_block`) under sustained burst. In batch mode, the producer queue amortizes better, achieving 100% success.

### Interpreting Results

```
Successful: 9760/10000 (97.60%)
Failed: 240/10000 (2.40%)
Throughput: ~40000.00 req/s
```

* **Failures** – Usually `Status::unknown("Server temporary unavaliable")` when the `task_dispatcher` channel is full, or Kafka producer backpressure. Not order validation failures.
* **Throughput** – Calculated as `total_requests / elapsed_time`. For batch mode, this is batches per second; multiply by batch size for effective orders/s.

### Client Performance Note

The JavaScript/TypeScript test scripts in `examples/js/` (e.g. `stress.ts`) are convenient for integration testing but **should not be used for performance benchmarking**. The Node.js `grpc-caller` client is the bottleneck, not the engine:

| Client | Mode | Throughput | Notes |
|--------|------|-----------|-------|
| Node.js (`ts-node stress.ts`) | dual-user | ~31 req/s | CPU-bound by JS event loop + promisify overhead |
| Node.js (compiled `dist/stress.js`) | dual-user | ~30 req/s | Same bottleneck; ts-node compilation is not the issue |
| Rust (`perftest`) | `order-put` | ~2,500 req/s | Single-worker, single-order gRPC |
| Rust (`perftest`) | `pair-trade` | ~3,200 req/s | 4 workers, dual-user matching |
| Rust (`perftest`) | `batch` | ~10,000 req/s | Single-worker, 20 orders/batch |

**Always use the Rust `perftest` binary for accurate throughput measurements.**

### Optimization History

Key changes that improved performance from ~2K req/s (initial JS benchmark) to ~40K req/s:

| Change | Before | After | Impact |
|--------|--------|-------|--------|
| gRPC `max_concurrent_streams` | default (100) | 10,000 | Eliminated HTTP/2 stream exhaustion |
| `task_dispatcher` channel | 16 | 1,024 | Eliminated Controller dispatch queuing |
| `DatabaseWriter` batch size | 5,000 | 20,000 | Reduced DB round-trips |
| `DatabaseWriter` channel | 1,000 | 10,000 | Reduced backpressure on operation log |
| `DatabaseWriter` spawn limit | 4 | 8 | More concurrent INSERT tasks |
| Kafka producer channel | 2,048 | 100,000 | Absorbed burst without blocking engine |
| Kafka backpressure threshold | 1,000 | 10,000 | Less aggressive blocking |
| Operation log | DB INSERT | `FastOperationLogWriter` (file) | ~2× throughput for log-heavy workloads |
| SQL generation | `format!` + `fold` | Pre-allocated `String` | Reduced allocation overhead |

---

## Continuous Integration

The project uses GitHub Actions (`.github/workflows/`). CI runs:

1. `cargo test` – unit tests
2. `cargo clippy` – linting
3. `cargo fmt --check` – formatting
4. Proto compilation verification

Ensure `cmake` and `librdkafka` are installed in the CI environment (see `.github/workflows/` for platform-specific setup).
