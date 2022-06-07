use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::scraping::scrape_links;
use crate::{CliMode, Config};

#[derive(Debug, Parser)]
#[clap(name = "autoeq2camilladsp")]
#[clap(author, version, about)]
struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Create Headphone List a JSON file
    Init,
}

pub async fn noninteractive_mode(client: &reqwest::Client, config: &Config) -> Result<()> {
    let cli = Cli::parse();
    if let Some(input) = cli.command {
        match input {
            Commands::Init => {
                let database_result_list = scrape_links(client, &config.repo_url()).await?;
                println!("{:?}", database_result_list);
            }
        }
    }
    Ok(())
}

pub fn cli_mode_check() -> CliMode {
    let cli = Cli::parse();
    match cli.command {
        Some(_input) => CliMode::NonInteractive,
        None => CliMode::Interactive,
    }
}
