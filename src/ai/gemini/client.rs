use reqwest::{Client, header};
use std::time::Duration;

pub fn build_client() -> Client {
    Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .build()
        .expect("Failed to build reqwest client")
}

pub fn parse_retry_after(headers: &header::HeaderMap) -> Option<u64> {
    headers
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
}

pub fn backoff_delay(attempt: u32) -> Duration {
    let base = 2.0f64;
    let exp = 2u64.pow(attempt.saturating_sub(1));
    let raw = base * (exp as f64);
    let capped = raw.min(60.0);
    let jitter = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as f64
        / 1_000_000_000.0)
        * 0.5
        - 0.25;
    Duration::from_secs_f64((capped * (1.0 + jitter)).max(0.5))
}

pub fn retry_delay(attempt: u32, headers: &header::HeaderMap) -> Duration {
    if let Some(s) = parse_retry_after(headers) {
        Duration::from_secs(s)
    } else {
        backoff_delay(attempt)
    }
}
