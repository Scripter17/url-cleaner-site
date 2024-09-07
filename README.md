# URL Cleaner Site

# /!\\ THIS IS NOT HARDENED AGAINST MALICIOUS INPUT /!\\

A user could send a job of the form `{"urls": ["https://bit.ly/abcdef", ... x1000], "params_diff": {"read_cache": false}}` to cause a very long running job.

This particular example also has the side effect of possibly having `bit.ly` (or any other shortlink site) block the IP URL Cleaner Site is running on.

In the future, there will be a way to block/limit the `params_diff` field but, as its existence is extremely useful, it will not be removed by default.

## Details

A very basic HTTP server and userscript to allow automatically applying [URL Cleaner](https://github.com/Scripter17/url-cleaner) to every URL on every webpage you visit.

It binds to `0.0.0.0:9149` by default and `http://localhost:9149/clean` takes a simple job of the following form

```Rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BulkJob {
    #[serde(alias = "urls", alias = "configs")]
    pub job_configs: Vec<JobConfig>,
    #[serde(default)]
    pub params_diff: Option<ParamsDiff>
}
```

and returns a response `Result<CleaningSuccess, CleaningError>` which is defined as

```Rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleaningSuccess {
    pub urls: Vec<Result<Url, JobError>>
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobError {
    pub r#type: JobErrorType,
    pub message: String,
    pub variant: String
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobErrorType {
    GetJobError,
    DoJobError
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleaningError {
    status: u16,
    reason: Option<&'static str>
}
```

It is intended to be byte-for-byte identical to the equivalent invocation of URL Cleaner in JSON mode.
