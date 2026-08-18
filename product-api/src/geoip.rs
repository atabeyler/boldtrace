//! Best-effort IP-to-country lookup used to guard login against a country
//! that doesn't match the one an account registered with. A lookup failure
//! (provider down, no network, malformed response) fails *open* — login
//! proceeds unchecked — because the alternative is that a third-party
//! outage locks every user out of BOLDTRACE, which is worse than skipping
//! one login's location check.

use serde::Deserialize;
use std::net::IpAddr;
use std::time::Duration;

#[derive(Deserialize)]
struct IpApiResponse {
    country_code: Option<String>,
}

/// Extracts the caller's IP from `X-Forwarded-For` (set by Northflank's
/// front proxy) or, failing that, `X-Real-IP`. Returns `None` for loopback
/// or unparseable values, e.g. local development.
pub fn client_ip(headers: &axum::http::HeaderMap) -> Option<String> {
    let forwarded = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(str::trim);
    let candidate = forwarded.or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()));
    let ip: IpAddr = candidate?.parse().ok()?;
    if ip.is_loopback() {
        return None;
    }
    Some(ip.to_string())
}

/// Resolves an IP to an ISO 3166-1 alpha-2 country code, or `None` if the
/// lookup couldn't be completed for any reason.
pub async fn lookup_country(ip: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .ok()?;
    let res = client
        .get(format!("https://ipapi.co/{ip}/json/"))
        .send()
        .await
        .ok()?;
    if !res.status().is_success() {
        return None;
    }
    let body: IpApiResponse = res.json().await.ok()?;
    body.country_code
        .map(|c| c.trim().to_uppercase())
        .filter(|c| c.len() == 2)
}
