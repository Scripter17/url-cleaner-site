# URL Cleaner Site

# /!\\ THIS IS NOT HARDENED AGAINST MALICIOUS INPUT /!\\

A user could send a job of the form `{"urls": ["https://bit.ly/abcdef", ... x1000], "params_diff": {"read_cache": false}}` to cause a very long running job.

This particular example also has the side effect of possibly having `bit.ly` (or any other shortlink site) block the IP URL Cleaner Site is running on.

In the future, there will be a way to block/limit the `params_diff` field but, as its existence is extremely useful, it will not be removed by default.

# Performance

In practice, URL Cleaner Site and its userscript are far slower than "just" using URL Cleaner.

This is both because each HTTP request, at least on my machine, takes 10ms regardless of content or work, and because the initial cleaning happens at a time where a LOT is happeneing.

The initial clean often takes up to a full second and subsequent cleans can take up to 50ms.

It's worth noting that, for me, Greasemonkey gives far better times than Tampermonkey. Like 50ms per subsequent cleans better. Tampermonkey should still work but if it's too slow for you you may want to test Greasemonkey.

## Details

A basic HTTP server and userscript to allow automatically applying [URL Cleaner](https://github.com/Scripter17/url-cleaner) to every URL on every webpage you visit.

It binds to `127.0.0.1:9149` by default and `http://localhost:9149/clean` takes a simple job of the following form:

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
pub struct CleaningSuccess {
    pub urls: Vec<Result<Result<Url, StringDoJobError>, StringMakeJobError>>
}

pub struct StringMakeJobError {
    pub message: String,
    pub variant: String
}

pub struct StringDoJobError {
    pub message: String,
    pub variant: String
}

pub struct CleaningError {
    pub status: u16,
    pub reason: Option<&'static str>
}
```

It is intended to be byte-for-byte identical to the equivalent invocation of URL Cleaner in JSON mode.  
This way if one wants to transition from URL Cleaner to URL Cleaner Site (or vice versa) there's very little code to change.
