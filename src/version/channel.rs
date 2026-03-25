use crate::core::Result;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Channel {
    Stable,
    Beta,
    Nightly,
}

impl Channel {
    pub fn as_str(&self) -> &str {
        match self {
            Channel::Stable => "stable",
            Channel::Beta => "beta",
            Channel::Nightly => "nightly",
        }
    }
}

impl FromStr for Channel {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "stable" => Ok(Channel::Stable),
            "beta" => Ok(Channel::Beta),
            "nightly" | "alpha" => Ok(Channel::Nightly),
            _ => Err(format!("Invalid channel: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    pub channel: Channel,
    pub version: String,
    pub release_date: String,
}

fn extract_release_fields(release: &serde_json::Value) -> Option<(String, String)> {
    let version = release.get("tag_name")?.as_str()?.to_string();
    let date = release.get("published_at")?.as_str()?.to_string();
    Some((version, date))
}

fn api_base_url() -> String {
    std::env::var("MIHOMO_API_BASE_URL").unwrap_or_else(|_| "https://api.github.com".to_string())
}

fn parse_latest_channel_info(channel: Channel, data: serde_json::Value) -> Result<ChannelInfo> {
    let (version, date) = if channel == Channel::Stable {
        extract_release_fields(&data).ok_or_else(|| {
            crate::core::MihomoError::Version("Invalid stable release payload".to_string())
        })?
    } else {
        let release = data
            .as_array()
            .and_then(|releases| releases.first())
            .ok_or_else(|| crate::core::MihomoError::Version("No releases found".to_string()))?;

        extract_release_fields(release).ok_or_else(|| {
            crate::core::MihomoError::Version("Invalid release payload".to_string())
        })?
    };

    Ok(ChannelInfo {
        channel,
        version,
        release_date: date,
    })
}

pub(crate) async fn fetch_latest_with_client(
    channel: Channel,
    client: &reqwest::Client,
    base_url: &str,
) -> Result<ChannelInfo> {
    let url = match channel {
        Channel::Stable => format!("{}/repos/MetaCubeX/mihomo/releases/latest", base_url),
        Channel::Beta => {
            format!(
                "{}/repos/MetaCubeX/mihomo/releases?per_page=1&prerelease=true",
                base_url
            )
        }
        Channel::Nightly => format!("{}/repos/MetaCubeX/mihomo/releases?per_page=1", base_url),
    };

    let resp = client
        .get(url)
        .header("User-Agent", "mihomo-rs")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(crate::core::MihomoError::Version(format!(
            "GitHub API error: {}",
            resp.status()
        )));
    }

    let data: serde_json::Value = resp.json().await?;
    parse_latest_channel_info(channel, data)
}

pub async fn fetch_latest(channel: Channel) -> Result<ChannelInfo> {
    let client = reqwest::Client::new();
    fetch_latest_with_client(channel, &client, &api_base_url()).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseInfo {
    #[serde(rename = "tag_name")]
    pub version: String,
    pub name: String,
    pub published_at: String,
    pub prerelease: bool,
}

async fn fetch_releases_with_client(
    limit: usize,
    client: &reqwest::Client,
    base_url: &str,
) -> Result<Vec<ReleaseInfo>> {
    let resp = client
        .get(format!(
            "{}/repos/MetaCubeX/mihomo/releases?per_page={}",
            base_url, limit
        ))
        .header("User-Agent", "mihomo-rs")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(crate::core::MihomoError::Version(format!(
            "GitHub API error: {}",
            resp.status()
        )));
    }

    let releases: Vec<ReleaseInfo> = resp.json().await?;
    Ok(releases)
}

pub async fn fetch_releases(limit: usize) -> Result<Vec<ReleaseInfo>> {
    let client = reqwest::Client::new();
    fetch_releases_with_client(limit, &client, &api_base_url()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn channel_parse_is_case_insensitive() {
        assert_eq!("stable".parse::<Channel>().unwrap(), Channel::Stable);
        assert_eq!("BeTa".parse::<Channel>().unwrap(), Channel::Beta);
        assert_eq!("nightly".parse::<Channel>().unwrap(), Channel::Nightly);
        assert_eq!("alpha".parse::<Channel>().unwrap(), Channel::Nightly);
    }

    #[test]
    fn channel_parse_invalid_value_returns_error() {
        let err = "dev".parse::<Channel>().unwrap_err();
        assert!(err.contains("Invalid channel"));
    }

    #[test]
    fn channel_as_str_matches_expected_values() {
        assert_eq!(Channel::Stable.as_str(), "stable");
        assert_eq!(Channel::Beta.as_str(), "beta");
        assert_eq!(Channel::Nightly.as_str(), "nightly");
    }

    #[test]
    fn parse_latest_stable_payload_success() {
        let data = serde_json::json!({
            "tag_name": "v1.19.0",
            "published_at": "2026-03-25T12:00:00Z"
        });

        let parsed = parse_latest_channel_info(Channel::Stable, data).unwrap();
        assert_eq!(parsed.version, "v1.19.0");
        assert_eq!(parsed.release_date, "2026-03-25T12:00:00Z");
    }

    #[test]
    fn parse_latest_non_stable_payload_success() {
        let data = serde_json::json!([
            {
                "tag_name": "v1.19.0-beta.1",
                "published_at": "2026-03-20T08:00:00Z"
            }
        ]);

        let parsed = parse_latest_channel_info(Channel::Beta, data).unwrap();
        assert_eq!(parsed.version, "v1.19.0-beta.1");
        assert_eq!(parsed.release_date, "2026-03-20T08:00:00Z");
    }

    #[test]
    fn parse_latest_non_stable_empty_payload_fails() {
        let data = serde_json::json!([]);
        let err = parse_latest_channel_info(Channel::Nightly, data).unwrap_err();
        assert!(matches!(err, crate::core::MihomoError::Version(_)));
    }

    #[test]
    fn parse_latest_stable_invalid_payload_fails() {
        let data = serde_json::json!({ "name": "bad" });
        let err = parse_latest_channel_info(Channel::Stable, data).unwrap_err();
        assert!(matches!(err, crate::core::MihomoError::Version(_)));
    }

    #[test]
    fn parse_latest_non_stable_invalid_payload_fails() {
        let data = serde_json::json!([{ "name": "bad" }]);
        let err = parse_latest_channel_info(Channel::Nightly, data).unwrap_err();
        assert!(matches!(err, crate::core::MihomoError::Version(_)));
    }

    #[tokio::test]
    async fn fetch_latest_with_client_stable_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/repos/MetaCubeX/mihomo/releases/latest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"tag_name":"v1.20.0","published_at":"2026-03-25T12:00:00Z"}"#)
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let info = fetch_latest_with_client(Channel::Stable, &client, &server.url())
            .await
            .unwrap();

        mock.assert_async().await;
        assert_eq!(info.version, "v1.20.0");
    }

    #[tokio::test]
    async fn fetch_latest_with_client_api_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/repos/MetaCubeX/mihomo/releases/latest")
            .with_status(503)
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let err = fetch_latest_with_client(Channel::Stable, &client, &server.url())
            .await
            .unwrap_err();

        mock.assert_async().await;
        assert!(matches!(err, crate::core::MihomoError::Version(_)));
    }

    #[tokio::test]
    async fn fetch_latest_with_client_beta_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/repos/MetaCubeX/mihomo/releases")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("per_page".into(), "1".into()),
                mockito::Matcher::UrlEncoded("prerelease".into(), "true".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"tag_name":"v1.20.0-beta.1","published_at":"2026-03-26T01:00:00Z"}]"#)
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let info = fetch_latest_with_client(Channel::Beta, &client, &server.url())
            .await
            .unwrap();

        mock.assert_async().await;
        assert_eq!(info.version, "v1.20.0-beta.1");
    }

    #[tokio::test]
    async fn fetch_latest_with_client_nightly_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/repos/MetaCubeX/mihomo/releases")
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "1".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"tag_name":"nightly-20260326","published_at":"2026-03-26T02:00:00Z"}]"#)
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let info = fetch_latest_with_client(Channel::Nightly, &client, &server.url())
            .await
            .unwrap();

        mock.assert_async().await;
        assert_eq!(info.version, "nightly-20260326");
    }

    #[tokio::test]
    async fn fetch_releases_with_client_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/repos/MetaCubeX/mihomo/releases")
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "2".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[{"tag_name":"v1.0.0","name":"r1","published_at":"2026-01-01","prerelease":false}]"#,
            )
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let list = fetch_releases_with_client(2, &client, &server.url())
            .await
            .unwrap();

        mock.assert_async().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].version, "v1.0.0");
    }

    #[tokio::test]
    async fn fetch_releases_with_client_api_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/repos/MetaCubeX/mihomo/releases")
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "1".into()))
            .with_status(500)
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let err = fetch_releases_with_client(1, &client, &server.url())
            .await
            .unwrap_err();

        mock.assert_async().await;
        assert!(matches!(err, crate::core::MihomoError::Version(_)));
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn fetch_latest_public_uses_env_base_url() {
        let _guard = env_lock().lock().expect("env lock");
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/repos/MetaCubeX/mihomo/releases/latest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"tag_name":"v9.9.9","published_at":"2026-03-26T03:00:00Z"}"#)
            .create_async()
            .await;

        let old = std::env::var("MIHOMO_API_BASE_URL").ok();
        // SAFETY: env updates are serialized in this module via env_lock.
        unsafe { std::env::set_var("MIHOMO_API_BASE_URL", server.url()) };
        let info = fetch_latest(Channel::Stable).await.unwrap();
        mock.assert_async().await;
        assert_eq!(info.version, "v9.9.9");

        if let Some(prev) = old {
            // SAFETY: env updates are serialized in this module via env_lock.
            unsafe { std::env::set_var("MIHOMO_API_BASE_URL", prev) };
        } else {
            // SAFETY: env updates are serialized in this module via env_lock.
            unsafe { std::env::remove_var("MIHOMO_API_BASE_URL") };
        }
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn fetch_releases_public_uses_env_base_url() {
        let _guard = env_lock().lock().expect("env lock");
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/repos/MetaCubeX/mihomo/releases")
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "1".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[{"tag_name":"v9.9.8","name":"r","published_at":"2026-03-26","prerelease":false}]"#,
            )
            .create_async()
            .await;

        let old = std::env::var("MIHOMO_API_BASE_URL").ok();
        // SAFETY: env updates are serialized in this module via env_lock.
        unsafe { std::env::set_var("MIHOMO_API_BASE_URL", server.url()) };
        let list = fetch_releases(1).await.unwrap();
        mock.assert_async().await;
        assert_eq!(list[0].version, "v9.9.8");

        if let Some(prev) = old {
            // SAFETY: env updates are serialized in this module via env_lock.
            unsafe { std::env::set_var("MIHOMO_API_BASE_URL", prev) };
        } else {
            // SAFETY: env updates are serialized in this module via env_lock.
            unsafe { std::env::remove_var("MIHOMO_API_BASE_URL") };
        }
    }
}
