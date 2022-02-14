mod configcreation;
mod scraping;
mod userinterface;

use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use indicatif::ProgressBar;
use serde::Deserialize;

use configcreation::{build_configuration, write_yml_file, Crossfeed, DevicesFile};
use scraping::{parse_filter_line, pick_url, scrape_links, CorrectionFilterSet};
use userinterface::{Cli, CliTheme};

#[derive(Debug, Deserialize)]
struct Config {
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
    println!();

    cli.query_headphone()?;
    cli.query_database(&database_result_list)?;

    progress_bar.set_message("Loading EQ settings...");
    let headphone_query_link_list =
        scrape_links(&client, &config.headphone_url(&cli.headphone_url)).await?;
    progress_bar.finish_with_message("...EQ settings loaded.");
    println!();

    cli.query_custom_devices()?;
    println!();

    let crossfeed_query: bool = Confirm::with_theme(&ColorfulTheme::clitheme())
        .with_prompt(
            "Would you like to include Crossfeed modeled after the analogue implementation by Pow Chu Moy?"
        )
        .interact()?;

    let crossfeed = match crossfeed_query {
        true => Crossfeed::PowChuMoy,
        false => Crossfeed::None,
    };

    println!();

    progress_bar.set_message("Parsing AutoEq settings to CamillaDSP...");
    match pick_url(headphone_query_link_list, &config.parametric_eq_query) {
        Some(url) => {
            let eq_url =
                "https://raw.githubusercontent.com".to_owned() + &url.1.replace("/blob", "");
            let eq_file = client.get(eq_url).send().await?.text().await?;

            let mut data = eq_file.lines();
            let preamp_gain = data
                .next()
                .unwrap()
                .split(' ')
                .nth(1)
                .unwrap()
                .parse::<f32>()
                .unwrap();
            let mut headphone_correction = CorrectionFilterSet::new(preamp_gain);
            data.into_iter().skip(0).for_each(|line| {
                let filter = parse_filter_line(line);
                match filter {
                    Ok(eq) => {
                        headphone_correction.eq_bands.push(eq);
                    }
                    Err(error) => {
                        panic!("{}", error);
                    }
                }
            });

            let configuration = build_configuration(headphone_correction, crossfeed)?;
            write_yml_file(configuration, cli.headphone, cli.devices)?;

            progress_bar.finish_with_message(
                "...Your config for CamillaDSP was created successfully. Happy listening! :)",
            );
        }
        None => {
            progress_bar.finish_with_message("...Something went wrong unfortunately :(");
        }
    }

    Ok(())
}
