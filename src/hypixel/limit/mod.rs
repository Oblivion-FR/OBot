use reqwest::header::HeaderMap;
use std::sync::Mutex;
use std::time::Duration;
use tokio::time::Instant;

/// Requests left unused when the budget runs low: responses still in flight may have spent them,
/// and the same key may be used elsewhere (scripts, manual calls)
const MARGIN: u64 = 20;

/// Length of Hypixel's rate limit window, used when a response doesn't say when it resets
const WINDOW: Duration = Duration::from_secs(60);

struct Budget {
    /// Requests allowed per window, from `RateLimit-Limit`
    limit: Option<u64>,
    /// Requests left in the current window, `None` until a response tells
    remaining: Option<u64>,
    resets_at: Instant,
}

impl Budget {
    /// Capped at half the limit, otherwise a key allowed `MARGIN` requests or fewer per
    /// window would never send any
    fn margin(&self) -> u64 {
        self.limit.map_or(MARGIN, |limit| MARGIN.min(limit / 2))
    }
}

/// Keeps track of the Hypixel API key's per minute budget from the `RateLimit-*` headers, and
/// holds requests back until the next window instead of letting them fail
pub struct RateLimiter(Mutex<Budget>);

impl Default for RateLimiter {
    fn default() -> Self {
        Self(Mutex::new(Budget {
            limit: None,
            remaining: None,
            resets_at: Instant::now(),
        }))
    }
}

impl RateLimiter {
    /// Waits until a request fits in the budget, then reserves it. Reserving before sending
    /// keeps concurrent requests from all seeing the same last request as available.
    pub async fn acquire(&self) {
        loop {
            let wait = {
                let mut budget = self.0.lock().unwrap_or_else(|e| e.into_inner());
                let now = Instant::now();
                if now >= budget.resets_at {
                    budget.remaining = budget.limit;
                    budget.resets_at = now + WINDOW;
                }
                match budget.remaining {
                    Some(remaining) if remaining <= budget.margin() => budget.resets_at - now,
                    Some(remaining) => {
                        budget.remaining = Some(remaining - 1);
                        return;
                    }
                    None => return,
                }
            };
            tokio::time::sleep(wait).await;
        }
    }

    /// Updates the budget from a response's headers
    pub fn record(&self, headers: &HeaderMap, rate_limited: bool) {
        let number = |name: &str| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.trim().parse::<u64>().ok())
        };
        let mut budget = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();

        if let Some(limit) = number("ratelimit-limit") {
            budget.limit = Some(limit);
        }
        let reset = number("ratelimit-reset").map(|seconds| now + Duration::from_secs(seconds));
        // A reset later than the known one means this response counts in a new window
        let new_window =
            reset.is_some_and(|reset| reset > budget.resets_at + Duration::from_secs(1));
        if let Some(reset) = reset {
            budget.resets_at = reset;
        }
        if let Some(remaining) = number("ratelimit-remaining") {
            // In the same window, responses can arrive out of order: the lowest count is the latest
            budget.remaining = match budget.remaining {
                Some(known) if !new_window => Some(known.min(remaining)),
                _ => Some(remaining),
            };
        }
        if rate_limited {
            budget.remaining = Some(0);
            if reset.is_none() {
                budget.resets_at = budget.resets_at.max(now + WINDOW);
            }
        }
    }
}

#[cfg(test)]
mod tests;
