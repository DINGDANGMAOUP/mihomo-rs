use clap::Parser;
use mihomo_rs::cli::{print_error, print_info, print_success, print_table, Cli, Commands};
use mihomo_rs::config::ConfigManager;
use mihomo_rs::core::MihomoClient;
use mihomo_rs::proxy::ProxyManager;
use mihomo_rs::service::{ServiceManager, ServiceStatus};
use mihomo_rs::version::{Channel, VersionManager};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        print_error(&format!("Error: {}", e));
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install { version } => {
            let vm = VersionManager::new()?;
            let version = if let Some(v) = version {
                if let Some(channel) = Channel::from_str(&v) {
                    print_info(&format!("Installing {} channel...", channel.as_str()));
                    vm.install_channel(channel).await?
                } else {
                    print_info(&format!("Installing version {}...", v));
                    vm.install(&v).await?;
                    v
                }
            } else {
                print_info("Installing stable channel...");
                vm.install_channel(Channel::Stable).await?
            };
            print_success(&format!("Installed version {}", version));
        }

        Commands::Update => {
            let vm = VersionManager::new()?;
            print_info("Updating to latest stable version...");
            let version = vm.install_channel(Channel::Stable).await?;
            vm.set_default(&version).await?;
            print_success(&format!("Updated to version {}", version));
        }

        Commands::Default { version } => {
            let vm = VersionManager::new()?;
            vm.set_default(&version).await?;
            print_success(&format!("Set default version to {}", version));
        }

        Commands::List => {
            let vm = VersionManager::new()?;
            let versions = vm.list_installed().await?;
            if versions.is_empty() {
                print_info("No versions installed");
            } else {
                let rows: Vec<Vec<String>> = versions
                    .iter()
                    .map(|v| {
                        vec![
                            if v.is_default { "* " } else { "  " }.to_string() + &v.version,
                            v.path.display().to_string(),
                        ]
                    })
                    .collect();
                print_table(&["Version", "Path"], rows);
            }
        }

        Commands::ListRemote { limit } => {
            print_info(&format!("Fetching {} latest releases...", limit));
            let releases = mihomo_rs::version::fetch_releases(limit).await?;
            if releases.is_empty() {
                print_info("No releases found");
            } else {
                let rows: Vec<Vec<String>> = releases
                    .iter()
                    .map(|r| {
                        vec![
                            r.version.clone(),
                            r.name.clone(),
                            if r.prerelease { "Yes" } else { "No" }.to_string(),
                            r.published_at[..10].to_string(),
                        ]
                    })
                    .collect();
                print_table(&["Version", "Name", "Prerelease", "Date"], rows);
            }
        }

        Commands::Uninstall { version } => {
            let vm = VersionManager::new()?;
            vm.uninstall(&version).await?;
            print_success(&format!("Uninstalled version {}", version));
        }

        Commands::Config { action } => {
            use mihomo_rs::cli::ConfigAction;
            let cm = ConfigManager::new()?;

            match action {
                ConfigAction::List => {
                    let profiles = cm.list_profiles().await?;
                    if profiles.is_empty() {
                        print_info("No profiles found");
                    } else {
                        let rows: Vec<Vec<String>> = profiles
                            .iter()
                            .map(|p| {
                                vec![
                                    if p.active { "* " } else { "  " }.to_string() + &p.name,
                                    p.path.display().to_string(),
                                ]
                            })
                            .collect();
                        print_table(&["Profile", "Path"], rows);
                    }
                }

                ConfigAction::Use { profile } => {
                    cm.set_current(&profile).await?;
                    print_success(&format!("Switched to profile '{}'", profile));
                }

                ConfigAction::Show { profile } => {
                    let profile = if let Some(p) = profile {
                        p
                    } else {
                        cm.get_current().await.unwrap_or_else(|_| "default".to_string())
                    };
                    let content = cm.load(&profile).await?;
                    println!("{}", content);
                }

                ConfigAction::Delete { profile } => {
                    cm.delete_profile(&profile).await?;
                    print_success(&format!("Deleted profile '{}'", profile));
                }
            }
        }

        Commands::Start => {
            let vm = VersionManager::new()?;
            let cm = ConfigManager::new()?;
            let binary = vm.get_binary_path(None).await?;
            let config = cm.get_current_path().await?;
            let sm = ServiceManager::new(binary, config);
            sm.start().await?;
            print_success("Service started");
        }

        Commands::Stop => {
            let vm = VersionManager::new()?;
            let cm = ConfigManager::new()?;
            let binary = vm.get_binary_path(None).await?;
            let config = cm.get_current_path().await?;
            let sm = ServiceManager::new(binary, config);
            sm.stop().await?;
            print_success("Service stopped");
        }

        Commands::Restart => {
            let vm = VersionManager::new()?;
            let cm = ConfigManager::new()?;
            let binary = vm.get_binary_path(None).await?;
            let config = cm.get_current_path().await?;
            let sm = ServiceManager::new(binary, config);
            sm.restart().await?;
            print_success("Service restarted");
        }

        Commands::Status => {
            let vm = VersionManager::new()?;
            let cm = ConfigManager::new()?;
            let binary = vm.get_binary_path(None).await?;
            let config = cm.get_current_path().await?;
            let sm = ServiceManager::new(binary, config);
            match sm.status().await? {
                ServiceStatus::Running(pid) => {
                    print_success(&format!("Service is running (PID: {})", pid));
                }
                ServiceStatus::Stopped => {
                    print_info("Service is stopped");
                }
            }
        }

        Commands::Proxy { action } => {
            use mihomo_rs::cli::ProxyAction;
            let client = MihomoClient::new("http://127.0.0.1:9090", None)?;
            let pm = ProxyManager::new(client);

            match action {
                ProxyAction::List => {
                    let proxies = pm.list_proxies().await?;
                    if proxies.is_empty() {
                        print_info("No proxies found");
                    } else {
                        let rows: Vec<Vec<String>> = proxies
                            .iter()
                            .map(|p| {
                                vec![
                                    p.name.clone(),
                                    p.proxy_type.clone(),
                                    p.delay.map(|d| format!("{}ms", d)).unwrap_or_else(|| "-".to_string()),
                                ]
                            })
                            .collect();
                        print_table(&["Name", "Type", "Delay"], rows);
                    }
                }

                ProxyAction::Groups => {
                    let groups = pm.list_groups().await?;
                    if groups.is_empty() {
                        print_info("No groups found");
                    } else {
                        let rows: Vec<Vec<String>> = groups
                            .iter()
                            .map(|g| {
                                vec![
                                    g.name.clone(),
                                    g.group_type.clone(),
                                    g.now.clone(),
                                    g.all.len().to_string(),
                                ]
                            })
                            .collect();
                        print_table(&["Name", "Type", "Current", "Total"], rows);
                    }
                }

                ProxyAction::Switch { group, proxy } => {
                    pm.switch(&group, &proxy).await?;
                    print_success(&format!("Switched {} to {}", group, proxy));
                }

                ProxyAction::Test { proxy, url, timeout } => {
                    if let Some(proxy) = proxy {
                        let client = MihomoClient::new("http://127.0.0.1:9090", None)?;
                        let delay = client.test_delay(&proxy, &url, timeout).await?;
                        print_success(&format!("{}: {}ms", proxy, delay));
                    } else {
                        print_info("Testing all proxies...");
                        let client = MihomoClient::new("http://127.0.0.1:9090", None)?;
                        let results = mihomo_rs::proxy::test_all_delays(&client, &url, timeout).await?;
                        let mut rows: Vec<Vec<String>> = results
                            .iter()
                            .map(|(name, delay)| vec![name.clone(), format!("{}ms", delay)])
                            .collect();
                        rows.sort_by(|a, b| a[0].cmp(&b[0]));
                        print_table(&["Proxy", "Delay"], rows);
                    }
                }

                ProxyAction::Current => {
                    let groups = pm.list_groups().await?;
                    if groups.is_empty() {
                        print_info("No groups found");
                    } else {
                        let rows: Vec<Vec<String>> = groups
                            .iter()
                            .map(|g| vec![g.name.clone(), g.now.clone()])
                            .collect();
                        print_table(&["Group", "Current Proxy"], rows);
                    }
                }
            }
        }
    }

    Ok(())
}
