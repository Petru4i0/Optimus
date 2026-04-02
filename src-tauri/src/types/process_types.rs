use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use windows::Win32::System::Threading::{
    ABOVE_NORMAL_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS,
    IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, PROCESS_CREATION_FLAGS, REALTIME_PRIORITY_CLASS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PriorityClassDto {
    Realtime,
    High,
    AboveNormal,
    Normal,
    BelowNormal,
    Low,
}

impl PriorityClassDto {
    pub(crate) fn to_windows_flag(self) -> PROCESS_CREATION_FLAGS {
        match self {
            PriorityClassDto::Realtime => REALTIME_PRIORITY_CLASS,
            PriorityClassDto::High => HIGH_PRIORITY_CLASS,
            PriorityClassDto::AboveNormal => ABOVE_NORMAL_PRIORITY_CLASS,
            PriorityClassDto::Normal => NORMAL_PRIORITY_CLASS,
            PriorityClassDto::BelowNormal => BELOW_NORMAL_PRIORITY_CLASS,
            PriorityClassDto::Low => IDLE_PRIORITY_CLASS,
        }
    }

    pub(crate) fn from_windows_raw(raw: u32) -> Option<Self> {
        match raw {
            0x0100 => Some(PriorityClassDto::Realtime),
            0x0080 => Some(PriorityClassDto::High),
            0x8000 => Some(PriorityClassDto::AboveNormal),
            0x0020 => Some(PriorityClassDto::Normal),
            0x4000 => Some(PriorityClassDto::BelowNormal),
            0x0040 => Some(PriorityClassDto::Low),
            _ => None,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            PriorityClassDto::Realtime => "Realtime",
            PriorityClassDto::High => "High",
            PriorityClassDto::AboveNormal => "Above Normal",
            PriorityClassDto::Normal => "Normal",
            PriorityClassDto::BelowNormal => "Below Normal",
            PriorityClassDto::Low => "Low",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApplyResultDto {
    pub(crate) pid: u32,
    pub(crate) success: bool,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProcessPrioritySnapshotDto {
    pub(crate) pid: u32,
    pub(crate) priority: Option<PriorityClassDto>,
    pub(crate) priority_raw: Option<u32>,
    pub(crate) priority_label: String,
}

#[derive(Debug)]
pub(crate) struct PriorityRead {
    pub(crate) class: Option<PriorityClassDto>,
    pub(crate) raw: Option<u32>,
    pub(crate) access_denied: bool,
    pub(crate) label: String,
}

#[derive(Debug, Clone)]
pub(crate) struct SampledProcess {
    pub(crate) pid: u32,
    pub(crate) app_name: String,
    pub(crate) app_name_lower: String,
    pub(crate) exe_path: Option<PathBuf>,
    pub(crate) memory_bytes: u64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ProcessSamplerSnapshot {
    pub(crate) processes: Vec<SampledProcess>,
}
