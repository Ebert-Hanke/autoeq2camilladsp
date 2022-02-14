use anyhow::{Context, Result};
use scraper::{Html, Selector};
use serde::Serialize;
use std::collections::HashMap;

use crate::configcreation::{BiquadParameters, PeakingWidth};

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
pub enum QueryResult {
    Success(Link),
    Suggestions(Vec<String>),
    NotFound,
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

// pub async fn get_correction_result_list(
//     client: &reqwest::Client,
//     url: &str,
// ) -> Result<HashMap<String, String>> {
//     let mut link_list: HashMap<String, String> = HashMap::new();
//     let ul_selector = Selector::parse("ul").unwrap();
//     let li_selector = Selector::parse("li").unwrap();
//     let a_selector = Selector::parse("a").unwrap();
//     let raw_result = client.get(url).send().await?.text().await?;
//     let document = Html::parse_document(&raw_result);
//     for ul in document.select(&ul_selector) {
//         for li in ul.select(&li_selector) {
//             for a in li.select(&a_selector) {
//                 let link_text = a.inner_html().to_lowercase();
//                 let link_url = match a.value().attr("href") {
//                     Some(url) => url.to_string(),
//                     _ => "Nor Url found.".to_string(),
//                 };
//                 if !link_text.len() > 100 && !link_text.contains('<') && !link_text.contains('>') {
//                     link_list.insert(link_text, link_url);
//                 }
//             }
//         }
//     }

//     Ok(link_list)
// }

// pub async fn collect_datafile_links(
//     client: &reqwest::Client,
//     url: &str,
// ) -> Result<HashMap<String, String>> {
//     let mut link_list: HashMap<String, String> = HashMap::new();
//     let a_selector = Selector::parse("a").unwrap();
//     let raw_result = client.get(url).send().await?.text().await?;
//     let document = Html::parse_document(&raw_result);
//     for a in document.select(&a_selector) {
//         let link_text = a.inner_html().to_lowercase();
//         let link_url = match a.value().attr("href") {
//             Some(url) => url.to_string(),
//             _ => "Nor Url found.".to_string(),
//         };
//         if !link_text.len() > 100 && !link_text.contains('<') && !link_text.contains('>') {
//             link_list.insert(link_text, link_url);
//         }
//     }

//     Ok(link_list)
// }

pub fn filter_link_list(link_list: &HashMap<String, String>, query: &str) -> QueryResult {
    match link_list.get(&query.to_lowercase()) {
        Some(url) => {
            println!("Great! The {} could be found in AutoEq.", query);
            QueryResult::Success(Link::new(query.to_string(), url.to_string()))
        }
        None => {
            let mut suggestions: Vec<String> = link_list.keys().cloned().collect();
            query
                .to_lowercase()
                .split_whitespace()
                .into_iter()
                .for_each(|part| suggestions.retain(|key| key.to_lowercase().contains(part)));
            if !suggestions.is_empty() {
                return QueryResult::Suggestions(suggestions);
            }
            QueryResult::NotFound
        }
    }
}

pub fn pick_url(link_list: HashMap<String, String>, query: &str) -> Option<(String, String)> {
    link_list
        .into_iter()
        .find(|(k, _v)| k.to_lowercase().contains(&query.to_lowercase()))
        .map(|(k, v)| (k, v))
}

pub fn parse_filter_line(line: &str) -> Result<BiquadParameters> {
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
