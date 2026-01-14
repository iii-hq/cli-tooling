//! Template manifest types and parsing

use serde::{Deserialize, Serialize};

/// File patterns associated with each language
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguageFiles {
    /// Files that require Python to be selected
    #[serde(default)]
    pub python: Vec<String>,

    /// Files that require TypeScript to be selected
    #[serde(default)]
    pub typescript: Vec<String>,

    /// Files that require JavaScript to be selected
    #[serde(default)]
    pub javascript: Vec<String>,

    /// Files that require either JavaScript or TypeScript
    #[serde(default)]
    pub node: Vec<String>,
}

impl LanguageFiles {
    /// Merge another LanguageFiles into this one (other takes precedence for additions)
    pub fn merge(&mut self, other: &LanguageFiles) {
        self.python.extend(other.python.iter().cloned());
        self.typescript.extend(other.typescript.iter().cloned());
        self.javascript.extend(other.javascript.iter().cloned());
        self.node.extend(other.node.iter().cloned());
    }

    /// Check if a filename matches any pattern in a list
    fn matches_any(filename: &str, patterns: &[String]) -> bool {
        patterns.iter().any(|pattern| {
            if pattern.starts_with('*') {
                // Suffix match: *.ts matches foo.ts
                filename.ends_with(&pattern[1..])
            } else if pattern.ends_with('*') {
                // Prefix match: requirements* matches requirements.txt
                filename.starts_with(&pattern[..pattern.len() - 1])
            } else {
                // Exact match
                filename == pattern
            }
        })
    }

    /// Determine which language(s) a file is associated with
    /// Returns None if the file should always be included
    pub fn get_language_for_file(&self, file_path: &str) -> Option<FileLanguage> {
        let filename = file_path.rsplit('/').next().unwrap_or(file_path);

        if Self::matches_any(filename, &self.python) {
            return Some(FileLanguage::Python);
        }
        if Self::matches_any(filename, &self.typescript) {
            return Some(FileLanguage::TypeScript);
        }
        if Self::matches_any(filename, &self.javascript) {
            return Some(FileLanguage::JavaScript);
        }
        if Self::matches_any(filename, &self.node) {
            return Some(FileLanguage::Node);
        }

        None // Not language-specific, always include
    }
}

/// Which language a file is associated with
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileLanguage {
    Python,
    TypeScript,
    JavaScript,
    Node, // Either JS or TS
}

/// Root template manifest (templates/template.yaml)
/// Lists available template directories and global language file associations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootManifest {
    /// List of template directory names
    pub templates: Vec<String>,

    /// Global language-specific file patterns (optional)
    #[serde(default)]
    pub language_files: LanguageFiles,
}

/// Per-template manifest (templates/<name>/template.yaml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateManifest {
    /// Display name of the template
    pub name: String,

    /// Description of what the template provides
    pub description: String,

    /// Semver version for CLI compatibility checking
    pub version: String,

    /// Languages that must be included (user cannot deselect)
    #[serde(default)]
    pub requires: Vec<String>,

    /// Languages that can optionally be included
    #[serde(default)]
    pub optional: Vec<String>,

    /// Explicit list of files to copy
    pub files: Vec<String>,

    /// Template-specific language file overrides (merged with root)
    #[serde(default)]
    pub language_files: LanguageFiles,
}

impl TemplateManifest {
    /// Check if a language is required by this template
    pub fn is_required(&self, language: &str) -> bool {
        self.requires
            .iter()
            .any(|r| r.eq_ignore_ascii_case(language))
    }

    /// Check if a language is optional for this template
    pub fn is_optional(&self, language: &str) -> bool {
        self.optional
            .iter()
            .any(|o| o.eq_ignore_ascii_case(language))
    }

    /// Get all available languages (required + optional)
    pub fn all_languages(&self) -> Vec<&str> {
        self.requires
            .iter()
            .chain(self.optional.iter())
            .map(|s| s.as_str())
            .collect()
    }
}
