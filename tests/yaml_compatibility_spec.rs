//! Contracts retained when replacing deprecated serde_yaml with serde_norway.
use mihomo_rs::config::Profile;
use mihomo_rs::{ConfigManager, MihomoError};
use serde_yaml::Value;
use tempfile::tempdir;

const CONFIG: &str = include_str!("fixtures/mihomo-yaml-compat.yaml");

#[test]
fn yaml_values_aliases_tags_and_complex_keys_roundtrip() {
    let parsed: Value = serde_yaml::from_str(CONFIG).expect("parse representative config");
    assert_eq!(parsed["port"].as_u64(), Some(7890));
    assert_eq!(parsed["allow-lan"].as_bool(), Some(false));
    assert_eq!(parsed["secret"].as_str(), Some("012345"));
    assert_eq!(parsed["proxies"][0]["password"].as_str(), Some("false"));
    assert_eq!(parsed["large-number"].as_u64(), Some(u64::MAX));
    assert_eq!(parsed["notes"].as_str(), Some("first line\n第二行\n"));
    // serde_yaml resolved aliases but did not apply merges unless explicitly asked.
    assert_eq!(parsed["proxies"][0]["<<"], parsed["proxy-defaults"]);
    assert!(parsed["proxies"][0].get("udp").is_none());
    assert!(matches!(parsed["metadata"], Value::Tagged(_)));
    let key = Value::Sequence(vec![Value::from("one"), Value::from("two")]);
    assert_eq!(
        parsed["extra"].as_mapping().unwrap()[&key],
        Value::from("pair")
    );
    let emitted = serde_yaml::to_string(&parsed).expect("serialize config");
    assert_eq!(serde_yaml::from_str::<Value>(&emitted).unwrap(), parsed);
}

#[tokio::test]
async fn config_save_load_and_profile_validation_preserve_original_yaml() {
    let home = tempdir().unwrap();
    let manager = ConfigManager::with_home(home.path().to_path_buf()).unwrap();
    manager.save("compat", CONFIG).await.unwrap();
    manager.set_current("compat").await.unwrap();
    assert_eq!(manager.load("compat").await.unwrap(), CONFIG);
    Profile::new(
        "compat".into(),
        manager.get_current_path().await.unwrap(),
        true,
    )
    .validate()
    .await
    .unwrap();
    assert_eq!(
        manager.get_external_controller().await.unwrap(),
        "http://api.example.test:9090"
    );
}

#[tokio::test]
async fn controller_rewrite_preserves_yaml_semantics() {
    let home = tempdir().unwrap();
    let manager = ConfigManager::with_home(home.path().to_path_buf()).unwrap();
    let source = CONFIG.replace("external-controller: api.example.test:9090\n", "");
    manager.save("compat", &source).await.unwrap();
    manager.set_current("compat").await.unwrap();
    let controller = manager.ensure_external_controller().await.unwrap();
    assert!(controller.starts_with("http://127.0.0.1:"));
    let mut rewritten: Value =
        serde_yaml::from_str(&manager.load("compat").await.unwrap()).unwrap();
    let expected: Value = serde_yaml::from_str(&source).unwrap();
    rewritten
        .as_mapping_mut()
        .unwrap()
        .remove(Value::from("external-controller"));
    assert_eq!(rewritten, expected);
}

#[tokio::test]
async fn invalid_yaml_is_reported_and_does_not_overwrite_saved_profile() {
    let home = tempdir().unwrap();
    let manager = ConfigManager::with_home(home.path().to_path_buf()).unwrap();
    manager.save("compat", CONFIG).await.unwrap();
    for invalid in [
        "proxies: [unterminated",
        "port: 7890\nport: 7891\n",
        "proxies: *missing\n",
        "---\nport: 7890\n---\nport: 7891\n",
    ] {
        let error = manager.save("compat", invalid).await.unwrap_err();
        assert!(matches!(error, MihomoError::Yaml(_)), "{error}");
        assert!(error.to_string().starts_with("YAML error: "));
        assert_eq!(manager.load("compat").await.unwrap(), CONFIG);
        let path = home.path().join("invalid.yaml");
        tokio::fs::write(&path, invalid).await.unwrap();
        let error = Profile::new("invalid".into(), path, false)
            .validate()
            .await
            .unwrap_err();
        assert!(matches!(error, MihomoError::Yaml(_)));
    }
}
