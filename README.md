# Dingir Exchange
Dingir Exchange is a high performance exchange trading server.   
The core matching engine is a fully async, single threaded, memory based matching engine with thousands of TPS. 

* Features: order matching, order state change notification, user balance management, market data...   
* Non Features: user account system, cryptocurrency deposit/withdraw...

## Technical Details

* Language: Rust
* API Interface: GRPC
* Server framework: Tokio/Hyper/Tonic
* Storage: SQL Databases
* Persistence: (a)Append operation log and (b)Redis-like fork-and-save persistence

The archtecture is heavily inspired by Redis and [Viabtc Exchange](https://github.com/viabtc/viabtc_exchange_server)

## Todos

* push notifications using GRPC/websockets
* API Documentation
* Better test coverage

## Example

```
# Simple test
$ cd $DingirExchangeDir
$ cd docker
$ docker-compose up # Lanuch the external dependency services like Postgresql and Kafka
$ cd $DingirExchangeDir
$ cargo run --bin matchengine
$ cd $DingirExchangeDir/examples/js
$ node trade.js # This script will put orders into the exchange. Then you can see some others got matched, deals(trades) are generated, and users' balances are changed accordingly. 
```

## Related Projects

[Peatio](https://github.com/openware/peatio): A full-featured crypto exchange backend, with user account system and crypto deposit/withdraw. Written in Ruby/Rails. It can process less than 200 orders per second.  

[viabtc exchange server](https://github.com/viabtc/viabtc_exchange_server): A high performance trading server written in C/libev. Most components of the project are written from scratch including network, RPC. It can process thousands of orders per second.
