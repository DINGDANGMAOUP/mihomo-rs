use crate::cli::{print_info, print_success, print_table, ConnectionAction};
use crate::config::ConfigManager;
use crate::connection::ConnectionManager;
use crate::core::{Connection, MihomoClient};
use anyhow::bail;
use std::cmp::Reverse;
use std::io::{self, Write};

enum CloseTarget {
    Id(String),
    All,
    Host(String),
    Process(String),
}

pub async fn handle_connection(action: ConnectionAction) -> anyhow::Result<()> {
    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;
    let conn_mgr = ConnectionManager::new(client);

    match action {
        ConnectionAction::List { host, process } => {
            let connections =
                load_connections(&conn_mgr, host.as_deref(), process.as_deref()).await?;
            render_connection_list(&connections, host.as_deref(), process.as_deref());
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
                    sorted
                        .sort_by_key(|connection| Reverse(connection.download + connection.upload));
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
        ConnectionAction::Close {
            legacy_id,
            id,
            all,
            host,
            process,
            force,
        } => {
            let target = parse_close_target(legacy_id, id, all, host, process)?;
            execute_close(&conn_mgr, target, force).await?;
        }
        ConnectionAction::CloseAll { force } => {
            execute_close(&conn_mgr, CloseTarget::All, force).await?;
        }
        ConnectionAction::FilterHost { host } => {
            let connections = load_connections(&conn_mgr, Some(&host), None).await?;
            render_connection_list(&connections, Some(&host), None);
        }
        ConnectionAction::FilterProcess { process } => {
            let connections = load_connections(&conn_mgr, None, Some(&process)).await?;
            render_connection_list(&connections, None, Some(&process));
        }
        ConnectionAction::CloseByHost { host, force } => {
            execute_close(&conn_mgr, CloseTarget::Host(host), force).await?;
        }
        ConnectionAction::CloseByProcess { process, force } => {
            execute_close(&conn_mgr, CloseTarget::Process(process), force).await?;
        }
    }

    Ok(())
}

fn connection_host_label(connection: &Connection) -> String {
    if !connection.metadata.host.is_empty() {
        connection.metadata.host.clone()
    } else {
        format!(
            "{}:{}",
            connection.metadata.destination_ip, connection.metadata.destination_port
        )
    }
}

fn connection_chain_label(connection: &Connection) -> String {
    if !connection.chains.is_empty() {
        connection.chains.join(" -> ")
    } else {
        "-".to_string()
    }
}

async fn load_connections(
    conn_mgr: &ConnectionManager,
    host: Option<&str>,
    process: Option<&str>,
) -> crate::core::Result<Vec<Connection>> {
    let mut connections = conn_mgr.list().await?;
    if let Some(host_filter) = host {
        connections.retain(|c| {
            c.metadata.host.contains(host_filter) || c.metadata.destination_ip.contains(host_filter)
        });
    }
    if let Some(process_filter) = process {
        connections.retain(|c| c.metadata.process_path.contains(process_filter));
    }
    Ok(connections)
}

fn render_connection_list(connections: &[Connection], host: Option<&str>, process: Option<&str>) {
    if connections.is_empty() {
        match (host, process) {
            (Some(host), Some(process)) => print_info(&format!(
                "No connections found for host '{}' and process '{}'",
                host, process
            )),
            (Some(host), None) => print_info(&format!("No connections found for host '{}'", host)),
            (None, Some(process)) => {
                print_info(&format!("No connections found for process '{}'", process))
            }
            (None, None) => print_info("No active connections"),
        }
        return;
    }

    if process.is_some() {
        let rows: Vec<Vec<String>> = connections
            .iter()
            .map(|c| {
                vec![
                    super::truncate_for_display(&c.id, 8),
                    connection_host_label(c),
                    c.metadata.process_path.clone(),
                    format!("{:.1} KB", c.download as f64 / 1024.0),
                    format!("{:.1} KB", c.upload as f64 / 1024.0),
                ]
            })
            .collect();
        print_table(&["ID", "Host", "Process", "Download", "Upload"], rows);
    } else {
        let rows: Vec<Vec<String>> = connections
            .iter()
            .map(|c| {
                vec![
                    super::truncate_for_display(&c.id, 8),
                    connection_host_label(c),
                    connection_chain_label(c),
                    format!("{:.1} KB", c.download as f64 / 1024.0),
                    format!("{:.1} KB", c.upload as f64 / 1024.0),
                ]
            })
            .collect();
        print_table(&["ID", "Host", "Chain", "Download", "Upload"], rows);
    }

    match (host, process) {
        (Some(host), Some(process)) => println!(
            "\nFound {} connection(s) for host '{}' and process '{}'",
            connections.len(),
            host,
            process
        ),
        (Some(host), None) => {
            println!("\nFound {} connection(s) for '{}'", connections.len(), host)
        }
        (None, Some(process)) => println!(
            "\nFound {} connection(s) for process '{}'",
            connections.len(),
            process
        ),
        (None, None) => println!("\nTotal connections: {}", connections.len()),
    }
}

fn parse_close_target(
    legacy_id: Option<String>,
    id: Option<String>,
    all: bool,
    host: Option<String>,
    process: Option<String>,
) -> anyhow::Result<CloseTarget> {
    let selected = legacy_id.is_some() as u8
        + id.is_some() as u8
        + all as u8
        + host.is_some() as u8
        + process.is_some() as u8;
    if selected != 1 {
        bail!("Specify exactly one of ID, --id, --all, --host, or --process");
    }

    if let Some(id) = legacy_id.or(id) {
        return Ok(CloseTarget::Id(id));
    }
    if all {
        return Ok(CloseTarget::All);
    }
    if let Some(host) = host {
        return Ok(CloseTarget::Host(host));
    }
    if let Some(process) = process {
        return Ok(CloseTarget::Process(process));
    }

    bail!("Specify exactly one of ID, --id, --all, --host, or --process");
}

fn confirm(prompt: &str) -> anyhow::Result<bool> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().eq_ignore_ascii_case("y"))
}

async fn execute_close(
    conn_mgr: &ConnectionManager,
    target: CloseTarget,
    force: bool,
) -> anyhow::Result<()> {
    match target {
        CloseTarget::Id(id) => {
            conn_mgr.close(&id).await?;
            print_success(&format!(
                "Closed connection {}",
                super::truncate_for_display(&id, 8)
            ));
        }
        CloseTarget::All => {
            if !force && !confirm("Are you sure you want to close all connections? [y/N]: ")? {
                print_info("Cancelled");
                return Ok(());
            }
            conn_mgr.close_all().await?;
            print_success("Closed all connections");
        }
        CloseTarget::Host(host) => {
            let connections = load_connections(conn_mgr, Some(&host), None).await?;
            if connections.is_empty() {
                print_info(&format!("No connections found for host '{}'", host));
                return Ok(());
            }
            if !force
                && !confirm(&format!(
                    "About to close {} connection(s) for host '{}'. Continue? [y/N]: ",
                    connections.len(),
                    host
                ))?
            {
                print_info("Cancelled");
                return Ok(());
            }
            let count = conn_mgr.close_by_host(&host).await?;
            print_success(&format!(
                "Closed {} connection(s) for host '{}'",
                count, host
            ));
        }
        CloseTarget::Process(process) => {
            let connections = load_connections(conn_mgr, None, Some(&process)).await?;
            if connections.is_empty() {
                print_info(&format!("No connections found for process '{}'", process));
                return Ok(());
            }
            if !force
                && !confirm(&format!(
                    "About to close {} connection(s) for process '{}'. Continue? [y/N]: ",
                    connections.len(),
                    process
                ))?
            {
                print_info("Cancelled");
                return Ok(());
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

#[cfg(test)]
mod tests {
    use super::{parse_close_target, CloseTarget};

    #[test]
    fn parse_close_target_accepts_new_and_legacy_forms() {
        match parse_close_target(Some("legacy-id".to_string()), None, false, None, None)
            .expect("legacy id should parse")
        {
            CloseTarget::Id(id) => assert_eq!(id, "legacy-id"),
            _ => panic!("expected id target"),
        }

        match parse_close_target(None, Some("flag-id".to_string()), false, None, None)
            .expect("flag id should parse")
        {
            CloseTarget::Id(id) => assert_eq!(id, "flag-id"),
            _ => panic!("expected id target"),
        }

        assert!(matches!(
            parse_close_target(None, None, true, None, None).expect("all should parse"),
            CloseTarget::All
        ));
        assert!(matches!(
            parse_close_target(None, None, false, Some("example".to_string()), None)
                .expect("host should parse"),
            CloseTarget::Host(_)
        ));
        assert!(matches!(
            parse_close_target(None, None, false, None, Some("curl".to_string()))
                .expect("process should parse"),
            CloseTarget::Process(_)
        ));
    }

    #[test]
    fn parse_close_target_rejects_missing_or_ambiguous_selection() {
        assert!(parse_close_target(None, None, false, None, None).is_err());
        assert!(parse_close_target(None, Some("id".to_string()), true, None, None).is_err());
        assert!(parse_close_target(
            Some("legacy".to_string()),
            None,
            false,
            Some("example".to_string()),
            None,
        )
        .is_err());
    }
}
