//! 统一日志记录模块
//!
//! 提供统一的日志记录接口和配置管理。

use log::{debug, error, info, warn};
use std::sync::Once;

static INIT: Once = Once::new();

/// 日志配置
#[derive(Debug, Clone)]
pub struct LoggerConfig {
    /// 日志级别
    pub level: log::LevelFilter,
    /// 是否显示时间戳
    pub show_timestamp: bool,
    /// 是否显示模块路径
    pub show_module: bool,
    /// 是否显示行号
    pub show_line: bool,
    /// 日志格式
    pub format: LogFormat,
}

/// 日志格式
#[derive(Debug, Clone)]
pub enum LogFormat {
    /// 简洁格式
    Compact,
    /// 详细格式
    Full,
    /// JSON格式
    Json,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            level: log::LevelFilter::Info,
            show_timestamp: true,
            show_module: false,
            show_line: false,
            format: LogFormat::Compact,
        }
    }
}

/// 初始化日志系统
///
/// # Arguments
///
/// * `config` - 日志配置，如果为None则使用默认配置
///
/// # Examples
///
/// ```
/// use mihomo_rs::logger::{init_logger, LoggerConfig};
///
/// // 使用默认配置
/// init_logger(None);
///
/// // 使用自定义配置
/// let config = LoggerConfig {
///     level: log::LevelFilter::Debug,
///     show_timestamp: true,
///     show_module: true,
///     ..Default::default()
/// };
/// init_logger(Some(config));
/// ```
pub fn init_logger(config: Option<LoggerConfig>) {
    INIT.call_once(|| {
        let config = config.unwrap_or_default();

        let mut builder = env_logger::Builder::from_default_env();
        builder.filter_level(config.level);

        match config.format {
            LogFormat::Compact => {
                builder.format(move |buf, record| {
                    use std::io::Write;

                    let level_style = match record.level() {
                        log::Level::Error => "\x1b[31m", // 红色
                        log::Level::Warn => "\x1b[33m",  // 黄色
                        log::Level::Info => "\x1b[32m",  // 绿色
                        log::Level::Debug => "\x1b[36m", // 青色
                        log::Level::Trace => "\x1b[37m", // 白色
                    };

                    let reset = "\x1b[0m";

                    if config.show_timestamp {
                        writeln!(
                            buf,
                            "[{}] {}{:5}{} {}",
                            chrono::Local::now().format("%H:%M:%S"),
                            level_style,
                            record.level(),
                            reset,
                            record.args()
                        )
                    } else {
                        writeln!(
                            buf,
                            "{}{:5}{} {}",
                            level_style,
                            record.level(),
                            reset,
                            record.args()
                        )
                    }
                });
            }
            LogFormat::Full => {
                builder.format(move |buf, record| {
                    use std::io::Write;

                    let mut parts = Vec::new();

                    if config.show_timestamp {
                        parts.push(format!(
                            "[{}]",
                            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
                        ));
                    }

                    parts.push(format!("[{}]", record.level()));

                    if config.show_module {
                        if let Some(module) = record.module_path() {
                            parts.push(format!("[{}]", module));
                        }
                    }

                    if config.show_line {
                        if let (Some(file), Some(line)) = (record.file(), record.line()) {
                            parts.push(format!("[{}:{}]", file, line));
                        }
                    }

                    parts.push(record.args().to_string());

                    writeln!(buf, "{}", parts.join(" "))
                });
            }
            LogFormat::Json => {
                builder.format(move |buf, record| {
                    use std::io::Write;

                    let log_entry = serde_json::json!({
                        "timestamp": chrono::Local::now().to_rfc3339(),
                        "level": record.level().to_string(),
                        "message": record.args().to_string(),
                        "module": record.module_path(),
                        "file": record.file(),
                        "line": record.line(),
                    });

                    writeln!(buf, "{}", log_entry)
                });
            }
        }

        builder.init();
    });
}

/// 日志宏包装器
pub struct Logger;

impl Logger {
    /// 记录调试信息
    pub fn debug(message: &str) {
        debug!("{}", message);
    }

    /// 记录一般信息
    pub fn info(message: &str) {
        info!("{}", message);
    }

    /// 记录警告信息
    pub fn warn(message: &str) {
        warn!("{}", message);
    }

    /// 记录错误信息
    pub fn error(message: &str) {
        error!("{}", message);
    }

    /// 记录带格式的调试信息
    pub fn debug_fmt(_format: &str, args: std::fmt::Arguments) {
        debug!("{}", format_args!("{}", args));
    }

    /// 记录带格式的一般信息
    pub fn info_fmt(_format: &str, args: std::fmt::Arguments) {
        info!("{}", format_args!("{}", args));
    }

    /// 记录带格式的警告信息
    pub fn warn_fmt(_format: &str, args: std::fmt::Arguments) {
        warn!("{}", format_args!("{}", args));
    }

    /// 记录带格式的错误信息
    pub fn error_fmt(_format: &str, args: std::fmt::Arguments) {
        error!("{}", format_args!("{}", args));
    }
}

/// 便捷宏定义
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        log::debug!($($arg)*);
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        log::info!($($arg)*);
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        log::warn!($($arg)*);
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        log::error!($($arg)*);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_config_default() {
        let config = LoggerConfig::default();
        assert_eq!(config.level, log::LevelFilter::Info);
        assert!(config.show_timestamp);
        assert!(!config.show_module);
        assert!(!config.show_line);
    }

    #[test]
    fn test_logger_init() {
        // 测试初始化不会panic
        init_logger(None);

        // 测试重复初始化不会panic
        init_logger(None);
    }

    #[test]
    fn test_logger_methods() {
        init_logger(None);

        Logger::debug("Debug message");
        Logger::info("Info message");
        Logger::warn("Warning message");
        Logger::error("Error message");
    }

    #[test]
    fn test_log_macros() {
        init_logger(None);

        log_debug!("Debug: {}", "test");
        log_info!("Info: {}", "test");
        log_warn!("Warning: {}", "test");
        log_error!("Error: {}", "test");
    }
}
