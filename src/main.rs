mod configcreation;
mod scraping;

use configcreation::{format_eq_filters, write_yml_file};
use console::style;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use indicatif::ProgressBar;
use scraping::{
    collect_links, filter_link_list, parse_filter_line, pick_url, CorrectionFilterSet, QueryResult,
};

// url for Jaako Pasanen's AutoEq
const GITHUB_URL: &str = "https://github.com";
const REPO_URL: &str = "/jaakkopasanen/AutoEq/blob/master/results/";
// query for ParametricEQ raw file
const PARAM_EQ: &str = "ParametricEQ.txt";

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
    let database_result_list = collect_links(&client, &url).await?;
    progress_bar.finish_with_message("...Database loaded.");

    let headphone_query: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Which Headphones or IEMs do you want to EQ with AutoEq?")
        .interact_text()
        .unwrap();

    let query_result_url = match filter_link_list(&database_result_list, &headphone_query) {
        QueryResult::Success(url) => {
            println!(
                "Great! The {} could be found in the AutoEq database.",
                headphone_query
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
    let headphone_url = GITHUB_URL.to_owned() + &query_result_url;
    let headphone_query_link_list = collect_links(&client, &headphone_url).await?;
    match pick_url(headphone_query_link_list, PARAM_EQ) {
        Some(url) => {
            let eq_url =
                "https://raw.githubusercontent.com".to_owned() + &url.1.replace("/blob", "");
            let eq_file = client.get(eq_url).send().await?.text().await?;
            progress_bar.finish_with_message("...EQ settings loaded.");

            progress_bar.set_message("Parsing AutoEq settings to CamillaDSP...");
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

            write_yml_file(formatted);
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
