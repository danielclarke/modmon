// https://patshaughnessy.net/2020/1/20/downloading-100000-files-using-async-rust
// https://rust-lang.github.io/async-book/01_getting_started/01_chapter.html

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use futures::StreamExt;

#[derive(Clone, Copy, Debug)]
enum ModerationState {
    Banned,
    Passed,
    Pending,
    Error,
}

struct NamedParam {
    name: String,
    value: Option<String>,
}

#[derive(Debug)]
struct TransientConfig {
    cookie: Option<String>,
    id_file: Option<String>,
    offset: Option<u32>,
}

impl TransientConfig {
    fn new() -> TransientConfig {
        TransientConfig {
            cookie: None,
            id_file: None,
            offset: None,
        }
    }
}

#[derive(Debug)]
struct Config {
    cookie: String,
    id_file: String,
    offset: u32,
}

fn parse(arg: &str) -> Result<NamedParam, &str> {
    let binding = arg.replace("--", "");
    let a: Vec<&str> = binding.split('=').collect();
    match (a.first(), a.get(1)) {
        (None, _) => Err("unknown parameter"),
        (Some(name), None) => Ok(NamedParam {
            name: name.to_string(),
            value: None,
        }),
        (Some(_), Some(&"")) => Err("missing argument"),
        (Some(name), Some(value)) => Ok(NamedParam {
            name: name.to_string(),
            value: Some(value.to_string()),
        }),
    }
}

fn parse_args(args: &[String]) -> Result<Config, &str> {
    let mut config = TransientConfig::new();
    for arg in args {
        let NamedParam { name, value } = parse(arg)?;
        match (name.as_str(), value) {
            (_, None) => return Err("missing parameter"),
            ("cookie", Some(v)) => config.cookie = Some(v),
            ("id_file", Some(v)) => config.id_file = Some(v),
            ("offset", Some(v)) => match v.parse::<u32>() {
                Ok(v) => config.offset = Some(v),
                Err(_) => return Err("offset param not a number"),
            },
            _ => return Err("unknown argument"),
        }
    }

    match config {
        TransientConfig {
            cookie: None,
            id_file: _,
            offset: _,
        } => Err("cookie: missing parameter"),
        TransientConfig {
            cookie: _,
            id_file: None,
            offset: _,
        } => Err("id_file: missing parameter"),
        TransientConfig {
            cookie: Some(cookie),
            id_file: Some(id_file),
            offset,
        } => match offset {
            Some(offset) => Ok(Config {
                cookie,
                id_file,
                offset,
            }),
            None => Ok(Config {
                cookie,
                id_file,
                offset: 0,
            }),
        },
    }
}

fn read_lines<P>(filename: P) -> Result<Vec<String>, Box<dyn Error>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file)
        .lines()
        .filter_map(Result::ok)
        .collect())
}

async fn query(client: &reqwest::Client, rbxassetid: &str) {
    let resp = client
        .get(format!(
            "https://www.roblox.com/library/{rbxassetid}",
            rbxassetid = rbxassetid
        ))
        .send()
        .await;

    let state = match resp {
        Ok(resp) => match resp.text().await {
            Ok(resp) => match resp.find("data-mediathumb-url") {
                None => match resp.find("Decal Image") {
                    None => ModerationState::Banned,
                    Some(_) => ModerationState::Passed,
                },
                Some(_) => ModerationState::Passed,
            },
            Err(_) => ModerationState::Error,
        },
        Err(_) => ModerationState::Error,
    };

    println!("{} {:?}", &rbxassetid, state);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let config = parse_args(&args[1..args.len()])?;

    let mut headers = reqwest::header::HeaderMap::new();
    let c = format!(".ROBLOSECURITY={cookie}", cookie = config.cookie);
    let header_value = reqwest::header::HeaderValue::from_str(&c)?;
    headers.insert("Cookie", header_value);

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let client_ref = &client;

    if let Ok(lines) = read_lines(config.id_file) {
        let f = futures::stream::iter(lines[config.offset as usize..lines.len()].iter().map(
            |rbxassetid| async move {
                query(client_ref, rbxassetid).await;
            },
        ))
        .buffer_unordered(8)
        .collect::<Vec<()>>();
        f.await;
    }
    Ok(())
}
