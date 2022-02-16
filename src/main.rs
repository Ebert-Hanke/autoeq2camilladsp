mod configcreation;
mod scraping;
mod userinterface;

use anyhow::{Context, Result};
use console::style;
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

fn format_msg(msg: &str, cli: &Cli) -> String {
    let msg = msg.replace("{}", &cli.headphone);
    format!("\n{}", style(msg).magenta().bold())
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
    progress_bar.set_message(format_msg("Loading Database...", &cli));
    let database_result_list = scrape_links(&client, &config.repo_url()).await?;
    progress_bar.finish_with_message(format_msg("...Database loaded.", &cli));

    cli.select_headphone(&database_result_list)?;

    progress_bar.set_message(format_msg("Loading EQ settings for {}...", &cli));
    let headphone_query_link_list =
        scrape_links(&client, &config.headphone_url(&cli.headphone_url)).await?;
    progress_bar.finish_with_message(format_msg("...EQ settings for {} loaded.", &cli));

    cli.query_custom_devices()?;

    cli.query_crossfeed()?;

    progress_bar.set_message(format_msg(
        "Parsing AutoEq settings for {} to CamillaDSP...",
        &cli,
    ));

    let filterset = scrape_eq_settings(headphone_query_link_list, &client, &config).await;

    match filterset {
        Ok(filterset) => {
            let configuration = build_configuration(filterset, &cli.crossfeed)?;
            write_yml_file(configuration, &cli.headphone, &cli.devices)?;

            progress_bar.finish_with_message(format_msg(
                "...Your config for CamillaDSP was created successfully. Happy listening! :)",
                &cli,
            ));
        }
        Err(error) => {
            progress_bar.finish_with_message(format!(
                "...Something went wrong unfortunately :(\n{}",
                error
            ));
        }
    }
    Ok(())
}
