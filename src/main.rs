//! Mihomo RS 命令行工具
//! 
//! 提供 mihomo 代理服务的管理功能

use clap::{Parser, Subcommand};
use mihomo_rs::{
    client::MihomoClient,
    config::ConfigManager,
    proxy::ProxyManager,
    monitor::Monitor,
    rules::RuleEngine,
    service::{ServiceManager, ServiceConfig},
    init_logger,
};
use std::time::Duration;

/// Mihomo RS 命令行工具
#[derive(Parser)]
#[command(name = "mihomo-rs")]
#[command(about = "Mihomo 代理服务管理工具")]
#[command(version = "0.1.0")]
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
    Rules,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // 初始化日志
    if cli.verbose {
        std::env::set_var("RUST_LOG", "debug");
    } else {
        std::env::set_var("RUST_LOG", "info");
    }
    init_logger();
    
    // 创建客户端
    let client = MihomoClient::new(&cli.url, cli.secret)?;
    
    match cli.command {
        Commands::Status => handle_status(&client).await?,
        Commands::Proxy { action } => handle_proxy(&client, action).await?,
        Commands::Config { action } => handle_config(&client, action).await?,
        Commands::Monitor { interval, duration } => {
            handle_monitor(&client, interval, duration).await?
        },
        Commands::Rules => handle_rules(&client).await?,
        Commands::Connections { action } => handle_connections(&client, action).await?,
        Commands::Service { action } => handle_service(&client, action).await?,
    }
    
    Ok(())
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
        },
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
         },
        VersionAction::Latest => {
            println!("⬇️ 下载并安装最新版本...");
            service_manager.download_latest().await?;
            println!("✅ 最新版本下载并安装成功");
        },
        VersionAction::Current => {
            println!("🔍 获取当前版本...");
            match service_manager.get_current_version().await {
                Ok(version) => println!("📦 当前版本: {}", version),
                Err(_) => println!("❌ 未找到当前版本信息"),
            }
        },
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
            println!("📝 默认配置文件已生成: {}", config_dir.join("config.yaml").display());
        },
        ServiceAction::Start => {
            let mut service_manager = ServiceManager::new_with_defaults()?;
            println!("🚀 启动服务...");
            service_manager.start().await?;
            println!("✅ 服务启动成功");
        },
        ServiceAction::Stop => {
            let mut service_manager = ServiceManager::new_with_defaults()?;
            println!("🛑 停止服务...");
            service_manager.stop().await?;
            println!("✅ 服务已停止");
        },
        ServiceAction::Restart => {
            let mut service_manager = ServiceManager::new_with_defaults()?;
            println!("🔄 重启服务...");
            service_manager.restart().await?;
            println!("✅ 服务重启成功");
        },
        ServiceAction::Status => {
            let service_manager = ServiceManager::new_with_defaults()?;
            println!("🔍 获取服务状态...");
            let status = service_manager.get_status().await?;
            println!("📊 服务状态: {:?}", status);
        },
        ServiceAction::Version { action } => {
            let mut service_manager = ServiceManager::new_with_defaults()?;
            handle_version(&mut service_manager, action).await?;
        },
    }
    
    Ok(())
}

/// 处理状态命令
async fn handle_status(client: &MihomoClient) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 获取服务状态...");
    
    let version = client.version().await?;
    let traffic = client.traffic().await?;
    let memory = client.memory().await?;
    
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
                println!("  {} (类型: {:?}, 当前: {})", name, group.group_type, group.now);
            }
        },
        ProxyAction::Switch { group, proxy } => {
            println!("🔄 切换代理: {} -> {}", group, proxy);
            proxy_manager.switch_proxy(&group, &proxy).await?;
            println!("✅ 代理切换成功");
        },
        ProxyAction::Test { proxy, url, timeout } => {
            println!("🧪 测试代理延迟: {}", proxy);
            let delay = proxy_manager.test_proxy_delay(&proxy, Some(&url), Some(timeout)).await?;
            if delay.delay > 0 {
                println!("✅ 延迟: {} ms", delay.delay);
            } else {
                println!("❌ 代理不可用");
            }
        },
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
        },
        ConfigAction::Reload => {
            println!("🔄 重新加载配置...");
            client.reload_config().await?;
            println!("✅ 配置重新加载成功");
        },
        ConfigAction::Validate { path } => {
            println!("🔍 验证配置文件: {}", path);
            let mut config_manager = ConfigManager::new();
            match config_manager.load_from_file(&path) {
                Ok(_) => println!("✅ 配置文件有效"),
                Err(e) => println!("❌ 配置文件无效: {}", e),
            }
        },
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
    
    let monitor = Monitor::new(client.clone());
    let start_time = std::time::Instant::now();
    
    while start_time.elapsed().as_secs() < duration {
        match monitor.get_system_status().await {
            Ok(status) => {
                println!("\n📊 系统状态 [{}]:", chrono::Utc::now().format("%H:%M:%S"));
                println!("  版本: {}", status.version.version);
                println!("  上传: {} MB/s", status.traffic.up / 1024 / 1024);
                println!("  下载: {} MB/s", status.traffic.down / 1024 / 1024);
                println!("  内存: {} MB", status.memory.in_use / 1024 / 1024);
                println!("  连接数: {}", status.active_connections);
                println!("  健康状态: {:?}", status.health);
            },
            Err(e) => {
                println!("❌ 获取状态失败: {}", e);
            }
        }
        
        tokio::time::sleep(Duration::from_secs(interval)).await;
    }
    
    println!("\n✅ 监控完成");
    Ok(())
}

/// 处理规则命令
async fn handle_rules(client: &MihomoClient) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 获取规则列表...");
    
    let rules = client.rules().await?;
    let mut rule_engine = RuleEngine::new(client.clone());
    
    println!("\n📋 规则列表 (共 {} 条):", rules.len());
    for (i, rule) in rules.iter().enumerate().take(20) {
        println!("  {}: {} -> {}", i + 1, rule.payload, rule.proxy);
    }
    
    if rules.len() > 20 {
        println!("  ... 还有 {} 条规则", rules.len() - 20);
    }
    
    // 测试规则匹配
    println!("\n🧪 测试规则匹配:");
    let test_domains = ["www.google.com", "www.baidu.com", "github.com"];
    
    for domain in &test_domains {
        match rule_engine.match_rule(domain, Some(80), Some("tcp")).await {
            Ok(Some((rule, proxy))) => println!("  {} -> {} (规则: {:?})", domain, proxy, rule.rule_type),
            Ok(None) => println!("  {} -> DIRECT", domain),
            Err(e) => println!("  {} -> 错误: {}", domain, e),
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
        },
        Some(ConnectionAction::CloseAll) => {
            println!("🔄 关闭所有连接...");
            client.close_all_connections().await?;
            println!("✅ 所有连接已关闭");
        },
        None => {
            println!("🔍 获取连接列表...");
            let connections = client.connections().await?;
            
            println!("\n📋 活跃连接 (共 {} 个):", connections.len());
            for (i, conn) in connections.iter().enumerate().take(10) {
                println!("  {}: {} -> {} ({})", 
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
