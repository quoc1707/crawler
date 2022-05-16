use reqwest::header::USER_AGENT;
use reqwest::Client;
use select::document::Document;
use select::predicate::Name;
use std::collections::HashSet;
use std::env;
use std::iter::Iterator;
use std::time::{Duration, Instant};
use tokio;
use url::Url;

#[derive(Clone)]
struct Spider {
    pub scope: String,
    pub domain: String,
    pub done: i16,
    pub found: i16,
    pub harvested: HashSet<String>,
    pub queued: HashSet<String>,
    pub client: Client,
}

fn spawn(domain: String) -> Spider {
    println!("Generating spider for host: {:?}", domain);
    Spider {
        scope: get_scope(domain.clone()),
        domain: domain.clone(),
        done: 0,
        found: 0,
        harvested: HashSet::new(),
        queued: HashSet::new(),
        client: Client::new(),
    }
}

#[allow(unused_must_use, mutable_borrow_reservation_conflict)]
async fn crawl(mut spider: Spider, args: Vec<String>) -> Spider {
    println!("Crawling {:?}", spider.domain);
    let resp: String = spider
        .client
        .get(spider.domain.clone())
        .header(
            USER_AGENT,
            "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:96.0) Gecko/20100101 Firefox/96.0",
        )
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let mut parsed: Vec<String> = parse_element(resp.clone(), "a".to_string()).await;

    spider.done += 1;

    let harvested = harvest(spider.domain.clone());

    spider.harvested.insert(harvested.clone());

    for p in parsed {
        let p: String = format_link(p, spider.scope.clone()).await;
        if p.len() > 0 && !is_found(p.clone(), spider.clone()) {
            spider.queued.insert(p);
            spider.found += 1;
        }
    }

    if !args.contains(&"--noimage".to_string()) && !args.contains(&"-n".to_string()) {
        parsed = parse_element(resp, "img".to_string()).await;
        for p in parsed {
            let p: String = format_link(p, spider.scope.clone()).await;
            if p.len() > 0 && !is_found(p.clone(), spider.clone()) {
                spider.queued.insert(p);
                spider.found += 1;
            }
        }
    }

    println!("Harvested\n{:?}", spider.harvested.clone());
    println!("\n\nFound #:\n{:?}", spider.found.clone());
    println!("\n\nDone #:\n{:?}", spider.done.clone());
    println!("Queued\n{:?}", spider.queued.clone());

    if let Some(next) = spider.queued.iter().next() {
        spider.domain = next.to_string();
        &spider.queued.remove(&next.to_string());
    }
    spider
}

async fn format_link(mut host: String, mut scope: String) -> String {
    for c in ['#', '?'] {
        if host.contains(c) {
            let splits: Vec<&str> = host.split(c).collect();
            host = splits[0].to_string();
        }
    }

    if host.starts_with(&scope) {
        return host;
    }

    if host.starts_with("/") {
        scope.push_str(&host);
        return scope;
    }
    String::new()
}

fn harvest(host: String) -> String {
    let url: Url = Url::parse(&host).unwrap();
    url.path().to_string()
}

fn is_found(host: String, spider: Spider) -> bool {
    let url: String = harvest(host);
    spider.harvested.contains(&url) || spider.queued.contains(&url)
}

fn get_scope(mut host: String) -> String {
    let url: Url = Url::parse(&host).unwrap();
    let path: &str = url.path();
    host = host
        .clone()
        .strip_suffix(&path)
        .unwrap_or(&host)
        .to_string();
    println!("{:?}", path);
    host
}

async fn parse_element(body: String, ele: String) -> Vec<String> {
    let el: &str = &*ele;
    let attr: &str = if el == "a" { "href" } else { "src" };
    Document::from_read(body.as_bytes())
        .unwrap()
        .find(Name(el))
        .filter_map(|n| n.attr(attr))
        .map(|x| x.to_string())
        .collect()
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() <= 1 {
        panic!("Usage: Cargo run <host>");
    }

    let mut spider: Spider = spawn(args[1].clone());
    let mut i: i16 = 0;
    let now: Instant = Instant::now();
    let z: Duration = Duration::from_secs(0);

    while spider.done.clone() == 0 || spider.done.clone() < spider.found.clone() {
        let total: i16 = spider.found.clone() + spider.done.clone();
        let curr: Duration = now.elapsed();

        if curr > z {
            println!("Link #{:?}\nElapsed duration: {:?}s\nTotal links scraped: {:.0}\nLinks per second: {:.0}", i, curr, total, total as f32/curr.as_secs_f32());
        }
        i += 1;
        spider = crawl(spider, args.clone()).await;
    }
}
