use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mihomo-rs")]
#[command(about = "A Rust SDK and CLI tool for mihomo proxy management", long_about = None)]
pub struct Cli {
    #[arg(short, long, global = true, help = "Enable verbose logging")]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Install mihomo kernel version")]
    Install {
        #[arg(help = "Version to install (e.g., v1.18.0) or channel (stable/beta/nightly)")]
        version: Option<String>,
    },

    #[command(about = "Update to latest version")]
    Update,

    #[command(about = "Set default version")]
    Default {
        #[arg(help = "Version to set as default")]
        version: String,
    },

    #[command(about = "List installed versions")]
    List,

    #[command(about = "List available remote versions")]
    ListRemote {
        #[arg(short, long, default_value = "20", help = "Number of versions to show")]
        limit: usize,
    },

    #[command(about = "Uninstall a version")]
    Uninstall {
        #[arg(help = "Version to uninstall")]
        version: String,
    },

    #[command(about = "Configuration management")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    #[command(about = "Start mihomo service")]
    Start,

    #[command(about = "Stop mihomo service")]
    Stop,

    #[command(about = "Restart mihomo service")]
    Restart,

    #[command(about = "Show service status")]
    Status,

    #[command(about = "Proxy management")]
    Proxy {
        #[command(subcommand)]
        action: ProxyAction,
    },

    #[command(about = "Stream mihomo logs")]
    Logs {
        #[arg(
            short,
            long,
            help = "Log level filter (info/warning/error/debug/silent)"
        )]
        level: Option<String>,
    },

    #[command(about = "Stream traffic statistics")]
    Traffic,

    #[command(about = "Show memory usage")]
    Memory,

    #[command(about = "Connection management")]
    Connection {
        #[command(subcommand)]
        action: ConnectionAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    #[command(about = "List config profiles")]
    List,

    #[command(about = "Switch to a profile")]
    Use {
        #[arg(help = "Profile name")]
        profile: String,
    },

    #[command(about = "Show config content")]
    Show {
        #[arg(help = "Profile name (default: current)")]
        profile: Option<String>,
    },

    #[command(about = "Delete a profile")]
    Delete {
        #[arg(help = "Profile name")]
        profile: String,
    },
}

#[derive(Subcommand)]
pub enum ProxyAction {
    #[command(about = "List all proxies")]
    List,

    #[command(about = "List proxy groups")]
    Groups,

    #[command(about = "Switch proxy in group")]
    Switch {
        #[arg(help = "Group name")]
        group: String,
        #[arg(help = "Proxy name")]
        proxy: String,
    },

    #[command(about = "Test proxy delay")]
    Test {
        #[arg(help = "Proxy name (default: test all)")]
        proxy: Option<String>,
        #[arg(short, long, default_value = "http://www.gstatic.com/generate_204")]
        url: String,
        #[arg(short, long, default_value = "5000")]
        timeout: u32,
    },

    #[command(about = "Show current proxies")]
    Current,
}

#[derive(Subcommand)]
pub enum ConnectionAction {
    #[command(about = "List active connections")]
    List,

    #[command(about = "Show connection statistics")]
    Stats,

    #[command(about = "Stream connections in real-time")]
    Stream,

    #[command(about = "Close a specific connection")]
    Close {
        #[arg(help = "Connection ID")]
        id: String,
    },

    #[command(about = "Close all connections")]
    CloseAll {
        #[arg(
            short,
            long,
            help = "Skip confirmation prompt",
            default_value = "false"
        )]
        force: bool,
    },

    #[command(about = "Filter connections by host")]
    FilterHost {
        #[arg(help = "Host name or IP to filter")]
        host: String,
    },

    #[command(about = "Filter connections by process")]
    FilterProcess {
        #[arg(help = "Process name to filter")]
        process: String,
    },

    #[command(about = "Close connections by host")]
    CloseByHost {
        #[arg(help = "Host name or IP")]
        host: String,
        #[arg(
            short,
            long,
            help = "Skip confirmation prompt",
            default_value = "false"
        )]
        force: bool,
    },

    #[command(about = "Close connections by process")]
    CloseByProcess {
        #[arg(help = "Process name")]
        process: String,
        #[arg(
            short,
            long,
            help = "Skip confirmation prompt",
            default_value = "false"
        )]
        force: bool,
    },
}
