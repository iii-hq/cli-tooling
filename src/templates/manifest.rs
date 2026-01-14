//! Template manifest types and parsing

use serde::{Deserialize, Serialize};

/// Root template manifest (templates/template.yaml)
/// Lists available template directories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootManifest {
    /// List of template directory names
    pub templates: Vec<String>,
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
