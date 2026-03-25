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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn prefers_mihomo_home_env() {
        let _guard = env_lock().lock().expect("env lock");
        let old = std::env::var("MIHOMO_HOME").ok();

        // SAFETY: Tests serialize env access with a global mutex.
        unsafe { std::env::set_var("MIHOMO_HOME", "/tmp/mihomo-home-test") };
        let result = get_home_dir().expect("home path");
        assert_eq!(result, PathBuf::from("/tmp/mihomo-home-test"));

        if let Some(prev) = old {
            // SAFETY: Tests serialize env access with a global mutex.
            unsafe { std::env::set_var("MIHOMO_HOME", prev) };
        } else {
            // SAFETY: Tests serialize env access with a global mutex.
            unsafe { std::env::remove_var("MIHOMO_HOME") };
        }
    }

    #[test]
    fn falls_back_to_default_home_config_path() {
        let _guard = env_lock().lock().expect("env lock");
        let old = std::env::var("MIHOMO_HOME").ok();
        // SAFETY: Tests serialize env access with a global mutex.
        unsafe { std::env::remove_var("MIHOMO_HOME") };

        let result = get_home_dir().expect("home path");
        assert!(result.ends_with(".config/mihomo-rs"));

        if let Some(prev) = old {
            // SAFETY: Tests serialize env access with a global mutex.
            unsafe { std::env::set_var("MIHOMO_HOME", prev) };
        }
    }
}
