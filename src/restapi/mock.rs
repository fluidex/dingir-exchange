use super::types::{KlineReq, KlineResult};
use core::cmp::max;
use rand::Rng;

pub fn fake_kline_result(req: &KlineReq) -> KlineResult {
    let mut r = KlineResult::default();
    let from = req.from / req.resolution * req.resolution;
    let to = req.to / req.resolution * req.resolution;
    let resolution = req.resolution * 60;
    let kline_result_limit = 10;
    let mut t = max(from, to - kline_result_limit * resolution);

    r.s = "ok".to_owned();
    let mut rnd = rand::thread_rng();
    while t <= to {
        let mut fake_prices = Vec::new();
        for _ in 0..4 {
            fake_prices.push(rnd.gen_range(5.0..100.0));
        }
        fake_prices.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

        r.t.push(t);
        if rand::random::<bool>() {
            r.o.push(fake_prices[1]);
            r.c.push(fake_prices[2]);
        } else {
            r.o.push(fake_prices[2]);
            r.c.push(fake_prices[1]);
        }
        r.h.push(fake_prices[3]);
        r.l.push(fake_prices[0]);
        r.v.push(rnd.gen_range(1.0..3.0));

        t += resolution;
    }
    r
}
