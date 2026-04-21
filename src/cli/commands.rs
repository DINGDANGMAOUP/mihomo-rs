use crate::core::{validate_profile_name, validate_version_name};
use clap::{Parser, Subcommand, ValueEnum};

fn parse_profile_arg(value: &str) -> std::result::Result<String, String> {
    validate_profile_name(value)
        .map(|_| value.to_string())
        .map_err(|_| format!("Invalid profile name '{}'", value))
}

fn parse_version_arg(value: &str) -> std::result::Result<String, String> {
    validate_version_name(value)
        .map(|_| value.to_string())
        .map_err(|_| format!("Invalid version '{}'", value))
}

fn parse_install_target(value: &str) -> std::result::Result<String, String> {
    let lower = value.to_ascii_lowercase();
    if matches!(lower.as_str(), "stable" | "beta" | "nightly" | "alpha") {
        return Ok(value.to_string());
    }
    parse_version_arg(value)
}

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
    #[command(about = "Version management")]
    Version {
        #[command(subcommand)]
        action: VersionAction,
    },

    #[command(about = "Install mihomo kernel version", hide = true)]
    Install {
        #[arg(
            help = "Version to install (e.g., v1.18.0) or channel (stable/beta/nightly)",
            value_parser = parse_install_target
        )]
        version: Option<String>,
    },

    #[command(about = "Update to latest version", hide = true)]
    Update,

    #[command(about = "Set default version", hide = true)]
    Default {
        #[arg(help = "Version to set as default", value_parser = parse_version_arg)]
        version: String,
    },

    #[command(about = "List installed versions", hide = true)]
    List,

    #[command(about = "List available remote versions", hide = true)]
    ListRemote {
        #[arg(short, long, default_value = "20", help = "Number of versions to show")]
        limit: usize,
    },

    #[command(about = "Uninstall a version", hide = true)]
    Uninstall {
        #[arg(help = "Version to uninstall", value_parser = parse_version_arg)]
        version: String,
    },

    #[command(about = "Configuration profiles and paths")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    #[command(about = "Service lifecycle and telemetry")]
    Service {
        #[command(subcommand)]
        action: ServiceAction,
    },

    #[command(about = "Start mihomo service", hide = true)]
    Start,

    #[command(about = "Stop mihomo service", hide = true)]
    Stop,

    #[command(about = "Restart mihomo service", hide = true)]
    Restart,

    #[command(about = "Show service status", hide = true)]
    Status,

    #[command(about = "Proxy management")]
    Proxy {
        #[command(subcommand)]
        action: ProxyAction,
    },

    #[command(about = "Stream mihomo logs", hide = true)]
    Logs {
        #[arg(
            short,
            long,
            help = "Log level filter (info/warning/error/debug/silent)"
        )]
        level: Option<String>,
    },

    #[command(about = "Stream traffic statistics", hide = true)]
    Traffic,

    #[command(about = "Show memory usage", hide = true)]
    Memory,

    #[command(about = "Connection management")]
    Connection {
        #[command(subcommand)]
        action: ConnectionAction,
    },

    #[command(about = "Read-only environment and runtime diagnostics")]
    Doctor {
        #[command(subcommand)]
        action: DoctorAction,
    },
}

#[derive(Subcommand)]
pub enum VersionAction {
    #[command(about = "Install mihomo kernel version")]
    Install {
        #[arg(
            help = "Version to install (e.g., v1.18.0) or channel (stable/beta/nightly)",
            value_parser = parse_install_target
        )]
        version: Option<String>,
    },

    #[command(about = "Update to latest version")]
    Update,

    #[command(about = "Set default version")]
    Use {
        #[arg(help = "Version to use as default", value_parser = parse_version_arg)]
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
        #[arg(help = "Version to uninstall", value_parser = parse_version_arg)]
        version: String,
    },
}

#[derive(Subcommand)]
pub enum ServiceAction {
    #[command(about = "Start mihomo service")]
    Start,

    #[command(about = "Stop mihomo service")]
    Stop,

    #[command(about = "Restart mihomo service")]
    Restart,

    #[command(about = "Show service status")]
    Status,

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
}

#[derive(Subcommand)]
pub enum ConfigAction {
    #[command(about = "List config profiles")]
    List,

    #[command(about = "Show current profile and config path")]
    Current,

    #[command(about = "Show resolved config directory path")]
    Path,

    #[command(about = "Set a config option")]
    Set {
        #[arg(help = "Config key", value_enum)]
        key: ConfigKey,
        #[arg(help = "Config value")]
        value: String,
    },

    #[command(about = "Unset a config option")]
    Unset {
        #[arg(help = "Config key", value_enum)]
        key: ConfigKey,
    },

    #[command(about = "Switch to a profile")]
    Use {
        #[arg(help = "Profile name", value_parser = parse_profile_arg)]
        profile: String,
    },

    #[command(about = "Show config content")]
    Show {
        #[arg(help = "Profile name (default: current)", value_parser = parse_profile_arg)]
        profile: Option<String>,
    },

    #[command(about = "Delete a profile")]
    Delete {
        #[arg(help = "Profile name", value_parser = parse_profile_arg)]
        profile: String,
    },
}

#[derive(Clone, Debug, ValueEnum, PartialEq, Eq)]
pub enum ConfigKey {
    ConfigsDir,
}

#[derive(Subcommand)]
pub enum DoctorAction {
    #[command(about = "Run doctor checks")]
    Run {
        #[arg(
            long,
            help = "Comma-separated check ids or categories to run (e.g. config.current_profile,config)"
        )]
        only: Option<String>,
        #[arg(long, help = "Render the report as JSON")]
        json: bool,
    },

    #[command(about = "Apply safe doctor fixes")]
    Fix {
        #[arg(
            long,
            help = "Comma-separated check ids or categories to fix (e.g. config.current_yaml,config)"
        )]
        only: Option<String>,
    },

    #[command(about = "List available doctor checks")]
    List,

    #[command(about = "Explain a doctor check")]
    Explain {
        #[arg(help = "Check id")]
        check_id: String,
    },
}

#[cfg(test)]
mod tests {
    use super::{
        Cli, Commands, ConfigAction, ConfigKey, ConnectionAction, DoctorAction, ProxyAction,
        ServiceAction, VersionAction,
    };
    use clap::{CommandFactory, Parser};

    #[test]
    fn cli_rejects_invalid_profile_argument() {
        let parsed = Cli::try_parse_from(["mihomo-rs", "config", "use", "../evil"]);
        assert!(parsed.is_err());
    }

    #[test]
    fn cli_rejects_invalid_version_argument() {
        let parsed = Cli::try_parse_from(["mihomo-rs", "default", "../v1"]);
        assert!(parsed.is_err());
    }

    #[test]
    fn cli_accepts_channel_install_target() {
        let parsed =
            Cli::try_parse_from(["mihomo-rs", "install", "stable"]).expect("channel should parse");
        match parsed.command {
            Commands::Install { version } => assert_eq!(version.as_deref(), Some("stable")),
            _ => panic!("expected install command"),
        }
    }

    #[test]
    fn cli_accepts_valid_profile_argument() {
        let parsed = Cli::try_parse_from(["mihomo-rs", "config", "show", "alpha-1.2_ok"])
            .expect("valid profile should parse");
        match parsed.command {
            Commands::Config {
                action: ConfigAction::Show { profile },
            } => assert_eq!(profile.as_deref(), Some("alpha-1.2_ok")),
            _ => panic!("expected config show command"),
        }
    }

    #[test]
    fn cli_accepts_config_path_command() {
        let path =
            Cli::try_parse_from(["mihomo-rs", "config", "path"]).expect("config path should parse");
        match path.command {
            Commands::Config {
                action: ConfigAction::Path,
            } => {}
            _ => panic!("expected config path command"),
        }
    }

    #[test]
    fn cli_accepts_config_current_command() {
        let current = Cli::try_parse_from(["mihomo-rs", "config", "current"])
            .expect("config current should parse");
        match current.command {
            Commands::Config {
                action: ConfigAction::Current,
            } => {}
            _ => panic!("expected config current command"),
        }
    }

    #[test]
    fn cli_accepts_config_set_and_unset_commands() {
        let set = Cli::try_parse_from([
            "mihomo-rs",
            "config",
            "set",
            "configs-dir",
            "~/Library/Mobile Documents/mihomo-rs/configs",
        ])
        .expect("config set should parse");
        match set.command {
            Commands::Config {
                action: ConfigAction::Set { key, value },
            } => {
                assert_eq!(key, ConfigKey::ConfigsDir);
                assert_eq!(value, "~/Library/Mobile Documents/mihomo-rs/configs");
            }
            _ => panic!("expected config set command"),
        }

        let unset = Cli::try_parse_from(["mihomo-rs", "config", "unset", "configs-dir"])
            .expect("config unset should parse");
        match unset.command {
            Commands::Config {
                action: ConfigAction::Unset { key },
            } => assert_eq!(key, ConfigKey::ConfigsDir),
            _ => panic!("expected config unset command"),
        }
    }

    #[test]
    fn cli_accepts_namespaced_version_and_service_commands() {
        let version = Cli::try_parse_from(["mihomo-rs", "version", "use", "v1.2.3"])
            .expect("version use should parse");
        match version.command {
            Commands::Version {
                action: VersionAction::Use { version },
            } => assert_eq!(version, "v1.2.3"),
            _ => panic!("expected version use command"),
        }

        let service = Cli::try_parse_from(["mihomo-rs", "service", "status"])
            .expect("service status should parse");
        match service.command {
            Commands::Service {
                action: ServiceAction::Status,
            } => {}
            _ => panic!("expected service status command"),
        }
    }

    #[test]
    fn cli_accepts_doctor_commands() {
        let run = Cli::try_parse_from([
            "mihomo-rs",
            "doctor",
            "run",
            "--only",
            "config.current_profile,version.binary_available",
            "--json",
        ])
        .expect("doctor run should parse");
        match run.command {
            Commands::Doctor {
                action: DoctorAction::Run { only, json },
            } => {
                assert_eq!(
                    only.as_deref(),
                    Some("config.current_profile,version.binary_available")
                );
                assert!(json);
            }
            _ => panic!("expected doctor run command"),
        }

        let list =
            Cli::try_parse_from(["mihomo-rs", "doctor", "list"]).expect("doctor list should parse");
        match list.command {
            Commands::Doctor {
                action: DoctorAction::List,
            } => {}
            _ => panic!("expected doctor list command"),
        }

        let fix = Cli::try_parse_from(["mihomo-rs", "doctor", "fix", "--only", "config"])
            .expect("doctor fix should parse");
        match fix.command {
            Commands::Doctor {
                action: DoctorAction::Fix { only },
            } => assert_eq!(only.as_deref(), Some("config")),
            _ => panic!("expected doctor fix command"),
        }

        let explain =
            Cli::try_parse_from(["mihomo-rs", "doctor", "explain", "config.current_yaml"])
                .expect("doctor explain should parse");
        match explain.command {
            Commands::Doctor {
                action: DoctorAction::Explain { check_id },
            } => assert_eq!(check_id, "config.current_yaml"),
            _ => panic!("expected doctor explain command"),
        }
    }

    #[test]
    fn cli_accepts_connection_flags_and_legacy_forms() {
        let list = Cli::try_parse_from([
            "mihomo-rs",
            "connection",
            "list",
            "--host",
            "example",
            "--process",
            "curl",
        ])
        .expect("connection list flags should parse");
        match list.command {
            Commands::Connection {
                action: ConnectionAction::List { host, process },
            } => {
                assert_eq!(host.as_deref(), Some("example"));
                assert_eq!(process.as_deref(), Some("curl"));
            }
            _ => panic!("expected connection list command"),
        }

        let close = Cli::try_parse_from([
            "mihomo-rs",
            "connection",
            "close",
            "--host",
            "example",
            "--force",
        ])
        .expect("connection close flags should parse");
        match close.command {
            Commands::Connection {
                action:
                    ConnectionAction::Close {
                        host, force, id, ..
                    },
            } => {
                assert_eq!(host.as_deref(), Some("example"));
                assert!(force);
                assert!(id.is_none());
            }
            _ => panic!("expected connection close command"),
        }

        let legacy = Cli::try_parse_from(["mihomo-rs", "connection", "filter-host", "example"])
            .expect("legacy filter-host should parse");
        match legacy.command {
            Commands::Connection {
                action: ConnectionAction::FilterHost { host },
            } => assert_eq!(host, "example"),
            _ => panic!("expected legacy filter-host command"),
        }
    }

    #[test]
    fn root_help_prefers_namespaced_commands() {
        let mut command = Cli::command();
        let mut output = Vec::new();
        command
            .write_long_help(&mut output)
            .expect("render root help");
        let help = String::from_utf8(output).expect("help should be utf8");

        assert!(help.contains("  version"));
        assert!(help.contains("  config"));
        assert!(help.contains("  service"));
        assert!(help.contains("  proxy"));
        assert!(help.contains("  connection"));
        assert!(!help.contains("  install      Install mihomo kernel version"));
        assert!(!help.contains("  start        Start mihomo service"));
        assert!(!help.contains("  logs         Stream mihomo logs"));
    }

    #[test]
    fn cli_keeps_legacy_root_aliases_available() {
        let install =
            Cli::try_parse_from(["mihomo-rs", "install", "stable"]).expect("legacy install");
        match install.command {
            Commands::Install { version } => assert_eq!(version.as_deref(), Some("stable")),
            _ => panic!("expected legacy install command"),
        }

        let start = Cli::try_parse_from(["mihomo-rs", "start"]).expect("legacy start");
        match start.command {
            Commands::Start => {}
            _ => panic!("expected legacy start command"),
        }
    }

    #[test]
    fn cli_accepts_proxy_switch_and_test_all_forms() {
        let switch = Cli::try_parse_from(["mihomo-rs", "proxy", "switch", "GLOBAL", "HK-01"])
            .expect("proxy switch should parse");
        match switch.command {
            Commands::Proxy {
                action: ProxyAction::Switch { group, proxy },
            } => {
                assert_eq!(group, "GLOBAL");
                assert_eq!(proxy, "HK-01");
            }
            _ => panic!("expected proxy switch command"),
        }

        let test_all =
            Cli::try_parse_from(["mihomo-rs", "proxy", "test"]).expect("proxy test should parse");
        match test_all.command {
            Commands::Proxy {
                action:
                    ProxyAction::Test {
                        proxy,
                        timeout,
                        url,
                    },
            } => {
                assert!(proxy.is_none());
                assert_eq!(timeout, 5000);
                assert_eq!(url, "http://www.gstatic.com/generate_204");
            }
            _ => panic!("expected proxy test command"),
        }
    }

    #[test]
    fn proxy_help_uses_clearer_terms() {
        let mut command = Cli::command();
        let proxy = command
            .find_subcommand_mut("proxy")
            .expect("proxy subcommand should exist");
        let mut output = Vec::new();
        proxy
            .write_long_help(&mut output)
            .expect("render proxy help");
        let help = String::from_utf8(output).expect("help should be utf8");

        assert!(help.contains("List proxy nodes"));
        assert!(help.contains("List selectable proxy groups"));
        assert!(help.contains("Show current proxy selection by group"));
    }
}

#[derive(Subcommand)]
pub enum ProxyAction {
    #[command(about = "List proxy nodes")]
    List,

    #[command(about = "List selectable proxy groups")]
    Groups,

    #[command(about = "Select a proxy for a group")]
    Switch {
        #[arg(help = "Group name")]
        group: String,
        #[arg(help = "Proxy name")]
        proxy: String,
    },

    #[command(about = "Test one proxy or all proxies")]
    Test {
        #[arg(help = "Proxy name; omit to test all proxies")]
        proxy: Option<String>,
        #[arg(short, long, default_value = "http://www.gstatic.com/generate_204")]
        url: String,
        #[arg(short, long, default_value = "5000")]
        timeout: u32,
    },

    #[command(about = "Show current proxy selection by group")]
    Current,
}

#[derive(Subcommand)]
pub enum ConnectionAction {
    #[command(about = "List active connections")]
    List {
        #[arg(long, help = "Filter by host name or IP")]
        host: Option<String>,
        #[arg(long, help = "Filter by process name")]
        process: Option<String>,
    },

    #[command(about = "Show connection statistics")]
    Stats,

    #[command(about = "Stream connections in real-time")]
    Stream,

    #[command(about = "Close connections")]
    Close {
        #[arg(
            value_name = "ID",
            help = "Connection ID (legacy positional form)",
            conflicts_with_all = ["id", "all", "host", "process"],
            required = false
        )]
        legacy_id: Option<String>,
        #[arg(long, help = "Connection ID", conflicts_with_all = ["legacy_id", "all", "host", "process"])]
        id: Option<String>,
        #[arg(long, help = "Close all connections", conflicts_with_all = ["legacy_id", "id", "host", "process"], default_value_t = false)]
        all: bool,
        #[arg(long, help = "Close by host name or IP", conflicts_with_all = ["legacy_id", "id", "all", "process"])]
        host: Option<String>,
        #[arg(long, help = "Close by process name", conflicts_with_all = ["legacy_id", "id", "all", "host"])]
        process: Option<String>,
        #[arg(
            short,
            long,
            help = "Skip confirmation prompt",
            default_value = "false"
        )]
        force: bool,
    },

    #[command(about = "Close all connections", hide = true)]
    CloseAll {
        #[arg(
            short,
            long,
            help = "Skip confirmation prompt",
            default_value = "false"
        )]
        force: bool,
    },

    #[command(about = "Filter connections by host", hide = true)]
    FilterHost {
        #[arg(help = "Host name or IP to filter")]
        host: String,
    },

    #[command(about = "Filter connections by process", hide = true)]
    FilterProcess {
        #[arg(help = "Process name to filter")]
        process: String,
    },

    #[command(about = "Close connections by host", hide = true)]
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

    #[command(about = "Close connections by process", hide = true)]
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
