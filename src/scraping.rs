use crate::configcreation::BiquadParameters;
use scraper::{Html, Selector};
use std::collections::HashMap;

#[derive(Debug)]
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

pub async fn collect_links(
    client: &reqwest::Client,
    url: &str,
) -> Result<HashMap<String, String>, reqwest::Error> {
    let mut link_list: HashMap<String, String> = HashMap::new();
    let link_selector = Selector::parse("a").unwrap();
    let raw_result = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&raw_result);
    //let mut link_list: Vec<ScrapedLink> = Vec::new();
    for element in document.select(&link_selector) {
        let link_text = element.inner_html().to_lowercase();
        let link_url = match element.value().attr("href") {
            Some(url) => url.to_string(),
            _ => "No Url found.".to_string(),
        };
        //let link = ScrapedLink::new(link_text, link_url.to_string());
        link_list.insert(link_text, link_url);
    }
    Ok(link_list)
}

pub enum QueryResult {
    Success(String),
    Suggestions(Vec<String>),
    NotFound,
}

pub fn filter_link_list(link_list: &HashMap<String, String>, query: &str) -> QueryResult {
    match link_list.get(&query.to_lowercase()) {
        Some(url) => {
            println!("Great! The {} could be found in AutoEq.", query);
            QueryResult::Success(url.to_string())
        }
        None => {
            let mut suggestions: Vec<String> = link_list.keys().cloned().collect();
            suggestions.retain(|key| key.to_lowercase().contains(&query.to_lowercase()));
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

pub fn parse_filter_line(line: &str) -> Result<BiquadParameters, Box<dyn std::error::Error>> {
    let mut split_line = line.split(' ');
    let fc = split_line.nth(5);
    let gain = split_line.nth(2);
    let q = split_line.nth(2);
    match (fc, gain, q) {
        (Some(fc), Some(gain), Some(q)) => {
            let fc: f32 = fc.parse()?;
            let gain: f32 = gain.parse()?;
            let q: f32 = q.parse()?;
            let eq = BiquadParameters::new(fc, q, gain);
            Ok(eq)
        }
        _ => panic!("The value could not be found."),
    }
}
