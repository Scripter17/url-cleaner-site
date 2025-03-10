//! A basic HTTP server and userscript to allow automatically applying [URL Cleaner](https://github.com/Scripter17/url-cleaner) to every URL on every webpage you visit.

use std::net::IpAddr;
use std::path::PathBuf;
use std::fs::read_to_string;
use std::str::FromStr;
use std::borrow::Cow;
use std::sync::Mutex;
use std::num::NonZero;

#[macro_use] extern crate rocket;
use rocket::serde::json::Json;
use rocket::http::Status;
use rocket::Request;
use rocket::data::Limits;
use rocket::State;
use clap::{Parser, CommandFactory};

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
    #[arg(long, short)] pub config: Option<PathBuf>,
    /// A url_cleaner::types::Config JSON file. Has to be set because this instance of URL Cleaner Site was compiled without a default config.
    #[cfg(not(feature = "default-config"))]
    #[arg(long, short)] pub config: PathBuf,
    /// A url_cleaner::types::ParamsDiff JSON file to apply to the config by default.
    #[arg(long)] pub params_diff: Vec<PathBuf>,
    /// The max size of a POST request to the `/clean` endpoint.
    /// 
    /// The included userscript uses the `/get-max-json-size` endpoint to query this value and adjust its batch sizes accordingly.
    #[arg(long, default_value = DEFAULT_MAX_JSON_SIZE, value_parser = parse_byte_unit)] pub max_size: rocket::data::ByteUnit,
    /// The IP to listen to.
    #[arg(long, default_value = DEFAULT_BIND_IP, aliases = ["ip", "address"])] pub bind: IpAddr,
    /// The port to listen to.
    #[arg(long, default_value_t = DEFAULT_PORT)] pub port: u16,
    /// Overrides the config's [`Config::cache_path`].
    #[arg(             long)]
    #[cfg(feature = "cache")]
    pub cache_path: Option<CachePath>,
    /// Stuff to make a [`ParamsDiff`] from the CLI.
    #[command(flatten)]
    pub params_diff_args: ParamsDiffArgParser,
    /// Amount of threads to process jobs in.
    /// 
    /// Zero gets the current CPU threads.
    #[arg(long, default_value_t = 0)]
    pub threads: usize,
    /// The (optional) TLS/HTTPS cert. If specified, requires `--key`.
    #[arg(long, requires = "key")]
    pub cert: Option<PathBuf>,
    /// The (optional) TLS/HTTPS key. If specified, requires `--cert`.
    #[arg(long, requires = "cert")]
    pub key: Option<PathBuf>
}

/// The config for the server.
#[derive(Debug)]
pub struct ServerConfig<'a> {
    /// The [`JobsConfig`] to use.
    pub jobs_config: JobsConfig<'a>,
    /// The config to use as a [`String`].
    pub config_string: String,
    /// The number of threads to spawn for each [`BulkJob`].
    pub threads: NonZero<usize>,
    /// The max size for a [`BulkSize`]'s JSON representation.
    pub max_json_size: rocket::data::ByteUnit
}

/// The state of the server.
#[derive(Debug)]
pub struct ServerState<'a> {
    /// The [`ServerConfig`] to use.
    pub config: ServerConfig<'a>,
    /// The number of [`BulkJob`]s handled. Used for naming threads.
    pub bulk_jobs_count: Mutex<usize>,
}

/// Make the server.
#[launch]
async fn rocket() -> _ {
    let args = Args::parse();

    #[cfg(feature = "default-config")]
    let config_string = args.config.as_deref().map(|path| read_to_string(path).expect("The config file to be readable.")).unwrap_or(DEFAULT_CONFIG_STR.to_string());
    #[cfg(not(feature = "default-config"))]
    let config_string = read_to_string(&args.config).expect("The config file to be readable.");
    let mut config: Config = serde_json::from_str(&config_string).expect("The config file to contain a valid Config.");
    let mut params_diffs: Vec<ParamsDiff> = args.params_diff
        .into_iter()
        .map(|path| serde_json::from_str(&std::fs::read_to_string(path).expect("Reading the ParamsDiff file to a string to not error.")).expect("The read ParamsDiff file to be a valid ParamsDiff."))
        .collect::<Vec<_>>();
    if args.params_diff_args.does_anything() {
        match args.params_diff_args.try_into() {
            Ok(params_diff) => params_diffs.push(params_diff),
            Err(e) => Args::command().error(clap::error::ErrorKind::WrongNumberOfValues, e.as_str()).exit()
        }
    }

    for params_diff in params_diffs {
        params_diff.apply(&mut config.params);
    }

    let server_state = ServerState {
        config: ServerConfig {
            jobs_config: JobsConfig {
                #[cfg(feature = "cache")]
                cache: args.cache_path.as_ref().unwrap_or(&config.cache_path).clone().into(),
                config: Cow::Owned(config)
            },
            config_string,
            threads: NonZero::new(args.threads).unwrap_or_else(|| std::thread::available_parallelism().expect("To be able to get the available parallelism.")),
            max_json_size: args.max_size
        },
        bulk_jobs_count: Mutex::new(0)
    };

    rocket::custom(rocket::Config {
        address: args.bind,
        port: args.port,
        limits: Limits::default().limit("json", args.max_size),
        tls: args.cert.zip(args.key).map(|(cert, key)| rocket::config::TlsConfig::from_paths(cert, key)), // No unwraps.
        ..rocket::Config::default()
    })
        .mount("/", routes![index])
        .mount("/clean", routes![clean])
        .register("/clean", catchers![clean_error])
        .mount("/get-max-json-size", routes![get_max_json_size])
        .mount("/get-config", routes![get_config])
        .mount("/host-parts", routes![host_parts])
        .manage(server_state)
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
async fn get_config<'a>(state: &'a State<ServerState<'_>>) -> &'a str {
    &state.config.config_string
}

/// The `/clean` route.
#[post("/", data="<bulk_job>")]
async fn clean(state: &State<ServerState<'_>>, bulk_job: Json<BulkJob>) -> Json<Result<CleaningSuccess, ()>> {
    let bulk_job = bulk_job.0;
    let mut jobs_config = Cow::Borrowed(&state.config.jobs_config);
    if let Some(params_diff) = bulk_job.params_diff {
        params_diff.apply(&mut jobs_config.to_mut().config.to_mut().params);
    }
    let jobs_config_ref = &jobs_config;
    let jobs_context_ref = &bulk_job.context;

    let (in_senders , in_recievers ) = (0..state.config.threads.get()).map(|_| std::sync::mpsc::channel::<serde_json::Value>()).collect::<(Vec<_>, Vec<_>)>();
    let (out_senders, out_recievers) = (0..state.config.threads.get()).map(|_| std::sync::mpsc::channel::<Result<Result<url::Url, DoJobError>, MakeJobError>>()).collect::<(Vec<_>, Vec<_>)>();

    let ret_urls = std::sync::Mutex::new(Vec::with_capacity(bulk_job.jobs.len()));
    let ret_urls_ref = &ret_urls;

    let mut temp = state.bulk_jobs_count.lock().expect("No panics.");
    let id = *temp;
    #[allow(clippy::arithmetic_side_effects, reason = "Not gonna happen.")]
    {*temp += 1;}
    drop(temp);

    std::thread::scope(|s| {
        std::thread::Builder::new().name(format!("({id}) Job collector")).spawn_scoped(s, move || {
            for (i, job_value) in bulk_job.jobs.into_iter().enumerate() {
                #[allow(clippy::arithmetic_side_effects, reason = "`threads` is never zero, and if it is this panicking is an entirely reasonable response.")]
                in_senders.get(i % state.config.threads).expect("The amount of senders to not exceed the count of senders to make.").send(job_value).expect("To successfuly send the Job.");
            }
        }).expect("Spawning a thread to work fine.");

        in_recievers.into_iter().zip(out_senders).enumerate().map(|(i, (ir, os))| {
            std::thread::Builder::new().name(format!("({id}) Worker {i}")).spawn_scoped(s, move || {
                while let Ok(lazy_job_config) = ir.recv() {
                    os.send(serde_json::from_value::<JobConfig>(lazy_job_config)
                        .map(|job_config| jobs_config_ref.new_job(job_config, jobs_context_ref).r#do())
                        .map_err(|e| MakeJobError::MakeJobConfigError(MakeJobConfigError::SerdeJsonError(e)))
                    ).expect("The receiver to still exist.");
                }
            }).expect("Spawning a thread to work fine.");
        }).for_each(drop);

        std::thread::Builder::new().name(format!("({id}) Job returner")).spawn_scoped(s, move || {
            let mut ret_urls_handle = ret_urls_ref.lock().expect("No panics.");

            let mut disconnected = 0usize;
            for or in out_recievers.iter().cycle() {
                let recieved = or.recv();
                match recieved {
                    Ok(x) => {
                        ret_urls_handle.push(match x {
                            Ok(Ok(url)) => Ok(Ok(url)),
                            Ok(Err(e))  => Ok(Err(e.into())),
                            Err(e)      => Err(e.into())
                        })
                    }
                    Err(_) => {
                        #[allow(clippy::arithmetic_side_effects, reason = "Can't happen.")]
                        {disconnected += 1;}
                        if disconnected == state.config.threads.get() {break;}
                    }
                }
            }
        }).expect("Spawning a thread to work fine.");
    });

    Json(Ok(CleaningSuccess {
        urls: ret_urls.into_inner().expect("No panics.")
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
async fn get_max_json_size(state: &State<ServerState<'_>>) -> String {
    state.config.max_json_size.as_u64().to_string()
}

/// The `host-parts` route.
#[post("/", data="<host>")]
async fn host_parts(host: &str) -> Json<Result<HostParts<'_>, CouldntParseHost>> {
    Json(HostParts::try_from(host).map_err(|_| CouldntParseHost))
}
