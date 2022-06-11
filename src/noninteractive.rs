use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::configcreation::{build_configuration, write_yml_file, Crossfeed, DevicesFile};
use crate::scraping::{parse_filters, parse_preamp_gain, scrape_links, CorrectionFilterSet};
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
    /// output List of AutoEq entries and Crossfeed preset options as JSON
    Init,
    /// create a config file based on the provided selection
    Create { input_json: String },
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

#[derive(Debug, Deserialize)]
struct InputJson {
    headphone: Headphone,
    crossfeed: Crossfeed,
}

#[derive(Debug, Serialize, Deserialize)]
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
                create_json_output(client, config).await?;
            }
            Commands::Create { input_json } => {
                let input: InputJson = serde_json::from_str(&input_json)?;
                create_config(client, config, input).await?;
            }
        }
    }
    Ok(())
}

async fn create_json_output(client: &reqwest::Client, config: &Config) -> Result<()> {
    let database_result_list = scrape_links(client, &config.repo_url()).await?;

    let mut json = OutputJson::new();
    for (key, val) in database_result_list.iter() {
        json.autoeq_list.push(Headphone {
            name: key.to_owned(),
            link: format!("{}.txt", val.to_owned()),
        });
    }

    let json_out = serde_json::to_string(&json)?;

    println!("{}", json_out);

    Ok(())
}

async fn create_config(client: &reqwest::Client, config: &Config, input: InputJson) -> Result<()> {
    let filterset = create_filterset(client, config, &input.headphone.link).await;

    match filterset {
        Ok(filterset) => {
            let configuration = build_configuration(filterset, &input.crossfeed)?;
            write_yml_file(
                configuration,
                &input.headphone.name,
                &DevicesFile::Default,
                &input.crossfeed,
            )?;
        }
        Err(error) => {
            println!("...Something went wrong unfortunately :(\n{}", error);
        }
    }

    Ok(())
}

async fn create_filterset(
    client: &reqwest::Client,
    config: &Config,
    link: &str,
) -> Result<CorrectionFilterSet> {
    let eq_file = client
        .get(config.raw_eq_url(link))
        .send()
        .await?
        .text()
        .await?;
    let mut data = eq_file.lines();
    let preamp_gain = parse_preamp_gain(&mut data)?;
    let mut filterset = CorrectionFilterSet::new(preamp_gain);
    parse_filters(&mut data, &mut filterset)?;
    Ok(filterset)
}
