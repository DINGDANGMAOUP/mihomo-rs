use clap::Parser;
use mihomo_rs::cli::{print_error, print_info, print_success, print_table, Cli, Commands};
use mihomo_rs::config::ConfigManager;
use mihomo_rs::connection::ConnectionManager;
use mihomo_rs::core::MihomoClient;
use mihomo_rs::proxy::ProxyManager;
use mihomo_rs::service::{ServiceManager, ServiceStatus};
use mihomo_rs::version::{Channel, VersionManager};
use std::io::{self, Write};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        print_error(&format!("Error: {}", e));
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    env_logger::Builder::from_default_env()
        .filter_level(if cli.verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .init();

    match cli.command {
        Commands::Install { version } => {
            let vm = VersionManager::new()?;
            let version = if let Some(v) = version {
                if let Ok(channel) = v.parse::<Channel>() {
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
                        cm.get_current()
                            .await
                            .unwrap_or_else(|_| "default".to_string())
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

            // Ensure default config exists
            cm.ensure_default_config().await?;

            // Ensure external-controller is configured before starting
            let controller_url = cm.ensure_external_controller().await?;
            log::info!("External controller configured at: {}", controller_url);

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

            // Ensure default config exists
            cm.ensure_default_config().await?;

            // Ensure external-controller is configured before restarting
            let controller_url = cm.ensure_external_controller().await?;
            log::info!("External controller configured at: {}", controller_url);

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
            let cm = ConfigManager::new()?;
            let url = cm.get_external_controller().await?;
            let client = MihomoClient::new(&url, None)?;
            let pm = ProxyManager::new(client.clone());

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
                                    p.delay
                                        .map(|d| format!("{}ms", d))
                                        .unwrap_or_else(|| "-".to_string()),
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

                ProxyAction::Test {
                    proxy,
                    url,
                    timeout,
                } => {
                    if let Some(proxy) = proxy {
                        let delay = client.test_delay(&proxy, &url, timeout).await?;
                        print_success(&format!("{}: {}ms", proxy, delay));
                    } else {
                        print_info("Testing all proxies...");
                        let results =
                            mihomo_rs::proxy::test_all_delays(&client, &url, timeout).await?;
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

        Commands::Logs { level } => {
            let cm = ConfigManager::new()?;
            let url = cm.get_external_controller().await?;
            let client = MihomoClient::new(&url, None)?;
            print_info("Streaming logs... (Press Ctrl+C to stop)");

            let mut rx = client.stream_logs(level.as_deref()).await?;
            while let Some(log) = rx.recv().await {
                println!("{}", log);
            }
        }

        Commands::Traffic => {
            let cm = ConfigManager::new()?;
            let url = cm.get_external_controller().await?;
            let client = MihomoClient::new(&url, None)?;
            print_info("Streaming traffic... (Press Ctrl+C to stop)");

            let mut rx = client.stream_traffic().await?;
            while let Some(traffic) = rx.recv().await {
                println!(
                    "↑ {} KB/s  ↓ {} KB/s",
                    traffic.up / 1024,
                    traffic.down / 1024
                );
            }
        }

        Commands::Memory => {
            let cm = ConfigManager::new()?;
            let url = cm.get_external_controller().await?;
            let client = MihomoClient::new(&url, None)?;

            let memory = client.get_memory().await?;
            println!("Memory Usage:");
            println!("  In Use:   {} MB", memory.in_use / 1024 / 1024);
            println!("  OS Limit: {} MB", memory.os_limit / 1024 / 1024);
        }

        Commands::Connection { action } => {
            use mihomo_rs::cli::ConnectionAction;
            let cm = ConfigManager::new()?;
            let url = cm.get_external_controller().await?;
            let client = MihomoClient::new(&url, None)?;
            let conn_mgr = ConnectionManager::new(client);

            match action {
                ConnectionAction::List => {
                    let connections = conn_mgr.list().await?;
                    if connections.is_empty() {
                        print_info("No active connections");
                    } else {
                        let rows: Vec<Vec<String>> = connections
                            .iter()
                            .map(|c| {
                                let host = if !c.metadata.host.is_empty() {
                                    c.metadata.host.clone()
                                } else {
                                    format!(
                                        "{}:{}",
                                        c.metadata.destination_ip, c.metadata.destination_port
                                    )
                                };
                                let chain = if !c.chains.is_empty() {
                                    c.chains.join(" -> ")
                                } else {
                                    "-".to_string()
                                };
                                vec![
                                    c.id[..8].to_string(),
                                    host,
                                    chain,
                                    format!("{:.1} KB", c.download as f64 / 1024.0),
                                    format!("{:.1} KB", c.upload as f64 / 1024.0),
                                ]
                            })
                            .collect();
                        print_table(&["ID", "Host", "Chain", "Download", "Upload"], rows);
                        println!("\nTotal connections: {}", connections.len());
                    }
                }

                ConnectionAction::Stats => {
                    let (download, upload, count) = conn_mgr.get_statistics().await?;
                    println!("Connection Statistics:");
                    println!("  Active Connections: {}", count);
                    println!(
                        "  Total Download:     {:.2} MB",
                        download as f64 / 1024.0 / 1024.0
                    );
                    println!(
                        "  Total Upload:       {:.2} MB",
                        upload as f64 / 1024.0 / 1024.0
                    );
                }

                ConnectionAction::Stream => {
                    print_info("Streaming connections... (Press Ctrl+C to stop)");
                    let mut rx = conn_mgr.stream().await?;
                    let mut update_count = 0;

                    while let Some(snapshot) = rx.recv().await {
                        update_count += 1;
                        println!("\n=== Update #{} ===", update_count);
                        println!(
                            "Download: {:.2} MB | Upload: {:.2} MB | Connections: {}",
                            snapshot.download_total as f64 / 1024.0 / 1024.0,
                            snapshot.upload_total as f64 / 1024.0 / 1024.0,
                            snapshot.connections.len()
                        );

                        if !snapshot.connections.is_empty() {
                            let mut sorted = snapshot.connections.clone();
                            sorted.sort_by(|a, b| {
                                (b.download + b.upload).cmp(&(a.download + a.upload))
                            });
                            println!("\nTop 3 by traffic:");
                            for (i, conn) in sorted.iter().take(3).enumerate() {
                                let host = if !conn.metadata.host.is_empty() {
                                    &conn.metadata.host
                                } else {
                                    &conn.metadata.destination_ip
                                };
                                println!(
                                    "  {}. {} - ↓{:.1}KB ↑{:.1}KB",
                                    i + 1,
                                    host,
                                    conn.download as f64 / 1024.0,
                                    conn.upload as f64 / 1024.0
                                );
                            }
                        }
                    }
                }

                ConnectionAction::Close { id } => {
                    conn_mgr.close(&id).await?;
                    print_success(&format!("Closed connection {}", &id[..8]));
                }

                ConnectionAction::CloseAll { force } => {
                    if !force {
                        print!("Are you sure you want to close all connections? [y/N]: ");
                        io::stdout().flush()?;
                        let mut input = String::new();
                        io::stdin().read_line(&mut input)?;
                        if !input.trim().eq_ignore_ascii_case("y") {
                            print_info("Cancelled");
                            return Ok(());
                        }
                    }

                    conn_mgr.close_all().await?;
                    print_success("Closed all connections");
                }

                ConnectionAction::FilterHost { host } => {
                    let connections = conn_mgr.filter_by_host(&host).await?;
                    if connections.is_empty() {
                        print_info(&format!("No connections found for host '{}'", host));
                    } else {
                        let rows: Vec<Vec<String>> = connections
                            .iter()
                            .map(|c| {
                                vec![
                                    c.id[..8].to_string(),
                                    c.metadata.host.clone(),
                                    c.chains.join(" -> "),
                                    format!("{:.1} KB", c.download as f64 / 1024.0),
                                    format!("{:.1} KB", c.upload as f64 / 1024.0),
                                ]
                            })
                            .collect();
                        print_table(&["ID", "Host", "Chain", "Download", "Upload"], rows);
                        println!("\nFound {} connection(s) for '{}'", connections.len(), host);
                    }
                }

                ConnectionAction::FilterProcess { process } => {
                    let connections = conn_mgr.filter_by_process(&process).await?;
                    if connections.is_empty() {
                        print_info(&format!("No connections found for process '{}'", process));
                    } else {
                        let rows: Vec<Vec<String>> = connections
                            .iter()
                            .map(|c| {
                                let host = if !c.metadata.host.is_empty() {
                                    c.metadata.host.clone()
                                } else {
                                    c.metadata.destination_ip.clone()
                                };
                                vec![
                                    c.id[..8].to_string(),
                                    host,
                                    c.metadata.process_path.clone(),
                                    format!("{:.1} KB", c.download as f64 / 1024.0),
                                    format!("{:.1} KB", c.upload as f64 / 1024.0),
                                ]
                            })
                            .collect();
                        print_table(&["ID", "Host", "Process", "Download", "Upload"], rows);
                        println!(
                            "\nFound {} connection(s) for process '{}'",
                            connections.len(),
                            process
                        );
                    }
                }

                ConnectionAction::CloseByHost { host, force } => {
                    let connections = conn_mgr.filter_by_host(&host).await?;
                    if connections.is_empty() {
                        print_info(&format!("No connections found for host '{}'", host));
                        return Ok(());
                    }

                    if !force {
                        print!(
                            "About to close {} connection(s) for host '{}'. Continue? [y/N]: ",
                            connections.len(),
                            host
                        );
                        io::stdout().flush()?;
                        let mut input = String::new();
                        io::stdin().read_line(&mut input)?;
                        if !input.trim().eq_ignore_ascii_case("y") {
                            print_info("Cancelled");
                            return Ok(());
                        }
                    }

                    let count = conn_mgr.close_by_host(&host).await?;
                    print_success(&format!("Closed {} connection(s) for host '{}'", count, host));
                }

                ConnectionAction::CloseByProcess { process, force } => {
                    let connections = conn_mgr.filter_by_process(&process).await?;
                    if connections.is_empty() {
                        print_info(&format!("No connections found for process '{}'", process));
                        return Ok(());
                    }

                    if !force {
                        print!(
                            "About to close {} connection(s) for process '{}'. Continue? [y/N]: ",
                            connections.len(),
                            process
                        );
                        io::stdout().flush()?;
                        let mut input = String::new();
                        io::stdin().read_line(&mut input)?;
                        if !input.trim().eq_ignore_ascii_case("y") {
                            print_info("Cancelled");
                            return Ok(());
                        }
                    }

                    let count = conn_mgr.close_by_process(&process).await?;
                    print_success(&format!(
                        "Closed {} connection(s) for process '{}'",
                        count, process
                    ));
                }
            }
        }
    }

    Ok(())
}
