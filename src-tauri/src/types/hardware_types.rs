use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum MsiPriorityDto {
    Undefined,
    Low,
    Normal,
    High,
}

impl MsiPriorityDto {
    pub(crate) fn from_registry(raw: u32) -> Self {
        match raw {
            1 => MsiPriorityDto::Low,
            2 => MsiPriorityDto::Normal,
            3 => MsiPriorityDto::High,
            _ => MsiPriorityDto::Undefined,
        }
    }

    pub(crate) fn to_registry(self) -> u32 {
        match self {
            MsiPriorityDto::Undefined => 0,
            MsiPriorityDto::Low => 1,
            MsiPriorityDto::Normal => 2,
            MsiPriorityDto::High => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PciDeviceDto {
    pub(crate) device_id: String,
    pub(crate) display_name: String,
    pub(crate) readable: bool,
    pub(crate) msi_supported: bool,
    pub(crate) msi_enabled: bool,
    pub(crate) priority: MsiPriorityDto,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DriverDto {
    pub(crate) published_name: String,
    pub(crate) original_name: String,
    pub(crate) provider_name: String,
    pub(crate) class_name: String,
    pub(crate) driver_version: String,
    pub(crate) driver_date: String,
    pub(crate) safety_level: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GhostDeviceDto {
    pub(crate) instance_id: String,
    pub(crate) device_description: String,
    pub(crate) class_name: String,
    pub(crate) safety_level: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MsiApplyDto {
    pub(crate) device_id: String,
    pub(crate) enable: bool,
    pub(crate) priority: MsiPriorityDto,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MsiBatchReportDto {
    pub(crate) total: i32,
    pub(crate) successful: i32,
    pub(crate) failed: i32,
    pub(crate) errors: Vec<String>,
}

