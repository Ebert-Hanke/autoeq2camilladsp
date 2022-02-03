use crate::scraping::CorrectionFilterSet;
use serde::Serialize;
use std::{collections::BTreeMap, io::Write};

#[derive(Debug, Serialize)]
pub struct Configuration {
    filters: BTreeMap<String, Filter>,
    pipeline: Vec<PipelineStep>,
}
impl Configuration {
    fn new() -> Self {
        Configuration {
            filters: BTreeMap::new(),
            pipeline: Vec::new(),
        }
    }
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
    Filter { channel: usize, names: Vec<String> },
}

#[derive(Debug, Serialize)]
pub struct BiquadParameters {
    freq: f32,
    q: f32,
    gain: f32,
}
impl BiquadParameters {
    pub fn new(freq: f32, q: f32, gain: f32) -> Self {
        BiquadParameters { freq, q, gain }
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

pub fn write_yml_file(filterset: Configuration) {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open("testfile.yml")
        .expect("Could not open file.");
    let devices = include_bytes!("devices.yml");
    let serialized_yaml = serde_yaml::to_vec(&filterset).unwrap().split_off(4);
    file.write_all(devices).unwrap();
    file.write_all(&serialized_yaml).unwrap();
}
