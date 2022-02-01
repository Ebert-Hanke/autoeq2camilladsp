mod configcreation;

use clap::Parser;
use scraper::{Html, Selector};

#[derive(Parser, Debug)]
#[clap(name = "AutoEq2CamillaDSP", version, author = "by m.ebert-hanke")]
#[clap(about = "A simple tool to create a config for Henrik Enquist's CamillaDSP based on corrections from Jaako Pasanen's AutoEq.", long_about = None)]
struct Args {
    #[clap(short = 'p', long, value_name = "'your headphones'")]
    headphone: String,
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let args = Args::parse();

    let client = reqwest::Client::builder()
        .user_agent("autoeq_parser")
        .build()?;

    // define github repository for AutoEq
    let base_url = "https://github.com";
    let repo_url = "/jaakkopasanen/AutoEq/blob/master/results/";
    let url = base_url.to_owned() + repo_url;

    // scrape all links from the results overview page
    let link_results = collect_links(&client, &url).await?;
    println!("Headphone: {}", args.headphone);

    // filter for queried headphone
    let query_result = match_query(&link_results, args.headphone.as_str());
    println!("Name:{}, Url:{}", &query_result.name, &query_result.url);

    // filter for file with parametric eq data
    let query_url = base_url.to_owned() + &query_result.url;
    let query_links = collect_links(&client, &query_url).await?;
    let param_eq_query = "ParametricEQ.txt";
    let eq_result = match_query(&query_links, param_eq_query);
    println!("Name:{}, Url:{}", eq_result.name, eq_result.url);

    // construct url for raw file and get it
    let eq_url =
        "https://raw.githubusercontent.com".to_owned() + &eq_result.url.replace("/blob", "");
    println!("{}", eq_url);
    let eq_file = client.get(eq_url).send().await?.text().await?;

    let mut data = eq_file.lines();
    let preamp_gain = data
        .next()
        .unwrap()
        .split(' ')
        .nth(1)
        .unwrap()
        .parse::<f32>()
        .unwrap();
    let mut headphone_correction = CorrectionFilterSet::new(preamp_gain);
    data.into_iter().skip(0).for_each(|line| {
        let filter = parse_filter_line(line);
        match filter {
            Ok(eq) => {
                headphone_correction.eq_bands.push(eq);
            }
            Err(error) => {
                panic!("{}", error);
            }
        }
    });
    println!("{:?}", headphone_correction);

    Ok(())
}

#[derive(Debug)]
struct CorrectionFilterSet {
    gain: f32,
    eq_bands: Vec<PeakEq>,
}
impl CorrectionFilterSet {
    fn new(gain: f32) -> CorrectionFilterSet {
        CorrectionFilterSet {
            gain,
            eq_bands: Vec::new(),
        }
    }
}
#[derive(Debug)]
struct PeakEq {
    fc: f32,
    gain: f32,
    q: f32,
}
impl PeakEq {
    fn new(fc: f32, gain: f32, q: f32) -> PeakEq {
        PeakEq { fc, gain, q }
    }
}

#[derive(Debug, Clone)]
struct ScrapedLink {
    name: String,
    url: String,
}
impl ScrapedLink {
    fn new(name: String, url: String) -> ScrapedLink {
        ScrapedLink { name, url }
    }
}

async fn collect_links(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<ScrapedLink>, reqwest::Error> {
    let a_selector = Selector::parse("a").unwrap();
    let raw_result = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&raw_result);
    let mut link_list: Vec<ScrapedLink> = Vec::new();
    for element in document.select(&a_selector) {
        let link_text = element.inner_html().to_string();
        let link_url = match element.value().attr("href") {
            Some(url) => url,
            _ => "No Url found.",
        };
        let link = ScrapedLink::new(link_text, link_url.to_string());
        link_list.push(link);
    }
    Ok(link_list)
}

fn match_query(scraped_links: &[ScrapedLink], query: &str) -> ScrapedLink {
    match scraped_links.iter().find(|link| {
        link.name
            .to_lowercase()
            .contains(query.to_lowercase().trim())
    }) {
        Some(link) => link.clone(),
        _ => ScrapedLink::new("no link".to_string(), "no url".to_string()),
    }
}

fn parse_filter_line(line: &str) -> Result<PeakEq, Box<dyn std::error::Error>> {
    // println!("The Line:{}", line);
    let mut split_line = line.split(' ');
    let fc = split_line.nth(5);
    let gain = split_line.nth(2);
    let q = split_line.nth(2);
    // println!("fc:{:?},gain:{:?},q:{:?}", fc, gain, q);
    match (fc, gain, q) {
        (Some(fc), Some(gain), Some(q)) => {
            let fc: f32 = fc.parse()?;
            let gain: f32 = gain.parse()?;
            let q: f32 = q.parse()?;
            let eq = PeakEq::new(fc, gain, q);
            Ok(eq)
        }
        _ => panic!("The value could not be found."),
    }
}
