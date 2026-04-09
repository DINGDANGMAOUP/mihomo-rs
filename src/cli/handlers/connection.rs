use crate::cli::{print_info, print_success, print_table, ConnectionAction};
use crate::config::ConfigManager;
use crate::connection::ConnectionManager;
use crate::core::MihomoClient;
use std::io::{self, Write};

pub async fn handle_connection(action: ConnectionAction) -> anyhow::Result<()> {
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
                            super::truncate_for_display(&c.id, 8),
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
                    sorted.sort_by(|a, b| (b.download + b.upload).cmp(&(a.download + a.upload)));
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
            print_success(&format!(
                "Closed connection {}",
                super::truncate_for_display(&id, 8)
            ));
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
                            super::truncate_for_display(&c.id, 8),
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
                            super::truncate_for_display(&c.id, 8),
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
            print_success(&format!(
                "Closed {} connection(s) for host '{}'",
                count, host
            ));
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

    Ok(())
}
