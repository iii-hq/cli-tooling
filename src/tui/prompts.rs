//! Node-style CLI prompts using inquire

use crate::config::generator;
use crate::runtime::{check, iii};
use crate::templates::{copier, fetcher::TemplateFetcher, fetcher::TemplateSource, version};
use crate::templates::manifest::{LanguageFiles, TemplateManifest};
use crate::{Args, CLI_VERSION};
use anyhow::Result;
use colored::Colorize;
use inquire::ui::{Color, RenderConfig, StyleSheet, Styled};
use inquire::{Confirm, MultiSelect, Select, Text};
use std::fmt;
use std::path::PathBuf;

/// Create a custom render config for Motia's style
fn motia_render_config<'a>() -> RenderConfig<'a> {
    RenderConfig::empty()
        // Prompt prefix: blue diamond
        .with_prompt_prefix(Styled::new("◆").with_fg(Color::LightBlue))
        // Answered prompt prefix: blue filled circle
        .with_answered_prompt_prefix(Styled::new("●").with_fg(Color::LightBlue))
        // Highlighted option: arrow indicator, no color on text
        .with_highlighted_option_prefix(Styled::new("›").with_fg(Color::LightBlue))
        // Selected option style: none (no highlighting)
        .with_selected_option(Some(StyleSheet::empty()))
        // Checkboxes for multi-select
        .with_selected_checkbox(Styled::new("[●]").with_fg(Color::LightBlue))
        .with_unselected_checkbox(Styled::new("[ ]"))
        // Scroll indicators
        .with_scroll_up_prefix(Styled::new("↑").with_fg(Color::DarkGrey))
        .with_scroll_down_prefix(Styled::new("↓").with_fg(Color::DarkGrey))
        // Help message style
        .with_help_message(StyleSheet::empty().with_fg(Color::DarkGrey))
        // Answer display
        .with_answer(StyleSheet::empty().with_fg(Color::LightBlue))
        // Canceled indicator
        .with_canceled_prompt_indicator(Styled::new("canceled").with_fg(Color::DarkGrey))
}

/// Run the CLI with interactive prompts
pub async fn run(args: Args) -> Result<()> {
    print_header();

    // Step 1: Check iii installation
    handle_iii_check().await?;

    // Step 2: Setup template fetcher
    let mut fetcher = setup_fetcher(&args);
    
    // Step 3: Select template (also returns merged language_files)
    let (template_name, manifest, language_files) = select_template(&mut fetcher, args.template.as_deref()).await?;

    // Check version compatibility
    if let Some(warning) = version::check_compatibility(CLI_VERSION, &manifest.version) {
        println!();
        println!("  {} {}", "△".yellow(), "Version warning".yellow());
        for line in warning.lines() {
            println!("    {}", line.dimmed());
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
    create_project(&mut fetcher, &template_name, &manifest, &project_dir, &selected_languages, &language_files).await?;

    // Step 8: Show next steps
    print_next_steps(&project_dir, &selected_languages);

    Ok(())
}

fn print_header() {
    println!();
    println!("  {}  {}", "◆".blue().bold(), "Motia".bold());
    println!();
}

async fn handle_iii_check() -> Result<()> {
    let installed = iii::is_installed();

    if installed {
        let version = iii::get_version().unwrap_or_else(|| "unknown".to_string());
        println!("  {} iii installed {}", "●".blue(), format!("({})", version).dimmed());
        return Ok(());
    }

    println!("  {} iii not installed", "○".yellow());
    println!("    {}", "iii is required to run Motia applications.".dimmed());
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
        .with_render_config(motia_render_config())
        .with_help_message("↑↓ navigate · enter select")
        .prompt()?;;

    match action {
        IiiAction::Install => {
            // Show the command that will be executed
            println!();
            println!("    {}", "This will execute:".dimmed());
            println!("    {}", iii::INSTALL_COMMAND.dimmed());
            println!();

            let confirm = Confirm::new("Proceed with installation?")
                .with_render_config(motia_render_config())
                .with_default(true)
                .prompt()?;;

            if confirm {
                match iii::install().await {
                    Ok(_) => {
                        println!();
                    }
                    Err(e) => {
                        println!();
                        println!("  {} {}", "✗".red(), format!("Installation failed: {}", e));
                        println!();
                        
                        let continue_anyway = Confirm::new("Continue without iii?")
                            .with_render_config(motia_render_config())
                            .with_default(false)
                            .prompt()?;;
                        
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
            println!("  {}", "After installing iii, run this command again.".dimmed());
            std::process::exit(0);
        }
        IiiAction::Skip => {
            println!("  {}", "Continuing without iii...".dimmed());
        }
    }

    Ok(())
}

fn setup_fetcher(args: &Args) -> TemplateFetcher {
    let source = match &args.template_dir {
        Some(path) => {
            println!("  {} Local templates {}", "◇".blue(), format!("({})", path.display()).dimmed());
            TemplateSource::local(path.clone())
        }
        None => {
            println!("  {} Remote templates", "◇".blue());
            TemplateSource::default_remote()
        }
    };
    println!();

    TemplateFetcher::new(source)
}

async fn select_template(fetcher: &mut TemplateFetcher, specified_template: Option<&str>) -> Result<(String, TemplateManifest, LanguageFiles)> {
    println!("  {}", "Loading templates...".dimmed());

    let root_manifest = fetcher.fetch_root_manifest().await?;

    // Helper to merge language files from root and template
    let merge_language_files = |manifest: &TemplateManifest| -> LanguageFiles {
        let mut merged = root_manifest.language_files.clone();
        merged.merge(&manifest.language_files);
        merged
    };

    // If a template was specified via --template flag, use it directly
    if let Some(template_name) = specified_template {
        // Check if the specified template exists in the root manifest
        if !root_manifest.templates.contains(&template_name.to_string()) {
            let available = root_manifest.templates.join(", ");
            anyhow::bail!(
                "Template '{}' not found. Available templates: {}",
                template_name,
                available
            );
        }

        let manifest = fetcher.fetch_template_manifest(template_name).await?;
        let language_files = merge_language_files(&manifest);
        println!("  {} Template: {} {}", "●".blue(), manifest.name.bold(), format!("— {}", manifest.description).dimmed());
        return Ok((template_name.to_string(), manifest, language_files));
    }

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
        let language_files = merge_language_files(&manifest);
        println!("  {} Template: {} {}", "●".blue(), manifest.name.bold(), format!("— {}", manifest.description).dimmed());
        return Ok((name, manifest, language_files));
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
        .with_render_config(motia_render_config())
        .with_help_message("↑↓ navigate · enter select")
        .prompt()?;;

    let language_files = merge_language_files(&selected.manifest);
    println!("  {} Template: {}", "●".blue(), selected.manifest.name.bold());

    Ok((selected.name, selected.manifest, language_files))
}

fn select_directory() -> Result<PathBuf> {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let input = Text::new("Project directory:")
        .with_render_config(motia_render_config())
        .with_default(".")
        .with_help_message(&format!("Press enter for current directory ({})", current_dir.display()))
        .prompt()?;;

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
                println!("  {} Directory has {} existing items", "△".yellow(), count);
                let confirm = Confirm::new("Continue anyway?")
                    .with_render_config(motia_render_config())
                    .with_default(true)
                    .prompt()?;;
                
                if !confirm {
                    anyhow::bail!("Setup cancelled.");
                }
            }
        }
    }

    println!("  {} Directory: {}", "●".blue(), path.display());

    Ok(path)
}

fn select_languages(manifest: &TemplateManifest) -> Result<Vec<check::Language>> {
    // Separate required languages from optional ones
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

    // Show required languages as fixed (not selectable)
    if !required_languages.is_empty() {
        let required_names: Vec<&str> = required_languages.iter().map(|l| l.display_name()).collect();
        println!("  {} Required: {}", "◇".dimmed(), required_names.join(", "));
    }

    // Start with required languages
    let mut selected_languages = required_languages.clone();

    // If there are optional languages, let user select them
    if !optional_languages.is_empty() {
        let options: Vec<check::Language> = optional_languages;

        let selected = MultiSelect::new("Select additional languages (optional):", options)
            .with_render_config(motia_render_config())
            .with_help_message("↑↓ navigate · space toggle · enter confirm")
            .prompt()?;

        selected_languages.extend(selected);
    }

    if selected_languages.is_empty() {
        anyhow::bail!("No languages available for this template.");
    }

    let lang_names: Vec<&str> = selected_languages.iter().map(|l| l.display_name()).collect();
    println!("  {} Languages: {}", "●".blue(), lang_names.join(", "));

    Ok(selected_languages)
}

fn check_runtimes(languages: &[check::Language]) -> Result<()> {
    println!();
    println!("  {}", "Checking runtimes...".dimmed());

    match check::check_runtimes(languages) {
        Ok(runtimes) => {
            for runtime in runtimes {
                let version = runtime.version.as_deref().unwrap_or("unknown");
                println!("  {} {} {}", "●".blue(), runtime.name, format!("({})", version).dimmed());
            }
            Ok(())
        }
        Err(e) => {
            println!();
            println!("  {} {}", "✗".red(), "Missing required runtimes:".red());
            println!("    {}", e);
            println!();
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
    println!();
    println!("  {}", "Creating project...".dimmed());

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

    println!("    {} {} files", "└".dimmed(), copied_files.len());

    // Generate iii config
    generator::write_config(project_dir, selected_languages).await?;
    println!("    {} iii.yaml", "└".dimmed());

    println!();
    println!("  {} {}", "◆".blue().bold(), "Project created".bold());

    Ok(())
}

fn print_next_steps(project_dir: &PathBuf, languages: &[check::Language]) {
    let has_js_ts = languages
        .iter()
        .any(|l| matches!(l, check::Language::TypeScript | check::Language::JavaScript));
    let has_python = languages.contains(&check::Language::Python);

    println!();
    println!("  {}", "Next steps".bold());
    println!();

    let mut step = 1;

    // cd to directory if not current
    let current = std::env::current_dir().ok();
    if current.as_ref() != Some(project_dir) {
        println!("  {}  {}", format!("{}.", step).dimmed(), format!("cd {}", project_dir.display()));
        step += 1;
    }

    if has_js_ts {
        println!("  {}  {}", format!("{}.", step).dimmed(), "npm install");
        step += 1;
    }

    if has_python {
        println!("  {}  Set up Python environment:", format!("{}.", step).dimmed());
        println!("      {}", "uv venv && source .venv/bin/activate && uv pip install -r requirements".dimmed());
        println!("      {}", "— or —".dimmed());
        println!("      {}", "python3 -m venv .venv && source .venv/bin/activate && pip install -r requirements".dimmed());
        step += 1;
    }

    if has_js_ts {
        println!("  {}  {}", format!("{}.", step).dimmed(), "npm run build");
        step += 1;
    }

    println!("  {}  {}", format!("{}.", step).dimmed(), "iii start".blue());

    println!();
}
