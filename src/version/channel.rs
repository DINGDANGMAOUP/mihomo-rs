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

pub async fn fetch_latest(channel: Channel) -> Result<ChannelInfo> {
    fetch_latest_with_base("https://api.github.com", channel).await
}

async fn fetch_latest_with_base(api_base: &str, channel: Channel) -> Result<ChannelInfo> {
    let url = match channel {
        Channel::Stable => format!("{}/repos/MetaCubeX/mihomo/releases/latest", api_base),
        Channel::Beta | Channel::Nightly => {
            format!("{}/repos/MetaCubeX/mihomo/releases?per_page=20", api_base)
        }
    };

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "mihomo-rs")
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(crate::core::MihomoError::version(format!(
            "GitHub API error: {}",
            resp.status()
        )));
    }

    let data: serde_json::Value = resp.json().await?;

    let (version, date) = if channel == Channel::Stable {
        let tag = data["tag_name"].as_str().unwrap_or_default().to_string();
        let date = data["published_at"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        if tag.is_empty() {
            return Err(crate::core::MihomoError::version("No stable release found"));
        }
        (tag, date)
    } else {
        let empty_vec = vec![];
        let releases = data.as_array().unwrap_or(&empty_vec);
        let selected = match channel {
            Channel::Beta => releases
                .iter()
                .find(|release| release["prerelease"].as_bool().unwrap_or(false)),
            Channel::Nightly => releases
                .iter()
                .find(|release| {
                    let tag = release["tag_name"]
                        .as_str()
                        .unwrap_or_default()
                        .to_lowercase();
                    release["prerelease"].as_bool().unwrap_or(false)
                        || tag.contains("nightly")
                        || tag.contains("alpha")
                })
                .or_else(|| releases.first()),
            Channel::Stable => None,
        };

        if let Some(release) = selected {
            let tag = release["tag_name"].as_str().unwrap_or_default().to_string();
            let date = release["published_at"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            if tag.is_empty() {
                return Err(crate::core::MihomoError::version(
                    "Invalid release data: empty tag_name",
                ));
            }
            (tag, date)
        } else {
            return Err(crate::core::MihomoError::version(
                "No releases found for selected channel",
            ));
        }
    };

    Ok(ChannelInfo {
        channel,
        version,
        release_date: date,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseInfo {
    #[serde(rename = "tag_name")]
    pub version: String,
    pub name: String,
    pub published_at: String,
    pub prerelease: bool,
}

pub async fn fetch_releases(limit: usize) -> Result<Vec<ReleaseInfo>> {
    fetch_releases_with_base("https://api.github.com", limit).await
}

async fn fetch_releases_with_base(api_base: &str, limit: usize) -> Result<Vec<ReleaseInfo>> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "{}/repos/MetaCubeX/mihomo/releases?per_page={}",
            api_base, limit
        ))
        .header("User-Agent", "mihomo-rs")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(crate::core::MihomoError::version(format!(
            "GitHub API error: {}",
            resp.status()
        )));
    }

    let releases: Vec<ReleaseInfo> = resp.json().await?;
    Ok(releases)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn fetch_latest_stable_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/repos/MetaCubeX/mihomo/releases/latest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"tag_name":"v1.20.1","published_at":"2026-01-01T00:00:00Z"}"#)
            .create_async()
            .await;

        let info = fetch_latest_with_base(&server.url(), Channel::Stable)
            .await
            .expect("fetch stable");
        mock.assert_async().await;

        assert_eq!(info.channel, Channel::Stable);
        assert_eq!(info.version, "v1.20.1");
        assert_eq!(info.release_date, "2026-01-01T00:00:00Z");
    }

    #[tokio::test]
    async fn fetch_latest_beta_and_nightly_selection() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/repos/MetaCubeX/mihomo/releases")
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "20".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {"tag_name":"v1.20.1","published_at":"2026-01-02T00:00:00Z","prerelease":false},
                    {"tag_name":"v1.21.0-beta.1","published_at":"2026-01-03T00:00:00Z","prerelease":true},
                    {"tag_name":"nightly-20260104","published_at":"2026-01-04T00:00:00Z","prerelease":false}
                ]"#,
            )
            .expect(2)
            .create_async()
            .await;

        let beta = fetch_latest_with_base(&server.url(), Channel::Beta)
            .await
            .expect("fetch beta");
        let nightly = fetch_latest_with_base(&server.url(), Channel::Nightly)
            .await
            .expect("fetch nightly");
        mock.assert_async().await;

        assert_eq!(beta.version, "v1.21.0-beta.1");
        assert_eq!(nightly.version, "v1.21.0-beta.1");
    }

    #[test]
    fn channel_as_str_returns_expected_values() {
        assert_eq!(Channel::Stable.as_str(), "stable");
        assert_eq!(Channel::Beta.as_str(), "beta");
        assert_eq!(Channel::Nightly.as_str(), "nightly");
    }

    #[tokio::test]
    async fn fetch_latest_nightly_falls_back_to_first_release() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/repos/MetaCubeX/mihomo/releases")
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "20".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {"tag_name":"v1.20.2","published_at":"2026-01-05T00:00:00Z","prerelease":false},
                    {"tag_name":"v1.20.1","published_at":"2026-01-04T00:00:00Z","prerelease":false}
                ]"#,
            )
            .create_async()
            .await;

        let nightly = fetch_latest_with_base(&server.url(), Channel::Nightly)
            .await
            .expect("nightly should fall back to first release");
        mock.assert_async().await;
        assert_eq!(nightly.version, "v1.20.2");
    }

    #[tokio::test]
    async fn fetch_latest_beta_rejects_selected_release_with_empty_tag() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/repos/MetaCubeX/mihomo/releases")
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "20".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {"tag_name":"","published_at":"2026-01-06T00:00:00Z","prerelease":true}
                ]"#,
            )
            .create_async()
            .await;

        let err = fetch_latest_with_base(&server.url(), Channel::Beta)
            .await
            .expect_err("empty tag in selected release should fail");
        mock.assert_async().await;
        assert!(err.to_string().contains("empty tag_name"));
    }

    #[tokio::test]
    async fn fetch_latest_error_paths() {
        let mut server = Server::new_async().await;
        let stable_err = server
            .mock("GET", "/repos/MetaCubeX/mihomo/releases/latest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"tag_name":"","published_at":"2026-01-01T00:00:00Z"}"#)
            .create_async()
            .await;

        let list_err = server
            .mock("GET", "/repos/MetaCubeX/mihomo/releases")
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "20".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("[]")
            .create_async()
            .await;

        let stable = fetch_latest_with_base(&server.url(), Channel::Stable).await;
        let beta = fetch_latest_with_base(&server.url(), Channel::Beta).await;
        stable_err.assert_async().await;
        list_err.assert_async().await;

        assert!(stable.is_err());
        assert!(beta.is_err());
    }

    #[tokio::test]
    async fn fetch_releases_success_and_http_error() {
        let mut server = Server::new_async().await;
        let ok = server
            .mock("GET", "/repos/MetaCubeX/mihomo/releases")
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "2".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {"tag_name":"v1.20.0","name":"v1.20.0","published_at":"2026-01-01T00:00:00Z","prerelease":false},
                    {"tag_name":"v1.21.0-beta.1","name":"v1.21.0-beta.1","published_at":"2026-01-02T00:00:00Z","prerelease":true}
                ]"#,
            )
            .create_async()
            .await;

        let releases = fetch_releases_with_base(&server.url(), 2)
            .await
            .expect("fetch releases");
        ok.assert_async().await;
        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].version, "v1.20.0");

        let fail = server
            .mock("GET", "/repos/MetaCubeX/mihomo/releases")
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "1".into()))
            .with_status(500)
            .create_async()
            .await;
        let result = fetch_releases_with_base(&server.url(), 1).await;
        fail.assert_async().await;
        assert!(result.is_err());
    }
}
