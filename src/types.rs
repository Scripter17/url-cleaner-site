//! The types used to make API responses.

use serde::{Serialize, Deserialize, ser::Serializer};
use url::Url;
use thiserror::Error;

use url_cleaner::types::*;

/// The payload of the `/clean` route.
/// 
/// Used to construct a [`Jobs`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BulkJob {
    /// The [`JobConfig`]s to use.
    pub jobs: Vec<serde_json::Value>,
    /// The [`JobsContext`] to use.
    #[serde(default)]
    pub context: JobsContext,
    /// The [`ParamsDiff`] to use.
    #[serde(default)]
    pub params_diff: Option<ParamsDiff>
}

/// A [`Serialize`]able version of [`MakeJobError`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StringMakeJobError {
    /// The error message.
    pub message: String,
    /// The result of [`Debug`] formatting the error. Since URL Cleaner uses [`thiserror`] this looks like the variants of each contained error enum.
    pub variant: String
}

impl From<MakeJobError> for StringMakeJobError {
    fn from(value: MakeJobError) -> Self {
        Self {
            message: value.to_string(),
            variant: format!("{value:?}")
        }
    }
}

/// A [`Serialize`]able version of [`DoJobError`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StringDoJobError {
    /// The error message.
    pub message: String,
    /// The result of [`Debug`] formatting the error. Since URL Cleaner uses [`thiserror`] this looks like the variants of each contained error enum.
    pub variant: String
}

impl From<DoJobError> for StringDoJobError {
    fn from(value: DoJobError) -> Self {
        Self {
            message: value.to_string(),
            variant: format!("{value:?}")
        }
    }
}

/// The success state of doing a [`BulkJob`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleaningSuccess {
    /// The [`Job`] results.
    pub urls: Vec<Result<Result<Url, StringDoJobError>, StringMakeJobError>>
}

/// The error state of doing a [`BulkJob`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleaningError {
    /// The HTTP status code.
    pub status: u16,
    /// The HTTP status reason.
    pub reason: Option<&'static str>
}

/// The error returned by the `host-parts` route when given an invalid host.
#[derive(Debug, Error)]
#[error("Couldn't parse host")]
pub struct CouldntParseHost;

impl Serialize for CouldntParseHost {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("Couldn't parse the host.")
    }
}

/// Various parts of a host.
#[derive(Debug, Serialize)]
pub enum HostParts<'a> {
    /// Various parts of a domain.
    Domain(DomainParts<'a>),
    /// Various parts of an IPv4 host.
    Ipv4(Ipv4Parts<'a>),
    /// Various parts of an IPv6 host.
    Ipv6(Ipv6Parts<'a>)
}

impl<'a> TryFrom<&'a str> for HostParts<'a> {
    type Error = url::ParseError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Ok(match HostDetails::from_host_str(value)? {
            HostDetails::Domain(dd) => Self::Domain(DomainParts {
                whole: value,
                subdomain : dd.subdomain_bounds ().and_then(|x| value.get(x)),
                not_suffix: dd.not_suffix_bounds().and_then(|x| value.get(x)),
                middle    : dd.middle_bounds    ().and_then(|x| value.get(x)),
                reg_domain: dd.reg_domain_bounds().and_then(|x| value.get(x)),
                suffix    : dd.suffix_bounds    ().and_then(|x| value.get(x))
            }),
            HostDetails::Ipv4(_) => Self::Ipv4(Ipv4Parts {whole: value}),
            HostDetails::Ipv6(_) => Self::Ipv6(Ipv6Parts {whole: value})
        })
    }
}

/// Various parts of a domain.
#[derive(Debug, Serialize)]
pub struct DomainParts<'a> {
    /// The entire domain.
    pub whole     : &'a str,
    /// The [`UrlPart::Subdomain`].
    pub subdomain : Option<&'a str>,
    /// The [`UrlPart::NotSuffix`].
    pub not_suffix: Option<&'a str>,
    /// The [`UrlPart::Middle`].
    pub middle    : Option<&'a str>,
    /// The [`UrlPart::RegDomain`].
    pub reg_domain: Option<&'a str>,
    /// The [`UrlPart::Suffix`].
    pub suffix    : Option<&'a str>
}

/// Various parts of an IPv4 host.
#[derive(Debug, Serialize)]
pub struct Ipv4Parts<'a> {
    /// The whole IPv4 host.
    pub whole: &'a str
}

/// Various parts of an IPv6 host.
#[derive(Debug, Serialize)]
pub struct Ipv6Parts<'a> {
    /// The whole IPv6 host.
    pub whole: &'a str
}
