//! iii runtime detection and installation

use anyhow::Result;
use std::process::Command;

/// Check if iii is installed and available in PATH
pub fn is_installed() -> bool {
    Command::new("which")
        .arg("iii")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Get the installed iii version (if available)
pub fn get_version() -> Option<String> {
    Command::new("iii")
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

/// Install iii using the official install script
pub async fn install() -> Result<()> {
    let status = Command::new("sh")
        .arg("-c")
        .arg("curl -fsSL https://iii.sh/install.sh | sh")
        .status()?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("Failed to install iii. Please try manual installation.")
    }
}

/// Open the iii documentation in the default browser
pub fn open_docs() -> Result<()> {
    open::that("https://iii.sh")?;
    Ok(())
}

/// The iii documentation URL
pub const DOCS_URL: &str = "https://iii.sh";

/// The iii install script URL
pub const INSTALL_SCRIPT_URL: &str = "https://iii.sh/install.sh";
