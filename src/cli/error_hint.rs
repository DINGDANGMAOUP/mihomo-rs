use crate::core::ErrorCode;
use crate::MihomoError;

struct ErrorHintRule {
    code: ErrorCode,
    hint: &'static str,
}

const ERROR_HINT_RULES: &[ErrorHintRule] = &[
    ErrorHintRule {
        code: ErrorCode::InvalidExternalController,
        hint: "expected formats: 127.0.0.1:9090 | :9090 | http://host:9090 | https://host:9090 | /path/to/mihomo.sock | unix:///path/to/mihomo.sock",
    },
    ErrorHintRule {
        code: ErrorCode::InvalidProfileName,
        hint: "profile can only include letters, numbers, '.', '_' and '-'",
    },
    ErrorHintRule {
        code: ErrorCode::InvalidVersion,
        hint: "version can only include letters, numbers, '.', '_', '-' and '+'",
    },
];

fn hint_for_error_code(code: ErrorCode) -> Option<&'static str> {
    ERROR_HINT_RULES
        .iter()
        .find(|rule| rule.code == code)
        .map(|rule| rule.hint)
}

pub fn format_cli_error(err: &anyhow::Error) -> String {
    if let Some(MihomoError::Config(msg) | MihomoError::Version(msg)) =
        err.downcast_ref::<MihomoError>()
    {
        let plain = msg.message.clone();
        let hint = msg.code.and_then(hint_for_error_code);
        if let Some(hint) = hint {
            return format!("Error: {}\nHint: {}", plain, hint);
        }
        return format!("Error: {}", plain);
    }
    format!("Error: {}", err)
}

#[cfg(test)]
mod tests {
    use super::format_cli_error;
    use crate::core::ErrorCode;
    use crate::MihomoError;

    #[test]
    fn format_cli_error_adds_hint_for_invalid_external_controller() {
        let err = anyhow::Error::new(MihomoError::config_with_code(
            ErrorCode::InvalidExternalController,
            "Invalid external-controller value '://invalid'",
        ));
        let rendered = format_cli_error(&err);
        assert!(rendered.contains("Invalid external-controller value '://invalid'"));
        assert!(rendered.contains("Hint: expected formats:"));
    }

    #[test]
    fn format_cli_error_adds_hint_for_invalid_profile_name() {
        let err = anyhow::Error::new(MihomoError::config_with_code(
            ErrorCode::InvalidProfileName,
            "Invalid profile name '../evil'",
        ));
        let rendered = format_cli_error(&err);
        assert!(rendered.contains("Hint: profile can only include"));
    }

    #[test]
    fn format_cli_error_adds_hint_for_invalid_version() {
        let err = anyhow::Error::new(MihomoError::version_with_code(
            ErrorCode::InvalidVersion,
            "Invalid version '../v1'",
        ));
        let rendered = format_cli_error(&err);
        assert!(rendered.contains("Invalid version '../v1'"));
        assert!(rendered.contains("Hint: version can only include"));
    }

    #[test]
    fn format_cli_error_falls_back_for_other_errors() {
        let err = anyhow::Error::new(MihomoError::NotFound("Profile 'x' not found".to_string()));
        let rendered = format_cli_error(&err);
        assert_eq!(rendered, "Error: Not found: Profile 'x' not found");
    }
}
