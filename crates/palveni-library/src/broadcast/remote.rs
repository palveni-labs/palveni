use std::{
    future::Future,
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use palveni_core::broadcast::MAX_BROADCAST_BYTES;
use reqwest::{Client, Url, header::LOCATION, redirect::Policy};

use super::public_media_url;

fn publicly_routable(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(a) => {
            let [first, second, third, _] = a.octets();
            !a.is_private()
                && !a.is_loopback()
                && !a.is_link_local()
                && !a.is_documentation()
                && (1..224).contains(&first)
                && !(first == 100 && (64..128).contains(&second))
                && !(first == 198 && matches!(second, 18 | 19))
                && !(first == 192 && (second == 0 && third == 0 || second == 88 && third == 99))
        }
        IpAddr::V6(a) => {
            let s = a.segments();
            (0x2000..0x4000).contains(&s[0])
                && s[0] != 0x2002
                && !(s[0] == 0x2001 && (s[1] <= 0x01ff || s[1] == 0x0db8))
                && !(s[0] == 0x3fff && s[1] < 0x1000)
        }
    }
}

async fn fetch(mut url: Url) -> Result<(String, Url), &'static str> {
    for _ in 0..=3 {
        let host = url
            .host_str()
            .ok_or("Invalid subscription host")?
            .trim_start_matches('[')
            .trim_end_matches(']');
        let port = url
            .port_or_known_default()
            .ok_or("Invalid subscription port")?;
        let addresses: Vec<SocketAddr> = match host.parse::<IpAddr>() {
            Ok(ip) => vec![SocketAddr::new(ip, port)],
            Err(_) => tokio::net::lookup_host((host, port))
                .await
                .map_err(|_| "Could not resolve the subscription host")?
                .take(32)
                .collect(),
        };
        if addresses.is_empty() || addresses.iter().any(|addr| !publicly_routable(addr.ip())) {
            return Err("Remote subscriptions require a publicly reachable host");
        }
        // The transport uses exactly the addresses we checked, including on each redirect.
        let client = Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .referer(false)
            .resolve_to_addrs(host, &addresses)
            .connect_timeout(Duration::from_secs(4))
            .timeout(Duration::from_secs(12))
            .build()
            .map_err(|_| "Could not start the subscription client")?;
        let mut response = client
            .get(url.clone())
            .send()
            .await
            .map_err(|_| "Could not read the remote subscription")?;
        if response.status().is_redirection() {
            let next = response
                .headers()
                .get(LOCATION)
                .and_then(|h| h.to_str().ok())
                .and_then(|location| public_media_url(location, Some(&url)))
                .ok_or("Invalid subscription redirect")?;
            if url.scheme() == "https" && next.scheme() != "https" {
                return Err("An HTTPS subscription cannot redirect to HTTP");
            }
            url = next;
            continue;
        }
        if !response.status().is_success() {
            return Err("Remote subscription returned an unsuccessful status");
        }
        if response
            .content_length()
            .is_some_and(|n| n > MAX_BROADCAST_BYTES as u64)
        {
            return Err("Remote subscription exceeds 1 MiB");
        }
        let mut content = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Could not read the remote subscription")?
        {
            if content.len() + chunk.len() > MAX_BROADCAST_BYTES {
                return Err("Remote subscription exceeds 1 MiB");
            }
            content.extend_from_slice(&chunk);
        }
        return String::from_utf8(content)
            .map(|s| (s, url))
            .map_err(|_| "Subscriptions must use UTF-8");
    }
    Err("Too many subscription redirects")
}

pub async fn fetch_public(url: Url) -> Result<(String, Url), &'static str> {
    let url = public_media_url(url.as_str(), None).ok_or("Invalid subscription URL")?;
    within_deadline(Duration::from_secs(20), fetch(url)).await
}

async fn within_deadline(
    limit: Duration,
    reading: impl Future<Output = Result<(String, Url), &'static str>>,
) -> Result<(String, Url), &'static str> {
    tokio::time::timeout(limit, reading)
        .await
        .map_err(|_| "Reading the subscription timed out")?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn validates_initial_target_before_requesting_a_subscription() {
        for raw in ["http://127.0.0.1/feed", "http://[::1]/guide"] {
            let result = fetch_public(Url::parse(raw).unwrap()).await;
            assert_eq!(
                result.unwrap_err(),
                "Remote subscriptions require a publicly reachable host"
            );
        }
        for raw in [
            "ftp://example.com/feed",
            "https://member@example.com/feed",
            "https://example.com/?token=test",
        ] {
            let result = fetch_public(Url::parse(raw).unwrap()).await;
            assert_eq!(result.unwrap_err(), "Invalid subscription URL");
        }
    }

    #[tokio::test]
    async fn total_read_deadline_cancels_an_unfinished_subscription() {
        let result = within_deadline(Duration::from_millis(2), std::future::pending()).await;
        assert_eq!(result.unwrap_err(), "Reading the subscription timed out");
    }

    #[test]
    fn restricts_local_and_transition_ranges() {
        for addr in [
            "0.0.0.0",
            "127.0.0.1",
            "192.168.0.1",
            "100.64.0.1",
            "169.254.169.254",
            "198.18.0.1",
            "224.1.2.3",
            "::1",
            "::ffff:192.168.1.1",
            "64:ff9b::7f00:1",
            "2002:7f00:1::",
            "2001:db8::1",
        ] {
            assert!(!publicly_routable(addr.parse().unwrap()), "{addr}");
        }
        for addr in ["8.8.8.8", "2606:4700:4700::1111"] {
            assert!(publicly_routable(addr.parse().unwrap()));
        }
    }
}
