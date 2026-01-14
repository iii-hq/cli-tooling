//! Runtime detection for Node.js, Bun, and Python

use anyhow::Result;
use std::fmt;
use std::process::Command;

/// Supported languages/runtimes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    TypeScript,
    JavaScript,
    Python,
}

impl Language {
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::TypeScript => "TypeScript",
            Language::JavaScript => "JavaScript",
            Language::Python => "Python",
        }
    }

    /// File extensions that correspond to this language
    pub fn file_extensions(&self) -> &'static [&'static str] {
        match self {
            Language::TypeScript => &[".step.ts", ".step.tsx", ".config.ts"],
            Language::JavaScript => &[".step.js", ".step.jsx"],
            Language::Python => &["_step.py"],
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Runtime detection result
#[derive(Debug, Clone)]
pub struct RuntimeInfo {
    pub name: &'static str,
    pub version: Option<String>,
    pub path: Option<String>,
    pub available: bool,
}

/// Check if Node.js is available
pub fn check_node() -> RuntimeInfo {
    let output = Command::new("node").arg("--version").output();

    match output {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let path = Command::new("which")
                .arg("node")
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                    } else {
                        None
                    }
                });
            RuntimeInfo {
                name: "Node.js",
                version: Some(version),
                path,
                available: true,
            }
        }
        _ => RuntimeInfo {
            name: "Node.js",
            version: None,
            path: None,
            available: false,
        },
    }
}

/// Check if Bun is available
pub fn check_bun() -> RuntimeInfo {
    let output = Command::new("bun").arg("--version").output();

    match output {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let path = Command::new("which")
                .arg("bun")
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                    } else {
                        None
                    }
                });
            RuntimeInfo {
                name: "Bun",
                version: Some(version),
                path,
                available: true,
            }
        }
        _ => RuntimeInfo {
            name: "Bun",
            version: None,
            path: None,
            available: false,
        },
    }
}

/// Check if Python 3 is available
pub fn check_python() -> RuntimeInfo {
    let output = Command::new("python3").arg("--version").output();

    match output {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let path = Command::new("which")
                .arg("python3")
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                    } else {
                        None
                    }
                });
            RuntimeInfo {
                name: "Python 3",
                version: Some(version),
                path,
                available: true,
            }
        }
        _ => RuntimeInfo {
            name: "Python 3",
            version: None,
            path: None,
            available: false,
        },
    }
}

/// Check all required runtimes based on selected languages
pub fn check_runtimes(languages: &[Language]) -> Result<Vec<RuntimeInfo>> {
    let mut results = Vec::new();
    let mut missing = Vec::new();

    // TypeScript and JavaScript both need Node.js or Bun
    let needs_js_runtime = languages
        .iter()
        .any(|l| matches!(l, Language::TypeScript | Language::JavaScript));

    if needs_js_runtime {
        let bun = check_bun();
        let node = check_node();

        // Prefer Bun, fall back to Node.js
        if bun.available {
            results.push(bun);
        } else if node.available {
            results.push(node);
        } else {
            missing.push("Node.js or Bun (install from https://nodejs.org or https://bun.sh)");
        }
    }

    // Python needs python3
    if languages.contains(&Language::Python) {
        let python = check_python();
        if python.available {
            results.push(python);
        } else {
            missing.push("Python 3 (install from https://python.org)");
        }
    }

    if !missing.is_empty() {
        anyhow::bail!(
            "Missing required runtimes:\n{}",
            missing
                .iter()
                .map(|m| format!("  • {}", m))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    Ok(results)
}

/// Get installation instructions for missing runtimes
pub fn get_install_instructions(languages: &[Language]) -> Vec<(&'static str, &'static str)> {
    let mut instructions = Vec::new();

    let needs_js_runtime = languages
        .iter()
        .any(|l| matches!(l, Language::TypeScript | Language::JavaScript));

    if needs_js_runtime {
        let bun = check_bun();
        let node = check_node();

        if !bun.available && !node.available {
            instructions.push(("Bun", "curl -fsSL https://bun.sh/install | bash"));
            instructions.push(("Node.js", "https://nodejs.org/en/download/"));
        }
    }

    if languages.contains(&Language::Python) {
        let python = check_python();
        if !python.available {
            instructions.push(("Python 3", "https://python.org/downloads/"));
        }
    }

    instructions
}
