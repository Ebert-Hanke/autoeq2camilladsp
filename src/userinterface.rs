use crate::{
    configcreation::{Crossfeed, DevicesFile},
    scraping::{filter_link_list, QueryResult},
};

use anyhow::Result;
use console::{style, Style};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::{collections::HashMap, env, fs::File};

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

#[derive(Debug)]
pub struct Cli {
    pub headphone: String,
    pub headphone_query_result: QueryResult,
    pub headphone_url: String,
    pub devices: DevicesFile,
    pub crossfeed: Crossfeed,
}
impl Cli {
    pub fn initialize() -> Self {
        Cli {
            headphone: String::new(),
            headphone_query_result: QueryResult::NotFound,
            headphone_url: String::new(),
            devices: DevicesFile::Default,
            crossfeed: Crossfeed::None,
        }
    }

    pub fn welcome() {
        let logo = format!(
            r"
           _                   _       
  __ _ _  _| |_ ___  ___ __ _  | |_ ___ 
 / _` | || |  _/ _ \/ -_) _` | |  _/ _ \
 \__,_|\_,_|\__\___/\___\__, |_ \__\___/
  __ __ _ _ __ (_) | |__ _ |_| |____ __ 
 / _/ _` | '  \| | | / _` / _` (_-< '_ \
 \__\__,_|_|_|_|_|_|_\__,_\__,_/__/ .__/
                                  |_|    
  {}
",
            env!("CARGO_PKG_VERSION")
        );

        print!("{}", style(logo).magenta().bold());
        println!("Make your Headphones or IEMs more enjoyable with AutoEq and Crossfeed");
        println!();
    }

    pub fn query_headphone(&mut self) -> Result<()> {
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
        self.headphone = headphone_query;
        Ok(())
    }

    pub fn consult_database(&mut self, database: &HashMap<String, String>) -> Result<()> {
        println!();
        self.headphone_query_result = filter_link_list(database, &self.headphone);
        self.headphone_url = match &mut self.headphone_query_result {
            QueryResult::Success(link) => {
                println!(
                    "Great! The {} could be found in the AutoEq database.",
                    link.name
                );
                link.url.to_string()
            }
            QueryResult::Suggestions(suggestions) => {
                suggestions.push("Nope, nothing here for me ...".to_string());
                let selection = Select::with_theme(&ColorfulTheme::clitheme())
                    .with_prompt("Maybe one of these is the one you are looking for?")
                    .default(0)
                    .items(&suggestions[..])
                    .interact()?;
                match filter_link_list(database, &suggestions[selection]) {
                    QueryResult::Success(link) => {
                        self.headphone = link.name;
                        link.url
                    }
                    _ => std::process::exit(0),
                }
            }
            QueryResult::NotFound => {
                println!(
                    "Sorry the {} or something similar could not be found in the AutoEq database.",
                    self.headphone
                );
                std::process::exit(0);
            }
        };
        println!();
        Ok(())
    }

    pub fn query_custom_devices(&mut self) -> Result<()> {
        let custom_explainer: &str = r"
You have the option to include a custom 'devices' section from a .yml file.
If you do not choose to do so, the configuration will be created with a default 'devices' section.
You then can edit this and use for future configurations.
";
        println!();
        print!("{}", style(custom_explainer).magenta());
        println!();

        let custom_devices_query: bool = Confirm::with_theme(&ColorfulTheme::clitheme())
        .with_prompt(
            "Would you like to include a custom 'devices' section for your CamillaDSP config file?",
        )
        .interact()?;

        self.devices = match custom_devices_query {
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
        Ok(())
    }

    pub fn query_crossfeed(&mut self) -> Result<()> {
        println!();
        let crossfeed_query: bool = Confirm::with_theme(&ColorfulTheme::clitheme())
        .with_prompt(
            "Would you like to include Crossfeed modeled after the analogue implementation by Pow Chu Moy?"
        )
        .interact()?;

        self.crossfeed = match crossfeed_query {
            true => Crossfeed::PowChuMoy,
            false => Crossfeed::None,
        };
        println!();
        Ok(())
    }
}
