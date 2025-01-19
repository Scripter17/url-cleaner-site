# URL Cleaner Site

A basic but fully featured HTTP frontend and userscript for URL Cleaner.

# /!\\ THIS IS NOT HARDENED AGAINST MALICIOUS INPUT /!\\

Running URL Cleaner Site outside of localhost and without a firewall is a very bad idea.

Users can make your computer do various bad things, including but not limited to:

- Send thousands of HTTP requests to a website like bit.ly by setting `"params_diff": {"read_cache": false}`, interfering with normal usage and possibly getting you some annoying IP bans/letters from your ISP.
- Consume arbitrary CPU resources by crafting expensive to clean URLs.
- If they control a website URL Cleaner Site is configured to send HTTP requests to, deanonymize you behind a proxy/onionsite. (Assuming you're not using a proxy for HTTP requests, which you probably should be.)

By default, this is not a concern because URL Cleaner Site only binds to localhost, which doesn't allow external traffic.  
But if you want to let your other devices to use the same server, you should configure DHCP and your firewall to allow ONLY trusted devices.

In the future, defences may be implemented for some or all of the above concerns, but you should never consider the above list exhaustive or the defences infallible.

# Details

A basic HTTP server and userscript to allow automatically applying [URL Cleaner](https://github.com/Scripter17/url-cleaner) to every URL on every webpage you visit.

To understand the privacy concerns, performance, and other specifics common to both URL Cleaner and URL Cleaner Site, please check URL Cleaner's README.

## API

It binds to `127.0.0.1:9149` by default and `http://localhost:9149/clean` takes a JSON "BulkJob" (better name pending) of the following form:

```Rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BulkJob {
    #[serde(alias = "urls", alias = "jobs")]
    pub job_configs: Vec<serde_json::Value>, // Allows making JobConfigs in parallel; Accepts the same object/string representations as normal.
    #[serde(default)]
    pub params_diff: Option<ParamsDiff>
}
```

and returns a JSON response `Result<CleaningSuccess, CleaningError>` which is defined as

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
As part of this (and also as a consequence of a performance thing), if one of the jobs are invalid (for example, null), the other jobs will still work.

## TLS/HTTPS

TLS/HTTPS can be used with the `--key` and `--cert` arguments.  
[Minica](https://github.com/jsha/minica) makes it easy to have stuff shut up about self signed certificates.  
For FireFox, where this is unreasonably difficult, simply opening `https://localhost:9149`, clicking "Advanced", then "Accept the Risk and Continue" seems to work.

Please note that this requires changing `window.URL_CLEANER_SITE = "http://localhost:9149";` in the userscript to https.

Currently there is no "canon" port for HTTPS URL Cleaner Site, as 9150 is already taken by Tor. Due to the relative unexpectedness of people hosting public URL Cleaner instances, this is considered a non-issue.

## Performance

Due to the overhead of using HTTP, getting all the jobs before running them, and optionally TLS, performance is significantly worse than the CLI.

On the same laptop used in URL Cleaner's example benchmarks and with TLS, hyperfine (using CURL) gave me the following benchmarks:

```Json
{
  "https://x.com?a=2": {
    "0":      24.932,
    "1":      24.980,
    "10":     24.925,
    "100":    25.360,
    "1000":   29.714,
    "10000":  69.333
  },
  "https://example.com?fb_action_ids&mc_eid&ml_subscriber_hash&oft_ck&s_cid&unicorn_click_id": {
    "0":      25.130,
    "1":      25.107,
    "10":     25.321,
    "100":    25.621,
    "1000":   32.174,
    "10000":  87.477
  },
  "https://www.amazon.ca/UGREEN-Charger-Compact-Adapter-MacBook/dp/B0C6DX66TN/ref=sr_1_5?crid=2CNEQ7A6QR5NM&keywords=ugreen&qid=1704364659&sprefix=ugreen%2Caps%2C139&sr=8-5&ufe=app_do%3Aamzn1.fos.b06bdbbe-20fd-4ebc-88cf-fa04f1ca0da8": {
    "0":      24.871,
    "1":      24.958,
    "10":     24.929,
    "100":    26.146,
    "1000":   36.087,
    "10000": 128.855
  }
}
```

Using the (currently still experimental) parallelization feature gives about the same speedup as it does in normal URL Cleaner.  
For me it's about 40ms faster for 10k of the amazon URL.

If you're using FireFox, you should know that Greasemonkey gives much better performance of the userscript than Tampermonkey.  
Both should always work, but GM bottoms out at 10ms per request whereas TM takes 50ms per request.

As for the performance of the userscript itself... I honestly can't say. Nothing strikes me as particularly bad in terms of either CPU or memory usage, but I haven't seriously used javascript in years.  
It probably has a very slow memory leak that would be caused by a long-running webpage session having billions of elements, but that's very unlikely to ever happen outside testing.
