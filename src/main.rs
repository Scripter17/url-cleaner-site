#[macro_use] extern crate rocket;
use rocket::serde::json::Json;
use rocket::http::Header;
use rocket::{Request, Response};
use rocket::fairing::{Fairing, Info, Kind};
use url::Url;
use serde::{Serialize, Deserialize};

#[launch]
fn rocket() -> _ {
    rocket::custom(rocket::Config {
        port: 9149,
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
    urls: Vec<Url>
}

#[post("/", data="<job>")]
fn clean(job: Json<Job>) -> Result<Json<JobResponse>, String> {
    let mut job = job.0;
    for url in job.urls.iter_mut() {
        url_cleaner::clean_url(url, None, job.params_diff.as_ref()).map_err(|e| e.to_string())?;
    }
    Ok(Json(JobResponse {urls: job.urls}))
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
