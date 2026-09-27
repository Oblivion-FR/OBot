use super::*;
use reqwest::header::{HeaderName, HeaderValue};

fn headers(limit: u64, remaining: u64, reset: u64) -> HeaderMap {
    [
        ("ratelimit-limit", limit),
        ("ratelimit-remaining", remaining),
        ("ratelimit-reset", reset),
    ]
    .into_iter()
    .map(|(name, value)| {
        (
            HeaderName::from_static(name),
            HeaderValue::from_str(&value.to_string()).expect("digits are a valid header"),
        )
    })
    .collect()
}

#[tokio::test(start_paused = true)]
async fn requests_go_through_while_budget_is_left() {
    let limiter = RateLimiter::default();
    let start = Instant::now();
    // Nothing known yet: no waiting
    limiter.acquire().await;
    limiter.record(&headers(300, MARGIN + 10, 40), false);
    for _ in 0..10 {
        limiter.acquire().await;
    }
    assert_eq!(start.elapsed(), Duration::ZERO);
}

#[tokio::test(start_paused = true)]
async fn waits_for_the_reset_when_only_the_margin_is_left() {
    let limiter = RateLimiter::default();
    limiter.record(&headers(300, MARGIN + 1, 40), false);
    let start = Instant::now();
    limiter.acquire().await;
    assert_eq!(
        start.elapsed(),
        Duration::ZERO,
        "one request above the margin"
    );

    limiter.acquire().await;
    assert_eq!(
        start.elapsed(),
        Duration::from_secs(40),
        "waited for the next window"
    );
}

#[tokio::test(start_paused = true)]
async fn a_new_window_refills_the_known_limit() {
    let limiter = RateLimiter::default();
    limiter.record(&headers(300, 0, 5), false);
    let start = Instant::now();
    limiter.acquire().await;
    assert_eq!(start.elapsed(), Duration::from_secs(5));
    // The request that waited took the first slot of the new window
    for _ in 1..(300 - MARGIN) {
        limiter.acquire().await;
    }
    assert_eq!(
        start.elapsed(),
        Duration::from_secs(5),
        "the whole limit but the margin fits in the new window"
    );
    limiter.acquire().await;
    assert_eq!(start.elapsed(), Duration::from_secs(65));
}

#[tokio::test(start_paused = true)]
async fn a_small_limit_keeps_half_of_it_as_margin() {
    let limiter = RateLimiter::default();
    // A limit of 10 can't keep 20 aside, so 5 are kept and the 6th request goes through
    limiter.record(&headers(10, 6, 30), false);
    let start = Instant::now();
    limiter.acquire().await;
    assert_eq!(start.elapsed(), Duration::ZERO);
    limiter.acquire().await;
    assert_eq!(start.elapsed(), Duration::from_secs(30));
}

#[tokio::test(start_paused = true)]
async fn out_of_order_responses_keep_the_lowest_count() {
    let limiter = RateLimiter::default();
    limiter.record(&headers(300, MARGIN, 30), false);
    // An older response of the same window arrives late with a higher count
    limiter.record(&headers(300, 250, 30), false);
    let start = Instant::now();
    limiter.acquire().await;
    assert_eq!(start.elapsed(), Duration::from_secs(30));
}

#[tokio::test(start_paused = true)]
async fn a_response_from_the_next_window_replaces_a_low_count() {
    let limiter = RateLimiter::default();
    limiter.record(&headers(300, MARGIN, 2), false);
    tokio::time::advance(Duration::from_secs(3)).await;
    limiter.record(&headers(300, 298, 57), false);
    let start = Instant::now();
    limiter.acquire().await;
    assert_eq!(start.elapsed(), Duration::ZERO);
}

#[tokio::test(start_paused = true)]
async fn a_429_without_headers_waits_a_full_window() {
    let limiter = RateLimiter::default();
    limiter.record(&HeaderMap::new(), true);
    let start = Instant::now();
    limiter.acquire().await;
    assert_eq!(start.elapsed(), WINDOW);
}
