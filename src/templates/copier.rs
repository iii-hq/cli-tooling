//! Template file copying with language filtering

use crate::runtime::check::Language;
use crate::templates::fetcher::TemplateFetcher;
use crate::templates::manifest::TemplateManifest;
use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs;

/// Copy template files to the target directory, filtering by selected languages
pub async fn copy_template(
    fetcher: &TemplateFetcher,
    template_name: &str,
    manifest: &TemplateManifest,
    target_dir: &Path,
    selected_languages: &[Language],
) -> Result<Vec<String>> {
    // Ensure target directory exists
    fs::create_dir_all(target_dir)
        .await
        .context("Failed to create target directory")?;

    let mut copied_files = Vec::new();

    for file_path in &manifest.files {
        // Check if this file should be included based on language selection
        if should_include_file(file_path, selected_languages) {
            // Ensure parent directories exist
            let target_path = target_dir.join(file_path);
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)
                    .await
                    .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
            }

            // Fetch and write the file
            let content = fetcher.fetch_file_bytes(template_name, file_path).await?;
            fs::write(&target_path, &content)
                .await
                .with_context(|| format!("Failed to write file: {}", target_path.display()))?;

            copied_files.push(file_path.clone());
        }
    }

    Ok(copied_files)
}

/// Determine if a file should be included based on selected languages
fn should_include_file(file_path: &str, selected_languages: &[Language]) -> bool {
    // Check if this is a language-specific step file
    let is_typescript = file_path.ends_with(".step.ts")
        || file_path.ends_with(".step.tsx")
        || file_path.ends_with(".config.ts");
    let is_javascript = file_path.ends_with(".step.js") || file_path.ends_with(".step.jsx");
    let is_python = file_path.ends_with("_step.py");

    // If it's not a step file, always include it (config files, etc.)
    if !is_typescript && !is_javascript && !is_python {
        return true;
    }

    // Check if the file's language is in the selected languages
    if is_typescript && selected_languages.contains(&Language::TypeScript) {
        return true;
    }
    if is_javascript && selected_languages.contains(&Language::JavaScript) {
        return true;
    }
    if is_python && selected_languages.contains(&Language::Python) {
        return true;
    }

    false
}

/// Get the list of files that would be copied for given language selection
pub fn preview_files<'a>(manifest: &'a TemplateManifest, selected_languages: &[Language]) -> Vec<&'a str> {
    manifest
        .files
        .iter()
        .filter(|f| should_include_file(f, selected_languages))
        .map(|s| s.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_include_typescript_files() {
        let languages = vec![Language::TypeScript];

        assert!(should_include_file("src/start.step.ts", &languages));
        assert!(should_include_file("src/start.step.tsx", &languages));
        assert!(should_include_file("src/tutorial.config.ts", &languages));
        assert!(!should_include_file("src/javascript.step.js", &languages));
        assert!(!should_include_file("src/python_step.py", &languages));
    }

    #[test]
    fn test_should_include_config_files() {
        let languages = vec![Language::TypeScript];

        // Non-step files should always be included
        assert!(should_include_file("package.json", &languages));
        assert!(should_include_file(".env", &languages));
        assert!(should_include_file("motia.config.ts", &languages)); // This is a config.ts, so TypeScript-filtered
    }

    #[test]
    fn test_should_include_multiple_languages() {
        let languages = vec![Language::TypeScript, Language::Python];

        assert!(should_include_file("src/start.step.ts", &languages));
        assert!(should_include_file("src/python_step.py", &languages));
        assert!(!should_include_file("src/javascript.step.js", &languages));
    }
}
