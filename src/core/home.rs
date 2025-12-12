use crate::core::MihomoError;
use std::path::PathBuf;

/// Get the mihomo-rs home directory
///
/// Priority:
/// 1. MIHOMO_HOME environment variable
/// 2. Default: ~/.config/mihomo-rs
pub fn get_home_dir() -> Result<PathBuf, MihomoError> {
    if let Ok(home) = std::env::var("MIHOMO_HOME") {
        log::debug!("Using MIHOMO_HOME: {}", home);
        return Ok(PathBuf::from(home));
    }

    let home = dirs::home_dir()
        .ok_or_else(|| MihomoError::Config("Could not determine home directory".to_string()))?;

    let mihomo_home = home.join(".config/mihomo-rs");
    log::debug!("Using default home: {}", mihomo_home.display());
    Ok(mihomo_home)
}
