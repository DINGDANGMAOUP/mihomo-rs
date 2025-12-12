/// Example demonstrating custom home directory in SDK usage
use mihomo_rs::{ConfigManager, Result, VersionManager};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Custom Home Directory SDK Example ===\n");

    // Method 1: Use default home (from MIHOMO_HOME env or ~/.config/mihomo-rs)
    println!("1. Using default home:");
    let vm_default = VersionManager::new()?;
    let versions = vm_default.list_installed().await?;
    println!("   Found {} versions in default home", versions.len());

    // Method 2: Specify custom home programmatically
    println!("\n2. Using custom home programmatically:");
    let custom_home = PathBuf::from("/tmp/mihomo-custom");
    let vm_custom = VersionManager::with_home(custom_home.clone())?;
    let versions_custom = vm_custom.list_installed().await?;
    println!(
        "   Found {} versions in {}",
        versions_custom.len(),
        custom_home.display()
    );

    // Method 3: Use different homes for different managers
    println!("\n3. Using different homes for different managers:");
    let home1 = PathBuf::from("/tmp/mihomo-instance1");
    let home2 = PathBuf::from("/tmp/mihomo-instance2");

    let _vm1 = VersionManager::with_home(home1.clone())?;
    let _cm1 = ConfigManager::with_home(home1.clone())?;
    println!("   Instance 1: {}", home1.display());

    let _vm2 = VersionManager::with_home(home2.clone())?;
    let _cm2 = ConfigManager::with_home(home2.clone())?;
    println!("   Instance 2: {}", home2.display());

    // This allows running multiple isolated instances
    println!("\n=== Use Cases ===");
    println!("✓ Testing without affecting production config");
    println!("✓ Running multiple isolated instances");
    println!("✓ Custom storage locations per application");
    println!("✓ Multi-tenant applications");

    println!("\n=== Code Example ===");
    println!(
        r#"
// Use custom home directory
let home = PathBuf::from("/opt/mihomo");
let vm = VersionManager::with_home(home.clone())?;
let cm = ConfigManager::with_home(home)?;

// Now all operations use the custom directory
vm.install("v1.18.0").await?;
cm.set_current("production").await?;
"#
    );

    Ok(())
}
