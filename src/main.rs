mod config;
mod runtime;
mod templates;
mod tui;

use anyhow::Result;
use clap::Parser;
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
    /// Local directory to use for templates instead of fetching from remote
    #[arg(long = "template-dir")]
    pub template_dir: Option<PathBuf>,

    /// Template name to use
    #[arg(short, long)]
    pub template: Option<String>,

    /// Build zip files for all templates in the template directory
    #[arg(long = "build-zips")]
    pub build_zips: bool,

    /// Project directory to create
    #[arg(short, long)]
    pub directory: Option<PathBuf>,

    /// Languages to include (comma-separated: typescript,javascript,python)
    #[arg(short, long, value_delimiter = ',')]
    pub languages: Option<Vec<String>>,

    /// Skip iii installation check
    #[arg(long = "skip-iii")]
    pub skip_iii: bool,

    /// Auto-confirm all prompts (non-interactive mode)
    #[arg(short, long)]
    pub yes: bool,
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

    // If building zips, do that and exit
    if args.build_zips {
        return templates::build_zips(&args.template_dir).await;
    }

    // Run the TUI application
    let result = tui::run(args).await;

    // Ensure cursor is visible on normal exit
    let _ = console::Term::stderr().show_cursor();

    result
}
