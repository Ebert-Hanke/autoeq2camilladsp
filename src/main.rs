mod configcreation;
mod scraping;

use anyhow::Result;
use configcreation::{build_configuration, write_yml_file, Crossfeed};
use console::{style, Style};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use indicatif::ProgressBar;
use scraping::{
    collect_datafile_links, filter_link_list, get_correction_result_list, parse_filter_line,
    pick_url, CorrectionFilterSet, QueryResult,
};
use std::fs::File;

// url for Jaako Pasanen's AutoEq
const GITHUB_URL: &str = "https://github.com";
const REPO_URL: &str = "/jaakkopasanen/AutoEq/blob/master/results/";
// query for ParametricEQ raw file
const PARAM_EQ: &str = "ParametricEQ.txt";

const LOGO: &str = r"
            _                   _       
  __ _ _  _| |_ ___  ___ __ _  | |_ ___ 
 / _` | || |  _/ _ \/ -_) _` | |  _/ _ \
 \__,_|\_,_|\__\___/\___\__, |_ \__\___/
  __ __ _ _ __ (_) | |__ _ |_| |____ __ 
 / _/ _` | '  \| | | / _` / _` (_-< '_ \
 \__\__,_|_|_|_|_|_|_\__,_\__,_/__/ .__/
  v0.2.0                          |_|   

";

pub trait CliTheme {
    fn clitheme() -> Self;
}
impl CliTheme for ColorfulTheme {
    fn clitheme() -> ColorfulTheme {
        ColorfulTheme {
            defaults_style: Style::new().for_stderr().magenta(),
            prompt_style: Style::new().for_stderr().magenta().bold(),
            prompt_prefix: style("?".to_string()).for_stderr().yellow(),
            prompt_suffix: style("›".to_string()).for_stderr().black().bright(),
            success_prefix: style("✓".to_string()).for_stderr().green(),
            success_suffix: style("·".to_string()).for_stderr().black().bright(),
            error_prefix: style("✕".to_string()).for_stderr().red(),
            error_style: Style::new().for_stderr().red(),
            hint_style: Style::new().for_stderr().black().bright(),
            values_style: Style::new().for_stderr().green(),
            active_item_style: Style::new().for_stderr().magenta(),
            inactive_item_style: Style::new().for_stderr(),
            active_item_prefix: style("❯".to_string()).for_stderr().yellow(),
            inactive_item_prefix: style(" ".to_string()).for_stderr(),
            checked_item_prefix: style("✓".to_string()).for_stderr().green(),
            unchecked_item_prefix: style("✓".to_string()).for_stderr().black(),
            picked_item_prefix: style("❯".to_string()).for_stderr().yellow(),
            unpicked_item_prefix: style(" ".to_string()).for_stderr(),
            #[cfg(feature = "fuzzy-select")]
            fuzzy_cursor_style: Style::new().for_stderr().black().on_white(),
            inline_selections: true,
        }
    }
}

pub enum DevicesFile {
    Default,
    Custom(String),
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent("AutoEq2CamillaDSP")
        .build()?;
    let progress_bar = ProgressBar::new_spinner();

    print!("{}", style(LOGO).magenta().bold());
    println!();

    progress_bar.set_message("Loading Database...");

    let url = GITHUB_URL.to_owned() + REPO_URL;
    let database_result_list = get_correction_result_list(&client, &url).await?;

    progress_bar.finish_with_message("...Database loaded.");
    println!();

    let headphone_query: String = Input::with_theme(&ColorfulTheme::clitheme())
        .with_prompt("Which Headphones or IEMs do you want to EQ with AutoEq?")
        .validate_with({
            let mut force = None;
            move|input: &String|->Result<(),&str>{
                if input.len() > 1 || force.as_ref().map_or(false, |old|old==input){
                    Ok(())
                }else{
                    force = Some(input.clone());
                    Err("Please give me a bit more information, this is just one letter. Type the same value again to force use.")
                }
            }
        })
        .interact_text()?;

    let query_result = match filter_link_list(&database_result_list, &headphone_query) {
        QueryResult::Success(url) => {
            println!(
                "Great! The {} could be found in the AutoEq database.",
                url.0
            );
            url
        }
        QueryResult::Suggestions(mut suggestions) => {
            suggestions.push("Nope, nothing here for me ...".to_string());
            let selection = Select::with_theme(&ColorfulTheme::clitheme())
                .with_prompt("Maybe one of these is the one you are looking for?")
                .default(0)
                .items(&suggestions[..])
                .interact()?;
            match filter_link_list(&database_result_list, &suggestions[selection]) {
                QueryResult::Success(url) => url,
                _ => std::process::exit(0),
            }
        }
        QueryResult::NotFound => {
            println!(
                "Sorry the {} or something similar could not be found in the AutoEq database.",
                headphone_query
            );
            std::process::exit(0);
        }
    };

    progress_bar.set_message("Loading EQ settings...");
    let headphone_url = GITHUB_URL.to_owned() + &query_result.1;
    let headphone_query_link_list = collect_datafile_links(&client, &headphone_url).await?;
    progress_bar.finish_with_message("...EQ settings loaded.");
    println!();

    let custom_explainer: &str = r"
You have the option to include a custom 'devices' section from a .yml file.
If you do not choose to do so, the configuration will be created with a default 'devices' section.
You then can edit this and use for future configurations.
";

    print!("{}", style(custom_explainer).magenta());
    println!();

    let custom_devices_query: bool = Confirm::with_theme(&ColorfulTheme::clitheme())
        .with_prompt(
            "Would you like to include a custom 'devices' section for your CamillaDSP config file?",
        )
        .interact()?;

    let devices_file = match custom_devices_query {
        true => {
            let mut custom_device_path: String = Input::with_theme(&ColorfulTheme::clitheme())
                .with_prompt("Please enter the relative path to your custom 'devices' file:")
                .interact_text()?;
            let mut valid = File::open(&custom_device_path);
            while valid.is_err() {
                custom_device_path = Input::with_theme(&ColorfulTheme::clitheme())
                    .with_prompt(
                        "Sorry this file does not seem to exist.\n
If you want to quit, please enter 'q'\n
Otherwise try again and enter the relative path to your custom 'devices' file:",
                    )
                    .interact_text()?;
                if custom_device_path.to_lowercase().trim() == "q" {
                    std::process::exit(0);
                }
                valid = File::open(&custom_device_path);
            }
            DevicesFile::Custom(custom_device_path)
        }
        false => DevicesFile::Default,
    };

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
    match pick_url(headphone_query_link_list, PARAM_EQ) {
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
            write_yml_file(configuration, query_result.0, devices_file)?;

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
