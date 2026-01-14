//! Node-style CLI prompts using inquire

use crate::config::generator;
use crate::runtime::{check, iii};
use crate::templates::{copier, fetcher::TemplateFetcher, fetcher::TemplateSource, version};
use crate::templates::manifest::TemplateManifest;
use crate::{Args, CLI_VERSION};
use anyhow::Result;
use colored::Colorize;
use inquire::{Confirm, MultiSelect, Select, Text};
use std::fmt;
use std::path::PathBuf;

/// Run the CLI with interactive prompts
pub async fn run(args: Args) -> Result<()> {
    print_header();

    // Step 1: Check iii installation
    handle_iii_check().await?;

    // Step 2: Setup template fetcher
    let fetcher = setup_fetcher(&args);
    
    // Step 3: Select template
    let (template_name, manifest) = select_template(&fetcher).await?;

    // Check version compatibility
    if let Some(warning) = version::check_compatibility(CLI_VERSION, &manifest.version) {
        println!();
        println!("{}", "⚠ Warning".yellow().bold());
        for line in warning.lines() {
            println!("  {}", line.yellow());
        }
        println!();
    }

    // Step 4: Select directory
    let project_dir = select_directory()?;

    // Step 5: Select languages
    let selected_languages = select_languages(&manifest)?;

    // Step 6: Check runtimes
    check_runtimes(&selected_languages)?;

    // Step 7: Create project
    create_project(&fetcher, &template_name, &manifest, &project_dir, &selected_languages).await?;

    // Step 8: Show next steps
    print_next_steps(&project_dir, &selected_languages);

    Ok(())
}

fn print_header() {
    println!();
    println!("{}", "╭─────────────────────────────────────╮".cyan());
    println!("{}", "│        Motia CLI Setup              │".cyan());
    println!("{}", "╰─────────────────────────────────────╯".cyan());
    println!();
}

async fn handle_iii_check() -> Result<()> {
    let installed = iii::is_installed();

    if installed {
        let version = iii::get_version().unwrap_or_else(|| "unknown".to_string());
        println!("{} iii is installed ({})", "✓".green(), version.dimmed());
        return Ok(());
    }

    println!("{} iii is not installed", "!".yellow());
    println!("  {}", "iii is required to run Motia applications.".dimmed());
    println!();

    #[derive(Clone)]
    enum IiiAction {
        Install,
        OpenDocs,
        Skip,
    }

    impl fmt::Display for IiiAction {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                IiiAction::Install => write!(f, "Install iii automatically"),
                IiiAction::OpenDocs => write!(f, "Open documentation (https://iii.sh)"),
                IiiAction::Skip => write!(f, "Skip and continue without iii"),
            }
        }
    }

    let options = vec![IiiAction::Install, IiiAction::OpenDocs, IiiAction::Skip];

    let action = Select::new("What would you like to do?", options)
        .with_help_message("↑↓ to move, enter to select")
        .prompt()?;

    match action {
        IiiAction::Install => {
            // Show the command that will be executed
            println!();
            println!("{}", "This will execute:".dimmed());
            println!("  {}", iii::INSTALL_COMMAND.yellow());
            println!();

            let confirm = Confirm::new("Proceed with installation?")
                .with_default(true)
                .prompt()?;

            if confirm {
                match iii::install().await {
                    Ok(_) => {
                        println!();
                    }
                    Err(e) => {
                        println!();
                        println!("{} {}", "Installation failed:".red(), e);
                        println!();
                        
                        let continue_anyway = Confirm::new("Continue without iii?")
                            .with_default(false)
                            .prompt()?;
                        
                        if !continue_anyway {
                            anyhow::bail!("Setup cancelled.");
                        }
                    }
                }
            }
        }
        IiiAction::OpenDocs => {
            iii::open_docs()?;
            println!();
            println!("{}", "After installing iii, run this command again.".dimmed());
            std::process::exit(0);
        }
        IiiAction::Skip => {
            println!("{}", "Continuing without iii...".dimmed());
        }
    }

    Ok(())
}

fn setup_fetcher(args: &Args) -> TemplateFetcher {
    let source = match &args.template_dir {
        Some(path) => {
            println!("{} Using local templates from {}", "→".blue(), path.display());
            TemplateSource::local(path.clone())
        }
        None => {
            println!("{} Using remote templates", "→".blue());
            TemplateSource::default_remote()
        }
    };
    println!();

    TemplateFetcher::new(source)
}

async fn select_template(fetcher: &TemplateFetcher) -> Result<(String, TemplateManifest)> {
    println!("{}", "Loading templates...".dimmed());

    let root_manifest = fetcher.fetch_root_manifest().await?;

    let mut templates: Vec<(String, TemplateManifest)> = Vec::new();
    for template_name in &root_manifest.templates {
        let manifest = fetcher.fetch_template_manifest(template_name).await?;
        templates.push((template_name.clone(), manifest));
    }

    if templates.is_empty() {
        anyhow::bail!("No templates found.");
    }

    // If only one template, use it automatically
    if templates.len() == 1 {
        let (name, manifest) = templates.into_iter().next().unwrap();
        println!("{} Using template: {} ({})", "✓".green(), manifest.name.bold(), manifest.description.dimmed());
        return Ok((name, manifest));
    }

    // Create display options
    struct TemplateOption {
        name: String,
        manifest: TemplateManifest,
    }

    impl fmt::Display for TemplateOption {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{} - {}", self.manifest.name, self.manifest.description)
        }
    }

    let options: Vec<TemplateOption> = templates
        .into_iter()
        .map(|(name, manifest)| TemplateOption { name, manifest })
        .collect();

    let selected = Select::new("Select a template:", options)
        .with_help_message("↑↓ to move, enter to select")
        .prompt()?;

    println!("{} Template: {}", "✓".green(), selected.manifest.name.bold());

    Ok((selected.name, selected.manifest))
}

fn select_directory() -> Result<PathBuf> {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let input = Text::new("Project directory:")
        .with_default(".")
        .with_help_message(&format!("Press enter for current directory ({})", current_dir.display()))
        .prompt()?;

    let path = if input.is_empty() || input == "." {
        current_dir
    } else {
        let p = PathBuf::from(&input);
        if p.is_absolute() {
            p
        } else {
            current_dir.join(p)
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
                println!("{} Directory exists with {} items (files may be overwritten)", "!".yellow(), count);
                let confirm = Confirm::new("Continue anyway?")
                    .with_default(true)
                    .prompt()?;
                
                if !confirm {
                    anyhow::bail!("Setup cancelled.");
                }
            }
        }
    }

    println!("{} Directory: {}", "✓".green(), path.display());

    Ok(path)
}

fn select_languages(manifest: &TemplateManifest) -> Result<Vec<check::Language>> {
    // Build options with required languages pre-selected and disabled
    struct LangOption {
        language: check::Language,
        required: bool,
    }

    impl fmt::Display for LangOption {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let suffix = if self.required { " (required)" } else { "" };
            write!(f, "{}{}", self.language.display_name(), suffix)
        }
    }

    let mut options: Vec<LangOption> = Vec::new();
    let mut defaults: Vec<usize> = Vec::new();

    // Add TypeScript
    if manifest.is_required("typescript") || manifest.is_optional("typescript") {
        let required = manifest.is_required("typescript");
        if required {
            defaults.push(options.len());
        }
        options.push(LangOption {
            language: check::Language::TypeScript,
            required,
        });
    }

    // Add JavaScript
    if manifest.is_required("javascript") || manifest.is_optional("javascript") {
        let required = manifest.is_required("javascript");
        if required {
            defaults.push(options.len());
        }
        options.push(LangOption {
            language: check::Language::JavaScript,
            required,
        });
    }

    // Add Python
    if manifest.is_required("python") || manifest.is_optional("python") {
        let required = manifest.is_required("python");
        if required {
            defaults.push(options.len());
        }
        options.push(LangOption {
            language: check::Language::Python,
            required,
        });
    }

    if options.is_empty() {
        anyhow::bail!("Template has no language options defined.");
    }

    // Use MultiSelect for language selection
    let selected = MultiSelect::new("Select languages to include:", options)
        .with_default(&defaults)
        .with_help_message("↑↓ to move, space to toggle, enter to confirm")
        .prompt()?;

    // Extract languages, ensuring required ones are included
    let mut languages: Vec<check::Language> = selected.iter().map(|o| o.language).collect();

    // Add back any required languages that might have been deselected
    if manifest.is_required("typescript") && !languages.contains(&check::Language::TypeScript) {
        languages.push(check::Language::TypeScript);
    }
    if manifest.is_required("javascript") && !languages.contains(&check::Language::JavaScript) {
        languages.push(check::Language::JavaScript);
    }
    if manifest.is_required("python") && !languages.contains(&check::Language::Python) {
        languages.push(check::Language::Python);
    }

    let lang_names: Vec<&str> = languages.iter().map(|l| l.display_name()).collect();
    println!("{} Languages: {}", "✓".green(), lang_names.join(", "));

    Ok(languages)
}

fn check_runtimes(languages: &[check::Language]) -> Result<()> {
    println!();
    println!("{}", "Checking runtimes...".dimmed());

    match check::check_runtimes(languages) {
        Ok(runtimes) => {
            for runtime in runtimes {
                let version = runtime.version.as_deref().unwrap_or("unknown");
                println!("{} {} ({})", "✓".green(), runtime.name, version.dimmed());
            }
            Ok(())
        }
        Err(e) => {
            println!();
            println!("{}", "Missing required runtimes:".red().bold());
            println!("{}", e);
            println!();
            anyhow::bail!("Please install the missing runtimes and try again.");
        }
    }
}

async fn create_project(
    fetcher: &TemplateFetcher,
    template_name: &str,
    manifest: &TemplateManifest,
    project_dir: &PathBuf,
    selected_languages: &[check::Language],
) -> Result<()> {
    println!();
    println!("{}", "Creating project...".cyan().bold());

    // Copy template files
    let copied_files = copier::copy_template(
        fetcher,
        template_name,
        manifest,
        project_dir,
        selected_languages,
    )
    .await?;

    println!("  {} {} files copied", "→".blue(), copied_files.len());

    // Generate iii config
    generator::write_config(project_dir, selected_languages).await?;
    println!("  {} iii.yaml generated", "→".blue());

    println!();
    println!("{}", "✓ Project created successfully!".green().bold());

    Ok(())
}

fn print_next_steps(project_dir: &PathBuf, languages: &[check::Language]) {
    let has_js_ts = languages
        .iter()
        .any(|l| matches!(l, check::Language::TypeScript | check::Language::JavaScript));
    let has_python = languages.contains(&check::Language::Python);

    println!();
    println!("{}", "Next steps:".cyan().bold());
    println!();

    let mut step = 1;

    // cd to directory if not current
    let current = std::env::current_dir().ok();
    if current.as_ref() != Some(project_dir) {
        println!("  {}. {}", step, format!("cd {}", project_dir.display()).yellow());
        step += 1;
    }

    if has_js_ts {
        println!("  {}. {}", step, "npm install".yellow());
        step += 1;
    }

    if has_python {
        println!("  {}. {}", step, "python3 -m venv .venv".yellow());
        step += 1;
        println!("  {}. {}", step, "source .venv/bin/activate".yellow());
        step += 1;
        println!("  {}. {}", step, "pip install -r requirements".yellow());
        step += 1;
    }

    if has_js_ts {
        println!("  {}. {}", step, "npm run build".yellow());
        step += 1;
    }

    println!("  {}. {}", step, "iii start".yellow());

    println!();
}
