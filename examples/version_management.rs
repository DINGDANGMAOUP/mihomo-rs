use mihomo_rs::{Result, VersionManager};

#[tokio::main]
async fn main() -> Result<()> {
    let vm = VersionManager::new()?;

    // List installed versions
    println!("=== Installed Versions ===");
    let versions = vm.list_installed().await?;
    if versions.is_empty() {
        println!("No versions installed");
    } else {
        for version in &versions {
            let marker = if version.is_default { "*" } else { " " };
            println!(
                "{} {} ({})",
                marker,
                version.version,
                version.path.display()
            );
        }
    }

    // List available remote versions
    println!("\n=== Available Remote Versions (latest 10) ===");
    let releases = mihomo_rs::version::fetch_releases(10).await?;
    for release in releases {
        let prerelease = if release.prerelease {
            " (prerelease)"
        } else {
            ""
        };
        println!("{} - {}{}", release.version, release.name, prerelease);
    }

    // Example: Install a specific version (commented out to avoid actual installation)
    // println!("\n=== Installing version v1.18.0 ===");
    // vm.install("v1.18.0").await?;
    // println!("✓ Installed v1.18.0");

    // Example: Install from channel
    // println!("\n=== Installing stable channel ===");
    // let version = vm.install_channel(Channel::Stable).await?;
    // println!("✓ Installed stable: {}", version);

    // Example: Set default version
    // if let Some(version) = versions.first() {
    //     println!("\n=== Setting default version ===");
    //     vm.set_default(&version.version).await?;
    //     println!("✓ Set {} as default", version.version);
    // }

    // Get binary path
    if !versions.is_empty() {
        println!("\n=== Binary Path ===");
        let binary = vm.get_binary_path(None).await?;
        println!("Default binary: {}", binary.display());
    }

    Ok(())
}
