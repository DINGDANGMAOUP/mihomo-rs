#[cfg(unix)]
mod unix_tests {
    use mihomo_rs::{start_service, stop_service, switch_proxy};
    use mockito::{Matcher, Server};
    use std::env;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;
    use tokio::fs;

    async fn write_fake_daemon(binary: &std::path::Path) {
        let script = r#"#!/bin/sh
trap 'exit 0' TERM INT
while true; do :; done
"#;
        fs::write(binary, script).await.expect("write fake daemon");

        let mut perms = fs::metadata(binary)
            .await
            .expect("read daemon metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(binary, perms)
            .await
            .expect("set execute permission");
    }

    #[tokio::test]
    async fn top_level_start_stop_and_switch_proxy_success_paths() {
        let mut server = Server::new_async().await;
        let switch_mock = server
            .mock("PUT", "/proxies/GLOBAL")
            .match_header(
                "content-type",
                Matcher::Regex("application/json".to_string()),
            )
            .match_body(Matcher::JsonString(r#"{"name":"DIRECT"}"#.to_string()))
            .with_status(204)
            .create_async()
            .await;

        let temp = tempdir().expect("create temp dir");
        let home = temp.path();
        let versions = home.join("versions").join("v-test");
        let configs = home.join("configs");
        fs::create_dir_all(&versions)
            .await
            .expect("create versions dir");
        fs::create_dir_all(&configs)
            .await
            .expect("create configs dir");

        let binary = versions.join("mihomo");
        write_fake_daemon(&binary).await;

        let config_path = configs.join("default.yaml");
        fs::write(
            &config_path,
            format!("port: 7890\nexternal-controller: {}\n", server.url()),
        )
        .await
        .expect("write profile config");

        fs::write(
            home.join("config.toml"),
            "[default]\nversion = \"v-test\"\nprofile = \"default\"\n",
        )
        .await
        .expect("write mihomo-rs config");

        let old_home = env::var("MIHOMO_HOME").ok();
        env::set_var("MIHOMO_HOME", home);

        start_service(&config_path)
            .await
            .expect("top-level start_service should succeed");

        switch_proxy("GLOBAL", "DIRECT")
            .await
            .expect("top-level switch_proxy should succeed");
        switch_mock.assert_async().await;

        stop_service(&config_path)
            .await
            .expect("top-level stop_service should succeed");

        if let Some(prev) = old_home {
            env::set_var("MIHOMO_HOME", prev);
        } else {
            env::remove_var("MIHOMO_HOME");
        }
    }
}
