# Operations

## Environment Setup

### System Requirements

| Component | Version | Purpose |
|-----------|---------|---------|
| Rust | 1.90.0+ | Compiler (edition 2024) |
| cmake | 3.x | Build librdkafka via rdkafka/cmake-build |
| librdkafka | any | Kafka C client library |
| PostgreSQL | 14+ | Primary database |
| TimescaleDB | 2.x+ | Time-series extension |
| Kafka | 3.x+ | Message bus |
| Docker | 20+ | Service orchestration |
| Docker Compose | 2+ | Multi-service local dev |

### macOS

```bash
# Install build tools
brew install cmake librdkafka

# Start services via Docker Compose
docker-compose --file "./orchestra/docker/docker-compose.yaml" up --detach
```

### Ubuntu / Debian

```bash
# Install build tools
apt update
apt install -y cmake librdkafka-dev pkg-config libssl-dev

# Start services via Docker Compose
docker-compose --file "./orchestra/docker/docker-compose.yaml" up --detach
```

### Docker Compose Services

The compose file (`orchestra/docker/docker-compose.yaml`) typically provides:

* **PostgreSQL** (port 5432) – with TimescaleDB extension
* **Kafka** (port 9092) – single-node or KRaft mode
* **ZooKeeper** – if using legacy Kafka mode

Verify services are running:

```bash
docker-compose ps
pg_isready -h 127.0.0.1 -p 5432
kafka-broker-api-versions --bootstrap-server 127.0.0.1:9092
```

---

## Configuration

Configuration is loaded from YAML files in `config/`, merged in order: `default.yaml` → `development.yaml` → environment-specific override.

### `config/default.yaml`

```yaml
slice_interval: 3600        # Snapshot interval in seconds
slice_keeptime: 259200      # Snapshot retention in seconds
disable_self_trade: true    # Reject orders that would self-trade
disable_market_order: true  # Reject market orders
check_eddsa_signatue: auto  # Signature verification mode
user_order_num_limit: 2000  # Max open orders per user
```

### `config/development.yaml`

```yaml
debug: true
db_log: postgres://exchange:exchange_AA9944@127.0.0.1/exchange
db_history: postgres://exchange:exchange_AA9944@127.0.0.1/exchange
brokers: '127.0.0.1:9092'
```

| Key | Description |
|-----|-------------|
| `db_log` | Connection string for operation log and state database |
| `db_history` | Connection string for historical data (can be same as `db_log`) |
| `brokers` | Kafka bootstrap servers |
| `debug` | Enable debug-level logging and debug RPCs |

### Environment Variables

Settings can be overridden via environment variables (using `.env` file or shell exports):

```bash
DATABASE_URL=postgres://user:pass@host/db
KAFKA_BROKER=127.0.0.1:9092
RUST_LOG=debug
```

`dotenv` is loaded at startup in each binary.

### REST API Config

`src/restapi/config.rs` loads additional settings:

* `manage_endpoint` – gRPC endpoint of matchengine for management proxying (e.g. `"http://127.0.0.1:50051"`). If unset, management endpoints return 403.
* `workers` – Number of Actix-web worker threads. If unset, uses CPU count.

---

## Building

```bash
# Full debug build (includes proto compilation)
cargo build

# Release build (recommended for production)
cargo build --release

# Build specific binary
cargo build --bin matchengine
cargo build --bin restapi
cargo build --bin persistor
cargo build --bin perftest

# Run with logging
RUST_LOG=info cargo run --bin matchengine
```

### Proto Compilation

`.proto` files are compiled automatically by `tonic-build` in `build.rs`. Ensure `protoc` is in your `PATH`. If you see errors like `"protoc" not found`, install it:

```bash
# macOS
brew install protobuf

# Ubuntu
apt install -y protobuf-compiler
```

---

## Running

### Option 1: Make (Recommended for Development)

```bash
make startall   # Starts matchengine, restapi, persistor
make stopall    # Stops all services
```

### Option 2: Manual

Terminal 1 – Matching Engine:
```bash
cargo run --bin matchengine
```

Terminal 2 – REST API:
```bash
cargo run --bin restapi
```

Terminal 3 – Persistor (Kafka → DB):
```bash
cargo run --bin persistor
```

### Port Map

| Service | Port | Protocol | Purpose |
|---------|------|----------|---------|
| matchengine | 50051 | gRPC | Trading API |
| restapi | 50053 | HTTP | Query & management API |
| PostgreSQL | 5432 | PostgreSQL | Primary database |
| Kafka | 9092 | Kafka | Message bus |

---

## Database Migrations

Migrations are managed by `sqlx migrate` and stored in `migrations/`:

```bash
# Run migrations (happens automatically on matchengine startup)
sqlx migrate run --database-url postgres://exchange:exchange_AA9944@127.0.0.1/exchange

# Add a new migration
sqlx migrate add <name>
```

The `matchengine` binary runs migrations automatically via `persist::MIGRATOR.run()` during startup.

---

## Deployment

### Docker

A `release/Dockerfile` is provided for containerized deployment:

```bash
# Build image
./release/release.sh YOUR_REGISTRY:PORT TAG

# Or manually
docker build -f release/Dockerfile -t dingir-exchange:latest .
```

### Static Linux Build

For minimal deployment images, build a fully static binary:

```bash
RUSTFLAGS="-C link-arg=-static -C target-feature=+crt-static" \
  cross build --bin matchengine --target x86_64-unknown-linux-gnu --release
```

This requires [cross](https://github.com/rust-embedded/cross):

```bash
cargo install cross
```

### Production Checklist

* [ ] Use `release` profile (`cargo build --release`)
* [ ] Set `debug: false` in config
* [ ] Configure proper `db_log` / `db_history` connection strings with connection pooling
* [ ] Set up Kafka with replication factor ≥ 3
* [ ] Enable PostgreSQL WAL archiving and point-in-time recovery
* [ ] Monitor `operation_log` growth – implement retention policy
* [ ] Set up log aggregation (the project uses `tracing` + `tracing-subscriber`)
* [ ] Configure `RUST_LOG=info` or `warn` (avoid `debug` in production)
* [ ] Ensure `disable_self_trade` and `check_eddsa_signatue` settings match business requirements
* [ ] Set appropriate `user_order_num_limit` to prevent order book spam

---

## Monitoring & Logging

The project uses `tracing` for structured logging:

```bash
# JSON logs (useful for log aggregation)
RUST_LOG=info cargo run --bin matchengine

# Pretty-printed logs (development)
RUST_LOG=debug cargo run --bin matchengine
```

Key metrics to monitor:

* `matchengine` gRPC request latency and error rate
* `task_dispatcher` channel saturation (if growing, Controller is bottlenecked)
* Kafka producer queue depth (`sender.len()`)
* `DatabaseWriter` pending count and batch flush latency
* PostgreSQL connection pool utilization
* `persistor` consumer lag (Kafka consumer group offset lag)
