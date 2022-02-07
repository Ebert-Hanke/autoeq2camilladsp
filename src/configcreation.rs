use crate::{scraping::CorrectionFilterSet, DevicesFile};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{Read, Write},
};

#[derive(Debug, Serialize)]
pub struct Configuration {
    #[serde(skip_serializing_if = "Option::is_none")]
    mixers: Option<HashMap<String, Mixer>>,
    filters: BTreeMap<String, Filter>,
    pipeline: Vec<PipelineStep>,
}
impl Configuration {
    fn new() -> Self {
        Configuration {
            mixers: None,
            filters: BTreeMap::new(),
            pipeline: Vec::new(),
        }
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
    Highpass { freq: f32, q: f32 },
    Lowpass { freq: f32, q: f32 },
    Peaking(PeakingWidth),
    HighshelfFO { freq: f32, gain: f32 },
    LowshelfFO { freq: f32, gain: f32 },
    HighpassFO { freq: f32 },
    LowpassFO { freq: f32 },
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PeakingWidth {
    Q {
        freq: f32,
        q: f32,
        gain: f32,
    },
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
// #[derive(Debug, Serialize)]
// struct GainParameters {
//     gain: f32,
//     inverted: bool,
//     mute: bool,
// }
// impl GainParameters {
//     fn new(gain: f32) -> Self {
//         GainParameters {
//             gain,
//             inverted: false,
//             mute: false,
//         }
//     }
// }

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum PipelineStep {
    Mixer { name: String },
    Filter { channel: usize, names: Vec<String> },
}

pub fn format_eq_filters(data: CorrectionFilterSet) -> Configuration {
    let mut config = Configuration::new();

    // crossfeed
    let crossfeed = true;
    if crossfeed {
        let crossfeed_mixers: HashMap<String, Mixer> =
            serde_yaml::from_slice(include_bytes!("crossfeed_mixer.yml")).unwrap();
        config.mixers = Some(crossfeed_mixers);

        config.filters.insert(
            "Crossfeed_Gain".to_string(),
            Filter::Gain {
                parameters: GainParameters::new(-8.0),
            },
        );

        config.filters.insert(
            "Crossfeed_EQ".to_string(),
            Filter::Biquad {
                parameters: BiquadParameters::Lowpass {
                    freq: 700.0,
                    q: 0.5,
                },
            },
        );

        config.filters.insert(
            "Crossfeed_Direct_Lowshelf".to_string(),
            Filter::Biquad {
                parameters: BiquadParameters::LowshelfFO {
                    freq: 900.0,
                    gain: -2.0,
                },
            },
        );

        config.pipeline.push(PipelineStep::Mixer {
            name: "Crossfeed_Split".to_string(),
        });

        config.pipeline.push(PipelineStep::Filter {
            channel: 0,
            names: vec!["Crossfeed_Gain".to_string(), "Crossfeed_EQ".to_string()],
        });

        config.pipeline.push(PipelineStep::Filter {
            channel: 1,
            names: vec!["Crossfeed_Direct_Lowshelf".to_string()],
        });

        config.pipeline.push(PipelineStep::Filter {
            channel: 2,
            names: vec!["Crossfeed_Direct_Lowshelf".to_string()],
        });

        config.pipeline.push(PipelineStep::Filter {
            channel: 3,
            names: vec!["Crossfeed_Gain".to_string(), "Crossfeed_EQ".to_string()],
        });

        config.pipeline.push(PipelineStep::Mixer {
            name: "Crossfeed_Sum".to_string(),
        });
    }

    // correction eq filters
    let mut correction_eq_filter_names: Vec<String> = Vec::new();
    config.filters.insert(
        "01_Preamp_Gain".to_string(),
        Filter::Gain {
            parameters: GainParameters::new(data.gain),
        },
    );
    correction_eq_filter_names.push("01_Preamp_Gain".to_string());
    data.eq_bands.into_iter().enumerate().for_each(|(i, band)| {
        let name = format!("Correction_Eq_Band_{}", i);
        correction_eq_filter_names.push(name.clone());
        config
            .filters
            .insert(name, Filter::Biquad { parameters: band });
    });

    // config.filters.iter().for_each(|(n, _)| {
    //     filter_names.push(n.clone());
    // });

    config.pipeline.push(PipelineStep::Filter {
        channel: 0,
        names: correction_eq_filter_names.clone(),
    });
    config.pipeline.push(PipelineStep::Filter {
        channel: 1,
        names: correction_eq_filter_names,
    });

    config
}

pub fn write_yml_file(filterset: Configuration, headphone_name: String, devices: DevicesFile) {
    let devices_config = match devices {
        DevicesFile::Default => include_str!("devices.yml").to_string(),
        DevicesFile::Custom(path) => {
            let mut file = File::open(path).expect("File could not be read.");
            let mut buffer = String::new();
            file.read_to_string(&mut buffer).unwrap();
            buffer
        }
    };
    let serialized_yaml = serde_yaml::to_vec(&filterset).unwrap().split_off(4);

    let filename = format!("{}-AutoEq.yml", headphone_name.replace(" ", "_"));
    let mut config_file = File::create(filename).expect("Could not create file.");
    writeln!(config_file, "---").unwrap();
    for line in devices_config.lines() {
        if line != "---" {
            writeln!(config_file, "{}", line).unwrap();
        }
    }
    config_file.write_all(&serialized_yaml).unwrap();
}
