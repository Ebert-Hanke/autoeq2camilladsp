use anyhow::Result;
use console::style;
use indicatif::ProgressBar;

use crate::configcreation::{build_configuration, write_yml_file};
use crate::scraping::{scrape_eq_settings, scrape_links};
use crate::userinterface::Cli;
use crate::Config;

pub async fn interactive_mode(client: &reqwest::Client, config: &Config) -> Result<()> {
    // setup for interactive mode
    let progress_bar = ProgressBar::new_spinner();
    let mut cli = Cli::initialize();

    Cli::welcome();
    progress_bar.set_message(format_msg("Loading Database...", &cli));
    let database_result_list = scrape_links(client, &config.repo_url()).await?;
    progress_bar.finish_with_message(format_msg("...Database loaded.", &cli));

    cli.select_headphone(&database_result_list)?;

    progress_bar.set_message(format_msg("Loading EQ settings for {}...", &cli));
    let headphone_query_link_list =
        scrape_links(client, &config.headphone_url(&cli.headphone_url)).await?;
    progress_bar.finish_with_message(format_msg("...EQ settings for {} loaded.", &cli));

    cli.query_custom_devices()?;

    cli.query_crossfeed()?;

    progress_bar.set_message(format_msg(
        "Parsing AutoEq settings for {} to CamillaDSP...",
        &cli,
    ));

    let filterset = scrape_eq_settings(headphone_query_link_list, client, config).await;

    match filterset {
        Ok(filterset) => {
            let configuration = build_configuration(filterset, &cli.crossfeed)?;
            write_yml_file(configuration, &cli.headphone, &cli.devices, &cli.crossfeed)?;

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

fn format_msg(msg: &str, cli: &Cli) -> String {
    let msg = msg.replace("{}", &cli.headphone);
    format!("\n{}", style(msg).magenta().bold())
}
