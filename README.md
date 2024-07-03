# URL Cleaner Site

# /!\\ THIS IS NOT HARDENED AGAINST MALICIOUS INPUT /!\\

A user could send a job of the form `{"urls": ["https://bit.ly/abcdef", ... x1000], "params_diff": {"read_cache": false}}` to cause a very long running job.

This particular example also has the side effect of possibly having `bit.ly` (or any other shortlink site) block the IP URL Cleaner Site is running on.

In the future, there will be a way to block/limit the `params_diff` field but, as its existence is extremely useful, it will not be removed by default.

## Details

A very basic HTTP server and userscript to allow automatically applying [URL Cleaner](https://github.com/Scripter17/url-cleaner) to every URL on every webpage you visit.

It binds to `0.0.0.0:9149` by default and `http://localhost:9149/clean` takes a simple job of the following form

```Rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Job {
    urls: Vec<String>,
    #[serde(default)]
    params_diff: Option<url_cleaner::types::ParamsDiff>
}
```

and returns a response of the following form

```Rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobResponse {
    urls: Vec<Result<Url, JobError>>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobError {
    r#type: String,
    source_url: String,
    error: String
}
```

It is intended to be byte-for-byte identical to the equivalent invocation of URL Cleaner in JSON mode.
