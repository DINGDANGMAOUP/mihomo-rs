use super::{ErrorCode, MihomoError, Result};
use std::path::{Component, Path};

fn validate_simple_name(name: &str, kind: &str, allow_plus: bool, code: ErrorCode) -> Result<()> {
    if name.is_empty() {
        return Err(MihomoError::config_with_code(
            code,
            format!("{kind} cannot be empty"),
        ));
    }

    let path = Path::new(name);
    if path.is_absolute()
        || name.contains('/')
        || name.contains('\\')
        || !matches!(path.components().next(), Some(Component::Normal(_)))
        || path.components().count() != 1
    {
        return Err(MihomoError::config_with_code(
            code,
            format!("Invalid {kind} '{name}'"),
        ));
    }

    if !name.chars().all(|c| {
        c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') || (allow_plus && c == '+')
    }) {
        return Err(MihomoError::config_with_code(
            code,
            format!("Invalid {kind} '{name}'"),
        ));
    }

    Ok(())
}

pub fn validate_profile_name(name: &str) -> Result<()> {
    validate_simple_name(name, "profile name", false, ErrorCode::InvalidProfileName)
}

pub fn validate_version_name(name: &str) -> Result<()> {
    validate_simple_name(name, "version", true, ErrorCode::InvalidVersion)
}

#[cfg(test)]
mod tests {
    use super::{validate_profile_name, validate_version_name};

    #[test]
    fn profile_name_validation_rules() {
        assert!(validate_profile_name("alpha-1.2_ok").is_ok());
        assert!(validate_profile_name("../evil").is_err());
        assert!(validate_profile_name("a/b").is_err());
        assert!(validate_profile_name("a\\b").is_err());
        assert!(validate_profile_name("bad name").is_err());
    }

    #[test]
    fn version_name_validation_rules() {
        assert!(validate_version_name("v1.2.3-alpha+build.1").is_ok());
        assert!(validate_version_name("../v1").is_err());
        assert!(validate_version_name("a/b").is_err());
        assert!(validate_version_name("a\\b").is_err());
        assert!(validate_version_name("bad version").is_err());
    }
}
