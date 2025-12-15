// Integration tests for top-level convenience functions

use mihomo_rs::Result;

// Note: These tests verify that the public API is exported correctly
// Actual functionality testing requires a running mihomo instance and network access

#[test]
fn test_result_type_alias() {
    // Test that Result type alias works correctly
    fn returns_result() -> Result<i32> {
        Ok(42)
    }

    assert_eq!(returns_result().unwrap(), 42);
}

#[test]
fn test_public_exports() {
    // Verify all main types are exported
    use mihomo_rs::{
        Channel, ConfigManager, MihomoClient, MihomoError, Profile, ProxyManager, Result,
        ServiceManager, ServiceStatus, VersionManager,
    };

    // Type existence checks
    let _: Option<Channel> = None;
    let _: Option<ConfigManager> = None;
    let _: Option<MihomoClient> = None;
    let _: Option<MihomoError> = None;
    let _: Option<ProxyManager> = None;
    let _: Option<ServiceManager> = None;
    let _: Option<ServiceStatus> = None;
    let _: Option<VersionManager> = None;
    let _: Option<Profile> = None;
    let _: Result<()> = Ok(());
}

#[test]
fn test_channel_enum() {
    use mihomo_rs::Channel;

    // Test Channel enum variants exist
    let _stable = Channel::Stable;
    let _beta = Channel::Beta;
    let _nightly = Channel::Nightly;
}

#[test]
fn test_service_status_enum() {
    use mihomo_rs::ServiceStatus;

    // Test ServiceStatus enum variants
    let running = ServiceStatus::Running(12345);
    let stopped = ServiceStatus::Stopped;

    match running {
        ServiceStatus::Running(pid) => assert_eq!(pid, 12345),
        ServiceStatus::Stopped => panic!("Should be running"),
    }

    match stopped {
        ServiceStatus::Stopped => {} // OK
        ServiceStatus::Running(_) => panic!("Should be stopped"),
    }
}

#[test]
fn test_error_type() {
    use mihomo_rs::MihomoError;

    // Test various error types can be created
    let config_err = MihomoError::Config("test".to_string());
    let service_err = MihomoError::Service("test".to_string());
    let version_err = MihomoError::Version("test".to_string());
    let proxy_err = MihomoError::Proxy("test".to_string());
    let not_found_err = MihomoError::NotFound("test".to_string());

    assert!(matches!(config_err, MihomoError::Config(_)));
    assert!(matches!(service_err, MihomoError::Service(_)));
    assert!(matches!(version_err, MihomoError::Version(_)));
    assert!(matches!(proxy_err, MihomoError::Proxy(_)));
    assert!(matches!(not_found_err, MihomoError::NotFound(_)));
}
