# URL Cleaner Site

A very basic HTTP server and userscript to allow automatically applying [URL Cleaner](https://github.com/Scripter17/url-cleaner) to every URL on every webpage you visit.

It binds to `0.0.0.0:9149` and `http://localhost:9149/clean` takes a simple job of the following form

```Rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Job {
    urls: Vec<Url>,
    #[serde(default)]
    params_diff: Option<url_cleaner::types::ParamsDiff>
}
```

and returns a response of the following form

```Rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobResponse {
    urls: Vec<Result<Url, String>>
}
```

The semantics of which I think are pretty clear.
