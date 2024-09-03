#[macro_use] extern crate rocket;
use rocket::serde::json::Json;
use rocket::http::Header;
use rocket::{Request, Response};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::data::Limits;

use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::fs::read_to_string;
use std::str::FromStr;
use std::borrow::Cow;

use clap::Parser;
use url::Url;
use serde::{Serialize, Deserialize};

use url_cleaner::types::*;
use url_cleaner::glue::*;

const DEFAULT_MAX_JSON_SIZE: &str = "25MiB";
const DEFAULT_BIND_IP      : &str = "127.0.0.1";
const DEFAULT_PORT         : u16  = 9149;

/// Clap doesn't like `<rocket::data::ByteUnit as FromStr>::Error`.
fn parse_byte_unit(s: &str) -> Result<rocket::data::ByteUnit, String> {
    rocket::data::ByteUnit::from_str(s).map_err(|x| x.to_string())
}

#[derive(Debug, Parser)]
struct Args {
    /// A url_cleaner::types::Config JSON file. If none is provided, uses URL Cleaner's default config.
    #[arg(long, short)] config: Option<PathBuf>,
    /// A url_cleaner::types::ParamsDiff JSON file to apply to the config by default.
    #[arg(long)] params_diff: Option<PathBuf>,
    /// The max size of a POST request to the `/clean` endpoint.
    /// 
    /// The included userscript uses the `/get-max-json-size` endpoint to query this value and adjust its batch sizes accordingly.
    #[arg(long, default_value = DEFAULT_MAX_JSON_SIZE, value_parser = parse_byte_unit)] max_size: rocket::data::ByteUnit,
    /// 127.0.0.1 should be used when only using the userscript.
    /// 
    /// 0.0.0.0 is the simplest way to allow other computers to use this instance of URL Cleaner Site.
    /// 
    /// Please note that while URL Cleaner Site is written in Rust, the default config makes HTTP requests and could therefore be used as a denial of service vector.
    /// 
    /// 0.0.0.0 should only be used on networks you trust and/or behind a firewall.
    #[arg(long, default_value = DEFAULT_BIND_IP, aliases = ["ip", "address"])] bind: IpAddr,
    #[arg(long, default_value_t = DEFAULT_PORT)] port: u16,
    #[arg(long)] cache_path: Option<PathBuf>
}

static CONFIG_STRING: OnceLock<String>                 = OnceLock::new();
static CONFIG       : OnceLock<Config>                 = OnceLock::new();
static MAX_JSON_SIZE: OnceLock<rocket::data::ByteUnit> = OnceLock::new();
static CACHE_HANDLER: OnceLock<CacheHandler>           = OnceLock::new();

#[launch]
fn rocket() -> _ {
    let args = Args::parse();

    CONFIG_STRING.set(args.config.as_deref().map(|path| read_to_string(path).unwrap()).unwrap_or(DEFAULT_CONFIG_STR.to_string())).unwrap();
    let mut config: Config = serde_json::from_str(CONFIG_STRING.get().unwrap()).unwrap();
    if let Some(params_diff) = args.params_diff {
        let params_diff: ParamsDiff = serde_json::from_str(&read_to_string(params_diff).unwrap()).unwrap();
        params_diff.apply(&mut config.params);
    }
    CACHE_HANDLER.set(args.cache_path.as_deref().unwrap_or(config.cache_path.as_path()).try_into().unwrap()).unwrap();
    CONFIG.set(config).unwrap();
    MAX_JSON_SIZE.set(args.max_size).unwrap();

    rocket::custom(rocket::Config {
        address: args.bind,
        port: args.port,
        limits: Limits::default().limit("json", args.max_size),
        ..rocket::Config::default()
    })
        .mount("/", routes![index])
        .mount("/clean", routes![clean])
        .mount("/get-max-json-size", routes![get_max_json_size])
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
    CONFIG_STRING.get().unwrap()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BulkJob {
    jobs: Vec<JobConfig>,
    #[serde(default)]
    params_diff: Option<ParamsDiff>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobError {
    r#type: &'static str,
    error: String
}

impl From<GetJobError> for JobError {
    fn from(value: GetJobError) -> Self {
        Self {
            r#type: "GetJobError",
            error: value.to_string()
        }
    }
}

impl From<DoJobError> for JobError {
    fn from(value: DoJobError) -> Self {
        Self {
            r#type: "DoJobError",
            error: value.to_string()
        }
    }
}

#[post("/", data="<bulk_job>")]
fn clean(bulk_job: Json<BulkJob>) -> Json<Vec<Result<Url, JobError>>> {
    let bulk_job = bulk_job.0;
    let mut config = Cow::Borrowed(CONFIG.get().unwrap());
    if let Some(params_diff) = bulk_job.params_diff {
        params_diff.apply(&mut config.to_mut().params);
    }
    Json(Jobs {
        config,
        cache_handler: CACHE_HANDLER.get().unwrap().clone(),
        job_source: Box::new(bulk_job.jobs.into_iter().map(Ok))
    }.r#do().into_iter().map(|job_result| Ok(job_result??)).collect())
}

#[get("/")]
fn get_max_json_size() -> String {
    MAX_JSON_SIZE.get().unwrap().as_u64().to_string()
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
