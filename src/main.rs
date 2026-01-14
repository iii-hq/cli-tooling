mod config;
mod runtime;
mod templates;
mod tui;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

/// CLI version - used for template compatibility checking
pub const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default remote template URL (raw GitHub content)
pub const DEFAULT_TEMPLATE_URL: &str =
    "https://raw.githubusercontent.com/MotiaDev/motia/main/packages/snap/src/create/templates";

#[derive(Parser, Debug)]
#[command(name = "motia")]
#[command(about = "CLI for scaffolding Motia projects with iii integration")]
#[command(version)]
pub struct Args {
    /// Local directory to use for templates instead of fetching from remote
    #[arg(long = "template-dir")]
    pub template_dir: Option<PathBuf>,

    /// Template name to use (skips template selection prompt)
    #[arg(short, long)]
    pub template: Option<String>,

    /// Build zip files for all templates in the template directory
    #[arg(long = "build-zips")]
    pub build_zips: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // If building zips, do that and exit
    if args.build_zips {
        return templates::build_zips(&args.template_dir).await;
    }

    // Run the TUI application
    tui::run(args).await
}
