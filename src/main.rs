mod config;
mod runtime;
mod templates;
mod tui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// CLI version - used for template compatibility checking
pub const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default remote template URL (GitHub API)
pub const DEFAULT_TEMPLATE_URL: &str =
    "https://api.github.com/repos/MotiaDev/motia-cli/contents/templates";

#[derive(Parser, Debug)]
#[command(name = "motia")]
#[command(about = "CLI for scaffolding Motia projects with iii integration")]
#[command(version)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Create a new Motia project
    Create(CreateArgs),
    /// Build zip files for all templates in the template directory (for development use)
    BuildZips(BuildZipsArgs),
}

#[derive(Parser, Debug)]
pub struct CreateArgs {
    /// Local directory to use for templates instead of fetching from remote (for development use)
    #[arg(long = "template-dir")]
    pub template_dir: Option<PathBuf>,

    /// Template name to use
    #[arg(short, long)]
    pub template: Option<String>,

    /// Project directory to create
    #[arg(short, long)]
    pub directory: Option<PathBuf>,

    /// Languages to include (comma-separated: ts,js,py or typescript,javascript,python)
    #[arg(short, long, value_delimiter = ',')]
    pub languages: Option<Vec<String>>,

    /// Skip iii installation check
    #[arg(long = "skip-iii")]
    pub skip_iii: bool,

    /// Auto-confirm all prompts (non-interactive mode)
    #[arg(short, long)]
    pub yes: bool,
}

#[derive(Parser, Debug)]
pub struct BuildZipsArgs {
    /// Local directory containing templates to build zips from (for development use)
    #[arg(long = "template-dir")]
    pub template_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Ensure terminal cursor is restored on panic
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = console::Term::stderr().show_cursor();
        default_panic(info);
    }));

    // Handle Ctrl+C gracefully
    ctrlc::set_handler(move || {
        let _ = console::Term::stderr().show_cursor();
        std::process::exit(130);
    })
    .ok();

    let args = Args::parse();

    // Handle subcommands
    match args.command {
        Some(Command::Create(create_args)) => {
            // Run the TUI application with the create args
            let result = tui::run(create_args).await;

            // Ensure cursor is visible on normal exit
            let _ = console::Term::stderr().show_cursor();

            result
        }
        Some(Command::BuildZips(build_args)) => {
            // Build zip files for templates
            templates::build_zips(&build_args.template_dir).await
        }
        None => {
            // No subcommand provided, default to create behavior (interactive mode)
            let create_args = CreateArgs {
                template_dir: None,
                template: None,
                directory: None,
                languages: None,
                skip_iii: false,
                yes: false,
            };
            let result = tui::run(create_args).await;

            // Ensure cursor is visible on normal exit
            let _ = console::Term::stderr().show_cursor();

            result
        }
    }
}
