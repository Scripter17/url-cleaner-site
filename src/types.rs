//! The types used to make API responses.

use serde::{Serialize, Deserialize};
use url::Url;

use url_cleaner::types::*;

/// The payload of the `/clean` route.
/// 
/// Used to construct a [`Jobs`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BulkJob {
    /// The [`JobConfig`]s to use.
    #[serde(alias = "urls", alias = "jobs")]
    pub job_configs: Vec<serde_json::Value>,
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
