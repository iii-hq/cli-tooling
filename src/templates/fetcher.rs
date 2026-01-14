//! Template fetching from remote (GitHub) or local directory

use super::manifest::{RootManifest, TemplateManifest};
use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::fs;

/// Template source - either remote URL or local directory
#[derive(Debug, Clone)]
pub enum TemplateSource {
    Remote(String),
    Local(PathBuf),
}

impl TemplateSource {
    /// Create a remote template source with the default URL
    pub fn default_remote() -> Self {
        Self::Remote(crate::DEFAULT_TEMPLATE_URL.to_string())
    }

    /// Create a local template source from a path
    pub fn local(path: PathBuf) -> Self {
        Self::Local(path)
    }
}

/// Template fetcher - handles retrieving templates from remote or local sources
pub struct TemplateFetcher {
    source: TemplateSource,
    client: reqwest::Client,
}

impl TemplateFetcher {
    pub fn new(source: TemplateSource) -> Self {
        Self {
            source,
            client: reqwest::Client::new(),
        }
    }

    /// Fetch the root manifest listing available templates
    pub async fn fetch_root_manifest(&self) -> Result<RootManifest> {
        match &self.source {
            TemplateSource::Remote(base_url) => {
                let url = format!("{}/template.yaml", base_url);
                let response = self
                    .client
                    .get(&url)
                    .send()
                    .await
                    .context("Failed to fetch root template manifest")?;

                if !response.status().is_success() {
                    anyhow::bail!(
                        "Failed to fetch root manifest: HTTP {}",
                        response.status()
                    );
                }

                let content = response.text().await?;
                serde_yaml::from_str(&content).context("Failed to parse root manifest")
            }
            TemplateSource::Local(path) => {
                let manifest_path = path.join("template.yaml");
                let content = fs::read_to_string(&manifest_path)
                    .await
                    .with_context(|| format!("Failed to read {}", manifest_path.display()))?;
                serde_yaml::from_str(&content).context("Failed to parse root manifest")
            }
        }
    }

    /// Fetch a specific template's manifest
    pub async fn fetch_template_manifest(&self, template_name: &str) -> Result<TemplateManifest> {
        match &self.source {
            TemplateSource::Remote(base_url) => {
                let url = format!("{}/{}/template.yaml", base_url, template_name);
                let response = self
                    .client
                    .get(&url)
                    .send()
                    .await
                    .with_context(|| format!("Failed to fetch template manifest for {}", template_name))?;

                if !response.status().is_success() {
                    anyhow::bail!(
                        "Failed to fetch template '{}' manifest: HTTP {}",
                        template_name,
                        response.status()
                    );
                }

                let content = response.text().await?;
                serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse template '{}' manifest", template_name))
            }
            TemplateSource::Local(path) => {
                let manifest_path = path.join(template_name).join("template.yaml");
                let content = fs::read_to_string(&manifest_path)
                    .await
                    .with_context(|| format!("Failed to read {}", manifest_path.display()))?;
                serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse template '{}' manifest", template_name))
            }
        }
    }

    /// Fetch a specific file from a template
    pub async fn fetch_file(&self, template_name: &str, file_path: &str) -> Result<String> {
        match &self.source {
            TemplateSource::Remote(base_url) => {
                let url = format!("{}/{}/{}", base_url, template_name, file_path);
                let response = self
                    .client
                    .get(&url)
                    .send()
                    .await
                    .with_context(|| format!("Failed to fetch file: {}", file_path))?;

                if !response.status().is_success() {
                    anyhow::bail!("Failed to fetch '{}': HTTP {}", file_path, response.status());
                }

                response.text().await.context("Failed to read file content")
            }
            TemplateSource::Local(path) => {
                let file_full_path = path.join(template_name).join(file_path);
                fs::read_to_string(&file_full_path)
                    .await
                    .with_context(|| format!("Failed to read {}", file_full_path.display()))
            }
        }
    }

    /// Fetch a file as bytes (for binary files)
    pub async fn fetch_file_bytes(&self, template_name: &str, file_path: &str) -> Result<Vec<u8>> {
        match &self.source {
            TemplateSource::Remote(base_url) => {
                let url = format!("{}/{}/{}", base_url, template_name, file_path);
                let response = self
                    .client
                    .get(&url)
                    .send()
                    .await
                    .with_context(|| format!("Failed to fetch file: {}", file_path))?;

                if !response.status().is_success() {
                    anyhow::bail!("Failed to fetch '{}': HTTP {}", file_path, response.status());
                }

                response
                    .bytes()
                    .await
                    .map(|b| b.to_vec())
                    .context("Failed to read file content")
            }
            TemplateSource::Local(path) => {
                let file_full_path = path.join(template_name).join(file_path);
                fs::read(&file_full_path)
                    .await
                    .with_context(|| format!("Failed to read {}", file_full_path.display()))
            }
        }
    }

    /// Get the template source
    pub fn source(&self) -> &TemplateSource {
        &self.source
    }
}
