//! A basic HTTP server and userscript to allow automatically applying [URL Cleaner](https://github.com/Scripter17/url-cleaner) to every URL on every webpage you visit.

#[macro_use] extern crate rocket;
use rocket::serde::json::Json;
use rocket::http::Status;
use rocket::Request;
use rocket::data::Limits;

use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::fs::read_to_string;
use std::str::FromStr;
use std::borrow::Cow;

use clap::Parser;

use url_cleaner::types::*;
use url_cleaner::glue::*;

mod types;
use types::*;

/// The default max size of a payload to the [`clean`] route.
const DEFAULT_MAX_JSON_SIZE: &str = "25MiB";
/// The default IP to listen to.
const DEFAULT_BIND_IP      : &str = "127.0.0.1";
/// The default port to listen to.
const DEFAULT_PORT         : u16  = 9149;

/// Clap doesn't like `<rocket::data::ByteUnit as FromStr>::Error`.
fn parse_byte_unit(s: &str) -> Result<rocket::data::ByteUnit, String> {
    rocket::data::ByteUnit::from_str(s).map_err(|x| x.to_string())
}

/// The command line argument format.
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
    /// The IP to listen to.
    #[arg(long, default_value = DEFAULT_BIND_IP, aliases = ["ip", "address"])] bind: IpAddr,
    /// The port to listen to.
    #[arg(long, default_value_t = DEFAULT_PORT)] port: u16,
    /// The path of the cache.
    #[cfg(feature = "cache")]
    #[arg(long)] cache_path: Option<String>
}

/// The [`Config`] to use as a [`String`].
static CONFIG_STRING: OnceLock<String>                 = OnceLock::new();
/// The [`Config`] to use.
static CONFIG       : OnceLock<Config>                 = OnceLock::new();
/// The max size of a payload to the [`clean`] route.
static MAX_JSON_SIZE: OnceLock<rocket::data::ByteUnit> = OnceLock::new();
/// The [`CacheHandler`] to use.
#[cfg(feature = "cache")]
static CACHE_HANDLER: OnceLock<CacheHandler>           = OnceLock::new();

/// Make the server.
#[launch]
async fn rocket() -> _ {
    let args = Args::parse();

    CONFIG_STRING.set(args.config.as_deref().map(|path| read_to_string(path).expect("Reading the Config file to a string to not error.")).unwrap_or(DEFAULT_CONFIG_STR.to_string())).expect("The CONFIG_STRING global static to have not been set.");
    let mut config: Config = serde_json::from_str(CONFIG_STRING.get().expect("The CONFIG_STRING global static to have just been set.")).expect("The CONFIG_STRING to be a valid Config.");
    if let Some(params_diff) = args.params_diff {
        let params_diff: ParamsDiff = serde_json::from_str(&read_to_string(params_diff).expect("Reading the ParamsDiff file to a string to not error.")).expect("The read ParamsDiff file to be a valid ParamsDiff.");
        params_diff.apply(&mut config.params);
    }
    #[cfg(feature = "cache")]
    CACHE_HANDLER.set(args.cache_path.as_deref().unwrap_or(&*config.cache_path).into()).expect("The CACHE_HANDLER global static have not been already set.");
    CONFIG.set(config).expect("The CONFIG global static to have not been already set.");
    MAX_JSON_SIZE.set(args.max_size).expect("The MAX_JSON_SIZE global static to have not been already set.");

    rocket::custom(rocket::Config {
        address: args.bind,
        port: args.port,
        limits: Limits::default().limit("json", args.max_size),
        ..rocket::Config::default()
    })
        .mount("/", routes![index])
        .mount("/clean", routes![clean])
        .register("/clean", catchers![clean_error])
        .mount("/get-max-json-size", routes![get_max_json_size])
        .mount("/get-config", routes![get_config])
}

/// The `/` route.
#[get("/")]
async fn index() -> &'static str {
    r#"Both URL Cleaner Site and URL Cleaner are licensed under the Affero General Public License V3 or later (SPDX: AGPL-3.0-or-later).
https://www.gnu.org/licenses/agpl-3.0.html

The original source code of URL Cleaner Site: https://github.com/Scripter17/url-cleaner-site
The original source code of URL Cleaner: https://github.com/Scripter17/url-cleaner

The modified source code of URL Cleaner Site (if applicable):
The modified source code of URL Cleaner (if applicable):"#
}

/// The `/get-config` route.
#[get("/")]
async fn get_config() -> &'static str {
    CONFIG_STRING.get().expect("The CONFIG_STRING global static to have been set.")
}

/// The `/clean` route.
#[post("/", data="<bulk_job>")]
async fn clean(bulk_job: Json<BulkJob>) -> Json<Result<CleaningSuccess, ()>> {
    let bulk_job = bulk_job.0;
    let mut config = Cow::Borrowed(CONFIG.get().expect("The CONFIG global static to have been set."));
    if let Some(params_diff) = bulk_job.params_diff {
        params_diff.apply(&mut config.to_mut().params);
    }
    Json(Ok(CleaningSuccess {
        urls: Jobs {
            config,
            #[cfg(feature = "cache")]
            cache_handler: CACHE_HANDLER.get().expect("The CACHE_HANDLER global static to have been set.").clone(), // It's a newtype around an Arc, so cloning is O(1).
            configs_source: Box::new(bulk_job.job_configs.into_iter().map(Ok))
        }.r#do().into_iter().map(|job_result| match job_result {
            Ok(Ok(url)) => Ok(Ok(url)),
            Ok(Err(e)) => Ok(Err(e.into())),
            Err(e) => Err(e.into())
        }).collect()
    }))
}

/// The error handler for the `/clean` route.
#[catch(default)]
async fn clean_error(status: Status, _request: &Request<'_>) -> Json<Result<(), crate::types::CleaningError>> {
    Json(Err(crate::types::CleaningError {
        status: status.code,
        reason: status.reason()
    }))
}

/// The `get-max-json-size` route.
#[get("/")]
async fn get_max_json_size() -> String {
    MAX_JSON_SIZE.get().expect("The MAX_JSON_SIZE global static to have been set.").as_u64().to_string()
}
