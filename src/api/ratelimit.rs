use std::{collections::BTreeMap, sync::Mutex};

use chrono::Utc;

#[derive(Default, Debug)]
pub struct RateLimits {
    global: Mutex<RateLimit>,
    endpoints: Mutex<BTreeMap<String, RateLimit>>,
}

impl RateLimits {
    pub fn pre_check(&self, url: &str) {
        self.global.lock().expect("Ratelimits poisoned").pre_check();
        if let Some(ratelimit) = self
            .endpoints
            .lock()
            .expect("Ratelimits poisoned")
            .get_mut(url)
        {
            ratelimit.pre_check();
        }
    }

    pub fn check_for_ratelimit(&self, url: &str, response: &ureq::Response) -> bool {
        if response.header("X-RateLimit-Global").is_some() {
            self.global
                .lock()
                .expect("Ratelimits poisoned")
                .check_for_ratelimit(response)
        } else {
            self.endpoints
                .lock()
                .expect("Ratelimits poisoned")
                .entry(url.to_owned())
                .or_insert_with(RateLimit::default)
                .check_for_ratelimit(response)
        }
    }
}

#[derive(Default, Debug)]
struct RateLimit {
    reset: isize,
    limit: isize,
    remaining: isize,
}

impl RateLimit {
    fn pre_check(&mut self) {
        if self.limit == 0 {
            // not initialized
            return;
        }

        let difference = self.reset - Utc::now().timestamp() as isize;
        if difference < 0 {
            self.reset += 3;
            self.remaining = self.limit;
            return;
        }

        if self.remaining <= 0 {
            let delay = difference as u64 * 1000 + 900;
            std::thread::sleep(std::time::Duration::from_millis(delay));
        }

        self.remaining -= 1;
    }

    fn check_for_ratelimit(&mut self, response: &ureq::Response) -> bool {
        if let Some(reset) = &response.header("X-RateLimit-Reset") {
            self.reset = reset.parse::<f64>().expect("unable to parse ratelimit") as isize;
        }
        if let Some(limit) = &response.header("X-RateLimit-Limit") {
            self.limit = limit.parse::<f64>().expect("unable to parse ratelimit") as isize;
        }
        if let Some(remaining) = &response.header("X-RateLimit-Remaining") {
            self.remaining = remaining.parse::<f64>().expect("unable to parse ratelimit") as isize;
        }
        if response.status() == 429 {
            let delay = response
                .header("Retry-After")
                .expect("unable to parse ratelimit")
                .parse::<u64>()
                .expect("unable to parse ratelimit")
                + 100;
            std::thread::sleep(std::time::Duration::from_millis(delay));
            return true;
        }
        false
    }
}
