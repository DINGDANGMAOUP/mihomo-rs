use mihomo_rs::{Channel, MihomoError, Result, ServiceStatus};

#[test]
fn public_exports_are_usable() {
    fn returns_result() -> Result<i32> {
        Ok(7)
    }

    let _: Option<Channel> = Some(Channel::Stable);
    let _: Option<ServiceStatus> = Some(ServiceStatus::Stopped);

    let err = MihomoError::config("bad config");
    assert!(matches!(err, MihomoError::Config(_)));
    assert_eq!(returns_result().expect("result alias works"), 7);
}

#[test]
fn channel_parsing_accepts_expected_values() {
    assert_eq!(
        "stable".parse::<Channel>().expect("stable parse"),
        Channel::Stable
    );
    assert_eq!(
        "beta".parse::<Channel>().expect("beta parse"),
        Channel::Beta
    );
    assert_eq!(
        "nightly".parse::<Channel>().expect("nightly parse"),
        Channel::Nightly
    );
    assert_eq!(
        "alpha".parse::<Channel>().expect("alpha parse"),
        Channel::Nightly
    );
    let err = "unknown"
        .parse::<Channel>()
        .expect_err("unknown channel should fail");
    assert_eq!(err, "Invalid channel: unknown");
}
