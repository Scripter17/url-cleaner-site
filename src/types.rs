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
    #[serde(alias = "urls", alias = "jobs", alias = "configs")]
    pub job_configs: Vec<JobConfig>,
    /// The [`ParamsDiff`] to use.
    #[serde(default)]
    pub params_diff: Option<ParamsDiff>
}

/// The value returned when a job fails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobError {
    /// The type of error.
    pub r#type: JobErrorType,
    /// The error message.
    pub message: String,
    /// The result of [`Debug`] formatting the error. Since URL Cleaner uses [`thiserror`] this looks like the variants of each contained error enum.
    pub variant: String
}

/// The "type" of a job error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobErrorType {
    /// The error was encountered when trying to make a [`Job`]. ([`GetJobError`]).
    GetJobError,
    /// The error eas encountered when trying to do a [`Job`]. ([`DoJobError`]).
    DoJobError
}

impl From<GetJobError> for JobError {
    fn from(value: GetJobError) -> Self {
        Self {
            r#type: JobErrorType::GetJobError,
            message: value.to_string(),
            variant: format!("{value:?}")
        }
    }
}

impl From<DoJobError> for JobError {
    fn from(value: DoJobError) -> Self {
        Self {
            r#type: JobErrorType::DoJobError,
            message: value.to_string(),
            variant: format!("{value:?}")
        }
    }
}

/// The success state of doing a [`BulkJob`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleaningSuccess {
    /// The [`Job`] results.
    pub urls: Vec<Result<Url, JobError>>
}

/// The error state of doing a [`BulkJob`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleaningError {
    /// The HTTP status code.
    pub status: u16,
    /// The HTTP status reason.
    pub reason: Option<&'static str>
}
