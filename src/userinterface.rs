use crate::{
    configcreation::{Crossfeed, DevicesFile},
    scraping::filter_link_list,
};

use anyhow::{anyhow, Result};
use console::{style, Style};
use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect, Input, Select};
use std::{collections::HashMap, env, fs::File, thread, time::Duration};

pub trait CliTheme {
    fn clitheme() -> Self;
}
impl CliTheme for ColorfulTheme {
    fn clitheme() -> ColorfulTheme {
        ColorfulTheme {
            active_item_prefix: style("❯".to_string()).for_stderr().yellow(),
            active_item_style: Style::new().for_stderr().magenta(),
            checked_item_prefix: style("✓".to_string()).for_stderr().green(),
            defaults_style: Style::new().for_stderr().magenta(),
            error_prefix: style("✕".to_string()).for_stderr().red(),
            error_style: Style::new().for_stderr().red(),
            fuzzy_cursor_style: Style::new().for_stderr().black().on_white(),
            fuzzy_match_highlight_style: Style::new().for_stderr().black().on_white(),
            hint_style: Style::new().for_stderr().black().bright(),
            inactive_item_prefix: style(" ".to_string()).for_stderr(),
            inactive_item_style: Style::new().for_stderr(),
            inline_selections: true,
            picked_item_prefix: style("❯".to_string()).for_stderr().yellow(),
            prompt_prefix: style("?".to_string()).for_stderr().yellow(),
            prompt_style: Style::new().for_stderr().magenta().bold(),
            prompt_suffix: style("›".to_string()).for_stderr().black().bright(),
            success_prefix: style("✓".to_string()).for_stderr().green(),
            success_suffix: style("·".to_string()).for_stderr().black().bright(),
            unchecked_item_prefix: style("✓".to_string()).for_stderr().black(),
            unpicked_item_prefix: style(" ".to_string()).for_stderr(),
            values_style: Style::new().for_stderr().green(),
        }
    }
}

#[derive(Debug)]
pub struct Cli {
    pub headphone: String,
    pub headphone_url: String,
    pub devices: DevicesFile,
    pub crossfeed: Crossfeed,
}
impl Cli {
    pub fn initialize() -> Self {
        Cli {
            headphone: String::new(),
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

---------------------------------------------
 Make your Headphones or IEMs more enjoyable
 with AutoEq and Crossfeed
---------------------------------------------


",
            env!("CARGO_PKG_VERSION")
        );

        print!("{}", style(logo).magenta().bold());
        thread::sleep(Duration::from_millis(2000));
    }

    pub fn select_headphone(&mut self, database: &HashMap<String, String>) -> Result<()> {
        let mut suggestions: Vec<String> = database.keys().cloned().collect();
        suggestions.push("Exit".to_string());
        println!();
        let selection = FuzzySelect::with_theme(&ColorfulTheme::clitheme())
            .with_prompt("Pick your device. Start typing to narrow down or type 'Exit' to quit.")
            .default(0)
            .items(&suggestions)
            .interact()?;
        if suggestions[selection] == "Exit" {
            std::process::exit(0);
        }
        let headphone_link = filter_link_list(database, &suggestions[selection]);
        match headphone_link {
            Some(link) => {
                self.headphone = link.name;
                self.headphone_url = link.url;
                println!();
                Ok(())
            }
            None => Err(anyhow!(
                "Something went wrong while accessing the database."
            )),
        }
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
        let items = vec![
            "None",
            "Pow Chu Moy Crossfeed",
            "MPM Crossfeed",
            "Natural Crossfeed",
        ];
        let crossfeed_query = Select::with_theme(&ColorfulTheme::clitheme())
        .with_prompt(
            "Please select the type of Crossfeed you would like to include in your configuration:"
        )
        .items(&items)
        .default(0)
        .interact()?;

        self.crossfeed = match crossfeed_query {
            0 => Crossfeed::None,
            1 => Crossfeed::PowChuMoy,
            2 => Crossfeed::Mpm,
            3 => Crossfeed::Natural,
            _ => Crossfeed::None,
        };
        println!();
        Ok(())
    }
}
