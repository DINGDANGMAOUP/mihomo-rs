pub mod channel;
pub mod download;
pub mod manager;

pub use channel::{fetch_latest, fetch_releases, Channel, ChannelInfo, ReleaseInfo};
pub use download::Downloader;
pub use manager::{VersionInfo, VersionManager};
