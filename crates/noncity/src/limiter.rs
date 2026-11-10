use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::Instant;

struct ClientBucket {
    tokens: f64,
    last_update: Instant,
    active_connections: usize,
}

pub struct RateLimiter {
    clients: Mutex<HashMap<IpAddr, ClientBucket>>,
    burst_capacity: f64,
    refill_rate_per_sec: f64,
    max_active_connections: usize,
}

impl RateLimiter {
    pub fn new(burst_capacity: f64, refill_rate_per_sec: f64, max_active_connections: usize) -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
            burst_capacity,
            refill_rate_per_sec,
            max_active_connections,
        }
    }

    pub fn acquire(&self, ip: IpAddr) -> bool {
        let mut map = self.clients.lock().unwrap();
        let now = Instant::now();

        let bucket = map.entry(ip).or_insert_with(|| ClientBucket {
            tokens: self.burst_capacity,
            last_update: now,
            active_connections: 0,
        });

        if bucket.active_connections >= self.max_active_connections {
            return false;
        }

        let elapsed = now.duration_since(bucket.last_update).as_secs_f64();
        bucket.last_update = now;
        bucket.tokens = (bucket.tokens + elapsed * self.refill_rate_per_sec).min(self.burst_capacity);

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            bucket.active_connections += 1;
            true
        } else {
            false
        }
    }

    pub fn release(&self, ip: IpAddr) {
        let mut map = self.clients.lock().unwrap();
        if let Some(bucket) = map.get_mut(&ip) {
            if bucket.active_connections > 0 {
                bucket.active_connections -= 1;
            }
        }
    }
}
