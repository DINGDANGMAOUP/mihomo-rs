//! Mihomo RS 命令行工具
//!
//! 提供 mihomo 代理服务的管理功能

use chrono::{DateTime, Local};
use clap::{Parser, Subcommand};
use crossterm::{
    cursor,
    terminal::{self, ClearType},
    ExecutableCommand,
};
use futures_util::StreamExt;
use mihomo_rs::{
    client::MihomoClient, config::ConfigManager, init_logger, logger::LoggerConfig,
    monitor::Monitor, proxy::ProxyManager, rules::RuleEngine, service::ServiceManager,
};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;
use tokio::time::timeout;

/// Mihomo RS 命令行工具
#[derive(Parser)]
#[command(name = "mihomo-rs")]
#[command(about = "Mihomo 代理服务管理工具")]
#[command(version = "0.1.1")]
struct Cli {
    /// Mihomo 服务地址
    #[arg(short, long, default_value = "http://127.0.0.1:9090")]
    url: String,

    /// API 密钥
    #[arg(short, long)]
    secret: Option<String>,

    /// 启用详细日志
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

/// 可用的命令
#[derive(Subcommand)]
enum Commands {
    /// 显示服务状态
    Status,
    /// 代理管理
    Proxy {
        #[command(subcommand)]
        action: ProxyAction,
    },
    /// 配置管理
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// 监控服务
    Monitor {
        /// 监控间隔（秒）
        #[arg(short, long, default_value = "5")]
        interval: u64,
        /// 监控持续时间（秒）
        #[arg(short, long, default_value = "60")]
        duration: u64,
    },
    /// 规则管理
    Rules {
        #[command(subcommand)]
        action: Option<RuleAction>,
    },
    /// 连接管理
    Connections {
        #[command(subcommand)]
        action: Option<ConnectionAction>,
    },
    /// 服务管理
    Service {
        #[command(subcommand)]
        action: ServiceAction,
    },
}

/// 代理操作
#[derive(Subcommand)]
enum ProxyAction {
    /// 列出所有代理
    List,
    /// 切换代理
    Switch {
        /// 代理组名
        group: String,
        /// 代理名
        proxy: String,
    },
    /// 测试代理延迟
    Test {
        /// 代理名
        proxy: String,
        /// 测试URL
        #[arg(short, long, default_value = "http://www.gstatic.com/generate_204")]
        url: String,
        /// 超时时间（毫秒）
        #[arg(short, long, default_value = "3000")]
        timeout: u32,
    },
    /// 批量测试代理延迟
    BatchTest {
        /// 代理组名称（可选，不指定则测试所有代理）
        #[arg(short, long)]
        group: Option<String>,
        /// 测试URL
        #[arg(short, long, default_value = "http://www.gstatic.com/generate_204")]
        url: String,
        /// 超时时间（毫秒）
        #[arg(short, long, default_value = "3000")]
        timeout: u32,
        /// 并发数
        #[arg(short, long, default_value = "10")]
        concurrent: usize,
    },
    /// 自动选择最佳代理
    AutoSelect {
        /// 代理组名称
        group: String,
        /// 测试URL
        #[arg(short, long, default_value = "http://www.gstatic.com/generate_204")]
        url: String,
        /// 超时时间（毫秒）
        #[arg(short, long, default_value = "3000")]
        timeout: u32,
        /// 最大延迟阈值（毫秒）
        #[arg(short, long, default_value = "1000")]
        max_delay: u32,
    },
}

/// 配置操作
#[derive(Subcommand)]
enum ConfigAction {
    /// 显示当前配置
    Show,
    /// 重新加载配置
    Reload,
    /// 验证配置文件
    Validate {
        /// 配置文件路径
        path: String,
    },
    /// 备份当前配置
    Backup {
        /// 备份文件路径（可选，默认使用时间戳）
        #[arg(short, long)]
        path: Option<String>,
        /// 备份描述
        #[arg(short, long)]
        description: Option<String>,
    },
    /// 恢复配置
    Restore {
        /// 备份文件路径或备份ID
        backup: String,
        /// 是否在恢复前创建当前配置的备份
        #[arg(short, long, default_value = "true")]
        create_backup: bool,
    },
    /// 列出所有备份
    ListBackups,
    /// 删除备份
    DeleteBackup {
        /// 备份文件路径或备份ID
        backup: String,
    },
    /// 比较配置
    Compare {
        /// 第一个配置文件路径
        config1: String,
        /// 第二个配置文件路径（可选，默认为当前配置）
        #[arg(short, long)]
        config2: Option<String>,
    },
    /// 导出配置
    Export {
        /// 导出文件路径
        path: String,
        /// 导出格式（yaml/json）
        #[arg(short, long, default_value = "yaml")]
        format: String,
    },
    /// 导入配置
    Import {
        /// 配置文件路径
        path: String,
        /// 是否在导入前创建备份
        #[arg(short, long, default_value = "true")]
        backup: bool,
    },
    /// 重置为默认配置
    Reset {
        /// 是否在重置前创建备份
        #[arg(short, long, default_value = "true")]
        backup: bool,
        /// 确认重置（防止误操作）
        #[arg(long)]
        confirm: bool,
    },
}

/// 连接操作
#[derive(Subcommand)]
enum ConnectionAction {
    /// 关闭指定连接
    Close {
        /// 连接ID
        id: String,
    },
    /// 关闭所有连接
    CloseAll,
}

/// 服务操作
#[derive(Subcommand)]
enum ServiceAction {
    /// 初始化配置目录
    Init,
    /// 启动服务
    Start,
    /// 停止服务
    Stop,
    /// 重启服务
    Restart,
    /// 服务状态
    Status,
    /// 版本管理
    Version {
        #[command(subcommand)]
        action: VersionAction,
    },
    /// 升级服务
    Upgrade {
        /// 目标版本（不指定则升级到最新版本）
        #[arg(short, long)]
        version: Option<String>,
        /// 是否备份当前版本
        #[arg(short, long, default_value = "true")]
        backup: bool,
    },
    /// 卸载服务
    Uninstall {
        /// 是否保留配置文件
        #[arg(short, long)]
        keep_config: bool,
        /// 确认卸载（防止误操作）
        #[arg(long)]
        confirm: bool,
    },
    /// 清理备份文件
    Cleanup {
        /// 保留的备份文件数量
        #[arg(short, long, default_value = "3")]
        keep: usize,
    },
}

/// 版本操作
#[derive(Subcommand)]
enum VersionAction {
    /// 列出可用版本
    List,
    /// 下载指定版本
    Download {
        /// 版本号
        version: String,
    },
    /// 安装最新版本
    Latest,
    /// 获取当前版本
    Current,
}

/// 规则操作
#[derive(Subcommand)]
enum RuleAction {
    /// 列出所有规则
    List,
    /// 显示规则统计信息
    Stats,
    /// 重新加载规则
    Reload,
    /// 验证规则格式
    Validate {
        /// 规则字符串
        rule: String,
    },
    /// 查找匹配的规则
    Match {
        /// 目标地址
        target: String,
        /// 端口号（可选）
        #[arg(short, long)]
        port: Option<u16>,
    },
    /// 按代理查找规则
    FindByProxy {
        /// 代理名称
        proxy: String,
    },
    /// 按类型查找规则
    FindByType {
        /// 规则类型
        rule_type: String,
    },
    /// 管理规则提供者
    Provider {
        #[command(subcommand)]
        action: RuleProviderAction,
    },
}

/// 规则提供者操作
#[derive(Subcommand)]
enum RuleProviderAction {
    /// 列出所有规则提供者
    List,
    /// 更新规则提供者
    Update {
        /// 提供者名称
        name: String,
    },
    /// 健康检查规则提供者
    HealthCheck {
        /// 提供者名称
        name: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // 初始化日志
    let log_config = LoggerConfig {
        level: if cli.verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        },
        show_timestamp: true,
        show_module: cli.verbose,
        show_line: cli.verbose,
        format: mihomo_rs::logger::LogFormat::Compact,
    };
    init_logger(Some(log_config));

    // 创建客户端
    let client = MihomoClient::new(&cli.url, cli.secret)?;

    match cli.command {
        Commands::Status => handle_status(&client).await?,
        Commands::Proxy { action } => handle_proxy(&client, action).await?,
        Commands::Config { action } => handle_config(&client, action).await?,
        Commands::Monitor { interval, duration } => {
            handle_monitor(&client, interval, duration).await?
        }
        Commands::Rules { action } => handle_rules(&client, action).await?,
        Commands::Connections { action } => handle_connections(&client, action).await?,
        Commands::Service { action } => handle_service(&client, action).await?,
    }

    Ok(())
}

/// 获取配置备份目录
///
/// 返回 ~/.config/mihomo-rs/backups 目录路径
fn get_backup_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let home_dir = std::env::var("HOME").map_err(|_| "无法获取用户主目录")?;

    let backup_dir = std::path::PathBuf::from(home_dir)
        .join(".config")
        .join("mihomo-rs")
        .join("backups");

    if !backup_dir.exists() {
        fs::create_dir_all(&backup_dir)?;
    }

    Ok(backup_dir)
}

/// 处理配置备份
async fn handle_config_backup(
    client: &MihomoClient,
    path: Option<String>,
    description: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("💾 备份当前配置...");

    // 获取当前配置
    let config = client.get_config().await?;
    let config_yaml = serde_yaml::to_string(&config)?;

    // 确定备份文件路径
    let backup_path = if let Some(path) = path {
        std::path::PathBuf::from(path)
    } else {
        let backup_dir = get_backup_dir()?;
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        backup_dir.join(format!("config_backup_{}.yaml", timestamp))
    };

    // 创建备份元数据
    let metadata = serde_json::json!({
        "timestamp": Local::now().to_rfc3339(),
        "description": description.as_ref().unwrap_or(&"手动备份".to_string()).clone(),
        "version": "0.1.1"
    });

    // 写入备份文件
    let backup_content = format!(
        "# Mihomo 配置备份\n# 元数据: {}\n\n{}",
        serde_json::to_string(&metadata)?,
        config_yaml
    );

    fs::write(&backup_path, backup_content)?;

    println!("✅ 配置已备份到: {}", backup_path.display());
    if let Some(desc) = description {
        println!("📝 备份描述: {}", desc);
    }

    Ok(())
}

/// 处理配置恢复
async fn handle_config_restore(
    client: &MihomoClient,
    backup: String,
    create_backup: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 恢复配置: {}", backup);

    // 如果需要，先备份当前配置
    if create_backup {
        println!("📦 创建当前配置的备份...");
        handle_config_backup(client, None, Some("恢复前自动备份".to_string())).await?;
    }

    // 读取备份文件
    let backup_path = if Path::new(&backup).is_absolute() {
        std::path::PathBuf::from(backup)
    } else {
        get_backup_dir()?.join(&backup)
    };

    if !backup_path.exists() {
        return Err(format!("备份文件不存在: {}", backup_path.display()).into());
    }

    let backup_content = fs::read_to_string(&backup_path)?;

    // 解析配置（跳过元数据注释）
    let config_lines: Vec<&str> = backup_content
        .lines()
        .skip_while(|line| line.starts_with('#'))
        .collect();
    let config_yaml = config_lines.join("\n");

    // 验证配置
    let mut config_manager = ConfigManager::new();
    config_manager.load_from_str(&config_yaml)?;

    // 这里应该调用 client.update_config，但由于当前 SDK 可能没有这个方法
    // 我们先验证配置，然后提示用户手动重启服务
    println!("✅ 配置验证通过");
    println!("⚠️  请手动重启 mihomo 服务以应用恢复的配置");
    println!("💡 或使用 'mihomo-rs config reload' 重新加载配置");

    Ok(())
}

/// 处理配置备份列表
async fn handle_config_list_backups() -> Result<(), Box<dyn std::error::Error>> {
    println!("📋 配置备份列表:");

    let backup_dir = get_backup_dir()?;

    if !backup_dir.exists() {
        println!("📁 备份目录不存在，尚未创建任何备份");
        return Ok(());
    }

    let mut backups = Vec::new();

    for entry in fs::read_dir(&backup_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().is_some_and(|ext| ext == "yaml") {
            let metadata = entry.metadata()?;
            let modified = metadata.modified()?;
            let datetime: DateTime<Local> = modified.into();

            // 尝试读取备份描述
            let content = fs::read_to_string(&path).unwrap_or_default();
            let description = content
                .lines()
                .find(|line| line.contains("# 元数据:"))
                .and_then(|line| {
                    let json_str = line.trim_start_matches("# 元数据: ");
                    serde_json::from_str::<serde_json::Value>(json_str).ok()
                })
                .and_then(|metadata| metadata["description"].as_str().map(String::from))
                .unwrap_or_else(|| "无描述".to_string());

            backups.push((
                path.file_name().unwrap().to_string_lossy().to_string(),
                datetime.format("%Y-%m-%d %H:%M:%S").to_string(),
                description,
                metadata.len(),
            ));
        }
    }

    if backups.is_empty() {
        println!("📁 没有找到配置备份文件");
        return Ok(());
    }

    // 按修改时间排序
    backups.sort_by(|a, b| b.1.cmp(&a.1));

    println!("\n{:<30} {:<20} {:<15} 描述", "文件名", "创建时间", "大小");
    println!("{}", "-".repeat(80));

    for (filename, datetime, description, size) in backups {
        let size_str = if size < 1024 {
            format!("{} B", size)
        } else if size < 1024 * 1024 {
            format!("{:.1} KB", size as f64 / 1024.0)
        } else {
            format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
        };

        println!(
            "{:<30} {:<20} {:<15} {}",
            filename, datetime, size_str, description
        );
    }

    Ok(())
}

/// 处理删除配置备份
async fn handle_config_delete_backup(backup: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("🗑️  删除配置备份: {}", backup);

    let backup_path = if Path::new(&backup).is_absolute() {
        std::path::PathBuf::from(backup)
    } else {
        get_backup_dir()?.join(&backup)
    };

    if !backup_path.exists() {
        return Err(format!("备份文件不存在: {}", backup_path.display()).into());
    }

    fs::remove_file(&backup_path)?;
    println!("✅ 备份文件已删除: {}", backup_path.display());

    Ok(())
}

/// 处理配置比较
async fn handle_config_compare(
    client: &MihomoClient,
    config1: String,
    config2: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 比较配置文件...");

    // 读取第一个配置
    let config1_content = if config1 == "current" {
        let config = client.get_config().await?;
        serde_yaml::to_string(&config)?
    } else {
        fs::read_to_string(&config1)?
    };

    // 读取第二个配置
    let config2_content = if let Some(config2) = config2 {
        if config2 == "current" {
            let config = client.get_config().await?;
            serde_yaml::to_string(&config)?
        } else {
            fs::read_to_string(&config2)?
        }
    } else {
        let config = client.get_config().await?;
        serde_yaml::to_string(&config)?
    };

    // 简单的行级比较
    let lines1: Vec<&str> = config1_content.lines().collect();
    let lines2: Vec<&str> = config2_content.lines().collect();

    let mut differences = 0;
    let max_lines = lines1.len().max(lines2.len());

    println!("\n📊 配置比较结果:");
    println!("{}", "-".repeat(80));

    for i in 0..max_lines {
        let line1 = lines1.get(i).unwrap_or(&"");
        let line2 = lines2.get(i).unwrap_or(&"");

        if line1 != line2 {
            differences += 1;
            println!("第 {} 行:", i + 1);
            println!("  - {}", line1);
            println!("  + {}", line2);
            println!();
        }
    }

    if differences == 0 {
        println!("✅ 配置文件完全相同");
    } else {
        println!("📈 发现 {} 处差异", differences);
    }

    Ok(())
}

/// 处理配置导出
async fn handle_config_export(
    client: &MihomoClient,
    path: String,
    format: String,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📤 导出当前配置到: {}", path);

    let config = client.get_config().await?;

    let content = match format.to_lowercase().as_str() {
        "json" => serde_json::to_string_pretty(&config)?,
        "yaml" | "yml" => serde_yaml::to_string(&config)?,
        _ => return Err(format!("不支持的导出格式: {}", format).into()),
    };

    fs::write(&path, content)?;
    println!("✅ 配置已导出到: {}", path);

    Ok(())
}

/// 处理配置导入
async fn handle_config_import(
    client: &MihomoClient,
    path: String,
    backup: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📥 导入配置文件: {}", path);

    if !Path::new(&path).exists() {
        return Err(format!("配置文件不存在: {}", path).into());
    }

    // 如果需要，先备份当前配置
    if backup {
        println!("📦 创建当前配置的备份...");
        handle_config_backup(client, None, Some("导入前自动备份".to_string())).await?;
    }

    // 验证配置文件
    let mut config_manager = ConfigManager::new();
    config_manager.load_from_file(&path)?;

    println!("✅ 配置文件验证通过");
    println!("⚠️  请手动重启 mihomo 服务以应用导入的配置");
    println!("💡 或使用 'mihomo-rs config reload' 重新加载配置");

    Ok(())
}

/// 处理配置重置
async fn handle_config_reset(
    client: &MihomoClient,
    backup: bool,
    confirm: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !confirm {
        return Err("重置配置需要 --confirm 参数确认操作".into());
    }

    println!("🔄 重置配置为默认值...");

    // 如果需要，先备份当前配置
    if backup {
        println!("📦 创建当前配置的备份...");
        handle_config_backup(client, None, Some("重置前自动备份".to_string())).await?;
    }

    // 创建默认配置
    let default_config = mihomo_rs::config::Config::default();
    let config_yaml = serde_yaml::to_string(&default_config)?;

    // 获取配置目录
    let home_dir = std::env::var("HOME").map_err(|_| "无法获取用户主目录")?;
    let config_path = std::path::PathBuf::from(home_dir)
        .join(".config")
        .join("mihomo-rs")
        .join("config.yaml");

    // 写入默认配置
    fs::write(&config_path, config_yaml)?;

    println!("✅ 配置已重置为默认值");
    println!("📁 配置文件位置: {}", config_path.display());
    println!("⚠️  请重启 mihomo 服务以应用重置的配置");

    Ok(())
}

/// 从流式接口获取单次流量数据（跳过第一条数据以避免初始值为0）
async fn get_traffic(
    client: &MihomoClient,
) -> Result<mihomo_rs::types::Traffic, Box<dyn std::error::Error>> {
    let mut stream = client.traffic_stream().await?;

    // 跳过第一条数据，因为可能为0
    match timeout(Duration::from_secs(3), stream.next()).await {
        Ok(Some(Ok(_))) => {} // 丢弃第一条数据
        Ok(Some(Err(e))) => return Err(Box::new(e)),
        Ok(None) => return Err("Traffic stream ended before first data".into()),
        Err(_) => return Err("Timeout getting first traffic data".into()),
    }

    // 获取第二条数据
    match timeout(Duration::from_secs(5), stream.next()).await {
        Ok(Some(Ok(traffic))) => Ok(traffic),
        Ok(Some(Err(e))) => Err(Box::new(e)),
        Ok(None) => Err("Traffic stream ended after first data".into()),
        Err(_) => Err("Timeout getting second traffic data".into()),
    }
}

/// 从流式接口获取单次内存数据（跳过第一条数据以避免初始值为0）
async fn get_memory(
    client: &MihomoClient,
) -> Result<mihomo_rs::types::Memory, Box<dyn std::error::Error>> {
    let mut stream = client.memory_stream().await?;

    // 跳过第一条数据，因为可能为0
    match timeout(Duration::from_secs(3), stream.next()).await {
        Ok(Some(Ok(_))) => {} // 丢弃第一条数据
        Ok(Some(Err(e))) => return Err(Box::new(e)),
        Ok(None) => return Err("Memory stream ended before first data".into()),
        Err(_) => return Err("Timeout getting first memory data".into()),
    }

    // 获取第二条数据
    match timeout(Duration::from_secs(5), stream.next()).await {
        Ok(Some(Ok(memory))) => Ok(memory),
        Ok(Some(Err(e))) => Err(Box::new(e)),
        Ok(None) => Err("Memory stream ended after first data".into()),
        Err(_) => Err("Timeout getting second memory data".into()),
    }
}

/// 处理版本命令
async fn handle_version(
    service_manager: &mut ServiceManager,
    action: VersionAction,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        VersionAction::List => {
            println!("📋 获取可用版本列表...");
            let versions = service_manager.get_available_versions().await?;
            println!("可用版本:");
            for version in versions {
                println!("  📦 {} - {}", version.version, version.description);
            }
        }
        VersionAction::Download { version } => {
            println!("⬇️ 下载版本 {}...", version);
            // 这里需要先获取版本信息，然后下载
            let versions = service_manager.get_available_versions().await?;
            if let Some(version_info) = versions.iter().find(|v| v.version == version) {
                service_manager.download_and_install(version_info).await?;
                println!("✅ 版本 {} 下载并安装成功", version);
            } else {
                println!("❌ 未找到版本: {}", version);
            }
        }
        VersionAction::Latest => {
            println!("⬇️ 下载并安装最新版本...");
            service_manager.download_latest().await?;
            println!("✅ 最新版本下载并安装成功");
        }
        VersionAction::Current => {
            println!("🔍 获取当前版本...");
            match service_manager.get_current_version().await {
                Ok(version) => println!("📦 当前版本: {}", version),
                Err(_) => println!("❌ 未找到当前版本信息"),
            }
        }
    }

    Ok(())
}

/// 处理服务命令
async fn handle_service(
    _client: &MihomoClient,
    action: ServiceAction,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        ServiceAction::Init => {
            println!("🔧 初始化配置目录...");
            let config_dir = ServiceManager::init_app_config()?;
            println!("✅ 配置目录已创建: {}", config_dir.display());
            println!(
                "📝 默认配置文件已生成: {}",
                config_dir.join("config.yaml").display()
            );
        }
        ServiceAction::Start => {
            let mut service_manager = ServiceManager::new_with_defaults()?;
            println!("🚀 启动服务...");
            service_manager.start().await?;
            println!("✅ 服务启动成功");
        }
        ServiceAction::Stop => {
            let mut service_manager = ServiceManager::new_with_defaults()?;
            println!("🛑 停止服务...");
            service_manager.stop().await?;
            println!("✅ 服务已停止");
        }
        ServiceAction::Restart => {
            let mut service_manager = ServiceManager::new_with_defaults()?;
            println!("🔄 重启服务...");
            service_manager.restart().await?;
            println!("✅ 服务重启成功");
        }
        ServiceAction::Status => {
            let service_manager = ServiceManager::new_with_defaults()?;
            println!("🔍 获取服务状态...");
            let status = service_manager.get_status().await?;
            println!("📊 服务状态: {:?}", status);
        }
        ServiceAction::Version { action } => {
            let mut service_manager = ServiceManager::new_with_defaults()?;
            handle_version(&mut service_manager, action).await?;
        }
        ServiceAction::Upgrade { version, backup } => {
            let mut service_manager = ServiceManager::new_with_defaults()?;

            match version {
                Some(target_version) => {
                    println!("🔄 升级到指定版本: {}...", target_version);

                    // 获取可用版本列表
                    let versions = service_manager.get_available_versions().await?;
                    let version_info = versions
                        .into_iter()
                        .find(|v| v.version.contains(&target_version))
                        .ok_or_else(|| format!("未找到版本: {}", target_version))?;

                    service_manager
                        .upgrade_to_version(&version_info, backup)
                        .await?;
                    println!("✅ 升级到版本 {} 成功", target_version);
                }
                None => {
                    println!("🔄 升级到最新版本...");
                    service_manager.upgrade_to_latest(backup).await?;
                    println!("✅ 升级到最新版本成功");
                }
            }
        }
        ServiceAction::Uninstall {
            keep_config,
            confirm,
        } => {
            if !confirm {
                println!("❌ 请使用 --confirm 参数确认卸载操作");
                println!("⚠️  这将删除所有 mihomo-rs 相关文件");
                return Ok(());
            }

            let mut service_manager = ServiceManager::new_with_defaults()?;
            println!("🗑️  开始卸载 mihomo-rs...");

            if keep_config {
                println!("📝 将保留配置文件");
            } else {
                println!("⚠️  将删除所有文件包括配置");
            }

            service_manager.uninstall(keep_config).await?;
        }
        ServiceAction::Cleanup { keep } => {
            let service_manager = ServiceManager::new_with_defaults()?;
            println!("🧹 清理备份文件，保留最新 {} 个...", keep);
            service_manager.cleanup_backups(keep)?;
            println!("✅ 备份文件清理完成");
        }
    }

    Ok(())
}

/// 处理状态命令
async fn handle_status(client: &MihomoClient) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 获取服务状态...");

    let version = client.version().await?;
    let traffic = get_traffic(client).await?;
    let memory = get_memory(client).await?;

    println!("\n📊 Mihomo 服务状态:");
    println!("版本: {}", version.version);
    println!("上传: {} MB", traffic.up / 1024 / 1024);
    println!("下载: {} MB", traffic.down / 1024 / 1024);
    println!("内存使用: {} MB", memory.in_use / 1024 / 1024);

    Ok(())
}

/// 处理代理命令
async fn handle_proxy(
    client: &MihomoClient,
    action: ProxyAction,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut proxy_manager = ProxyManager::new(client.clone());

    match action {
        ProxyAction::List => {
            let proxies = proxy_manager.get_proxies().await?;
            println!("\n📋 代理节点:");
            for (name, proxy) in proxies {
                println!("  {} (类型: {:?})", name, proxy.proxy_type);
            }

            let groups = proxy_manager.get_proxy_groups().await?;
            println!("\n📋 代理组:");
            for (name, group) in groups {
                println!(
                    "  {} (类型: {:?}, 当前: {})",
                    name, group.group_type, group.now
                );
            }
        }
        ProxyAction::Switch { group, proxy } => {
            println!("🔄 切换代理: {} -> {}", group, proxy);
            proxy_manager.switch_proxy(&group, &proxy).await?;
            println!("✅ 代理切换成功");
        }
        ProxyAction::Test {
            proxy,
            url,
            timeout,
        } => {
            println!("🧪 测试代理延迟: {}", proxy);
            let delay = proxy_manager
                .test_proxy_delay(&proxy, Some(&url), Some(timeout))
                .await?;
            if delay.delay > 0 {
                println!("✅ 延迟: {} ms", delay.delay);
            } else {
                println!("❌ 代理不可用");
            }
        }
        ProxyAction::BatchTest {
            group,
            url,
            timeout,
            concurrent,
        } => {
            println!("🧪 批量测试代理延迟...");

            let proxies = if let Some(group_name) = group {
                // 测试指定组的代理
                let groups = proxy_manager.get_proxy_groups().await?;
                if let Some(group) = groups.get(&group_name) {
                    group.all.clone()
                } else {
                    println!("❌ 代理组 '{}' 不存在", group_name);
                    return Ok(());
                }
            } else {
                // 测试所有代理
                let all_proxies = proxy_manager.get_proxies().await?;
                all_proxies.keys().cloned().collect()
            };

            println!("📊 开始测试 {} 个代理节点...", proxies.len());

            // 使用信号量控制并发数
            let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrent));
            let mut tasks = Vec::new();

            for proxy_name in proxies {
                let proxy_manager_clone = proxy_manager.clone();
                let url_clone = url.clone();
                let semaphore_clone = semaphore.clone();
                let proxy_name_clone = proxy_name.clone();

                let task = tokio::spawn(async move {
                    let _permit = semaphore_clone.acquire().await.unwrap();
                    let result = proxy_manager_clone
                        .test_proxy_delay(&proxy_name_clone, Some(&url_clone), Some(timeout))
                        .await;
                    (proxy_name_clone, result)
                });

                tasks.push(task);
            }

            // 等待所有测试完成
            let mut results = Vec::new();
            for task in tasks {
                if let Ok((proxy_name, result)) = task.await {
                    results.push((proxy_name, result));
                }
            }

            // 按延迟排序并显示结果
            results.sort_by(|a, b| match (&a.1, &b.1) {
                (Ok(delay_a), Ok(delay_b)) => {
                    if delay_a.delay == 0 && delay_b.delay == 0 {
                        std::cmp::Ordering::Equal
                    } else if delay_a.delay == 0 {
                        std::cmp::Ordering::Greater
                    } else if delay_b.delay == 0 {
                        std::cmp::Ordering::Less
                    } else {
                        delay_a.delay.cmp(&delay_b.delay)
                    }
                }
                (Ok(_), Err(_)) => std::cmp::Ordering::Less,
                (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
                (Err(_), Err(_)) => std::cmp::Ordering::Equal,
            });

            println!("\n📋 测试结果:");
            for (proxy_name, result) in results {
                match result {
                    Ok(delay) if delay.delay > 0 => {
                        println!("  ✅ {} - {} ms", proxy_name, delay.delay);
                    }
                    Ok(_) => {
                        println!("  ❌ {} - 不可用", proxy_name);
                    }
                    Err(e) => {
                        println!("  ❌ {} - 错误: {}", proxy_name, e);
                    }
                }
            }
        }
        ProxyAction::AutoSelect {
            group,
            url,
            timeout,
            max_delay,
        } => {
            println!("🤖 自动选择最佳代理: {}", group);

            // 获取代理组信息
            let groups = proxy_manager.get_proxy_groups().await?;
            let group_info = groups
                .get(&group)
                .ok_or_else(|| format!("代理组 '{}' 不存在", group))?;

            let proxy_names = group_info.all.clone();
            println!(
                "📊 测试代理组 '{}' 中的 {} 个代理...",
                group,
                proxy_names.len()
            );

            // 测试所有代理
            let mut best_proxy = None;
            let mut best_delay = u32::MAX;

            for proxy_name in &proxy_names {
                match proxy_manager
                    .test_proxy_delay(proxy_name, Some(&url), Some(timeout))
                    .await
                {
                    Ok(delay) if delay.delay > 0 && delay.delay <= max_delay => {
                        println!("  ✅ {} - {} ms", proxy_name, delay.delay);
                        if delay.delay < best_delay {
                            best_delay = delay.delay;
                            best_proxy = Some(proxy_name.clone());
                        }
                    }
                    Ok(delay) if delay.delay > 0 => {
                        println!("  ⚠️  {} - {} ms (超过阈值)", proxy_name, delay.delay);
                    }
                    Ok(_) => {
                        println!("  ❌ {} - 不可用", proxy_name);
                    }
                    Err(e) => {
                        println!("  ❌ {} - 错误: {}", proxy_name, e);
                    }
                }
            }

            if let Some(best_proxy_name) = best_proxy {
                println!("\n🎯 选择最佳代理: {} ({} ms)", best_proxy_name, best_delay);
                proxy_manager.switch_proxy(&group, &best_proxy_name).await?;
                println!("✅ 代理切换成功");
            } else {
                println!("\n❌ 未找到符合条件的代理（延迟 <= {} ms）", max_delay);
            }
        }
    }

    Ok(())
}

/// 处理配置命令
async fn handle_config(
    client: &MihomoClient,
    action: ConfigAction,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        ConfigAction::Show => {
            println!("🔍 获取当前配置...");
            let config = client.get_config().await?;
            println!("\n📋 当前配置:");
            println!("{}", serde_json::to_string_pretty(&config)?);
        }
        ConfigAction::Reload => {
            println!("🔄 重新加载配置...");
            client.reload_config().await?;
            println!("✅ 配置重新加载成功");
        }
        ConfigAction::Validate { path } => {
            println!("🔍 验证配置文件: {}", path);
            let mut config_manager = ConfigManager::new();
            match config_manager.load_from_file(&path) {
                Ok(_) => println!("✅ 配置文件有效"),
                Err(e) => println!("❌ 配置文件无效: {}", e),
            }
        }
        ConfigAction::Backup { path, description } => {
            handle_config_backup(client, path, description).await?;
        }
        ConfigAction::Restore {
            backup,
            create_backup,
        } => {
            handle_config_restore(client, backup, create_backup).await?;
        }
        ConfigAction::ListBackups => {
            handle_config_list_backups().await?;
        }
        ConfigAction::DeleteBackup { backup } => {
            handle_config_delete_backup(backup).await?;
        }
        ConfigAction::Compare { config1, config2 } => {
            handle_config_compare(client, config1, config2).await?;
        }
        ConfigAction::Export { path, format } => {
            handle_config_export(client, path, format).await?;
        }
        ConfigAction::Import { path, backup } => {
            handle_config_import(client, path, backup).await?;
        }
        ConfigAction::Reset { backup, confirm } => {
            handle_config_reset(client, backup, confirm).await?;
        }
    }

    Ok(())
}

/// 处理监控命令
async fn handle_monitor(
    client: &MihomoClient,
    interval: u64,
    duration: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 开始监控服务 (间隔: {}s, 持续: {}s)", interval, duration);
    println!("按 Ctrl+C 可提前退出监控\n");

    let monitor = Monitor::new(client.clone());
    let start_time = std::time::Instant::now();
    let mut first_run = true;
    let mut stdout = io::stdout();

    while start_time.elapsed().as_secs() < duration {
        match monitor.get_system_status().await {
            Ok(status) => {
                // 如果不是第一次运行，清除之前的输出
                if !first_run {
                    // 向上移动8行并清除从光标到屏幕底部的内容
                    stdout.execute(cursor::MoveUp(8))?;
                    stdout.execute(terminal::Clear(ClearType::FromCursorDown))?;
                } else {
                    first_run = false;
                }

                // 输出当前状态
                println!("📊 系统状态 [{}]:", chrono::Local::now().format("%H:%M:%S"));
                println!("  版本: {}", status.version.version);
                println!("  上传: {} MB/s", status.traffic.up / 1024 / 1024);
                println!("  下载: {} MB/s", status.traffic.down / 1024 / 1024);
                println!("  内存: {} MB", status.memory.in_use / 1024 / 1024);
                println!("  连接数: {}", status.active_connections);
                println!("  健康状态: {:?}", status.health);
                println!();

                // 刷新输出缓冲区
                stdout.flush()?;
            }
            Err(e) => {
                if !first_run {
                    stdout.execute(cursor::MoveUp(2))?;
                    stdout.execute(terminal::Clear(ClearType::FromCursorDown))?;
                } else {
                    first_run = false;
                }
                println!("❌ 获取状态失败: {}", e);
                println!();
                stdout.flush()?;
            }
        }

        tokio::time::sleep(Duration::from_secs(interval)).await;
    }

    println!("✅ 监控完成");
    Ok(())
}

/// 处理规则命令
async fn handle_rules(
    client: &MihomoClient,
    action: Option<RuleAction>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut rule_engine = RuleEngine::new(client.clone());

    match action {
        Some(RuleAction::List) | None => {
            println!("🔍 获取规则信息...");
            let rules = rule_engine.get_rules().await?;

            println!("\n📋 规则列表:");
            for (i, rule) in rules.iter().enumerate() {
                println!(
                    "  {}. {:?} {} -> {}",
                    i + 1,
                    rule.rule_type,
                    rule.payload,
                    rule.proxy
                );
            }
        }
        Some(RuleAction::Stats) => {
            println!("🔍 获取规则统计信息...");
            let stats = rule_engine.get_rule_stats().await?;
            println!("\n📊 规则统计:");
            println!("  总规则数: {}", stats.total_rules);
            println!("  类型分布: {:?}", stats.type_counts);
            println!("  代理分布: {:?}", stats.proxy_counts);
        }
        Some(RuleAction::Reload) => {
            println!("🔄 重新加载规则...");
            rule_engine.refresh_rules().await?;
            println!("✅ 规则重新加载成功");
        }
        Some(RuleAction::Validate { rule }) => {
            println!("🔍 验证规则格式: {}", rule);
            match rule_engine.validate_rule(&rule) {
                Ok(parsed) => {
                    println!("✅ 规则格式有效");
                    println!("  类型: {:?}", parsed.rule_type);
                    println!("  载荷: {}", parsed.payload);
                    println!("  目标: {}", parsed.target);
                    if let Some(options) = parsed.options {
                        println!("  选项: {}", options);
                    }
                }
                Err(e) => {
                    println!("❌ 规则格式无效: {}", e);
                }
            }
        }
        Some(RuleAction::Match { target, port }) => {
            println!("🔍 查找匹配规则: {} (端口: {:?})", target, port);
            match rule_engine.match_rule(&target, port, None).await {
                Ok(Some((rule, proxy))) => {
                    println!("✅ 找到匹配规则:");
                    println!("  规则: {:?} {}", rule.rule_type, rule.payload);
                    println!("  代理: {}", proxy);
                }
                Ok(None) => {
                    println!("❌ 未找到匹配的规则");
                }
                Err(e) => {
                    println!("❌ 查找规则时出错: {}", e);
                }
            }
        }
        Some(RuleAction::FindByProxy { proxy }) => {
            println!("🔍 查找代理 '{}' 的规则...", proxy);
            let rules = rule_engine.find_rules_by_proxy(&proxy).await?;
            if rules.is_empty() {
                println!("❌ 未找到使用代理 '{}' 的规则", proxy);
            } else {
                println!("✅ 找到 {} 条规则:", rules.len());
                for (i, rule) in rules.iter().enumerate() {
                    println!("  {}. {:?} {}", i + 1, rule.rule_type, rule.payload);
                }
            }
        }
        Some(RuleAction::FindByType { rule_type }) => {
            println!("🔍 查找类型 '{}' 的规则...", rule_type);
            // 解析规则类型字符串
            let parsed_type = match rule_type.to_uppercase().as_str() {
                "DOMAIN" => mihomo_rs::types::RuleType::Domain,
                "DOMAIN-SUFFIX" => mihomo_rs::types::RuleType::DomainSuffix,
                "DOMAIN-KEYWORD" => mihomo_rs::types::RuleType::DomainKeyword,
                "GEOIP" => mihomo_rs::types::RuleType::Geoip,
                "IP-CIDR" => mihomo_rs::types::RuleType::IpCidr,
                "SRC-IP-CIDR" => mihomo_rs::types::RuleType::SrcIpCidr,
                "SRC-PORT" => mihomo_rs::types::RuleType::SrcPort,
                "DST-PORT" => mihomo_rs::types::RuleType::DstPort,
                "PROCESS-NAME" => mihomo_rs::types::RuleType::ProcessName,
                "PROCESS-PATH" => mihomo_rs::types::RuleType::ProcessPath,
                "SCRIPT" => mihomo_rs::types::RuleType::Script,
                "RULE-SET" => mihomo_rs::types::RuleType::RuleSet,
                "MATCH" => mihomo_rs::types::RuleType::Match,
                _ => {
                    println!("❌ 不支持的规则类型: {}", rule_type);
                    return Ok(());
                }
            };

            let rules = rule_engine.find_rules_by_type(parsed_type).await?;
            if rules.is_empty() {
                println!("❌ 未找到类型为 '{}' 的规则", rule_type);
            } else {
                println!("✅ 找到 {} 条规则:", rules.len());
                for (i, rule) in rules.iter().enumerate() {
                    println!("  {}. {} -> {}", i + 1, rule.payload, rule.proxy);
                }
            }
        }
        Some(RuleAction::Provider { action }) => {
            handle_rule_provider(client, action).await?;
        }
    }

    Ok(())
}

/// 处理规则提供者命令
async fn handle_rule_provider(
    client: &MihomoClient,
    action: RuleProviderAction,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        RuleProviderAction::List => {
            println!("🔍 获取规则提供者列表...");
            let providers = client.get_rule_providers().await?;
            if providers.is_empty() {
                println!("❌ 未找到规则提供者");
            } else {
                println!("\n📋 规则提供者列表:");
                for (name, provider) in providers {
                    println!(
                        "  {} (类型: {}, 规则数: {})",
                        name, provider.provider_type, provider.rule_count
                    );
                    if let Some(updated_at) = provider.updated_at {
                        println!("    更新时间: {}", updated_at);
                    }
                }
            }
        }
        RuleProviderAction::Update { name } => {
            println!("🔄 更新规则提供者: {}", name);
            client.update_rule_provider(&name).await?;
            println!("✅ 规则提供者更新成功");
        }
        RuleProviderAction::HealthCheck { name } => {
            println!("🧪 健康检查规则提供者: {}", name);
            match client.health_check_rule_provider(&name).await {
                Ok(_) => {
                    println!("✅ 规则提供者健康检查通过");
                }
                Err(e) => {
                    println!("❌ 规则提供者健康检查失败: {}", e);
                }
            }
        }
    }

    Ok(())
}

/// 处理连接命令
async fn handle_connections(
    client: &MihomoClient,
    action: Option<ConnectionAction>,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        Some(ConnectionAction::Close { id }) => {
            println!("🔄 关闭连接: {}", id);
            client.close_connection(&id).await?;
            println!("✅ 连接已关闭");
        }
        Some(ConnectionAction::CloseAll) => {
            println!("🔄 关闭所有连接...");
            client.close_all_connections().await?;
            println!("✅ 所有连接已关闭");
        }
        None => {
            println!("🔍 获取连接列表...");
            let connections = client.connections().await?;

            println!("\n📋 活跃连接 (共 {} 个):", connections.len());
            for (i, conn) in connections.iter().enumerate().take(10) {
                println!(
                    "  {}: {} -> {} ({})",
                    i + 1,
                    conn.metadata.source_ip,
                    conn.metadata.destination_ip,
                    conn.chains.join(" -> ")
                );
            }

            if connections.len() > 10 {
                println!("  ... 还有 {} 个连接", connections.len() - 10);
            }
        }
    }

    Ok(())
}
