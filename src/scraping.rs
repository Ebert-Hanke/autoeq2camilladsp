use anyhow::{anyhow, Result};
use scraper::{Html, Selector};
use serde::Serialize;
use std::collections::HashMap;

use crate::configcreation::{BiquadParameters, PeakingWidth};
use crate::Config;

#[derive(Debug, Serialize)]
pub struct CorrectionFilterSet {
    pub gain: f32,
    pub eq_bands: Vec<BiquadParameters>,
}
impl CorrectionFilterSet {
    pub fn new(gain: f32) -> CorrectionFilterSet {
        CorrectionFilterSet {
            gain,
            eq_bands: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct Link {
    pub name: String,
    pub url: String,
}
impl Link {
    pub fn new(name: String, url: String) -> Self {
        Link { name, url }
    }
}

pub async fn scrape_links(client: &reqwest::Client, url: &str) -> Result<HashMap<String, String>> {
    let html = get_html(client, url).await?;
    let links = filter_links(html);
    Ok(links)
}

async fn get_html(client: &reqwest::Client, url: &str) -> Result<Html> {
    let raw_result = client.get(url).send().await?.text().await?;
    let html = Html::parse_document(&raw_result);
    Ok(html)
}

fn filter_links(html: Html) -> HashMap<String, String> {
    let mut link_list: HashMap<String, String> = HashMap::new();
    let select_a = Selector::parse("a").unwrap();

    for link in html.select(&select_a) {
        if let Some(url) = link.value().attr("href") {
            let link_text = link
                .inner_html()
                .to_lowercase()
                .trim()
                .replace("&amp;", "&")
                .to_string();
            let link_url = url.trim().to_string();
            if !link_text.len() > 100
                && !link_text.contains('<')
                && !link_text.contains('>')
                && link_url != "#"
            {
                link_list.insert(link_text, link_url);
            }
        };
    }
    link_list
}

pub fn filter_link_list(link_list: &HashMap<String, String>, query: &str) -> Option<Link> {
    link_list
        .get(&query.to_lowercase())
        .map(|url| Link::new(query.to_string(), url.to_string()))
}

pub async fn scrape_eq_settings(
    link_list: HashMap<String, String>,
    client: &reqwest::Client,
    config: &Config,
) -> Result<CorrectionFilterSet> {
    let eq_link = pick_url(link_list, &config.parametric_eq_query);
    match eq_link {
        Some(link) => {
            let eq_file = client
                .get(config.raw_eq_url(&link.url))
                .send()
                .await?
                .text()
                .await?;
            let mut data = eq_file.lines();
            let preamp_gain = parse_preamp_gain(&mut data)?;
            let mut filterset = CorrectionFilterSet::new(preamp_gain);
            parse_filters(&mut data, &mut filterset)?;
            Ok(filterset)
        }
        None => Err(anyhow!("The eq data could not be parsed.")),
    }
}

fn pick_url(link_list: HashMap<String, String>, query: &str) -> Option<Link> {
    link_list
        .into_iter()
        .find(|(k, _v)| k.to_lowercase().contains(&query.to_lowercase()))
        .map(|(k, v)| Link::new(k, v))
}

pub fn parse_preamp_gain(lines: &mut std::str::Lines) -> Result<f32> {
    let gain = lines
        .next()
        .ok_or_else(|| anyhow!("Not enough lines."))?
        .split(' ')
        .nth(1)
        .ok_or_else(|| anyhow!("Not enough elements."))?
        .parse::<f32>()?;
    Ok(gain)
}

pub fn parse_filters(
    lines: &mut std::str::Lines,
    filterset: &mut CorrectionFilterSet,
) -> Result<()> {
    for line in lines.skip(0) {
        let eq = parse_filter_line(line)?;
        filterset.eq_bands.push(eq);
    }
    Ok(())
}

fn parse_filter_line(line: &str) -> Result<BiquadParameters> {
    let mut split_line = line.split(' ');
    let fc = split_line.nth(5);
    let gain = split_line.nth(2);
    let q = split_line.nth(2);
    match (fc, gain, q) {
        (Some(fc), Some(gain), Some(q)) => {
            let freq: f32 = fc.parse()?;
            let gain: f32 = gain.parse()?;
            let q: f32 = q.parse()?;
            let eq = BiquadParameters::Peaking(PeakingWidth::Q { freq, q, gain });
            Ok(eq)
        }
        _ => panic!("The value could not be found."),
    }
}
