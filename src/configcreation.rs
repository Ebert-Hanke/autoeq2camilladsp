use crate::scraping::CorrectionFilterSet;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{Read, Write},
};

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum Filter {
    Biquad { parameters: BiquadParameters },
    Gain { parameters: GainParameters },
}

#[derive(Debug, Serialize)]
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
    // for future use
    #[allow(dead_code)]
    HighshelfFO {
        freq: f32,
        gain: f32,
    },
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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum PipelineStep {
    Mixer { name: String },
    Filter { channel: usize, names: Vec<String> },
}

pub enum DevicesFile {
    Default,
    Custom(String),
}

pub enum Crossfeed {
    None,
    PowChuMoy,
}

pub fn build_configuration(
    eq_data: CorrectionFilterSet,
    crossfeed: Crossfeed,
) -> Result<Configuration> {
    let mut configuration = Configuration::new();
    build_crossfeed(&mut configuration, crossfeed)?;
    add_correction_eq_filtes(&mut configuration, eq_data);
    Ok(configuration)
}

fn build_crossfeed(configuration: &mut Configuration, crossfeed: Crossfeed) -> Result<()> {
    match crossfeed {
        Crossfeed::None => (),
        Crossfeed::PowChuMoy => {
            add_preset_mixers(configuration)?;
            add_crossfeed_filters(configuration);
            add_crossfeed_pipeline(configuration);
        }
    }
    Ok(())
}

fn add_preset_mixers(configuration: &mut Configuration) -> Result<()> {
    let crossfeed_mixers: HashMap<String, Mixer> =
        serde_yaml::from_slice(include_bytes!("data/crossfeed_mixers.yml"))
            .context("crossfeed_mixers.yml could not be serialized.")?;
    configuration.add_mixers(crossfeed_mixers);
    Ok(())
}

fn add_crossfeed_filters(configuration: &mut Configuration) {
    let mut crossfeed_filters = BTreeMap::new();
    crossfeed_filters.insert(
        "XF_Cross_Gain".to_string(),
        Filter::Gain {
            parameters: GainParameters::new(-8.0),
        },
    );
    crossfeed_filters.insert(
        "XF_Cross_Lowpass".to_string(),
        Filter::Biquad {
            parameters: BiquadParameters::Lowpass {
                freq: 700.0,
                q: 0.5,
            },
        },
    );
    crossfeed_filters.insert(
        "XF_Direct_LowShelf".to_string(),
        Filter::Biquad {
            parameters: BiquadParameters::LowshelfFO {
                freq: 900.0,
                gain: -2.0,
            },
        },
    );
    configuration.add_filters(crossfeed_filters);
}

macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

fn add_crossfeed_pipeline(configuration: &mut Configuration) {
    let mut crossfeed_pipeline = vec![
        PipelineStep::Mixer {
            name: "XF_IN".to_string(),
        },
        PipelineStep::Filter {
            channel: 0,
            names: vec_of_strings!["XF_Cross_Gain", "XF_Cross_Lowpass"],
        },
        PipelineStep::Filter {
            channel: 1,
            names: vec_of_strings!["XF_Direct_LowShelf"],
        },
        PipelineStep::Filter {
            channel: 2,
            names: vec_of_strings!["XF_Direct_LowShelf"],
        },
        PipelineStep::Filter {
            channel: 3,
            names: vec_of_strings!["XF_Cross_Gain", "XF_Cross_Lowpass"],
        },
        PipelineStep::Mixer {
            name: "XF_OUT".to_string(),
        },
    ];
    configuration.add_pipeline_steps(&mut crossfeed_pipeline);
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
    headphone_name: String,
    devices: DevicesFile,
) -> Result<()> {
    let devices_config = get_devices(devices)?;
    let mut config_file = create_config_file(headphone_name)?;
    write_lines_to_file(
        &mut config_file,
        include_str!("data/header.yml").to_string(),
    )?;
    write_lines_to_file(&mut config_file, devices_config)?;
    serialize_and_write_yaml(&mut config_file, &configuration)?;
    Ok(())
}

fn get_devices(devices: DevicesFile) -> Result<String> {
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

fn create_config_file(headphone_name: String) -> Result<File> {
    let filename = format!("{}-EQ.yml", headphone_name.replace(" ", "_"));
    let mut config_file = File::create(filename).context("Could not create configuration file.")?;
    writeln!(config_file, "---").context("Could not write to configuration file.")?;
    Ok(config_file)
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
