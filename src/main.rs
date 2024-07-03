#[macro_use] extern crate rocket;
use rocket::serde::json::Json;
use rocket::http::Header;
use rocket::{Request, Response};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::data::Limits;
use url::Url;
use serde::{Serialize, Deserialize};
use std::net::IpAddr;
use clap::Parser;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::fs::read_to_string;
use std::borrow::Cow;
use std::str::FromStr;

/// Clap doesn't like `<rocket::data::ByteUnit as FromStr>::Error`.
fn parse_byte_unit(s: &str) -> Result<rocket::data::ByteUnit, String> {
    rocket::data::ByteUnit::from_str(s).map_err(|x| x.to_string())
}

#[derive(Debug, Parser)]
struct Args {
    /// A url_cleaner::types::Config JSON file. If none is provided, uses URL Cleaner's default config.
    #[arg(long, short)] config: Option<PathBuf>,
    /// A url_cleaner::types::ParamsDiff JSON file to apply to the config by default.
    #[arg(long       )] params_diff: Option<PathBuf>,
    #[arg(long       , default_value = "10MiB", value_parser = parse_byte_unit)] max_size: rocket::data::ByteUnit,
    #[arg(long       , default_value = "0.0.0.0")] ip: IpAddr,
    #[arg(long       , default_value = "9149"   )] port: u16
}

static CONFIG_STR: OnceLock<String> = OnceLock::new();
static CONFIG: OnceLock<url_cleaner::types::Config> = OnceLock::new();

#[launch]
fn rocket() -> _ {
    let args = Args::parse();

    CONFIG_STR.set(args.config.as_deref().map(|path| read_to_string(path).unwrap()).unwrap_or(url_cleaner::types::DEFAULT_CONFIG_STR.to_string())).unwrap();
    let mut config: url_cleaner::types::Config = serde_json::from_str(CONFIG_STR.get().unwrap()).unwrap();
    if let Some(params_diff) = args.params_diff {
        let params_diff: url_cleaner::types::ParamsDiff = serde_json::from_str(&read_to_string(params_diff).unwrap()).unwrap();
        params_diff.apply(&mut config.params);
    }
    CONFIG.set(config).unwrap();

    rocket::custom(rocket::Config {
        address: args.ip,
        port: args.port,
        limits: Limits::default().limit("json", args.max_size),
        ..rocket::Config::default()
    })
        .mount("/", routes![index])
        .mount("/clean", routes![clean])
        .mount("/get-config", routes![get_config])
        .attach(Anarcors)
}

#[get("/")]
fn index() -> &'static str {
    r#"Both URL Cleaner Site and URL Cleaner are licensed under the Affero General Public License V3 or later (SPDX: AGPL-3.0-or-later).
https://en.wikipedia.org/wiki/GNU_Affero_General_Public_License
https://www.gnu.org/licenses/agpl-3.0.html

The original source code of URL Cleaner: https://github.com/Scripter17/url-cleaner
The original source code of URL Cleaner Site: https://github.com/Scripter17/url-cleaner-site

The modified source code of URL Cleaner (if applicable): 
The modified source code of URL Cleaner Site (if applicable): "#
}

#[get("/")]
fn get_config() -> &'static str {
    CONFIG_STR.get().unwrap()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Job {
    urls: Vec<String>,
    #[serde(default)]
    params_diff: Option<url_cleaner::types::ParamsDiff>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobResponse {
    urls: Vec<Result<Url, JobError>>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobError {
    r#type: String,
    source: String,
    error: String
}

#[post("/", data="<job>")]
fn clean(job: Json<Job>) -> Json<JobResponse> {
    let job = job.0;
    let config = if let Some(params_diff) = job.params_diff {
        let mut config = CONFIG.get().unwrap().clone();
        params_diff.apply(&mut config.params);
        Cow::Owned(config)
    } else {
        Cow::Borrowed(CONFIG.get().unwrap())
    };
    Json(JobResponse {
        urls: job.urls
            .into_iter()
            .map(|url| {
                let mut url = Url::parse(&url)
                    .map_err(|e| JobError { r#type: "ParseError".to_string(), source: url            , error: e.to_string() })?;
                config.apply(&mut url)
                    .map_err(|e| JobError { r#type: "RuleError" .to_string(), source: url.to_string(), error: e.to_string() })?;
                Ok(url)
            })
            .collect()
    })
}

struct Anarcors;

#[rocket::async_trait]
impl Fairing for Anarcors {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to response",
            kind: Kind::Response
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "POST, PATCH, PUT, DELETE, HEAD, OPTIONS, GET"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}
