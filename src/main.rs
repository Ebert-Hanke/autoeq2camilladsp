mod configcreation;
mod interactive;
mod noninteractive;
mod scraping;
mod userinterface;

use anyhow::{Context, Result};
use serde::Deserialize;

use interactive::interactive_mode;
use noninteractive::{cli_mode_check, noninteractive_mode};

// basic exit codes
const EXIT_ERROR: i32 = 1; // catchall for errors
const EXIT_OK: i32 = 0; // all fine

pub enum CliMode {
    Interactive,
    NonInteractive,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    github_url: String,
    github_raw: String,
    repo_url: String,
    parametric_eq: String,
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
        let headphone = headphone_result.split('/').last().unwrap();
        format!(
            "{}{}/{}%20{}",
            self.github_raw,
            headphone_result.replace("/blob", ""),
            headphone,
            self.parametric_eq,
        )
    }
    pub fn raw_eq_url(&self, eq_url: &str) -> String {
        format!("{}{}", self.github_raw, eq_url.replace("/blob", ""))
    }
}

#[tokio::main]
async fn run() -> Result<()> {
    // setup
    let client = reqwest::Client::builder()
        .user_agent("AutoEq2CamillaDSP")
        .build()?;
    let config = Config::load()?;

    // non-interactive mode if subcommand is provided
    let mode = cli_mode_check();
    match mode {
        CliMode::Interactive => interactive_mode(&client, &config).await?,
        CliMode::NonInteractive => noninteractive_mode(&client, &config).await?,
    }

    Ok(())
}

fn main() {
    let exitstatus = run();
    match exitstatus {
        Err(_) => {
            std::process::exit(EXIT_ERROR);
        }
        Ok(_) => {
            std::process::exit(EXIT_OK);
        }
    }
}
