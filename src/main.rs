mod configcreation;
mod scraping;

use configcreation::{format_eq_filters, write_yml_file};
use console::style;
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

pub enum DevicesFile {
    Default,
    Custom(String),
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let client = reqwest::Client::builder()
        .user_agent("AutoEq2CamillaDSP")
        .build()?;
    let progress_bar = ProgressBar::new_spinner();

    let welcome = "___Welcome to AutoEq2CamillaDSP___";
    println!("{}", style(welcome).color256(55).on_black().bold());

    progress_bar.set_message("Loading Database...");
    let url = GITHUB_URL.to_owned() + REPO_URL;
    let database_result_list = get_correction_result_list(&client, &url).await?;
    progress_bar.finish_with_message("...Database loaded.");

    let headphone_query: String = Input::with_theme(&ColorfulTheme::default())
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
        .interact_text()
        .unwrap();

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
            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Maybe one of these is the one you are looking for?")
                .default(0)
                .items(&suggestions[..])
                .interact()
                .unwrap();
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

    println!("You have the option to include a custom 'devices' section from a .yml file.\n
If you do not choose to do so, the configuration will be created with a default 'devices' section which you then can edit and use for future configurations.");

    let custom_devices_query: bool = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(
            "Would you like to include a custom 'devices' section for your CamillaDSP config file?",
        )
        .interact()
        .unwrap();

    let devices_file = match custom_devices_query {
        true => {
            let mut custom_device_path: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Please enter the relative path to your custom 'devices' file:")
                .interact_text()
                .unwrap();
            let mut valid = File::open(&custom_device_path);
            while valid.is_err() {
                custom_device_path = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt(
                        "Sorry this file does not seem to exist.\n
If you want to quit, please enter 'q'\n
Otherwise try again and enter the relative path to your custom 'devices' file:",
                    )
                    .interact_text()
                    .unwrap();
                if custom_device_path.to_lowercase().trim() == "q" {
                    std::process::exit(0);
                }
                valid = File::open(&custom_device_path);
            }
            DevicesFile::Custom(custom_device_path)
        }
        false => DevicesFile::Default,
    };

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

            let formatted = format_eq_filters(headphone_correction);

            write_yml_file(formatted, query_result.0, devices_file);
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
