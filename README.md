# Dingir Exchange

Dingir Exchange is a high-performance cryptocurrency exchange trading server written in Rust.

The core matching engine is a **fully async, single-threaded, in-memory order book** capable of processing tens of thousands of orders per second. Its architecture is heavily inspired by [Redis](https://redis.io/) and [Viabtc Exchange Server](https://github.com/viabtc/viabtc_exchange_server).

## Features

* **In-Memory Order Matching** – Single-threaded execution eliminates lock contention; order book is maintained in `BTreeMap` for price-time priority.
* **gRPC API** – High-performance binary RPC with `BatchOrderPut` supporting up to 40 orders per call.
* **REST API** – Query endpoints for balances, orders, trades, market depth, and TradingView-compatible K-line data.
* **Balance Management** – Per-user, per-asset balance tracking with available/frozen separation.
* **Real-Time Messaging** – Kafka-based event streaming (orders, trades, balances) for downstream consumers.
* **Durability** – Three-layer persistence: Kafka message stream, PostgreSQL/TimescaleDB historical storage, and operation-log replay for state recovery.
* **Performance Tested** – Single-order mode ~40K req/s; batch mode ~200K batches/s (~4M effective orders/s).

## Non-Features

* User account system (registration/authentication)
* Cryptocurrency deposit/withdraw on-chain logic
* WebSocket push notifications (planned)

## Architecture Overview

```
┌─────────────┐     gRPC      ┌─────────────────┐
│   Clients   │ ────────────> │  GrpcHandler    │
│  (JS/CLI)   │               │  (Tokio/Tonic)  │
└─────────────┘               └────────┬────────┘
                                       │
                    ┌──────────────────┼──────────────────┐
                    │ read: RwLock     │ write: mpsc      │
                    ▼                  ▼                  │
            ┌──────────────┐   ┌──────────────┐          │
            │   Controller │   │   Controller │          │
            │   (read ops) │   │ (single-thread)        │
            └──────────────┘   └──────┬───────┘          │
                                      │                  │
                        ┌─────────────┼─────────────┐    │
                        ▼             ▼             ▼    │
                  ┌─────────┐  ┌──────────┐  ┌─────────┐ │
                  │ Markets │  │ Balance  │  │  Users  │ │
                  │(BTreeMap)│  │ Manager  │  │ Manager │ │
                  └────┬────┘  └────┬─────┘  └────┬────┘ │
                       │            │             │      │
                       └────────────┴─────────────┘      │
                                    │                    │
                                    ▼                    │
                         ┌──────────────────┐            │
                         │    Persistor     │            │
                         │ (Kafka / DB /    │            │
                         │  Operation Log)  │            │
                         └──────────────────┘            │
                                                          │
┌─────────────┐     HTTP    ┌─────────────────┐          │
│   Clients   │ ──────────> │    REST API     │          │
│  (Frontends)│             │  (Actix-web)    │          │
└─────────────┘             └────────┬────────┘          │
                                     │                   │
                                     └───────────────────┘
```

## Quick Start

### Prerequisites

* Rust 1.90.0+ (edition 2024)
* cmake
* librdkafka
* Docker & Docker Compose

**macOS:**
```bash
brew install cmake librdkafka
```

**Ubuntu / Debian:**
```bash
apt install cmake librdkafka-dev
```

### Run the Services

```bash
# 1. Start PostgreSQL + Kafka
docker-compose --file "./orchestra/docker/docker-compose.yaml" up --detach

# 2. Run database migrations and start all services
make startall

# Or start individual binaries:
cargo run --bin matchengine   # gRPC matching engine on :50051
cargo run --bin restapi       # REST API on :50053
cargo run --bin persistor     # Kafka consumer → DB persistor
```

### Test with Examples

```bash
cd examples/js && npm install
npx ts-node tests/trade.ts
```

## Performance

Benchmarked on Apple Silicon (M-series) with local PostgreSQL and Kafka:

| Mode | Requests/s | Success Rate | Notes |
|------|-----------|--------------|-------|
| `order-put` | ~40,000 | ~97.6% | Single orders via gRPC |
| `batch` | ~200,000 batches/s | ~100% | 20 orders/batch = ~4M orders/s |
| `pair-trade` | ~35,000 | ~100% | Alternating bid/ask pairs |

See [`docs/testing.md`](docs/testing.md) for detailed benchmark methodology.

## Build & Release

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Static Linux build via cross
RUSTFLAGS="-C link-arg=-static -C target-feature=+crt-static" \
  cross build --bin matchengine --target x86_64-unknown-linux-gnu --release

# Docker image
./release/release.sh YOUR_REGISTRY:PORT TAG
```

## Project Structure

```
├── src/
│   ├── bin/
│   │   ├── matchengine.rs    # gRPC matching engine binary
│   │   ├── restapi.rs        # REST API server binary
│   │   ├── persistor.rs      # Kafka consumer → DB persistor
│   │   └── perftest.rs       # Rust native performance test tool
│   ├── matchengine/          # Core matching engine
│   │   ├── server.rs         # gRPC server / task dispatcher
│   │   ├── controller.rs     # Business logic orchestrator
│   │   ├── market/           # Order book & matching logic
│   │   ├── asset/            # Balance manager
│   │   └── persist/          # State save / load / migration
│   ├── message/              # Kafka producer / consumer
│   ├── storage/              # Database models & SQL helpers
│   ├── restapi/              # HTTP REST endpoints
│   └── types.rs              # Core type definitions
├── proto/                    # gRPC protobuf definitions
├── config/                   # YAML configuration files
├── migrations/               # SQL schema migrations
├── examples/js/              # JavaScript integration tests
└── docs/                     # Documentation
```

## Documentation

* [`docs/architecture.md`](docs/architecture.md) – System design & component deep-dives
* [`docs/interfaces.md`](docs/interfaces.md) – gRPC & REST API reference
* [`docs/dependencies.md`](docs/dependencies.md) – Why TimescaleDB, gRPC, Kafka, etc.
* [`docs/testing.md`](docs/testing.md) – Testing strategy & performance benchmarks
* [`docs/operations.md`](docs/operations.md) – Environment setup, configuration & deployment
* [`docs/development.md`](docs/development.md) – Contributing guide & troubleshooting
* [`AGENTS.md`](AGENTS.md) – Agent/developer quick-reference

## Related Projects

* [Viabtc Exchange Server](https://github.com/viabtc/viabtc_exchange_server) – C/libev trading server, thousands of orders/s.
* [Peatio](https://github.com/openware/peatio) – Full-featured Ruby/Rails exchange backend (< 200 orders/s).
