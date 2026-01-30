//! Charm-style CLI prompts using cliclack

use crate::config::generator;
use crate::runtime::{check, iii};
use crate::templates::manifest::{LanguageFiles, TemplateManifest};
use crate::templates::{copier, fetcher::TemplateFetcher, fetcher::TemplateSource, version};
use crate::{CreateArgs, CLI_VERSION};
use anyhow::Result;
use std::path::PathBuf;

/// Run the CLI with interactive prompts
pub async fn run(args: CreateArgs) -> Result<()> {
    cliclack::intro("motia")?;

    // Step 1: Check iii installation (skip if --skip-iii)
    if !args.skip_iii {
        handle_iii_check(&args).await?;
    } else {
        cliclack::log::info("Skipping iii check")?;
    }

    // Step 2: Setup template fetcher
    let mut fetcher = setup_fetcher(&args.template_dir)?;

    // Step 3: Select template (also returns merged language_files)
    let (template_name, manifest, language_files) =
        select_template(&mut fetcher, args.template.as_deref()).await?;

    // Check version compatibility
    if let Some(warning) = version::check_compatibility(CLI_VERSION, &manifest.version) {
        cliclack::log::warning(format!(
            "Version warning: {}",
            warning.lines().next().unwrap_or(&warning)
        ))?;
    }

    // Step 4: Select directory
    let project_dir = select_directory(&args)?;

    // Step 5: Select languages
    let selected_languages = select_languages(&manifest, &args)?;

    // Step 6: Check runtimes
    check_runtimes(&selected_languages)?;

    // Step 7: Create project
    create_project(
        &mut fetcher,
        &template_name,
        &manifest,
        &project_dir,
        &selected_languages,
        &language_files,
    )
    .await?;

    // Step 8: Show next steps
    print_next_steps(&project_dir, &selected_languages)?;

    Ok(())
}

async fn handle_iii_check(args: &CreateArgs) -> Result<()> {
    let installed = iii::is_installed();

    if installed {
        let version = iii::get_version().unwrap_or_else(|| "unknown".to_string());
        cliclack::log::success(format!("iii installed ({})", version))?;
        return Ok(());
    }

    cliclack::log::warning("iii is not installed")?;

    // In non-interactive mode, just skip
    if args.yes {
        cliclack::log::info("Continuing without iii (--yes mode)")?;
        return Ok(());
    }

    let action: &str = cliclack::select("What would you like to do?")
        .item("install", "Install iii automatically", "")
        .item(
            "docs",
            format!("Open documentation ({})", iii::DOCS_URL),
            "",
        )
        .item("skip", "Skip and continue without iii", "")
        .interact()?;

    match action {
        "install" => {
            cliclack::log::info(format!("This will execute: {}", iii::install_command()))?;

            let confirm: bool = cliclack::confirm("Proceed with installation?")
                .initial_value(true)
                .interact()?;

            if confirm {
                match iii::install().await {
                    Ok(_) => {
                        cliclack::log::success("iii installed successfully")?;
                    }
                    Err(e) => {
                        cliclack::log::error(format!("{}", e))?;

                        let continue_anyway: bool = cliclack::confirm("Continue without iii?")
                            .initial_value(false)
                            .interact()?;

                        if !continue_anyway {
                            anyhow::bail!("Setup cancelled.");
                        }
                    }
                }
            } else {
                cliclack::log::info(format!(
                    "Continuing without iii. Refer to the docs for installation instructions: ({})",
                    iii::DOCS_URL
                ))?;
            }
        }
        "docs" => {
            iii::open_docs()?;
            cliclack::outro("After installing iii, run this command again.")?;
            std::process::exit(0);
        }
        "skip" => {
            cliclack::log::info(format!(
                "Continuing without iii. Refer to the docs for installation instructions: ({})",
                iii::DOCS_URL
            ))?;
        }
        _ => {}
    }

    Ok(())
}

fn setup_fetcher(template_dir: &Option<PathBuf>) -> Result<TemplateFetcher> {
    let source = match template_dir {
        Some(path) => {
            cliclack::log::info(format!("Using local templates from {}", path.display()))?;
            TemplateSource::local(path.clone())
        }
        None => {
            cliclack::log::info("Using remote templates")?;
            TemplateSource::default_remote()?
        }
    };

    Ok(TemplateFetcher::new(source))
}

async fn select_template(
    fetcher: &mut TemplateFetcher,
    specified_template: Option<&str>,
) -> Result<(String, TemplateManifest, LanguageFiles)> {
    let spinner = cliclack::spinner();
    spinner.start("Loading templates...");

    let root_manifest = fetcher.fetch_root_manifest().await?;

    // Helper to merge language files from root and template
    let merge_language_files = |manifest: &TemplateManifest| -> LanguageFiles {
        let mut merged = root_manifest.language_files.clone();
        merged.merge(&manifest.language_files);
        merged
    };

    // If a template was specified via --template flag, use it directly
    if let Some(template_name) = specified_template {
        if !root_manifest.templates.contains(&template_name.to_string()) {
            spinner.stop("Failed to load templates");
            let available = root_manifest.templates.join(", ");
            anyhow::bail!(
                "Template '{}' not found. Available templates: {}",
                template_name,
                available
            );
        }

        let manifest = fetcher.fetch_template_manifest(template_name).await?;
        let language_files = merge_language_files(&manifest);
        spinner.stop(format!(
            "Template: {} — {}",
            manifest.name, manifest.description
        ));
        return Ok((template_name.to_string(), manifest, language_files));
    }

    let mut templates: Vec<(String, TemplateManifest)> = Vec::new();
    for template_name in &root_manifest.templates {
        let manifest = fetcher.fetch_template_manifest(template_name).await?;
        templates.push((template_name.clone(), manifest));
    }

    spinner.stop("Templates loaded");

    if templates.is_empty() {
        anyhow::bail!("No templates found.");
    }

    // If only one template, use it automatically
    if templates.len() == 1 {
        let (name, manifest) = templates.into_iter().next().unwrap();
        let language_files = merge_language_files(&manifest);
        cliclack::log::info(format!(
            "Using template: {} — {}",
            manifest.name, manifest.description
        ))?;
        return Ok((name, manifest, language_files));
    }

    // Build select prompt - use indices to avoid borrow issues
    let mut select = cliclack::select("Select a template");
    for (idx, (_, manifest)) in templates.iter().enumerate() {
        select = select.item(idx, &manifest.name, &manifest.description);
    }

    let selected_idx: usize = select.interact()?;

    let (name, manifest) = templates.into_iter().nth(selected_idx).unwrap();

    let language_files = merge_language_files(&manifest);

    Ok((name, manifest, language_files))
}

fn select_directory(args: &CreateArgs) -> Result<PathBuf> {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Use --directory flag if provided
    let path = if let Some(dir) = &args.directory {
        let p = if dir.is_absolute() {
            dir.clone()
        } else {
            current_dir.join(dir)
        };
        cliclack::log::info(format!("Using directory: {}", p.display()))?;
        p
    } else {
        let input: String = cliclack::input("Project directory")
            .placeholder(".")
            .default_input(".")
            .interact()?;

        if input.is_empty() || input == "." {
            current_dir
        } else {
            let p = PathBuf::from(&input);
            if p.is_absolute() {
                p
            } else {
                current_dir.join(p)
            }
        }
    };

    // Validate parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.exists() && parent != std::path::Path::new("") {
            anyhow::bail!("Parent directory does not exist: {}", parent.display());
        }
    }

    // Warn if directory exists and has files
    if path.exists() && path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&path) {
            let count = entries.count();
            if count > 0 {
                cliclack::log::warning(format!("Directory has {} existing items", count))?;

                // Auto-confirm with --yes flag
                let confirm = if args.yes {
                    true
                } else {
                    cliclack::confirm("Continue anyway?")
                        .initial_value(true)
                        .interact()?
                };

                if !confirm {
                    anyhow::bail!("Setup cancelled.");
                }
            }
        }
    }

    Ok(path)
}

/// Parse language string to Language enum
fn parse_language(s: &str) -> Option<check::Language> {
    match s.to_lowercase().as_str() {
        "typescript" | "ts" => Some(check::Language::TypeScript),
        "javascript" | "js" => Some(check::Language::JavaScript),
        "python" | "py" => Some(check::Language::Python),
        _ => None,
    }
}

fn select_languages(
    manifest: &TemplateManifest,
    args: &CreateArgs,
) -> Result<Vec<check::Language>> {
    let mut required_languages: Vec<check::Language> = Vec::new();
    let mut optional_languages: Vec<check::Language> = Vec::new();

    // Categorize TypeScript
    if manifest.is_required("typescript") {
        required_languages.push(check::Language::TypeScript);
    } else if manifest.is_optional("typescript") {
        optional_languages.push(check::Language::TypeScript);
    }

    // Categorize JavaScript
    if manifest.is_required("javascript") {
        required_languages.push(check::Language::JavaScript);
    } else if manifest.is_optional("javascript") {
        optional_languages.push(check::Language::JavaScript);
    }

    // Categorize Python
    if manifest.is_required("python") {
        required_languages.push(check::Language::Python);
    } else if manifest.is_optional("python") {
        optional_languages.push(check::Language::Python);
    }

    // Show required languages
    if !required_languages.is_empty() {
        let required_names: Vec<&str> = required_languages
            .iter()
            .map(|l| l.display_name())
            .collect();
        cliclack::log::info(format!("Required: {}", required_names.join(", ")))?;
    }

    let mut selected_languages = required_languages.clone();

    // If --languages flag is provided, use those instead of prompting
    if let Some(lang_args) = &args.languages {
        for lang_str in lang_args {
            if let Some(lang) = parse_language(lang_str) {
                // Only add if it's optional and not already selected
                if optional_languages.contains(&lang) && !selected_languages.contains(&lang) {
                    selected_languages.push(lang);
                }
            } else {
                cliclack::log::warning(format!("Unknown language: {}", lang_str))?;
            }
        }
    } else if !optional_languages.is_empty() {
        // If there are optional languages and --yes flag, select all optional
        if args.yes {
            selected_languages.extend(optional_languages);
        } else {
            // Interactive selection
            let mut multi = cliclack::multiselect("Select additional languages (optional)");

            for lang in &optional_languages {
                multi = multi.item(lang.clone(), lang.display_name(), "");
            }

            let selected: Vec<check::Language> = multi.required(false).interact()?;
            selected_languages.extend(selected);
        }
    }

    if selected_languages.is_empty() {
        anyhow::bail!("No languages available for this template.");
    }

    let lang_names: Vec<&str> = selected_languages
        .iter()
        .map(|l| l.display_name())
        .collect();
    cliclack::log::success(format!("Languages: {}", lang_names.join(", ")))?;

    Ok(selected_languages)
}

fn check_runtimes(languages: &[check::Language]) -> Result<()> {
    let spinner = cliclack::spinner();
    spinner.start("Checking runtimes...");

    match check::check_runtimes(languages) {
        Ok(runtimes) => {
            let runtime_info: Vec<String> = runtimes
                .iter()
                .map(|r| format!("{} ({})", r.name, r.version.as_deref().unwrap_or("unknown")))
                .collect();
            spinner.stop(format!("Runtimes: {}", runtime_info.join(", ")));
            Ok(())
        }
        Err(e) => {
            spinner.stop("Missing runtimes");
            cliclack::log::error(format!("{}", e))?;
            anyhow::bail!("Please install the missing runtimes and try again.");
        }
    }
}

async fn create_project(
    fetcher: &mut TemplateFetcher,
    template_name: &str,
    manifest: &TemplateManifest,
    project_dir: &PathBuf,
    selected_languages: &[check::Language],
    language_files: &LanguageFiles,
) -> Result<()> {
    let spinner = cliclack::spinner();
    spinner.start("Creating project...");

    // Copy template files
    let copied_files = copier::copy_template(
        fetcher,
        template_name,
        manifest,
        project_dir,
        selected_languages,
        language_files,
    )
    .await?;

    spinner.stop(format!(
        "Created {} files in {}",
        copied_files.len() + 1,
        project_dir.display()
    ));

    Ok(())
}

fn print_next_steps(project_dir: &PathBuf, languages: &[check::Language]) -> Result<()> {
    let has_js_ts = languages
        .iter()
        .any(|l| matches!(l, check::Language::TypeScript | check::Language::JavaScript));
    let has_python = languages.contains(&check::Language::Python);

    println!();
    println!("  Next steps");
    println!();

    let mut step = 1;

    println!("  {}.  Start iii if it's not already running", step);
    step += 1;
    // cd to directory if not current
    let current = std::env::current_dir().ok();
    if current.as_ref() != Some(project_dir) {
        println!("  {}.  cd {}", step, project_dir.display());
        step += 1;
    }

    if has_js_ts {
        println!("  {}.  npm install @iii-dev/motia", step);
        step += 1;
    }

    if has_python {
        println!("  {}.  Set up Python environment:", step);
        println!("      uv venv && uv pip install -r requirements.txt");
        println!("      — or —");
        println!("      python3 -m venv .venv && .venv/bin/pip install -r requirements.txt");
        step += 1;
    }

    if has_js_ts {
        println!("  {}.  npm dev", step);
    }

    cliclack::outro("Happy coding!")?;

    Ok(())
}
