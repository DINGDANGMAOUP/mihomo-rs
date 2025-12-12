use crate::core::Result;
use serde::{Deserialize, Serialize};

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

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "stable" => Some(Channel::Stable),
            "beta" => Some(Channel::Beta),
            "nightly" | "alpha" => Some(Channel::Nightly),
            _ => None,
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
    let url = match channel {
        Channel::Stable => "https://api.github.com/repos/MetaCubeX/mihomo/releases/latest",
        Channel::Beta => {
            "https://api.github.com/repos/MetaCubeX/mihomo/releases?per_page=1&prerelease=true"
        }
        Channel::Nightly => "https://api.github.com/repos/MetaCubeX/mihomo/releases?per_page=1",
    };

    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header("User-Agent", "mihomo-rs")
        .send()
        .await?;

    let data: serde_json::Value = resp.json().await?;

    let (version, date) = if channel == Channel::Stable {
        let tag = data["tag_name"].as_str().unwrap_or("").to_string();
        let date = data["published_at"].as_str().unwrap_or("").to_string();
        (tag, date)
    } else {
        let empty_vec = vec![];
        let releases = data.as_array().unwrap_or(&empty_vec);
        if let Some(release) = releases.first() {
            let tag = release["tag_name"].as_str().unwrap_or("").to_string();
            let date = release["published_at"].as_str().unwrap_or("").to_string();
            (tag, date)
        } else {
            return Err(crate::core::MihomoError::Version(
                "No releases found".to_string(),
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
    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "https://api.github.com/repos/MetaCubeX/mihomo/releases?per_page={}",
            limit
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
