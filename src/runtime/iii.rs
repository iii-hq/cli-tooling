//! iii runtime detection and installation

use anyhow::Result;
use colored::Colorize;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;

/// The install command that will be executed
pub const INSTALL_COMMAND: &str = "curl -fsSL https://iii.sh/install.sh | sh";

/// Timeout for installation (30 seconds)
const INSTALL_TIMEOUT: Duration = Duration::from_secs(30);

/// Check if iii is installed and available in PATH
pub fn is_installed() -> bool {
    std::process::Command::new("which")
        .arg("iii")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Get the installed iii version (if available)
pub fn get_version() -> Option<String> {
    std::process::Command::new("iii")
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
/// Shows the command being executed and streams output
pub async fn install() -> Result<()> {
    println!();
    println!("{}", "Installing iii...".cyan().bold());
    println!("{} {}", "Running:".dimmed(), INSTALL_COMMAND.yellow());
    println!();

    // Create the command
    let mut child = TokioCommand::new("sh")
        .arg("-c")
        .arg(INSTALL_COMMAND)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Get stdout and stderr
    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    // Stream output with timeout
    let output_task = async {
        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => println!("  {}", line),
                        Ok(None) => break,
                        Err(e) => {
                            eprintln!("{} {}", "Error reading stdout:".red(), e);
                            break;
                        }
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => eprintln!("  {}", line.yellow()),
                        Ok(None) => {}
                        Err(e) => {
                            eprintln!("{} {}", "Error reading stderr:".red(), e);
                        }
                    }
                }
            }
        }
    };

    // Wait for output with timeout
    match timeout(INSTALL_TIMEOUT, output_task).await {
        Ok(_) => {}
        Err(_) => {
            // Kill the process on timeout
            let _ = child.kill().await;
            println!();
            anyhow::bail!(
                "Installation timed out after {} seconds.\n\
                 The server may be unreachable. Please try again later or install manually:\n\
                 {}",
                INSTALL_TIMEOUT.as_secs(),
                INSTALL_COMMAND
            );
        }
    }

    // Wait for process to complete with timeout
    match timeout(Duration::from_secs(5), child.wait()).await {
        Ok(Ok(status)) => {
            println!();
            if status.success() {
                println!("{}", "✓ iii installed successfully!".green().bold());
                Ok(())
            } else {
                anyhow::bail!(
                    "Installation failed with exit code: {}\n\
                     Please try installing manually: {}",
                    status.code().unwrap_or(-1),
                    INSTALL_COMMAND
                );
            }
        }
        Ok(Err(e)) => {
            anyhow::bail!("Failed to wait for installer: {}", e);
        }
        Err(_) => {
            let _ = child.kill().await;
            anyhow::bail!(
                "Installation process hung. Please try installing manually:\n{}",
                INSTALL_COMMAND
            );
        }
    }
}

/// Open the iii documentation in the default browser
pub fn open_docs() -> Result<()> {
    println!("{}", "Opening iii documentation in your browser...".cyan());
    open::that("https://iii.sh")?;
    Ok(())
}

/// The iii documentation URL
pub const DOCS_URL: &str = "https://iii.sh";

/// The iii install script URL  
pub const INSTALL_SCRIPT_URL: &str = "https://iii.sh/install.sh";
