#[macro_use] extern crate rocket;
use rocket::serde::json::Json;
use rocket::http::Header;
use rocket::{Request, Response};
use rocket::fairing::{Fairing, Info, Kind};
use url::Url;
use serde::{Serialize, Deserialize};
use std::net::{IpAddr, Ipv4Addr};

#[launch]
fn rocket() -> _ {
    rocket::custom(rocket::Config {
        port: 9149,
        address: IpAddr::V4(Ipv4Addr::new(0,0,0,0)),
        ..rocket::Config::default()
    }).mount("/clean", routes![clean])
        .attach(Anarcors)
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
    Json(JobResponse {
        urls: job.urls.into_iter()
            .map(|mut url| {url_cleaner::clean_url(&mut url, None, job.params_diff.as_ref()).map_err(|e| e.to_string())?; Ok(url)}).collect()
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
