use crate::config::ConfigManager;
use crate::core::{get_home_dir, MihomoClient, MihomoError};
use crate::service::{process, ServiceManager, ServiceStatus};
use crate::version::VersionManager;
use serde::Serialize;
use std::fmt;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorStatus {
    Pass,
    Warn,
    Fail,
    Skip,
}

impl DoctorStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warn => "WARN",
            Self::Fail => "FAIL",
            Self::Skip => "SKIP",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorCheckResult {
    pub id: String,
    pub category: String,
    pub status: DoctorStatus,
    pub summary: String,
    pub detail: Option<String>,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub started_at_unix: u64,
    pub finished_at_unix: u64,
    pub checks: Vec<DoctorCheckResult>,
}

impl DoctorReport {
    pub fn has_failures(&self) -> bool {
        self.checks
            .iter()
            .any(|check| check.status == DoctorStatus::Fail)
    }

    pub fn count_by_status(&self, status: DoctorStatus) -> usize {
        self.checks
            .iter()
            .filter(|check| check.status == status)
            .count()
    }
}

#[derive(Debug, Clone)]
pub struct DoctorRunOptions {
    pub only: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct DoctorCheckMeta {
    pub id: &'static str,
    pub category: &'static str,
    pub summary: &'static str,
    pub why: &'static str,
    pub fail_means: &'static str,
    pub hint: &'static str,
    pub fixable: bool,
    pub default_enabled: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct DoctorExplain {
    pub id: &'static str,
    pub category: &'static str,
    pub summary: &'static str,
    pub why: &'static str,
    pub fail_means: &'static str,
    pub hint: &'static str,
    pub fixable: bool,
    pub default_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorFixAction {
    pub id: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorFixReport {
    pub fixes: Vec<DoctorFixAction>,
}

impl fmt::Display for DoctorCheckMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id)
    }
}

const CHECKS: &[DoctorCheckMeta] = &[
    DoctorCheckMeta {
        id: "config.settings_parse",
        category: "config",
        summary: "config.toml parses cleanly",
        why: "Doctor should flag broken app-level settings before other checks derive paths from them.",
        fail_means: "The app settings file exists but cannot be parsed as TOML.",
        hint: "Fix the TOML syntax in config.toml or remove the broken fields.",
        fixable: false,
        default_enabled: true,
    },
    DoctorCheckMeta {
        id: "config.configs_dir",
        category: "config",
        summary: "resolved configs directory is understandable",
        why: "Users need to know which configs directory is active and whether it comes from env, config.toml, or default home.",
        fail_means: "The active configs directory cannot be resolved.",
        hint: "Check MIHOMO_CONFIGS_DIR and [paths].configs_dir for empty or invalid paths.",
        fixable: true,
        default_enabled: true,
    },
    DoctorCheckMeta {
        id: "config.current_profile",
        category: "config",
        summary: "current profile name and file resolve cleanly",
        why: "Most commands depend on the current profile and its YAML path.",
        fail_means: "The selected profile is invalid or its resolved path cannot be constructed.",
        hint: "Set a valid default.profile in config.toml or switch to a valid profile name.",
        fixable: false,
        default_enabled: true,
    },
    DoctorCheckMeta {
        id: "config.current_yaml",
        category: "config",
        summary: "current config YAML exists and parses",
        why: "Runtime operations depend on the current config file being present and valid YAML.",
        fail_means: "The current profile YAML is missing or invalid.",
        hint: "Create the current profile config or fix its YAML syntax.",
        fixable: true,
        default_enabled: true,
    },
    DoctorCheckMeta {
        id: "version.binary_available",
        category: "version",
        summary: "default mihomo binary is available",
        why: "Service lifecycle commands require a resolved default binary.",
        fail_means: "No default version is configured or its binary does not exist.",
        hint: "Install a version and set it as default with version use.",
        fixable: false,
        default_enabled: true,
    },
    DoctorCheckMeta {
        id: "service.pid_state",
        category: "service",
        summary: "service PID record is consistent",
        why: "Stale PID files make lifecycle diagnostics confusing.",
        fail_means: "The service state check itself failed unexpectedly.",
        hint: "If the service is stopped but the pid file keeps reappearing, inspect the app home directory.",
        fixable: false,
        default_enabled: true,
    },
    DoctorCheckMeta {
        id: "service.stale_pid",
        category: "service",
        summary: "pid file does not point to a stale process",
        why: "A stale or malformed pid file makes service status and restart behavior confusing.",
        fail_means: "The pid file is malformed or points to a dead or mismatched process.",
        hint: "Run doctor fix --only service.stale_pid to remove the stale pid file.",
        fixable: true,
        default_enabled: true,
    },
    DoctorCheckMeta {
        id: "controller.external_controller",
        category: "controller",
        summary: "external-controller resolves to a usable URL",
        why: "Proxy, telemetry, and connection features rely on a valid controller endpoint.",
        fail_means: "The current config cannot provide a valid external-controller value.",
        hint: "Set external-controller to host:port, http(s)://host:port, or a unix socket path.",
        fixable: true,
        default_enabled: true,
    },
    DoctorCheckMeta {
        id: "controller.api_reachable",
        category: "controller",
        summary: "controller API responds when service is running",
        why: "Connection, proxy, and telemetry features depend on the controller being reachable.",
        fail_means: "The service is running but the current controller endpoint does not respond.",
        hint: "Check the external-controller value and whether the running mihomo instance is healthy.",
        fixable: false,
        default_enabled: true,
    },
];

pub fn list_checks() -> &'static [DoctorCheckMeta] {
    CHECKS
}

pub fn explain_check(check_id: &str) -> anyhow::Result<DoctorExplain> {
    let check = CHECKS
        .iter()
        .find(|check| check.id == check_id)
        .ok_or_else(|| anyhow::anyhow!("unknown doctor check '{}'", check_id))?;

    Ok(DoctorExplain {
        id: check.id,
        category: check.category,
        summary: check.summary,
        why: check.why,
        fail_means: check.fail_means,
        hint: check.hint,
        fixable: check.fixable,
        default_enabled: check.default_enabled,
    })
}

pub async fn run_doctor(options: DoctorRunOptions) -> DoctorReport {
    let started_at_unix = unix_ts();
    let filter = CheckFilter::parse(options.only.as_deref());
    let mut checks = Vec::new();

    if filter.matches("config.settings_parse", "config") {
        checks.push(check_settings_parse().await);
    }
    if filter.matches("config.configs_dir", "config") {
        checks.push(check_configs_dir().await);
    }
    if filter.matches("config.current_profile", "config") {
        checks.push(check_current_profile().await);
    }
    if filter.matches("config.current_yaml", "config") {
        checks.push(check_current_yaml().await);
    }
    if filter.matches("version.binary_available", "version") {
        checks.push(check_binary_available().await);
    }
    if filter.matches("service.pid_state", "service") {
        checks.push(check_service_pid_state().await);
    }
    if filter.matches("service.stale_pid", "service") {
        checks.push(check_stale_pid().await);
    }
    if filter.matches("controller.external_controller", "controller") {
        checks.push(check_external_controller().await);
    }
    if filter.matches("controller.api_reachable", "controller") {
        checks.push(check_controller_api_reachable().await);
    }

    DoctorReport {
        started_at_unix,
        finished_at_unix: unix_ts(),
        checks,
    }
}

pub async fn fix_doctor(options: DoctorRunOptions) -> anyhow::Result<DoctorFixReport> {
    let filter = CheckFilter::parse(options.only.as_deref());
    let mut fixes = Vec::new();

    if filter.matches("config.configs_dir", "config") {
        if let Some(fix) = fix_configs_dir().await? {
            fixes.push(fix);
        }
    }
    if filter.matches("config.current_yaml", "config") {
        if let Some(fix) = fix_current_yaml().await? {
            fixes.push(fix);
        }
    }
    if filter.matches("controller.external_controller", "controller") {
        if let Some(fix) = fix_external_controller().await? {
            fixes.push(fix);
        }
    }
    if filter.matches("service.stale_pid", "service") {
        if let Some(fix) = fix_stale_pid().await? {
            fixes.push(fix);
        }
    }

    Ok(DoctorFixReport { fixes })
}

#[derive(Debug, Default)]
struct CheckFilter {
    tokens: Vec<String>,
}

impl CheckFilter {
    fn parse(raw: Option<&str>) -> Self {
        let tokens = raw
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(|part| part.to_string())
            .collect();
        Self { tokens }
    }

    fn matches(&self, id: &str, category: &str) -> bool {
        self.tokens.is_empty()
            || self
                .tokens
                .iter()
                .any(|token| token == id || token == category)
    }
}

async fn check_settings_parse() -> DoctorCheckResult {
    let manager = match ConfigManager::new() {
        Ok(manager) => manager,
        Err(err) => return fail_result("config.settings_parse", "config", err.to_string(), None),
    };

    let settings_file = home_settings_path(&manager);
    if !settings_file.exists() {
        return pass_result(
            "config.settings_parse",
            "config",
            "config.toml not found; defaults will be used",
            None,
        );
    }

    match tokio::fs::read_to_string(&settings_file).await {
        Ok(content) => match toml::from_str::<toml::Value>(&content) {
            Ok(_) => pass_result(
                "config.settings_parse",
                "config",
                &format!("Parsed '{}'", settings_file.display()),
                None,
            ),
            Err(err) => fail_result(
                "config.settings_parse",
                "config",
                format!("Invalid config.toml: {}", err),
                Some("Fix the TOML syntax in config.toml."),
            ),
        },
        Err(err) => fail_result(
            "config.settings_parse",
            "config",
            format!("Failed to read '{}': {}", settings_file.display(), err),
            None,
        ),
    }
}

async fn check_configs_dir() -> DoctorCheckResult {
    let manager = match ConfigManager::new() {
        Ok(manager) => manager,
        Err(err) => return fail_result("config.configs_dir", "config", err.to_string(), None),
    };

    match manager.get_config_dir_info() {
        Ok(info) => {
            if info.path.is_dir() {
                pass_result(
                    "config.configs_dir",
                    "config",
                    &format!(
                        "Using '{}' from {}",
                        info.path.display(),
                        info.source.as_str()
                    ),
                    None,
                )
            } else {
                let reason = if info.path.exists() {
                    "exists but is not a directory"
                } else {
                    "does not exist yet"
                };
                warn_result(
                    "config.configs_dir",
                    "config",
                    &format!(
                        "Resolved configs directory '{}' from {} {}",
                        info.path.display(),
                        info.source.as_str(),
                        reason
                    ),
                    Some("Run doctor fix --only config.configs_dir to create it."),
                )
            }
        }
        Err(err) => fail_result(
            "config.configs_dir",
            "config",
            format!("Cannot resolve configs directory: {}", err),
            Some("Check MIHOMO_CONFIGS_DIR and [paths].configs_dir."),
        ),
    }
}

async fn check_current_profile() -> DoctorCheckResult {
    let manager = match ConfigManager::new() {
        Ok(manager) => manager,
        Err(err) => return fail_result("config.current_profile", "config", err.to_string(), None),
    };

    match manager.get_current().await {
        Ok(profile) => match manager.get_current_path().await {
            Ok(path) => pass_result(
                "config.current_profile",
                "config",
                &format!(
                    "Current profile '{}' resolves to '{}'",
                    profile,
                    path.display()
                ),
                None,
            ),
            Err(err) => fail_result(
                "config.current_profile",
                "config",
                format!("Current profile '{}' is unusable: {}", profile, err),
                None,
            ),
        },
        Err(err) => fail_result(
            "config.current_profile",
            "config",
            format!("Cannot determine current profile: {}", err),
            None,
        ),
    }
}

async fn check_current_yaml() -> DoctorCheckResult {
    let manager = match ConfigManager::new() {
        Ok(manager) => manager,
        Err(err) => return fail_result("config.current_yaml", "config", err.to_string(), None),
    };

    let path = match manager.get_current_path().await {
        Ok(path) => path,
        Err(err) => {
            return skip_result(
                "config.current_yaml",
                "config",
                &format!(
                    "Skipped because current profile path is unavailable: {}",
                    err
                ),
            );
        }
    };

    if !path.exists() {
        return fail_result(
            "config.current_yaml",
            "config",
            format!("Current config '{}' does not exist", path.display()),
            Some("Run config show/current or create the current profile YAML."),
        );
    }

    match tokio::fs::read_to_string(&path).await {
        Ok(content) => match serde_yaml::from_str::<serde_yaml::Value>(&content) {
            Ok(_) => pass_result(
                "config.current_yaml",
                "config",
                &format!("Parsed '{}'", path.display()),
                None,
            ),
            Err(err) => fail_result(
                "config.current_yaml",
                "config",
                format!("Invalid YAML in '{}': {}", path.display(), err),
                Some("Fix the YAML syntax in the current profile file."),
            ),
        },
        Err(err) => fail_result(
            "config.current_yaml",
            "config",
            format!("Failed to read '{}': {}", path.display(), err),
            None,
        ),
    }
}

async fn check_binary_available() -> DoctorCheckResult {
    let manager = match VersionManager::new() {
        Ok(manager) => manager,
        Err(err) => {
            return fail_result("version.binary_available", "version", err.to_string(), None);
        }
    };

    match manager.get_binary_path(None).await {
        Ok(path) => pass_result(
            "version.binary_available",
            "version",
            &format!("Default binary found at '{}'", path.display()),
            None,
        ),
        Err(err) => fail_result(
            "version.binary_available",
            "version",
            format!("Default binary unavailable: {}", err),
            Some("Install a version and set it as default."),
        ),
    }
}

async fn check_service_pid_state() -> DoctorCheckResult {
    let service = ServiceManager::new(PathBuf::from("mihomo"), PathBuf::from("config.yaml"));
    match service.status().await {
        Ok(ServiceStatus::Running(pid)) => pass_result(
            "service.pid_state",
            "service",
            &format!("Service PID record is healthy (running pid {})", pid),
            None,
        ),
        Ok(ServiceStatus::Stopped) => pass_result(
            "service.pid_state",
            "service",
            "Service is stopped and PID state is clean",
            None,
        ),
        Err(err) => fail_result(
            "service.pid_state",
            "service",
            format!("Service PID state check failed: {}", err),
            None,
        ),
    }
}

async fn check_external_controller() -> DoctorCheckResult {
    let manager = match ConfigManager::new() {
        Ok(manager) => manager,
        Err(err) => {
            return fail_result(
                "controller.external_controller",
                "controller",
                err.to_string(),
                None,
            );
        }
    };

    match manager.get_external_controller().await {
        Ok(url) => pass_result(
            "controller.external_controller",
            "controller",
            &format!("Current external-controller resolves to '{}'", url),
            None,
        ),
        Err(MihomoError::NotFound(err)) => skip_result(
            "controller.external_controller",
            "controller",
            &format!("Skipped because current config is missing: {}", err),
        ),
        Err(err) => fail_result(
            "controller.external_controller",
            "controller",
            format!("Invalid external-controller: {}", err),
            Some("Set external-controller to a valid TCP URL or unix socket path."),
        ),
    }
}

async fn check_stale_pid() -> DoctorCheckResult {
    let pid_file = pid_file_path();
    check_stale_pid_at(&pid_file).await
}

async fn check_stale_pid_at(pid_file: &PathBuf) -> DoctorCheckResult {
    if !pid_file.exists() {
        return pass_result(
            "service.stale_pid",
            "service",
            "No pid file is present",
            None,
        );
    }

    match process::read_pid_record(&pid_file).await {
        Ok(record) => {
            if process::is_process_alive_checked(record.pid, record.start_time) {
                pass_result(
                    "service.stale_pid",
                    "service",
                    &format!("pid file '{}' tracks a live process", pid_file.display()),
                    None,
                )
            } else {
                warn_result(
                    "service.stale_pid",
                    "service",
                    &format!(
                        "pid file '{}' points to stale process {}",
                        pid_file.display(),
                        record.pid
                    ),
                    Some("Run doctor fix --only service.stale_pid to remove it."),
                )
            }
        }
        Err(err) => warn_result(
            "service.stale_pid",
            "service",
            &format!("pid file '{}' is invalid: {}", pid_file.display(), err),
            Some("Run doctor fix --only service.stale_pid to remove it."),
        ),
    }
}

async fn check_controller_api_reachable() -> DoctorCheckResult {
    match current_service_status().await {
        Ok(ServiceStatus::Stopped) => {
            return skip_result(
                "controller.api_reachable",
                "controller",
                "Skipped because service is not running",
            );
        }
        Ok(ServiceStatus::Running(_)) => {}
        Err(err) => {
            return fail_result(
                "controller.api_reachable",
                "controller",
                format!("Unable to determine service state: {}", err),
                None,
            );
        }
    }

    let manager = match ConfigManager::new() {
        Ok(manager) => manager,
        Err(err) => {
            return fail_result(
                "controller.api_reachable",
                "controller",
                format!("Cannot create ConfigManager: {}", err),
                None,
            );
        }
    };

    let url = match manager.get_external_controller().await {
        Ok(url) => url,
        Err(err) => {
            return fail_result(
                "controller.api_reachable",
                "controller",
                format!("Cannot resolve external-controller: {}", err),
                None,
            );
        }
    };

    let client = match MihomoClient::new(&url, None) {
        Ok(client) => client,
        Err(err) => {
            return fail_result(
                "controller.api_reachable",
                "controller",
                format!("Cannot create controller client: {}", err),
                None,
            );
        }
    };

    match client.get_version().await {
        Ok(version) => pass_result(
            "controller.api_reachable",
            "controller",
            &format!(
                "Controller '{}' responded with mihomo {}",
                url, version.version
            ),
            None,
        ),
        Err(err) => fail_result(
            "controller.api_reachable",
            "controller",
            format!("Controller '{}' is unreachable: {}", url, err),
            Some("Check whether the service and external-controller endpoint match."),
        ),
    }
}

async fn fix_configs_dir() -> anyhow::Result<Option<DoctorFixAction>> {
    let manager = ConfigManager::new()?;
    let info = manager.get_config_dir_info()?;
    if info.path.is_dir() {
        return Ok(None);
    }
    tokio::fs::create_dir_all(&info.path).await?;
    Ok(Some(DoctorFixAction {
        id: "config.configs_dir".to_string(),
        summary: format!("Created configs directory '{}'", info.path.display()),
    }))
}

async fn fix_current_yaml() -> anyhow::Result<Option<DoctorFixAction>> {
    let manager = ConfigManager::new()?;
    let path = manager.get_current_path().await?;
    if path.exists() {
        let content = tokio::fs::read_to_string(&path).await?;
        if serde_yaml::from_str::<serde_yaml::Value>(&content).is_ok() {
            return Ok(None);
        }
        anyhow::bail!(
            "refusing to overwrite invalid current config '{}'",
            path.display()
        );
    }
    manager.ensure_default_config().await?;
    Ok(Some(DoctorFixAction {
        id: "config.current_yaml".to_string(),
        summary: format!("Ensured default config exists at '{}'", path.display()),
    }))
}

async fn fix_external_controller() -> anyhow::Result<Option<DoctorFixAction>> {
    let manager = ConfigManager::new()?;
    let before = manager.get_external_controller().await.ok();
    let after = manager.ensure_external_controller().await?;
    if before.as_deref() == Some(after.as_str()) {
        return Ok(None);
    }
    Ok(Some(DoctorFixAction {
        id: "controller.external_controller".to_string(),
        summary: format!("Ensured external-controller resolves to '{}'", after),
    }))
}

async fn fix_stale_pid() -> anyhow::Result<Option<DoctorFixAction>> {
    let pid_file = pid_file_path();
    fix_stale_pid_at(&pid_file).await
}

async fn fix_stale_pid_at(pid_file: &PathBuf) -> anyhow::Result<Option<DoctorFixAction>> {
    if !pid_file.exists() {
        return Ok(None);
    }

    let should_remove = match process::read_pid_record(&pid_file).await {
        Ok(record) => !process::is_process_alive_checked(record.pid, record.start_time),
        Err(_) => true,
    };

    if !should_remove {
        return Ok(None);
    }

    process::remove_pid_file(&pid_file).await?;
    Ok(Some(DoctorFixAction {
        id: "service.stale_pid".to_string(),
        summary: format!("Removed stale pid file '{}'", pid_file.display()),
    }))
}

fn home_settings_path(manager: &ConfigManager) -> PathBuf {
    current_home_of(manager).join("config.toml")
}

fn current_home_of(_manager: &ConfigManager) -> PathBuf {
    get_home_dir().unwrap_or_else(|_| PathBuf::from("."))
}

async fn current_service_status() -> crate::core::Result<ServiceStatus> {
    let service = ServiceManager::new(PathBuf::from("mihomo"), PathBuf::from("config.yaml"));
    service.status().await
}

fn pid_file_path() -> PathBuf {
    get_home_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("mihomo.pid")
}

fn unix_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn pass_result(id: &str, category: &str, summary: &str, hint: Option<&str>) -> DoctorCheckResult {
    DoctorCheckResult {
        id: id.to_string(),
        category: category.to_string(),
        status: DoctorStatus::Pass,
        summary: summary.to_string(),
        detail: None,
        hint: hint.map(|value| value.to_string()),
    }
}

fn skip_result(id: &str, category: &str, summary: &str) -> DoctorCheckResult {
    DoctorCheckResult {
        id: id.to_string(),
        category: category.to_string(),
        status: DoctorStatus::Skip,
        summary: summary.to_string(),
        detail: None,
        hint: None,
    }
}

fn warn_result(id: &str, category: &str, summary: &str, hint: Option<&str>) -> DoctorCheckResult {
    DoctorCheckResult {
        id: id.to_string(),
        category: category.to_string(),
        status: DoctorStatus::Warn,
        summary: summary.to_string(),
        detail: None,
        hint: hint.map(|value| value.to_string()),
    }
}

fn fail_result(id: &str, category: &str, summary: String, hint: Option<&str>) -> DoctorCheckResult {
    DoctorCheckResult {
        id: id.to_string(),
        category: category.to_string(),
        status: DoctorStatus::Fail,
        summary,
        detail: None,
        hint: hint.map(|value| value.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        explain_check, fix_doctor, list_checks, run_doctor, DoctorRunOptions, DoctorStatus,
    };
    use crate::service::process;
    use tempfile::tempdir;

    #[test]
    fn explain_known_check() {
        let explain = explain_check("config.current_yaml").expect("explain current yaml");
        assert_eq!(explain.category, "config");
        assert!(explain.summary.contains("current config YAML"));
        assert!(explain.fixable);
    }

    #[test]
    fn list_contains_default_checks() {
        let checks = list_checks();
        assert!(checks
            .iter()
            .any(|check| check.id == "config.settings_parse"));
        assert!(checks
            .iter()
            .any(|check| check.id == "version.binary_available"));
        assert!(checks.iter().any(|check| check.id == "service.stale_pid"));
        assert!(checks
            .iter()
            .any(|check| check.id == "controller.api_reachable"));
    }

    #[tokio::test]
    async fn doctor_filter_matches_category_or_id() {
        let report = run_doctor(DoctorRunOptions {
            only: Some("version.binary_available,service".to_string()),
        })
        .await;

        assert_eq!(report.checks.len(), 3);
        assert!(report
            .checks
            .iter()
            .any(|check| check.id == "version.binary_available"));
        assert!(report
            .checks
            .iter()
            .any(|check| check.id == "service.pid_state"));
        assert!(report
            .checks
            .iter()
            .any(|check| check.id == "service.stale_pid"));
    }

    #[test]
    fn report_counts_statuses() {
        let report = super::DoctorReport {
            started_at_unix: 1,
            finished_at_unix: 2,
            checks: vec![
                super::DoctorCheckResult {
                    id: "a".to_string(),
                    category: "config".to_string(),
                    status: DoctorStatus::Pass,
                    summary: "ok".to_string(),
                    detail: None,
                    hint: None,
                },
                super::DoctorCheckResult {
                    id: "b".to_string(),
                    category: "config".to_string(),
                    status: DoctorStatus::Fail,
                    summary: "bad".to_string(),
                    detail: None,
                    hint: None,
                },
            ],
        };

        assert_eq!(report.count_by_status(DoctorStatus::Pass), 1);
        assert_eq!(report.count_by_status(DoctorStatus::Fail), 1);
        assert!(report.has_failures());
    }

    #[tokio::test]
    async fn doctor_fix_empty_filter_is_safe() {
        let report = fix_doctor(DoctorRunOptions {
            only: Some("version".to_string()),
        })
        .await
        .expect("fix doctor");
        assert!(report.fixes.is_empty());
    }

    #[tokio::test]
    async fn stale_pid_check_warns_and_fix_removes_file() {
        let temp = tempdir().expect("tempdir");
        let pid_file = temp.path().join("mihomo.pid");
        process::write_pid_record(&pid_file, u32::MAX, Some(1))
            .await
            .expect("write stale pid");

        let report = super::check_stale_pid_at(&pid_file).await;
        assert_eq!(report.status, DoctorStatus::Warn);

        let fixed = super::fix_stale_pid_at(&pid_file)
            .await
            .expect("fix stale pid");
        assert!(fixed.is_some());
        assert!(!pid_file.exists());
    }
}
