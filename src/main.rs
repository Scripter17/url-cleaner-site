#[macro_use] extern crate rocket;
use rocket::serde::json::Json;
use rocket::http::Header;
use rocket::{Request, Response};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::data::{Limits, ToByteUnit};
use url::Url;
use serde::{Serialize, Deserialize};
use std::net::{IpAddr, Ipv4Addr};
use clap::Parser;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::fs::read_to_string;
use std::borrow::Cow;

#[derive(Parser)]
struct Args {
    #[arg(long, short)] config: Option<PathBuf>
}

static CONFIG_STR: OnceLock<String> = OnceLock::new();
static CONFIG: OnceLock<url_cleaner::types::Config> = OnceLock::new();

#[launch]
fn rocket() -> _ {
    let args = Args::parse();

    CONFIG_STR.set(args.config.as_deref().map(|path| read_to_string(path).unwrap()).unwrap_or(url_cleaner::types::DEFAULT_CONFIG_STR.to_string())).unwrap();
    CONFIG.set(serde_json::from_str(CONFIG_STR.get().unwrap()).unwrap()).unwrap();

    rocket::custom(rocket::Config {
        port: 9149, // Vanity :3
        address: IpAddr::V4(Ipv4Addr::new(0,0,0,0)),
        limits: Limits::default().limit("/clean", 10.mebibytes()),
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
    urls: Vec<Url>,
    #[serde(default)]
    params_diff: Option<url_cleaner::types::ParamsDiff>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobResponse {
    urls: Vec<Result<Url, String>>
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
        urls: job.urls.into_iter()
            .map(|mut url| {config.apply(&mut url).map_err(|e| e.to_string())?; Ok(url)}).collect()
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
