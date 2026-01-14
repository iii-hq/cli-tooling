//! Template file copying with language filtering

use crate::runtime::check::Language;
use crate::templates::fetcher::TemplateFetcher;
use crate::templates::manifest::{FileLanguage, LanguageFiles, TemplateManifest};
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
    language_files: &LanguageFiles,
) -> Result<Vec<String>> {
    // Ensure target directory exists
    fs::create_dir_all(target_dir)
        .await
        .context("Failed to create target directory")?;

    let mut copied_files = Vec::new();

    for file_path in &manifest.files {
        // Check if this file should be included based on language selection
        if should_include_file(file_path, selected_languages, language_files) {
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

/// Determine if a file should be included based on selected languages and language_files config
fn should_include_file(
    file_path: &str,
    selected_languages: &[Language],
    language_files: &LanguageFiles,
) -> bool {
    let has_typescript = selected_languages.contains(&Language::TypeScript);
    let has_javascript = selected_languages.contains(&Language::JavaScript);
    let has_python = selected_languages.contains(&Language::Python);
    let has_js_or_ts = has_typescript || has_javascript;

    // Check if file matches any language-specific pattern from config
    if let Some(file_lang) = language_files.get_language_for_file(file_path) {
        return match file_lang {
            FileLanguage::Python => has_python,
            FileLanguage::TypeScript => has_typescript,
            FileLanguage::JavaScript => has_javascript,
            FileLanguage::Node => has_js_or_ts,
        };
    }

    // File not in any language-specific list, always include
    true
}

/// Get the list of files that would be copied for given language selection
pub fn preview_files<'a>(
    manifest: &'a TemplateManifest,
    selected_languages: &[Language],
    language_files: &LanguageFiles,
) -> Vec<&'a str> {
    manifest
        .files
        .iter()
        .filter(|f| should_include_file(f, selected_languages, language_files))
        .map(|s| s.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_language_files() -> LanguageFiles {
        LanguageFiles {
            python: vec![
                "*_step.py".to_string(),
                "requirements.txt".to_string(),
                "pyproject.toml".to_string(),
            ],
            typescript: vec![
                "*.step.ts".to_string(),
                "*.step.tsx".to_string(),
                "*.config.ts".to_string(),
                "tsconfig.json".to_string(),
            ],
            javascript: vec![
                "*.step.js".to_string(),
                "*.step.jsx".to_string(),
            ],
            node: vec![
                "package.json".to_string(),
            ],
        }
    }

    #[test]
    fn test_should_include_typescript_files() {
        let languages = vec![Language::TypeScript];
        let lf = test_language_files();

        assert!(should_include_file("src/start.step.ts", &languages, &lf));
        assert!(should_include_file("src/start.step.tsx", &languages, &lf));
        assert!(should_include_file("src/tutorial.config.ts", &languages, &lf));
        assert!(!should_include_file("src/javascript.step.js", &languages, &lf));
        assert!(!should_include_file("src/python_step.py", &languages, &lf));
    }

    #[test]
    fn test_should_include_config_files() {
        let languages = vec![Language::TypeScript];
        let lf = test_language_files();

        // package.json requires node (JS or TS)
        assert!(should_include_file("package.json", &languages, &lf));
        // .env is not in any list, always included
        assert!(should_include_file(".env", &languages, &lf));
        // motia.config.ts is TypeScript-filtered
        assert!(should_include_file("motia.config.ts", &languages, &lf));
    }

    #[test]
    fn test_should_include_multiple_languages() {
        let languages = vec![Language::TypeScript, Language::Python];
        let lf = test_language_files();

        assert!(should_include_file("src/start.step.ts", &languages, &lf));
        assert!(should_include_file("src/python_step.py", &languages, &lf));
        assert!(!should_include_file("src/javascript.step.js", &languages, &lf));
    }

    #[test]
    fn test_python_only_files() {
        let ts_only = vec![Language::TypeScript];
        let py_only = vec![Language::Python];
        let lf = test_language_files();

        // requirements.txt should only be included with Python
        assert!(!should_include_file("requirements.txt", &ts_only, &lf));
        assert!(should_include_file("requirements.txt", &py_only, &lf));
    }
}
