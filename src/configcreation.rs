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

// #[derive(Debug, Serialize, Deserialize)]
// pub struct Mixers {
//     mixers: HashMap<String, Mixer>,
// }

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
    Gain { parameters: GainParameters },
    Biquad { parameters: BiquadParameters },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum PipelineStep {
    Mixer { name: String },
    Filter { channel: usize, names: Vec<String> },
}

#[derive(Debug, Serialize)]
pub struct BiquadParameters {
    freq: f32,
    q: f32,
    gain: f32,
    #[serde(rename = "type")]
    name: String,
}
impl BiquadParameters {
    pub fn new(freq: f32, q: f32, gain: f32) -> Self {
        BiquadParameters {
            freq,
            q,
            gain,
            name: "Peaking".to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
struct GainParameters {
    gain: f32,
    inverted: bool,
    mute: bool,
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

pub fn format_eq_filters(data: CorrectionFilterSet) -> Configuration {
    let mut config = Configuration::new();

    // crossfeed
    let crossfeed = true;
    if crossfeed {
        let crossfeed_mixers: HashMap<String, Mixer> =
            serde_yaml::from_slice(include_bytes!("crossfeed_mixer.yml")).unwrap();
        config.mixers = Some(crossfeed_mixers);
    }

    // correction eq filters
    config.filters.insert(
        "01_Preamp_Gain".to_string(),
        Filter::Gain {
            parameters: GainParameters::new(data.gain),
        },
    );
    data.eq_bands.into_iter().enumerate().for_each(|(i, band)| {
        let name = format!("Correction_Eq_Band_{}", i);
        config
            .filters
            .insert(name, Filter::Biquad { parameters: band });
    });
    let mut filter_names: Vec<String> = Vec::new();
    config.filters.iter().for_each(|(n, _)| {
        filter_names.push(n.clone());
    });
    config.pipeline = vec![
        PipelineStep::Filter {
            channel: 0,
            names: filter_names.clone(),
        },
        PipelineStep::Filter {
            channel: 1,
            names: filter_names,
        },
    ];
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
