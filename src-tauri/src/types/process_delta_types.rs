use crate::types::PriorityClassDto;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProcessRowDto {
    pub(crate) pid: u32,
    pub(crate) app_name: String,
    pub(crate) icon_key: String,
    pub(crate) memory_bytes: u64,
    pub(crate) priority: Option<PriorityClassDto>,
    pub(crate) priority_raw: Option<u32>,
    pub(crate) priority_label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProcessDeltaPayload {
    pub(crate) sequence: u64,
    pub(crate) added: Vec<ProcessRowDto>,
    pub(crate) updated: Vec<ProcessRowDto>,
    pub(crate) removed: Vec<u32>,
    pub(crate) needs_elevation: bool,
    pub(crate) is_elevated: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ProcessDeltaState {
    pub(crate) last_rows: HashMap<u32, ProcessRowDto>,
    pub(crate) icon_sources: HashMap<String, PathBuf>,
    pub(crate) sequence: u64,
}

