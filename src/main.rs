//! A basic HTTP server and userscript to allow automatically applying [URL Cleaner](https://github.com/Scripter17/url-cleaner) to every URL on every webpage you visit.

use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::fs::read_to_string;
use std::str::FromStr;
use std::borrow::Cow;
use std::collections::HashMap;

#[macro_use] extern crate rocket;
use rocket::serde::json::Json;
use rocket::http::Status;
use rocket::Request;
use rocket::data::Limits;
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
    #[cfg(feature = "default-config")]
    #[arg(long, short)] config: Option<PathBuf>,
    /// A url_cleaner::types::Config JSON file. Has to be set because this instance of URL Cleaner Site was compiled without a default config.
    #[cfg(not(feature = "default-config"))]
    #[arg(long, short)] config: PathBuf,
    /// A url_cleaner::types::ParamsDiff JSON file to apply to the config by default.
    #[arg(long)] params_diff: Vec<PathBuf>,
    /// The max size of a POST request to the `/clean` endpoint.
    /// 
    /// The included userscript uses the `/get-max-json-size` endpoint to query this value and adjust its batch sizes accordingly.
    #[arg(long, default_value = DEFAULT_MAX_JSON_SIZE, value_parser = parse_byte_unit)] max_size: rocket::data::ByteUnit,
    /// The IP to listen to.
    #[arg(long, default_value = DEFAULT_BIND_IP, aliases = ["ip", "address"])] bind: IpAddr,
    /// The port to listen to.
    #[arg(long, default_value_t = DEFAULT_PORT)] port: u16,
    /// Set flags.
    #[arg(short      , long, value_names = ["NAME"])]
    pub flag  : Vec<String>,
    /// Unset flags set by the config.
    #[arg(short = 'F', long, value_names = ["NAME"])]
    pub unflag: Vec<String>,
    /// For each occurrence of this option, its first argument is the variable name and the second argument is its value.
    #[arg(short      , long, num_args(2), value_names = ["NAME", "VALUE"])]
    pub var: Vec<Vec<String>>,
    /// Unset variables set by the config.
    #[arg(short = 'V', long, value_names = ["NAME"])]
    pub unvar : Vec<String>,
    /// For each occurrence of this option, its first argument is the set name and subsequent arguments are the values to insert.
    #[arg(             long, num_args(2..), value_names = ["NAME", "VALUE"])]
    pub insert_into_set: Vec<Vec<String>>,
    /// For each occurrence of this option, its first argument is the set name and subsequent arguments are the values to remove.
    #[arg(             long, num_args(2..), value_names = ["NAME", "VALUE"])]
    pub remove_from_set: Vec<Vec<String>>,
    /// For each occurrence of this option, its first argument is the map name, the second is the map key, and subsequent arguments are the values to insert.
    #[arg(             long, num_args(3..), value_names = ["NAME", "KEY1", "VALUE1"])]
    pub insert_into_map: Vec<Vec<String>>,
    /// For each occurrence of this option, its first argument is the map name, the second is the map key, and subsequent arguments are the values to remove.
    #[arg(             long, num_args(2..), value_names = ["NAME", "KEY1"])]
    pub remove_from_map: Vec<Vec<String>>,
    /// Overrides the config's [`Config::cache_path`].
    #[arg(             long)]
    pub cache_path: Option<String>,
    /// Read stuff from caches. Default value is controlled by the config. Omitting a value means true.
    #[cfg(feature = "cache")]
    #[arg(             long, num_args(0..=1), default_missing_value("true"))]
    pub read_cache : Option<bool>,
    /// Write stuff to caches. Default value is controlled by the config. Omitting a value means true.
    #[cfg(feature = "cache")]
    #[arg(             long, num_args(0..=1), default_missing_value("true"))]
    pub write_cache: Option<bool>,
    /// The proxy to use. Example: socks5://localhost:9150
    #[cfg(feature = "http")]
    #[arg(             long)]
    pub proxy: Option<ProxyConfig>,
    /// Disables all HTTP proxying.
    #[cfg(feature = "http")]
    #[arg(             long, num_args(0..=1), default_missing_value("true"))]
    pub no_proxy: Option<bool>,
    /// Print the parsed arguments for debugging.
    /// When this, any other `--print-...` flag, or `--test-config` is set, no URLs are cleaned.
    #[arg(             long, verbatim_doc_comment)]
    pub print_args: bool,
    /// Print the ParamsDiffs loaded from `--params--diff` files and derived from the parsed arguments for debugging.
    /// When this, any other `--print-...` flag, or `--test-config` is set, no URLs are cleaned.
    #[arg(             long, verbatim_doc_comment)]
    pub print_params_diffs: bool,
    /// Print the config's params after applying the ParamsDiff.
    /// When this, any other `--print-...` flag, or `--test-config` is set, no URLs are cleaned.
    #[arg(             long, verbatim_doc_comment)]
    pub print_params: bool,
    /// Print the specified config as JSON after applying the ParamsDiff.
    /// When this, any other `--print-...` flag, or `--test-config` is set, no URLs are cleaned.
    #[arg(             long, verbatim_doc_comment)]
    pub print_config: bool,
    /// Print the config's documentation.
    /// When this, any other `--print-...` flag, or `--test-config` is set, no URLs are cleaned.
    #[arg(             long, verbatim_doc_comment)]
    pub print_docs: bool,
    /// Run the config's tests.
    /// When this or any `--print-...` flag is set, no URLs are cleaned.
    #[arg(             long, verbatim_doc_comment)]
    pub test_config : bool
}

/// The [`Config`] to use as a [`String`].
static CONFIG_STRING: OnceLock<String>                 = OnceLock::new();
/// The [`Config`] to use.
static CONFIG       : OnceLock<Config>                 = OnceLock::new();
/// The max size of a payload to the [`clean`] route.
static MAX_JSON_SIZE: OnceLock<rocket::data::ByteUnit> = OnceLock::new();
/// The [`Cache`] to use.
#[cfg(feature = "cache")]
static CACHE        : OnceLock<Cache>                  = OnceLock::new();

/// Make the server.
#[launch]
async fn rocket() -> _ {
    let args = Args::parse();

    #[cfg(feature = "default-config")]
    CONFIG_STRING.set(args.config.as_deref().map(|path| read_to_string(path).expect("Reading the Config file to a string to not error.")).unwrap_or(DEFAULT_CONFIG_STR.to_string())).expect("The CONFIG_STRING global static to have not been set.");
    #[cfg(not(feature = "default-config"))]
    CONFIG_STRING.set(read_to_string(&args.config).expect("Reading the Config file to a string to not error.")).expect("The CONFIG_STRING global static to have not been set.");
    let mut config: Config = serde_json::from_str(CONFIG_STRING.get().expect("The CONFIG_STRING global static to have just been set.")).expect("The CONFIG_STRING to be a valid Config.");
    let mut params_diffs = args.params_diff
        .into_iter()
        .map(|path| serde_json::from_str(&std::fs::read_to_string(path).expect("Reading the ParamsDiff file to a string to not error.")).expect("The read ParamsDiff file to be a valid ParamsDiff."))
        .collect::<Vec<_>>();
    #[allow(unused_mut, reason = "Attributes on expressions WHEN. PLEASE.")]
    let mut feature_flag_make_params_diff = false;
    #[cfg(feature = "cache")] #[allow(clippy::unnecessary_operation, reason = "False positive.")] {feature_flag_make_params_diff = feature_flag_make_params_diff || args.read_cache.is_some()};
    #[cfg(feature = "cache")] #[allow(clippy::unnecessary_operation, reason = "False positive.")] {feature_flag_make_params_diff = feature_flag_make_params_diff || args.write_cache.is_some()};
    #[cfg(feature = "http" )] #[allow(clippy::unnecessary_operation, reason = "False positive.")] {feature_flag_make_params_diff = feature_flag_make_params_diff || args.proxy.is_some()};
    if !args.flag.is_empty() || !args.unflag.is_empty() || !args.var.is_empty() || !args.unvar.is_empty() || !args.insert_into_set.is_empty() || !args.remove_from_set.is_empty() || !args.insert_into_map.is_empty() || !args.remove_from_map.is_empty() || feature_flag_make_params_diff {
        params_diffs.push(ParamsDiff {
            flags  : args.flag  .into_iter().collect(), // `impl<X: IntoIterator, Y: FromIterator<<X as IntoIterator>::Item>> From<X> for Y`?
            unflags: args.unflag.into_iter().collect(), // It's probably not a good thing to do a global impl for,
            vars   : args.var   .into_iter().map(|x| x.try_into().expect("Clap guarantees the length is always 2")).map(|[name, value]: [String; 2]| (name, value)).collect(), // Either let me TryFrom a Vec into a tuple or let me collect a [T; 2] into a HashMap. Preferably both.
            unvars : args.unvar .into_iter().collect(), // but surely once specialization lands in Rust 2150 it'll be fine?
            init_sets: Default::default(),
            insert_into_sets: args.insert_into_set.into_iter().map(|mut x| (x.swap_remove(0), x)).collect(),
            remove_from_sets: args.remove_from_set.into_iter().map(|mut x| (x.swap_remove(0), x)).collect(),
            delete_sets     : Default::default(),
            init_maps       : Default::default(),
            insert_into_maps: args.insert_into_map.into_iter().map(|x| {
                let mut values = HashMap::new();
                let mut args_iter = x.into_iter();
                let map = args_iter.next().expect("The validation to have worked.");
                while let Some(k) = args_iter.next() {
                    values.insert(k, args_iter.next().expect("The validation to have worked."));
                }
                (map, values)
            }).collect::<HashMap<_, _>>(),
            remove_from_maps: args.remove_from_map.into_iter().map(|mut x| (x.swap_remove(0), x)).collect::<HashMap<_, _>>(),
            delete_maps     : Default::default(),
            #[cfg(feature = "cache")] read_cache : args.read_cache,
            #[cfg(feature = "cache")] write_cache: args.write_cache,
            #[cfg(feature = "http")] http_client_config_diff: Some(HttpClientConfigDiff {
                set_proxies: args.proxy.map(|x| vec![x]),
                no_proxy: args.no_proxy,
                ..HttpClientConfigDiff::default()
            })
        });
    }

    for params_diff in params_diffs {
        params_diff.apply(&mut config.params);
    }

    #[cfg(feature = "cache")]
    CACHE.set(args.cache_path.as_deref().unwrap_or(&*config.cache_path).into()).expect("The CACHE global static have not been already set.");
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
            cache: CACHE.get().expect("The CACHE global static to have been set.").clone(), // It's a newtype around an Arc, so cloning is O(1).
            job_configs_source: Box::new(bulk_job.job_configs.into_iter().map(Ok))
        }.iter().map(|job_result| match job_result {
            Ok(job) => match job.r#do() {
                Ok(url) => Ok(Ok(url)),
                Err(e) => Ok(Err(e.into()))
            },
            Err(e) => Err(e.into())
        }).collect()
    }))
}

/// The error handler for the `/clean` route.
#[catch(default)]
async fn clean_error(status: Status, _request: &Request<'_>) -> Json<Result<(), CleaningError>> {
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
