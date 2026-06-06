# Development Guide

## Contributing

### Getting Started

1. **Fork and clone** the repository.
2. **Install prerequisites** – see [`operations.md`](operations.md) for environment setup.
3. **Start services** – `docker-compose --file "./orchestra/docker/docker-compose.yaml" up --detach`
4. **Run tests** – `cargo test`
5. **Make changes** – Follow the conventions below.
6. **Run integration tests** – `cd examples/js && npx ts-node tests/trade.ts`
7. **Format and lint** – `cargo fmt && cargo clippy`
8. **Submit PR** – Ensure CI passes.

### Code Style

* **Formatting** – Use `rustfmt` (`cargo fmt`). Configuration is in `rustfmt.toml`.
* **Linting** – Keep `cargo clippy` clean. The project allows some clippy lints at the crate level (`#![allow(clippy::...)]`) for ergonomic reasons; don't add new ones without justification.
* **Naming** – Follow Rust conventions (`snake_case` for functions/variables, `PascalCase` for types, `SCREAMING_SNAKE_CASE` for constants).
* **Error Handling** – Use `anyhow::Result` for application code, `thiserror` for library errors. Avoid `unwrap()` in production paths; use `?` or explicit error handling.
* **Decimal Arithmetic** – Never use `f64` for prices or amounts. Always use `rust_decimal::Decimal`.

### Adding a New gRPC Method

1. Update `proto/exchange/matchengine.proto`.
2. Run `cargo build` to regenerate Rust code.
3. Add the method to `Controller` (`src/matchengine/controller.rs`).
4. Wire it in `GrpcHandler` (`src/matchengine/server.rs`):
   * **Read** → `stub.read().await`
   * **Write** → `ControllerDispatch::new(...)` + `task_dispatcher.send(...)`
5. Add unit tests in the relevant module.
6. If the method produces new events, update `MessageManager` in `src/message/`.

### Adding a New Market Config Field

1. Add the field to `config::Market` in `src/config.rs`.
2. Update `Market::new()` in `src/matchengine/market/mod.rs` to use it.
3. Add the column to `market` table migration if it needs to be persisted.
4. Update `persist::init_config_from_db` if loaded from DB.

### Database Schema Changes

1. Create a new migration: `sqlx migrate add <descriptive_name>`
2. Write the `UP` and `DOWN` SQL in `migrations/<timestamp>_<name>.sql`.
3. Run `sqlx migrate run` to apply locally.
4. Update the corresponding Rust model in `src/storage/models.rs`.
5. Update `sqlxextend` impls if the table is used with `DatabaseWriter`.
6. Re-run `cargo test` to ensure sqlx compile-time checks pass.

### Commit Messages

Follow conventional commits style:

```
feat: add batch order cancellation
fix: handle zero-amount orders gracefully
docs: update API examples
perf: reduce operation log allocations
refactor: simplify balance update controller
```

---

## Troubleshooting

### Build Errors

#### `protoc` not found

```
error: failed to run custom build command for `dingir-exchange`
```

**Fix:** Install Protocol Buffers compiler:
```bash
# macOS
brew install protobuf

# Ubuntu
apt install -y protobuf-compiler
```

#### `librdkafka` linking errors

```
ld: library not found for -lrdkafka
```

**Fix:** Install librdkafka:
```bash
# macOS
brew install cmake librdkafka

# Ubuntu
apt install -y cmake librdkafka-dev pkg-config
```

On macOS, you may need to set:
```bash
export PKG_CONFIG_PATH="/opt/homebrew/lib/pkgconfig"
```

#### `sqlx` compile errors

```
error: error communicating with database: connection refused
```

**Fix:** sqlx requires a database for compile-time query checking. Either:
1. Start PostgreSQL and set `DATABASE_URL`:
   ```bash
   export DATABASE_URL=postgres://exchange:exchange_AA9944@127.0.0.1/exchange
   ```
2. Or run `cargo sqlx prepare` to generate offline query data.

### Runtime Errors

#### `cannot connect to db`

**Causes:**
* PostgreSQL not running
* Wrong connection string in `config/development.yaml`
* Database/user does not exist

**Fix:**
```bash
# Verify PostgreSQL is running
pg_isready -h 127.0.0.1 -p 5432

# Create database and user if needed
psql -h 127.0.0.1 -U postgres
CREATE USER exchange WITH PASSWORD 'exchange_AA9944';
CREATE DATABASE exchange OWNER exchange;
```

#### Kafka producer queue full / high failure rate in perftest

**Symptoms:** `perftest` shows < 100% success rate, logs show Kafka backpressure.

**Causes:**
* Kafka broker not running or unreachable
* Producer channel saturated (too many orders, too few consumers)

**Fix:**
1. Verify Kafka: `kafka-broker-api-versions --bootstrap-server 127.0.0.1:9092`
2. Check `persistor` binary is running (it consumes Kafka topics).
3. If testing without persistor, Kafka will accumulate messages and eventually backpressure. For pure engine testing, consider using `DummyPersistor` or `FileBasedPersistor`.
4. Increase Kafka consumer throughput or add more `persistor` instances.

#### `Server temporary unavaliable` in gRPC responses

**Cause:** The `task_dispatcher` channel (Controller dispatch queue) is full.

**Fix:**
* Increase channel size in `server.rs` (already set to 1,024; increase further if needed).
* Reduce client concurrency.
* Check if the Controller thread is bottlenecked (single-threaded matching has inherent limits).

#### Slow operation log INSERTs

**Symptoms:** High latency on write operations, `DatabaseWriter` backpressure.

**Fix:**
1. Use `FastOperationLogWriter` (file-based) instead of DB INSERT for operation logs:
   ```rust
   // In controller initialization
   let log_writer = FastOperationLogWriter::new("/var/log/dingir/operation.log");
   ```
2. Ensure `operation_log` table is `UNLOGGED` (skips WAL).
3. Increase `DatabaseWriter` batch size and spawn limit.

### Debug Tools

#### `debug_dump`

Dumps the current in-memory state (markets, balances, orders) to the database. Available only in debug builds.

```bash
grpcurl -plaintext -d '{}' localhost:50051 matchengine.Matchengine/DebugDump
```

#### `debug_reset`

**Warning:** Destructive. Clears all in-memory state and replays from `operation_log`.

Useful for verifying operation log completeness and recovery logic.

```bash
grpcurl -plaintext -d '{"hard": true}' localhost:50051 matchengine.Matchengine/DebugReset
```

#### `debug_reload`

Reloads state from the latest DB snapshot (`order_slice`, `balance_slice`). Faster than full operation-log replay.

### Performance Debugging

If you suspect a performance regression:

1. **Run `perftest` with `--mode order-put`** to establish baseline.
2. **Check logs** for `is_block()` triggering on Kafka producer or DatabaseWriter.
3. **Profile with `cargo flamegraph`**:
   ```bash
   cargo install flamegraph
   CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph --bin perftest -- --mode order-put
   ```
4. **Monitor channel depths** – Add temporary `log::info!` for `sender.len()` if backpressure is suspected.

### Common Configuration Mistakes

| Mistake | Symptom | Fix |
|---------|---------|-----|
| `db_log` == `db_history` but different DBs expected | Data inconsistency | Use same DB for simplicity, or separate with clear documentation |
| `manage_endpoint` not set in REST API | Management endpoints return 403 | Add `manage_endpoint: "http://127.0.0.1:50051"` to REST config |
| `disable_market_order: true` | All market orders rejected | Set to `false` if market orders are required |
| `check_eddsa_signatue: needed` without user registration | All orders rejected with invalid signature | Register users first, or set to `auto`/`none` |
| `user_order_num_limit: 2000` | Orders rejected after 2,000 open orders per user | Increase limit or implement order cancellation policy |
