//! Version comparison for CLI and template compatibility

use anyhow::Result;
use semver::Version;

/// Compare CLI version against template version
/// Returns a warning message if the CLI is older than the template expects
pub fn check_compatibility(
    cli_version: &str,
    template_version: &str,
    upgrade_command: &str,
) -> Option<String> {
    let cli_ver = match Version::parse(cli_version) {
        Ok(v) => v,
        Err(_) => return None, // Can't compare, skip warning
    };

    let template_ver = match Version::parse(template_version) {
        Ok(v) => v,
        Err(_) => return None, // Can't compare, skip warning
    };

    if cli_ver < template_ver {
        Some(format!(
            "Warning: This template was designed for CLI version {} or newer.\n\
             You are running version {}.\n\
             Consider updating: {}",
            template_version, cli_version, upgrade_command
        ))
    } else {
        None
    }
}

/// Parse version string, handling various formats like "0.11.0", "v0.11.0", "iii 0.11.0"
pub fn parse_version(version_str: &str) -> Result<Version> {
    let trimmed = version_str.trim();
    // Take the last whitespace-separated token (handles "iii 0.11.0", "iii-engine 0.11.0", etc.)
    let token = trimmed.rsplit_once(' ').map(|(_, v)| v).unwrap_or(trimmed);
    // Strip leading 'v' if present
    let cleaned = token.strip_prefix('v').unwrap_or(token);
    Version::parse(cleaned).map_err(|e| anyhow::anyhow!("Invalid version '{}': {}", version_str, e))
}

/// Check that the installed iii engine version meets the template's minimum requirement.
/// Returns Ok(version_string) on success, Err with a user-facing message on failure.
pub fn check_iii_engine_version(min_version: &str) -> std::result::Result<String, String> {
    let tool = crate::runtime::tool::iii_tool();

    let raw_version = tool
        .get_version()
        .ok_or_else(|| {
            format!(
                "This template requires iii >= {}, but iii is not installed or not in PATH.\n\
                 Install it from https://iii.dev/docs or run: curl -fsSL https://install.iii.dev/latest.sh | sh",
                min_version
            )
        })?;

    let installed = parse_version(&raw_version).map_err(|_| {
        format!(
            "Could not parse iii version from: {}\n\
             Please update iii: iii update",
            raw_version
        )
    })?;

    let required = Version::parse(min_version).map_err(|_| {
        format!("Invalid min_iii_version in template.yaml: {}", min_version)
    })?;

    if installed < required {
        Err(format!(
            "This template requires iii >= {}, but you have {}.\n\
             Please update: iii update",
            min_version, installed
        ))
    } else {
        Ok(installed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_older_than_template() {
        let warning = check_compatibility("0.1.0", "0.2.0", "cargo install test-cli --force");
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("0.2.0"));
    }

    #[test]
    fn test_cli_same_as_template() {
        let warning = check_compatibility("0.1.0", "0.1.0", "cargo install test-cli --force");
        assert!(warning.is_none());
    }

    #[test]
    fn test_cli_newer_than_template() {
        let warning = check_compatibility("0.2.0", "0.1.0", "cargo install test-cli --force");
        assert!(warning.is_none());
    }

    #[test]
    fn test_invalid_versions() {
        // Should return None (no warning) for invalid versions
        let warning = check_compatibility("invalid", "0.1.0", "cargo install test-cli --force");
        assert!(warning.is_none());
    }

    #[test]
    fn test_parse_version_plain() {
        let v = parse_version("0.11.0").unwrap();
        assert_eq!(v, Version::new(0, 11, 0));
    }

    #[test]
    fn test_parse_version_with_v_prefix() {
        let v = parse_version("v0.11.0").unwrap();
        assert_eq!(v, Version::new(0, 11, 0));
    }

    #[test]
    fn test_parse_version_with_tool_prefix() {
        let v = parse_version("iii 0.11.0").unwrap();
        assert_eq!(v, Version::new(0, 11, 0));
    }

    #[test]
    fn test_parse_version_with_tool_prefix_and_v() {
        let v = parse_version("iii v0.11.0").unwrap();
        assert_eq!(v, Version::new(0, 11, 0));
    }

    #[test]
    fn test_parse_version_with_whitespace() {
        let v = parse_version("  0.11.0  ").unwrap();
        assert_eq!(v, Version::new(0, 11, 0));
    }

    #[test]
    fn test_parse_version_invalid() {
        assert!(parse_version("not-a-version").is_err());
    }
}
