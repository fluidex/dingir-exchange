use anyhow::Result;
use clap::{Parser, ValueEnum};
use dingir_exchange::rpc::exchange::matchengine_client::MatchengineClient;
use dingir_exchange::rpc::exchange::*;
use rand::{Rng, SeedableRng};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Barrier;
use tonic::transport::Channel;

#[derive(Parser, Debug, Clone)]
#[command(name = "perftest")]
#[command(about = "Performance test tool for dingir-exchange")]
struct Args {
    #[arg(short, long, default_value = "http://127.0.0.1:50051")]
    endpoint: String,

    #[arg(short, long, default_value = "1")]
    workers: usize,

    #[arg(short, long, default_value = "10000")]
    requests: u64,

    #[arg(short, long, default_value = "10")]
    duration_secs: u64,

    #[arg(short, long, default_value = "order-put")]
    mode: TestMode,

    #[arg(short = 'k', long, default_value = "ETH_USDT")]
    market: String,

    #[arg(long, default_value = "1")]
    user_id: u32,

    #[arg(long, default_value = "10000000")]
    deposit_usdt: u64,

    #[arg(long, default_value = "10000")]
    deposit_eth: u64,

    #[arg(long, default_value = "20")]
    batch_size: usize,
}

#[derive(Debug, Clone, ValueEnum, PartialEq)]
enum TestMode {
    OrderPut,
    PairTrade,
    Batch,
    Mixed,
}

struct Stats {
    total_requests: AtomicU64,
    total_errors: AtomicU64,
    total_latency_ns: AtomicU64,
    last_requests: AtomicU64,
    last_errors: AtomicU64,
    last_latency_ns: AtomicU64,
    error_count: AtomicUsize,
}

impl Stats {
    fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            total_latency_ns: AtomicU64::new(0),
            last_requests: AtomicU64::new(0),
            last_errors: AtomicU64::new(0),
            last_latency_ns: AtomicU64::new(0),
            error_count: AtomicUsize::new(0),
        }
    }

    fn record(&self, success: bool, latency_ns: u64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ns.fetch_add(latency_ns, Ordering::Relaxed);
        self.last_requests.fetch_add(1, Ordering::Relaxed);
        self.last_latency_ns.fetch_add(latency_ns, Ordering::Relaxed);
        if !success {
            self.total_errors.fetch_add(1, Ordering::Relaxed);
            self.last_errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn snapshot_and_reset(&self) -> (u64, u64, u64) {
        let reqs = self.last_requests.swap(0, Ordering::Relaxed);
        let errs = self.last_errors.swap(0, Ordering::Relaxed);
        let lat = self.last_latency_ns.swap(0, Ordering::Relaxed);
        (reqs, errs, lat)
    }

    fn totals(&self) -> (u64, u64, u64) {
        (
            self.total_requests.load(Ordering::Relaxed),
            self.total_errors.load(Ordering::Relaxed),
            self.total_latency_ns.load(Ordering::Relaxed),
        )
    }
}

fn fmt_duration(ns: u64) -> String {
    if ns < 1_000 {
        format!("{}ns", ns)
    } else if ns < 1_000_000 {
        format!("{:.1}us", ns as f64 / 1_000.0)
    } else if ns < 1_000_000_000 {
        format!("{:.2}ms", ns as f64 / 1_000_000.0)
    } else {
        format!("{:.2}s", ns as f64 / 1_000_000_000.0)
    }
}

async fn print_stats(stats: Arc<Stats>, interval_secs: u64, stop: tokio::sync::watch::Receiver<bool>) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    interval.tick().await;
    println!(
        "{:>10} {:>12} {:>10} {:>12} {:>12} {:>10}",
        "elapsed", "reqs/s", "errors/s", "avg_lat", "total_reqs", "total_errs"
    );
    let start = Instant::now();
    loop {
        interval.tick().await;
        if *stop.borrow() {
            break;
        }
        let (reqs, errs, lat) = stats.snapshot_and_reset();
        let elapsed = start.elapsed().as_secs_f64();
        let avg_lat = if reqs > 0 { lat / reqs } else { 0 };
        let (total_reqs, total_errs, _) = stats.totals();
        println!(
            "{:>10.1}s {:>12.1} {:>10} {:>12} {:>12} {:>10}",
            elapsed,
            reqs as f64 / interval_secs as f64,
            errs,
            fmt_duration(avg_lat),
            total_reqs,
            total_errs,
        );
    }
}

async fn setup_user(client: &mut MatchengineClient<Channel>, user_id: u32, market: &str, usdt: u64, eth: u64) -> Result<()> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // Register user
    let _ = client
        .register_user(UserInfo {
            user_id,
            l1_address: format!("0x{:08x}", user_id),
            l2_pubkey: format!("0x{:08x}", user_id),
            log_metadata: None,
        })
        .await;

    // Deposit USDT
    client
        .balance_update(BalanceUpdateRequest {
            user_id,
            asset: "USDT".to_string(),
            business: "deposit".to_string(),
            business_id: ts,
            delta: usdt.to_string(),
            detail: "{\"key\":\"value\"}".to_string(),
            signature: None,
            log_metadata: None,
        })
        .await?;

    // Deposit ETH
    client
        .balance_update(BalanceUpdateRequest {
            user_id,
            asset: "ETH".to_string(),
            business: "deposit".to_string(),
            business_id: ts + 1,
            delta: eth.to_string(),
            detail: "{\"key\":\"value\"}".to_string(),
            signature: None,
            log_metadata: None,
        })
        .await?;

    // Cancel all existing orders
    let _ = client
        .order_cancel_all(OrderCancelAllRequest {
            user_id,
            market: market.to_string(),
        })
        .await;

    Ok(())
}

fn random_price<R: Rng>(rng: &mut R) -> String {
    let price = 1350.0 + rng.r#gen::<f64>() * 100.0;
    format!("{:.2}", price)
}

fn random_amount<R: Rng>(rng: &mut R) -> String {
    let amount = 0.5 + rng.r#gen::<f64>() * 1.0;
    format!("{:.4}", amount)
}

async fn run_order_put(mut client: MatchengineClient<Channel>, stats: Arc<Stats>, args: Args, barrier: Arc<Barrier>) {
    let market = args.market.clone();
    let user_id = args.user_id;
    let requests = args.requests / args.workers as u64;

    barrier.wait().await;

    let mut rng = rand::rngs::StdRng::from_entropy();
    for i in 0..requests {
        // Cancel all orders periodically to avoid hitting user_order_num_limit
        if i > 0 && i % 50 == 0 {
            let _ = client
                .order_cancel_all(OrderCancelAllRequest {
                    user_id,
                    market: market.clone(),
                })
                .await;
        }
        let side = if rng.r#gen_bool(0.5) {
            OrderSide::Ask as i32
        } else {
            OrderSide::Bid as i32
        };
        let req = OrderPutRequest {
            user_id,
            market: market.clone(),
            order_side: side,
            order_type: OrderType::Limit as i32,
            amount: random_amount(&mut rng),
            price: random_price(&mut rng),
            quote_limit: String::new(),
            taker_fee: "0".to_string(),
            maker_fee: "0".to_string(),
            post_only: false,
            signature: String::new(),
        };
        let start = Instant::now();
        let result = client.order_put(req).await;
        let latency = start.elapsed().as_nanos() as u64;
        if let Err(ref e) = result {
            let cnt = stats.error_count.fetch_add(1, Ordering::Relaxed);
            if cnt < 5 {
                eprintln!("order_put error: {:?}", e);
            }
        }
        stats.record(result.is_ok(), latency);
    }
}

async fn run_pair_trade(mut client: MatchengineClient<Channel>, stats: Arc<Stats>, args: Args, barrier: Arc<Barrier>) {
    let market = args.market.clone();
    let user_id_ask = args.user_id;
    let user_id_bid = args.user_id + 1;
    let requests = args.requests / args.workers as u64;

    barrier.wait().await;

    for i in 0..requests {
        // Alternate between ask and bid
        let (user_id, side) = if i % 2 == 0 {
            (user_id_ask, OrderSide::Ask as i32)
        } else {
            (user_id_bid, OrderSide::Bid as i32)
        };
        let price = "1400.00".to_string();
        let amount = "0.1000".to_string();
        let req = OrderPutRequest {
            user_id,
            market: market.clone(),
            order_side: side,
            order_type: OrderType::Limit as i32,
            amount,
            price,
            quote_limit: String::new(),
            taker_fee: "0".to_string(),
            maker_fee: "0".to_string(),
            post_only: false,
            signature: String::new(),
        };
        let start = Instant::now();
        let result = client.order_put(req).await;
        let latency = start.elapsed().as_nanos() as u64;
        if let Err(ref e) = result {
            let cnt = stats.error_count.fetch_add(1, Ordering::Relaxed);
            if cnt < 3 {
                eprintln!("pair_trade error: {:?}", e);
            }
        }
        stats.record(result.is_ok(), latency);
    }
}

async fn run_batch(mut client: MatchengineClient<Channel>, stats: Arc<Stats>, args: Args, barrier: Arc<Barrier>) {
    let market = args.market.clone();
    let user_id = args.user_id;
    let requests = args.requests / args.workers as u64;
    let batch_size = args.batch_size;

    barrier.wait().await;

    let mut rng = rand::rngs::StdRng::from_entropy();
    for _ in 0..requests {
        let mut orders = Vec::with_capacity(batch_size);
        for _ in 0..batch_size {
            let side = if rng.r#gen_bool(0.5) {
                OrderSide::Ask as i32
            } else {
                OrderSide::Bid as i32
            };
            orders.push(OrderPutRequest {
                user_id,
                market: market.clone(),
                order_side: side,
                order_type: OrderType::Limit as i32,
                amount: random_amount(&mut rng),
                price: random_price(&mut rng),
                quote_limit: String::new(),
                taker_fee: "0".to_string(),
                maker_fee: "0".to_string(),
                post_only: false,
                signature: String::new(),
            });
        }
        let req = BatchOrderPutRequest {
            market: market.clone(),
            reset: false,
            orders,
        };
        let start = Instant::now();
        let result = client.batch_order_put(req).await;
        let latency = start.elapsed().as_nanos() as u64;
        // Count each order in batch as a request for throughput comparison
        for _ in 0..batch_size {
            stats.record(result.is_ok(), latency / batch_size as u64);
        }
    }
}

async fn run_mixed(mut client: MatchengineClient<Channel>, stats: Arc<Stats>, args: Args, barrier: Arc<Barrier>) {
    let market = args.market.clone();
    let user_id = args.user_id;
    let requests = args.requests / args.workers as u64;

    barrier.wait().await;

    let mut rng = rand::rngs::StdRng::from_entropy();
    for _ in 0..requests {
        let action: f64 = rng.r#gen();
        let start = Instant::now();
        let result = if action < 0.7 {
            // 70% order put
            let side = if rng.r#gen_bool(0.5) {
                OrderSide::Ask as i32
            } else {
                OrderSide::Bid as i32
            };
            client
                .order_put(OrderPutRequest {
                    user_id,
                    market: market.clone(),
                    order_side: side,
                    order_type: OrderType::Limit as i32,
                    amount: random_amount(&mut rng),
                    price: random_price(&mut rng),
                    quote_limit: String::new(),
                    taker_fee: "0".to_string(),
                    maker_fee: "0".to_string(),
                    post_only: false,
                    signature: String::new(),
                })
                .await
                .map(|_| ())
        } else if action < 0.85 {
            // 15% balance query
            client
                .balance_query(BalanceQueryRequest { user_id, assets: vec![] })
                .await
                .map(|_| ())
        } else {
            // 15% market summary
            client
                .market_summary(MarketSummaryRequest {
                    markets: vec![market.clone()],
                })
                .await
                .map(|_| ())
        };
        let latency = start.elapsed().as_nanos() as u64;
        stats.record(result.is_ok(), latency);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("=== Dingir Exchange Performance Test ===");
    println!("Endpoint:     {}", args.endpoint);
    println!("Mode:         {:?}", args.mode);
    println!("Workers:      {}", args.workers);
    println!("Total reqs:   {}", args.requests);
    println!("Duration:     {}s", args.duration_secs);
    println!("Market:       {}", args.market);
    println!("User ID:      {}", args.user_id);
    println!();

    // Setup phase
    let channel = Channel::from_shared(args.endpoint.clone())?.connect().await?;
    let mut setup_client = MatchengineClient::new(channel.clone());

    println!("Resetting state...");
    let _ = setup_client.debug_reset(DebugResetRequest {}).await;

    // Setup primary user
    println!("Setting up user {}...", args.user_id);
    setup_user(&mut setup_client, args.user_id, &args.market, args.deposit_usdt, args.deposit_eth).await?;

    // For pair_trade mode, setup second user
    if args.mode == TestMode::PairTrade {
        println!("Setting up user {}...", args.user_id + 1);
        setup_user(
            &mut setup_client,
            args.user_id + 1,
            &args.market,
            args.deposit_usdt,
            args.deposit_eth,
        )
        .await?;
    }

    println!("Setup complete. Starting load test...\n");

    let test_start = Instant::now();
    let stats = Arc::new(Stats::new());
    let barrier = Arc::new(Barrier::new(args.workers));
    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);

    // Start stats printer
    let stats_clone = stats.clone();
    let stats_handle = tokio::spawn(async move {
        print_stats(stats_clone, 1, stop_rx).await;
    });

    // Spawn workers - each with an independent gRPC connection to bypass HTTP/2 stream limits
    let mut handles = Vec::new();
    for _ in 0..args.workers {
        let stats = stats.clone();
        let barrier = barrier.clone();
        let args = args.clone();
        let endpoint = args.endpoint.clone();
        let handle = tokio::spawn(async move {
            let ch = Channel::from_shared(endpoint).unwrap().connect().await.unwrap();
            let client = MatchengineClient::new(ch);
            match args.mode {
                TestMode::OrderPut => run_order_put(client, stats, args, barrier).await,
                TestMode::PairTrade => run_pair_trade(client, stats, args, barrier).await,
                TestMode::Batch => run_batch(client, stats, args, barrier).await,
                TestMode::Mixed => run_mixed(client, stats, args, barrier).await,
            }
        });
        handles.push(handle);
    }

    // Wait for completion or timeout
    let duration = Duration::from_secs(args.duration_secs);
    tokio::select! {
        _ = futures::future::join_all(handles) => {},
        _ = tokio::time::sleep(duration) => {
            println!("\nDuration limit reached.");
        }
    }

    // Final stats
    let _ = stop_tx.send(true);
    let _ = stats_handle.await;

    let (total_reqs, total_errs, total_lat) = stats.totals();
    let elapsed = test_start.elapsed().as_secs_f64();
    let avg_lat = if total_reqs > 0 { total_lat / total_reqs } else { 0 };

    println!("\n=== Final Results ===");
    println!("Total requests: {}", total_reqs);
    println!("Total errors:   {}", total_errs);
    println!(
        "Success rate:   {:.2}%",
        (total_reqs - total_errs) as f64 * 100.0 / total_reqs.max(1) as f64
    );
    println!("Actual elapsed: {:.3}s", elapsed);
    println!("Avg throughput: {:.1} req/s", total_reqs as f64 / elapsed);
    println!("Avg latency:    {}", fmt_duration(avg_lat));

    Ok(())
}
