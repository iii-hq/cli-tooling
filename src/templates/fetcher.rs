//! Template fetching from remote (GitHub) or local directory
//!
//! Both remote and local templates use zip files for consistency:
//! - Remote: Fetches pre-built zips from URL
//! - Local: Automatically builds zips from template folders, then uses them
//!
//! This ensures identical behavior between development and production.

use super::manifest::{RootManifest, TemplateManifest};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use tokio::fs;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

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

/// Cached template data extracted from zip
#[derive(Debug, Clone)]
struct TemplateCache {
    manifest: TemplateManifest,
    files: HashMap<String, Vec<u8>>,
}

/// Template fetcher - handles retrieving templates from remote or local sources
pub struct TemplateFetcher {
    source: TemplateSource,
    client: reqwest::Client,
    /// Cache of downloaded/built and extracted templates
    template_cache: HashMap<String, TemplateCache>,
}

impl TemplateFetcher {
    pub fn new(source: TemplateSource) -> Self {
        Self {
            source,
            client: reqwest::Client::new(),
            template_cache: HashMap::new(),
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

    /// Build a zip file for a local template (reads files list from template.yaml)
    pub fn build_local_zip(template_dir: &PathBuf, template_name: &str) -> Result<Vec<u8>> {
        let template_path = template_dir.join(template_name);
        let manifest_path = template_path.join("template.yaml");

        // Read and parse the template manifest to get the files list
        let manifest_content = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("Failed to read {}", manifest_path.display()))?;
        let manifest: TemplateManifest = serde_yaml::from_str(&manifest_content)
            .with_context(|| format!("Failed to parse template '{}' manifest", template_name))?;

        // Create zip in memory
        let mut zip_buffer = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut zip_buffer));
            let options = SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            // Always include template.yaml first
            let template_yaml_path = format!("{}/template.yaml", template_name);
            zip.start_file(&template_yaml_path, options)?;
            zip.write_all(manifest_content.as_bytes())?;

            // Add each file from the manifest's files list
            for file_path in &manifest.files {
                let full_path = template_path.join(file_path);
                if full_path.exists() {
                    let content = std::fs::read(&full_path)
                        .with_context(|| format!("Failed to read {}", full_path.display()))?;
                    let zip_path = format!("{}/{}", template_name, file_path);
                    zip.start_file(&zip_path, options)?;
                    zip.write_all(&content)?;
                } else {
                    // Warn but don't fail - file might be optional
                    eprintln!(
                        "Warning: File '{}' not found (specified in {})",
                        full_path.display(),
                        manifest_path.display()
                    );
                }
            }

            zip.finish()?;
        }

        Ok(zip_buffer)
    }

    /// Extract a zip into the template cache
    fn extract_zip_to_cache(
        zip_bytes: &[u8],
        template_name: &str,
    ) -> Result<TemplateCache> {
        let cursor = Cursor::new(zip_bytes);
        let mut archive = ZipArchive::new(cursor)
            .with_context(|| format!("Failed to read zip archive for template '{}'", template_name))?;

        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        let mut manifest: Option<TemplateManifest> = None;

        // The zip contains files with paths like: {template_name}/file.txt
        // We need to strip the template_name prefix
        let prefix = format!("{}/", template_name);

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let full_path = file.name().to_string();

            // Skip directories
            if file.is_dir() {
                continue;
            }

            // Strip the template_name prefix from the path
            let relative_path = if full_path.starts_with(&prefix) {
                full_path[prefix.len()..].to_string()
            } else {
                full_path.clone()
            };

            // Read file contents
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)?;

            // Check if this is the manifest
            if relative_path == "template.yaml" {
                let content_str = String::from_utf8_lossy(&contents);
                manifest = Some(
                    serde_yaml::from_str(&content_str)
                        .with_context(|| format!("Failed to parse template '{}' manifest", template_name))?,
                );
            }

            files.insert(relative_path, contents);
        }

        let manifest = manifest.ok_or_else(|| {
            anyhow::anyhow!("Template '{}' zip missing template.yaml", template_name)
        })?;

        Ok(TemplateCache { manifest, files })
    }

    /// Fetch/build and cache a template's zip file
    async fn fetch_and_cache_template(&mut self, template_name: &str) -> Result<()> {
        if self.template_cache.contains_key(template_name) {
            return Ok(());
        }

        let zip_bytes = match &self.source {
            TemplateSource::Remote(base_url) => {
                // Fetch the zip file from remote
                let zip_url = format!("{}/{}.zip", base_url, template_name);
                let response = self
                    .client
                    .get(&zip_url)
                    .send()
                    .await
                    .with_context(|| format!("Failed to fetch template zip: {}", template_name))?;

                if !response.status().is_success() {
                    anyhow::bail!(
                        "Failed to fetch template '{}' zip: HTTP {}",
                        template_name,
                        response.status()
                    );
                }

                response.bytes().await?.to_vec()
            }
            TemplateSource::Local(path) => {
                // Build zip from local template folder
                Self::build_local_zip(path, template_name)?
            }
        };

        let cache = Self::extract_zip_to_cache(&zip_bytes, template_name)?;
        self.template_cache.insert(template_name.to_string(), cache);

        Ok(())
    }

    /// Fetch a specific template's manifest
    pub async fn fetch_template_manifest(&mut self, template_name: &str) -> Result<TemplateManifest> {
        self.fetch_and_cache_template(template_name).await?;
        let cache = self.template_cache.get(template_name).ok_or_else(|| {
            anyhow::anyhow!("Template '{}' not found in cache", template_name)
        })?;
        Ok(cache.manifest.clone())
    }

    /// Fetch a specific file from a template as string
    #[allow(dead_code)]
    pub async fn fetch_file(&mut self, template_name: &str, file_path: &str) -> Result<String> {
        let bytes = self.fetch_file_bytes(template_name, file_path).await?;
        String::from_utf8(bytes).context("File is not valid UTF-8")
    }

    /// Fetch a file as bytes (for binary files)
    pub async fn fetch_file_bytes(&mut self, template_name: &str, file_path: &str) -> Result<Vec<u8>> {
        self.fetch_and_cache_template(template_name).await?;
        let cache = self.template_cache.get(template_name).ok_or_else(|| {
            anyhow::anyhow!("Template '{}' not found in cache", template_name)
        })?;
        cache.files.get(file_path).cloned().ok_or_else(|| {
            anyhow::anyhow!("File '{}' not found in template '{}'", file_path, template_name)
        })
    }

    /// Get the template source
    #[allow(dead_code)]
    pub fn source(&self) -> &TemplateSource {
        &self.source
    }
}
