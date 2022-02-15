mod configcreation;
mod scraping;
mod userinterface;

use anyhow::{Context, Result};
use indicatif::ProgressBar;
use serde::Deserialize;

use configcreation::{build_configuration, write_yml_file};
use scraping::{scrape_eq_settings, scrape_links};
use userinterface::Cli;

#[derive(Debug, Deserialize)]
pub struct Config {
    github_url: String,
    github_raw: String,
    repo_url: String,
    parametric_eq_query: String,
}
impl Config {
    fn load() -> Result<Self> {
        let config: Config = serde_yaml::from_slice(include_bytes!("data/config.yml"))
            .context("The configuration file could not be serialized")?;
        Ok(config)
    }
    fn repo_url(&self) -> String {
        format!("{}{}", self.github_url, self.repo_url)
    }
    fn headphone_url(&self, headphone_result: &str) -> String {
        format!("{}{}", self.github_url, headphone_result)
    }
    pub fn raw_eq_url(&self, eq_url: &str) -> String {
        format!("{}{}", self.github_raw, eq_url.replace("/blob", ""))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // setup
    let client = reqwest::Client::builder()
        .user_agent("AutoEq2CamillaDSP")
        .build()?;
    let progress_bar = ProgressBar::new_spinner();
    let config = Config::load()?;
    let mut cli = Cli::initialize();

    Cli::welcome();
    progress_bar.set_message("Loading Database...");
    let database_result_list = scrape_links(&client, &config.repo_url()).await?;
    progress_bar.finish_with_message("...Database loaded.");

    cli.query_headphone()?;
    cli.consult_database(&database_result_list)?;

    progress_bar.set_message("Loading EQ settings...");
    let headphone_query_link_list =
        scrape_links(&client, &config.headphone_url(&cli.headphone_url)).await?;
    progress_bar.finish_with_message("...EQ settings loaded.");

    cli.query_custom_devices()?;

    cli.query_crossfeed()?;

    progress_bar.set_message("Parsing AutoEq settings to CamillaDSP...");

    let filterset = scrape_eq_settings(headphone_query_link_list, &client, &config).await;

    match filterset {
        Ok(filterset) => {
            let configuration = build_configuration(filterset, cli.crossfeed)?;
            write_yml_file(configuration, cli.headphone, cli.devices)?;

            progress_bar.finish_with_message(
                "...Your config for CamillaDSP was created successfully. Happy listening! :)",
            );
        }
        Err(error) => {
            let msg = format!("...Something went wrong unfortunately :(\n{}", error);
            progress_bar.finish_with_message(msg);
        }
    }
    Ok(())
}
