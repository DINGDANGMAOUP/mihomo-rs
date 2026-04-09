use crate::cli::{print_info, print_success, print_table, ProxyAction};
use crate::config::ConfigManager;
use crate::core::MihomoClient;
use crate::proxy::ProxyManager;

pub async fn handle_proxy(action: ProxyAction) -> anyhow::Result<()> {
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
                let results = crate::proxy::test_all_delays(&client, &url, timeout).await?;
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

    Ok(())
}
