use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::Serialize;

use crate::configcreation::Crossfeed;
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

#[derive(Serialize)]
struct OutputJson {
    #[serde(rename(serialize = "autoeqList"))]
    autoeq_list: Vec<Headphone>,
    #[serde(rename(serialize = "crossfeedPresets"))]
    crossfeed_presets: Vec<String>,
}
impl OutputJson {
    fn new() -> OutputJson {
        OutputJson {
            autoeq_list: Vec::new(),
            crossfeed_presets: vec![
                Crossfeed::None.to_string(),
                Crossfeed::PowChuMoy.to_string(),
                Crossfeed::Mpm.to_string(),
                Crossfeed::Natural.to_string(),
            ],
        }
    }
}

#[derive(Serialize)]
struct Headphone {
    name: String,
    link: String,
}

pub fn cli_mode_check() -> CliMode {
    let cli = Cli::parse();
    match cli.command {
        Some(_input) => CliMode::NonInteractive,
        None => CliMode::Interactive,
    }
}

pub async fn noninteractive_mode(client: &reqwest::Client, config: &Config) -> Result<()> {
    let cli = Cli::parse();
    if let Some(input) = cli.command {
        match input {
            Commands::Init => {
                create_json(client, config).await?;
            }
        }
    }
    Ok(())
}

async fn create_json(client: &reqwest::Client, config: &Config) -> Result<()> {
    let database_result_list = scrape_links(client, &config.repo_url()).await?;

    let mut json = OutputJson::new();

    for (key, val) in database_result_list.iter() {
        json.autoeq_list.push(Headphone {
            name: key.to_owned(),
            link: val.to_owned(),
        });
    }

    let json_out = serde_json::to_string(&json)?;

    println!("{}", json_out);

    Ok(())
}
