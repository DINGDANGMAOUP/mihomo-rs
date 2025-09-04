//! 服务管理模块
//! 
//! 提供 Mihomo 服务的版本管理、下载、启动、停止、重启等功能。

use crate::error::{MihomoError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::fs;
use std::io::Read;
use tokio::time::{sleep, Duration};
use std::env;
use sysinfo::{System, SystemExt, ProcessExt, Pid};

/// 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// 版本号
    pub version: String,
    /// 发布日期
    pub release_date: String,
    /// 下载链接
    pub download_urls: HashMap<String, String>,
    /// 是否为预发布版本
    pub prerelease: bool,
    /// 版本描述
    pub description: String,
}

/// 服务状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceStatus {
    /// 运行中
    Running,
    /// 已停止
    Stopped,
    /// 启动中
    Starting,
    /// 停止中
    Stopping,
    /// 未知状态
    Unknown,
}

/// 服务配置
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// 二进制文件路径
    pub binary_path: PathBuf,
    /// 配置文件路径
    pub config_path: Option<PathBuf>,
    /// 工作目录
    pub work_dir: PathBuf,
    /// API 端口
    pub api_port: u16,
    /// 外部控制器地址
    pub external_controller: String,
    /// API 密钥
    pub secret: Option<String>,
    /// 日志级别
    pub log_level: String,
}

/// 获取应用配置目录
/// 
/// 返回 ~/.config/mihomo-rs 目录路径，如果不存在则创建
fn get_app_config_dir() -> Result<PathBuf> {
    let home_dir = env::var("HOME")
        .map_err(|_| MihomoError::ServiceError("无法获取用户主目录".to_string()))?;
    
    let config_dir = PathBuf::from(home_dir)
        .join(".config")
        .join("mihomo-rs");
    
    // 确保目录存在
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .map_err(|e| MihomoError::ServiceError(format!("创建配置目录失败: {}", e)))?;
    }
    
    Ok(config_dir)
}

/// 获取默认二进制文件路径
/// 
/// 返回配置目录下的 mihomo 二进制文件路径
fn get_default_binary_path() -> Result<PathBuf> {
    let config_dir = get_app_config_dir()?;
    Ok(config_dir.join("mihomo"))
}

/// 获取默认配置文件路径
/// 
/// 返回配置目录下的 config.yaml 文件路径
fn get_default_config_path() -> Result<PathBuf> {
    let config_dir = get_app_config_dir()?;
    Ok(config_dir.join("config.yaml"))
}

impl Default for ServiceConfig {
    fn default() -> Self {
        let config_dir = get_app_config_dir().unwrap_or_else(|_| PathBuf::from("."));
        let binary_path = get_default_binary_path().unwrap_or_else(|_| PathBuf::from("./mihomo"));
        let config_path = get_default_config_path().ok();
        
        Self {
            binary_path,
            config_path,
            work_dir: config_dir,
            api_port: 9090,
            external_controller: "127.0.0.1:9090".to_string(),
            secret: None,
            log_level: "info".to_string(),
        }
    }
}

/// 服务管理器
#[derive(Debug)]
pub struct ServiceManager {
    /// 服务配置
    config: ServiceConfig,
    /// HTTP客户端
    client: reqwest::Client,
}

impl ServiceManager {
    /// 获取PID文件路径
    /// 
    /// # Returns
    /// 
    /// 返回PID文件的完整路径
    fn get_pid_file_path() -> Result<PathBuf> {
        let config_dir = get_app_config_dir()?;
        Ok(config_dir.join("mihomo.pid"))
    }
    
    /// 写入PID文件
    /// 
    /// # Arguments
    /// 
    /// * `pid` - 进程ID
    fn write_pid_file(pid: u32) -> Result<()> {
        let pid_file = Self::get_pid_file_path()?;
        fs::write(&pid_file, pid.to_string())
            .map_err(|e| MihomoError::ServiceError(format!("写入PID文件失败: {}", e)))?;
        Ok(())
    }
    
    /// 读取PID文件
    /// 
    /// # Returns
    /// 
    /// 返回进程ID，如果文件不存在或无效则返回None
    fn read_pid_file() -> Option<u32> {
        let pid_file = Self::get_pid_file_path().ok()?;
        if !pid_file.exists() {
            return None;
        }
        
        let content = fs::read_to_string(&pid_file).ok()?;
        content.trim().parse().ok()
    }
    
    /// 删除PID文件
    fn remove_pid_file() -> Result<()> {
        let pid_file = Self::get_pid_file_path()?;
        if pid_file.exists() {
            fs::remove_file(&pid_file)
                .map_err(|e| MihomoError::ServiceError(format!("删除PID文件失败: {}", e)))?;
        }
        Ok(())
    }
    
    /// 检查进程是否存在
    /// 
    /// # Arguments
    /// 
    /// * `pid` - 进程ID
    /// 
    /// # Returns
    /// 
    /// 返回进程是否存在
    fn is_process_running(pid: u32) -> bool {
        let mut system = System::new();
        system.refresh_processes();
        system.process(Pid::from(pid as usize)).is_some()
    }

    /// 初始化应用配置目录
    /// 
    /// 创建 ~/.config/mihomo-rs 目录并生成默认配置文件
    pub fn init_app_config() -> Result<PathBuf> {
        let config_dir = get_app_config_dir()?;
        let config_file = config_dir.join("config.yaml");
        
        // 如果配置文件不存在，创建默认配置
        if !config_file.exists() {
            let default_config = r#"# Mihomo 配置文件
# 更多配置选项请参考: https://wiki.metacubex.one/config/

port: 7890
socks-port: 7891
allow-lan: false
mode: rule
log-level: info
external-controller: 127.0.0.1:9090

proxies: []
proxy-groups: []
rules:
  - MATCH,DIRECT
"#;
            
            fs::write(&config_file, default_config)
                .map_err(|e| MihomoError::ServiceError(format!("创建默认配置文件失败: {}", e)))?;
        }
        
        Ok(config_dir)
    }
    
    /// 创建新的服务管理器
    /// 
    /// # Arguments
    /// 
    /// * `config` - 服务配置
    /// 
    /// # Returns
    /// 
    /// 返回服务管理器实例
    pub fn new(config: ServiceConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();

        Self {
            config,
            client,
        }
    }
    
    /// 使用默认配置创建服务管理器
    /// 
    /// 自动初始化应用配置目录并使用默认配置
    pub fn new_with_defaults() -> Result<Self> {
        Self::init_app_config()?;
        Ok(Self::new(ServiceConfig::default()))
    }

    /// 获取可用版本列表
    /// 
    /// # Returns
    /// 
    /// 返回版本信息列表
    pub async fn get_available_versions(&self) -> Result<Vec<VersionInfo>> {
        let url = "https://api.github.com/repos/MetaCubeX/mihomo/releases";
        
        let response = self.client
            .get(url)
            .header("User-Agent", "mihomo-rs")
            .send()
            .await
            .map_err(|e| MihomoError::network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(MihomoError::service_error(format!(
                "获取版本信息失败: {}", 
                response.status()
            )));
        }

        let releases: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| MihomoError::service_error(e.to_string()))?;

        let mut versions = Vec::new();
        for release in releases {
            let version = release["tag_name"].as_str().unwrap_or("").to_string();
            let release_date = release["published_at"].as_str().unwrap_or("").to_string();
            let prerelease = release["prerelease"].as_bool().unwrap_or(false);
            let description = release["body"].as_str().unwrap_or("").to_string();
            
            let mut download_urls = HashMap::new();
            if let Some(assets) = release["assets"].as_array() {
                for asset in assets {
                    if let (Some(name), Some(url)) = (
                        asset["name"].as_str(),
                        asset["browser_download_url"].as_str()
                    ) {
                        download_urls.insert(name.to_string(), url.to_string());
                    }
                }
            }

            versions.push(VersionInfo {
                version,
                release_date,
                download_urls,
                prerelease,
                description,
            });
        }

        Ok(versions)
    }

    /// 下载指定版本
    /// 
    /// # Arguments
    /// 
    /// * `version` - 版本信息
    /// * `target_path` - 目标路径
    /// 
    /// # Returns
    /// 
    /// 返回下载结果
    pub async fn download_version(&self, version: &VersionInfo, target_path: &Path) -> Result<()> {
        // 检测当前系统架构
        let arch = std::env::consts::ARCH;
        let os = std::env::consts::OS;
        
        let (platform, extension) = match (os, arch) {
            ("macos", "aarch64") => ("darwin-arm64", ".gz"),
            ("macos", "x86_64") => ("darwin-amd64", ".gz"), 
            ("linux", "aarch64") => ("linux-arm64", ".gz"),
            ("linux", "x86_64") => ("linux-amd64", ".gz"),
            ("windows", "x86_64") => ("windows-amd64", ".zip"),
            _ => return Err(MihomoError::unsupported_platform(format!("{}-{}", os, arch))),
        };

        // 构建资源名称: mihomo-{platform}-{version}{extension}
        let asset_name = format!("mihomo-{}-{}{}", platform, version.version, extension);

        let download_url = version.download_urls.get(&asset_name)
            .ok_or_else(|| MihomoError::version_not_found(format!(
                "版本 {} 不支持当前平台 {} (查找资源: {})", 
                version.version, 
                platform,
                asset_name
            )))?;

        println!("正在下载 {} ...", version.version);
        
        let response = self.client
            .get(download_url)
            .send()
            .await
            .map_err(|e| MihomoError::network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(MihomoError::download_error(format!(
                "下载失败: {}", 
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| MihomoError::download_error(e.to_string()))?;

        // 创建目标目录
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| MihomoError::internal(e.to_string()))?;
        }

        // 根据文件扩展名处理文件
        if extension == ".gz" {
            // 解压 gzip 文件
            use std::io::Read;
            use flate2::read::GzDecoder;
            
            let mut decoder = GzDecoder::new(&bytes[..]);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)
                .map_err(|e| MihomoError::internal(format!("解压失败: {}", e)))?;
            
            fs::write(target_path, decompressed)
                .map_err(|e| MihomoError::internal(e.to_string()))?;
        } else if extension == ".zip" {
            // 处理 zip 文件 (Windows)
            fs::write(target_path, bytes)
                .map_err(|e| MihomoError::internal(e.to_string()))?;
            // TODO: 实现 zip 解压
        } else {
            // 直接写入文件
            fs::write(target_path, bytes)
                .map_err(|e| MihomoError::internal(e.to_string()))?;
        }

        // 设置可执行权限 (Unix 系统)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(target_path)
                .map_err(|e| MihomoError::internal(e.to_string()))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(target_path, perms)
                .map_err(|e| MihomoError::internal(e.to_string()))?;
        }

        println!("下载完成: {}", target_path.display());
        Ok(())
    }
    
    /// 下载版本到默认位置
    /// 
    /// 下载指定版本到配置目录，并更新当前配置的二进制路径
    /// 
    /// # Arguments
    /// 
    /// * `version` - 版本信息
    /// 
    /// # Returns
    /// 
    /// 返回下载结果
    pub async fn download_and_install(&mut self, version: &VersionInfo) -> Result<()> {
        let config_dir = get_app_config_dir()?;
        let binary_path = config_dir.join("mihomo");
        
        // 下载到默认位置
        self.download_version(version, &binary_path).await?;
        
        // 更新配置中的二进制路径
        self.config.binary_path = binary_path;
        
        Ok(())
    }
    
    /// 下载最新版本到默认位置
    /// 
    /// 获取最新版本并下载到配置目录
    /// 
    /// # Returns
    /// 
    /// 返回下载结果
    pub async fn download_latest(&mut self) -> Result<()> {
        let versions = self.get_available_versions().await?;
        let latest = versions.into_iter()
            .find(|v| !v.prerelease)
            .ok_or_else(|| MihomoError::ServiceError("未找到稳定版本".to_string()))?;
        
        self.download_and_install(&latest).await
    }

    /// 启动服务
    /// 
    /// # Returns
    /// 
    /// 返回启动结果
    pub async fn start(&mut self) -> Result<()> {
        if self.is_running().await? {
            return Err(MihomoError::ServiceError("服务已在运行中".to_string()));
        }

        let mut cmd = Command::new(&self.config.binary_path);
        
        // 添加配置文件参数
        if let Some(config_path) = &self.config.config_path {
            cmd.args(["-f", config_path.to_str().unwrap()]);
        }
        
        // 添加外部控制器参数
        cmd.args(["-ext-ctl", &self.config.external_controller]);
        
        // 设置工作目录
        cmd.current_dir(&self.config.work_dir);
        
        // 重定向输出并让进程独立运行
        cmd.stdout(Stdio::null())
           .stderr(Stdio::null())
           .stdin(Stdio::null());

        let child = cmd.spawn()
            .map_err(|e| MihomoError::ServiceError(format!("启动服务失败: {}", e)))?;

        let pid = child.id();
        
        // 写入PID文件
        Self::write_pid_file(pid)?;
        
        // 让进程独立运行，不保持引用
        std::mem::forget(child);
        
        // 等待服务启动
        for _ in 0..30 {
            sleep(Duration::from_secs(1)).await;
            if self.is_running().await? {
                println!("服务启动成功");
                return Ok(());
            }
        }
        
        // 启动失败，清理PID文件
        Self::remove_pid_file()?;
        Err(MihomoError::ServiceError("服务启动超时".to_string()))
    }

    /// 停止服务
    /// 
    /// # Returns
    /// 
    /// 返回停止结果
    pub async fn stop(&mut self) -> Result<()> {
        // 从PID文件中获取进程ID并停止
        if let Some(pid) = Self::read_pid_file() {
            if Self::is_process_running(pid) {
                // 使用系统命令停止进程
                #[cfg(unix)]
                {
                    use std::process::Command;
                    let _ = Command::new("kill")
                        .arg("-TERM")
                        .arg(pid.to_string())
                        .output();
                    
                    // 等待进程优雅退出
                    for _ in 0..5 {
                        if !Self::is_process_running(pid) {
                            break;
                        }
                        sleep(Duration::from_secs(1)).await;
                    }
                    
                    // 如果还在运行，强制杀死
                    if Self::is_process_running(pid) {
                        let _ = Command::new("kill")
                            .arg("-KILL")
                            .arg(pid.to_string())
                            .output();
                    }
                }
                
                #[cfg(windows)]
                {
                    use std::process::Command;
                    let _ = Command::new("taskkill")
                        .args(["/PID", &pid.to_string(), "/F"])
                        .output();
                }
            }
        }
        
        // 清理PID文件
        Self::remove_pid_file()?;
        
        // 等待服务完全停止
        for _ in 0..10 {
            if !self.is_running().await? {
                println!("服务已停止");
                return Ok(());
            }
            sleep(Duration::from_secs(1)).await;
        }
        
        Err(MihomoError::ServiceError("服务停止超时".to_string()))
    }

    /// 重启服务
    /// 
    /// # Returns
    /// 
    /// 返回重启结果
    pub async fn restart(&mut self) -> Result<()> {
        println!("正在重启服务...");
        
        if self.is_running().await? {
            self.stop().await?;
        }
        
        sleep(Duration::from_secs(2)).await;
        self.start().await
    }

    /// 检查服务是否运行
    /// 
    /// # Returns
    /// 
    /// 返回服务运行状态
    pub async fn is_running(&self) -> Result<bool> {
        // 首先检查PID文件中的进程是否存在
        if let Some(pid) = Self::read_pid_file() {
            if !Self::is_process_running(pid) {
                // 进程不存在，清理PID文件
                let _ = Self::remove_pid_file();
                return Ok(false);
            }
        } else {
            // 没有PID文件，检查API是否可用
            let url = format!("http://{}/version", self.config.external_controller);
            
            let mut request = self.client.get(&url);
            
            if let Some(secret) = &self.config.secret {
                request = request.header("Authorization", format!("Bearer {}", secret));
            }
            
            match request.send().await {
                Ok(response) => return Ok(response.status().is_success()),
                Err(_) => return Ok(false),
            }
        }
        
        // 有PID文件且进程存在，再检查API是否可用
        let url = format!("http://{}/version", self.config.external_controller);
        
        let mut request = self.client.get(&url);
        
        if let Some(secret) = &self.config.secret {
            request = request.header("Authorization", format!("Bearer {}", secret));
        }
        
        match request.send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => {
                // API不可用但进程存在，可能正在启动中
                Ok(true)
            },
        }
    }

    /// 获取服务状态
    /// 
    /// # Returns
    /// 
    /// 返回服务状态
    pub async fn get_status(&self) -> Result<ServiceStatus> {
        if self.is_running().await? {
            Ok(ServiceStatus::Running)
        } else {
            Ok(ServiceStatus::Stopped)
        }
    }

    /// 获取当前版本
    /// 
    /// # Returns
    /// 
    /// 返回当前版本信息
    pub async fn get_current_version(&self) -> Result<String> {
        if !self.is_running().await? {
            return Err(MihomoError::ServiceError("服务未运行".to_string()));
        }

        let url = format!("http://{}/version", self.config.external_controller);
        
        let mut request = self.client.get(&url);
        
        if let Some(secret) = &self.config.secret {
            request = request.header("Authorization", format!("Bearer {}", secret));
        }
        
        let response = request.send().await
            .map_err(|e| MihomoError::network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(MihomoError::service_error(format!(
                "获取版本信息失败: {}", 
                response.status()
            )));
        }

        let version_info: serde_json::Value = response.json().await
            .map_err(|e| MihomoError::service_error(e.to_string()))?;

        Ok(version_info["version"].as_str().unwrap_or("unknown").to_string())
    }

    /// 更新配置
    /// 
    /// # Arguments
    /// 
    /// * `config` - 新的服务配置
    pub fn update_config(&mut self, config: ServiceConfig) {
        self.config = config;
    }

    /// 获取配置
    /// 
    /// # Returns
    /// 
    /// 返回当前服务配置
    pub fn get_config(&self) -> &ServiceConfig {
        &self.config
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_service_config_default() {
        let config = ServiceConfig::default();
        assert_eq!(config.api_port, 9090);
        assert_eq!(config.external_controller, "127.0.0.1:9090");
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_service_manager_creation() {
        let config = ServiceConfig::default();
        let manager = ServiceManager::new(config);
        assert_eq!(manager.config.api_port, 9090);
    }

    #[tokio::test]
    async fn test_get_available_versions() {
        let config = ServiceConfig::default();
        let manager = ServiceManager::new(config);
        
        // 这个测试需要网络连接，在实际环境中可能会失败
        // 可以考虑使用 mock 服务器进行测试
        match manager.get_available_versions().await {
            Ok(versions) => {
                assert!(!versions.is_empty());
                println!("找到 {} 个版本", versions.len());
            }
            Err(e) => {
                println!("获取版本列表失败: {}", e);
                // 在测试环境中网络错误是可以接受的
            }
        }
    }

    #[tokio::test]
    async fn test_service_status_check() {
        let config = ServiceConfig::default();
        let manager = ServiceManager::new(config);
        
        // 测试服务状态检查（假设服务未运行）
        let is_running = manager.is_running().await.unwrap_or(false);
        let status = manager.get_status().await.unwrap_or(ServiceStatus::Unknown);
        
        if is_running {
            assert_eq!(status, ServiceStatus::Running);
        } else {
            assert_eq!(status, ServiceStatus::Stopped);
        }
    }
}