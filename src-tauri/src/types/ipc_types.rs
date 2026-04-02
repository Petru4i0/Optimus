use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppErrorEnvelope {
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) requires_admin: bool,
    pub(crate) retryable: bool,
}

