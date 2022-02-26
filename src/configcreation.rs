use crate::scraping::CorrectionFilterSet;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{Read, Write},
};

static POWCHUMOY: &[u8] = include_bytes!("data/pow_chu_moy.yml");
static MMP: &[u8] = include_bytes!("data/mmp.yml");

#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration {
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    mixers: HashMap<String, Mixer>,
    filters: BTreeMap<String, Filter>,
    pipeline: Vec<PipelineStep>,
}
impl Configuration {
    fn new() -> Self {
        Configuration {
            mixers: HashMap::new(),
            filters: BTreeMap::new(),
            pipeline: Vec::new(),
        }
    }
    // for future use
    #[allow(dead_code)]
    fn add_mixer(&mut self, mixer_name: String, mixer: Mixer) {
        self.mixers.insert(mixer_name, mixer);
    }
    fn add_mixers(&mut self, mixers: HashMap<String, Mixer>) {
        self.mixers.extend(mixers);
    }
    // for future use
    #[allow(dead_code)]
    fn add_filter(&mut self, filter_name: String, filter: Filter) {
        self.filters.insert(filter_name, filter);
    }
    fn add_filters(&mut self, filters: BTreeMap<String, Filter>) {
        self.filters.extend(filters);
    }
    fn add_pipeline_step(&mut self, pipeline_step: PipelineStep) {
        self.pipeline.push(pipeline_step);
    }
    fn add_pipeline_steps(&mut self, pipeline_steps: &mut Vec<PipelineStep>) {
        self.pipeline.append(pipeline_steps);
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Mixer {
    pub channels: MixerChannels,
    pub mapping: Vec<MixerMapping>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MixerMapping {
    pub dest: usize,
    pub sources: Vec<MixerSource>,
    pub mute: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MixerSource {
    pub channel: usize,
    pub gain: f32,
    pub inverted: bool,
    pub mute: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MixerChannels {
    pub r#in: usize,
    pub out: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Filter {
    Biquad { parameters: BiquadParameters },
    Gain { parameters: GainParameters },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BiquadParameters {
    // for future use
    #[allow(dead_code)]
    Highpass {
        freq: f32,
        q: f32,
    },
    Lowpass {
        freq: f32,
        q: f32,
    },
    Peaking(PeakingWidth),
    HighshelfFO {
        freq: f32,
        gain: f32,
    },
    // for future use
    #[allow(dead_code)]
    LowshelfFO {
        freq: f32,
        gain: f32,
    },
    // for future use
    #[allow(dead_code)]
    HighpassFO {
        freq: f32,
    },
    // for future use
    #[allow(dead_code)]
    LowpassFO {
        freq: f32,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PeakingWidth {
    Q {
        freq: f32,
        q: f32,
        gain: f32,
    },
    // for future use
    #[allow(dead_code)]
    Bandwidth {
        freq: f32,
        bandwidth: f32,
        gain: f32,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GainParameters {
    pub gain: f32,
    pub inverted: bool,
    pub mute: bool,
}
impl GainParameters {
    fn new(gain: f32) -> Self {
        GainParameters {
            gain,
            inverted: false,
            mute: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum PipelineStep {
    Mixer { name: String },
    Filter { channel: usize, names: Vec<String> },
}

#[derive(Debug)]
pub enum DevicesFile {
    Default,
    Custom(String),
}

#[derive(Debug)]
pub enum Crossfeed {
    None,
    PowChuMoy,
    Mmp,
}

pub fn build_configuration(
    eq_data: CorrectionFilterSet,
    crossfeed: &Crossfeed,
) -> Result<Configuration> {
    let mut configuration = Configuration::new();
    build_crossfeed(&mut configuration, crossfeed)?;
    add_correction_eq_filtes(&mut configuration, eq_data);
    Ok(configuration)
}

fn build_crossfeed(configuration: &mut Configuration, crossfeed: &Crossfeed) -> Result<()> {
    match crossfeed {
        Crossfeed::None => (),
        Crossfeed::PowChuMoy => {
            add_crossfeed_config(configuration, POWCHUMOY)?;
        }
        Crossfeed::Mmp => {
            add_crossfeed_config(configuration, MMP)?;
        }
    }
    Ok(())
}

fn add_crossfeed_config(configuration: &mut Configuration, config_bytes: &[u8]) -> Result<()> {
    let mut partial_configuration: Configuration = serde_yaml::from_slice(config_bytes)
        .context("Partial configuration could not be serialized.")?;
    configuration.add_mixers(partial_configuration.mixers);
    configuration.add_filters(partial_configuration.filters);
    configuration.add_pipeline_steps(&mut partial_configuration.pipeline);
    Ok(())
}

fn add_correction_eq_filtes(configuration: &mut Configuration, data: CorrectionFilterSet) {
    let mut correction_eq_filters = BTreeMap::new();

    correction_eq_filters.insert(
        "01_Preamp_Gain".to_string(),
        Filter::Gain {
            parameters: GainParameters::new(data.gain),
        },
    );

    data.eq_bands.into_iter().enumerate().for_each(|(i, band)| {
        let name = format!("Correction_Eq_Band_{}", i);
        correction_eq_filters.insert(name, Filter::Biquad { parameters: band });
    });

    let filter_names: Vec<String> = correction_eq_filters.keys().cloned().collect();

    configuration.add_pipeline_step(PipelineStep::Filter {
        channel: 0,
        names: filter_names.clone(),
    });
    configuration.add_pipeline_step(PipelineStep::Filter {
        channel: 1,
        names: filter_names,
    });

    configuration.add_filters(correction_eq_filters);
}

pub fn write_yml_file(
    configuration: Configuration,
    headphone_name: &str,
    devices: &DevicesFile,
    crossfeed: &Crossfeed,
) -> Result<()> {
    let devices_config = get_devices(devices)?;
    let mut config_file = create_config_file(headphone_name, crossfeed)?;
    write_lines_to_file(
        &mut config_file,
        include_str!("data/header.yml").to_string(),
    )?;
    write_lines_to_file(&mut config_file, devices_config)?;
    serialize_and_write_yaml(&mut config_file, &configuration)?;
    Ok(())
}

fn get_devices(devices: &DevicesFile) -> Result<String> {
    let devices_config = match devices {
        DevicesFile::Default => include_str!("data/default_devices.yml").to_string(),
        DevicesFile::Custom(path) => {
            let mut file =
                File::open(path).context("Could not open file with custom devices section.")?;
            let mut buffer = String::new();
            file.read_to_string(&mut buffer)
                .context("Could not read file with custom devices section.")?;
            buffer
        }
    };
    Ok(devices_config)
}

fn create_config_file(headphone_name: &str, crossfeed: &Crossfeed) -> Result<File> {
    let filename = create_filename(headphone_name, crossfeed);
    let mut config_file = File::create(filename).context("Could not create configuration file.")?;
    writeln!(config_file, "---").context("Could not write to configuration file.")?;
    Ok(config_file)
}

fn create_filename(headphone_name: &str, crossfeed: &Crossfeed) -> String {
    let crossfeed: &str = match crossfeed {
        Crossfeed::None => "EQ",
        Crossfeed::PowChuMoy => "EQ-ChuMoy",
        Crossfeed::Mmp => "EQ-MMP",
    };
    format!("{}-{}.yml", headphone_name.replace(' ', "_"), crossfeed)
}

fn write_lines_to_file(file: &mut File, data: String) -> Result<()> {
    for line in data.lines() {
        if line != "---" {
            writeln!(file, "{}", line)
                .context("Line could not be written to configuration file.")?
        }
    }
    Ok(())
}

fn serialize_and_write_yaml(file: &mut File, configuration: &Configuration) -> Result<()> {
    let serialized_yaml = serde_yaml::to_vec(configuration)
        .context("The ParametricEq filter settings could not be serialized to yaml.")?
        // split of the "---" at the beginning of yml file
        .split_off(4);
    file.write_all(&serialized_yaml)
        .context("Unaible to write serialized config to file.")?;
    Ok(())
}
