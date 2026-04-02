use crate::types::process_types::{PriorityClassDto, ProcessSamplerSnapshot};
use crate::types::{OptimizationDesiredState, ProcessDeltaState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::watch;
use windows::Win32::Foundation::{CloseHandle, HANDLE};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigDto {
    pub(crate) name: String,
    pub(crate) config_map: HashMap<String, PriorityClassDto>,
    pub(crate) updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WatchdogConfigDto {
    pub(crate) trigger_map: HashMap<String, WatchdogTriggerMappingDto>,
    pub(crate) sticky_modes: HashMap<String, u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WatchdogTriggerMappingDto {
    pub(crate) config_name: String,
    #[serde(default)]
    pub(crate) icon: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum WatchdogTriggerMappingOnDisk {
    LegacyConfigName(String),
    Mapping(WatchdogTriggerMappingDto),
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WatchdogConfigOnDisk {
    #[serde(default)]
    pub(crate) trigger_map: HashMap<String, WatchdogTriggerMappingOnDisk>,
    #[serde(default)]
    pub(crate) sticky_modes: HashMap<String, u8>,
    #[serde(default)]
    pub(crate) sticky_configs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeSettingsDto {
    pub(crate) watchdog_enabled: bool,
    pub(crate) autostart_enabled: bool,
    pub(crate) autostart_mode: AutostartModeDto,
    pub(crate) start_as_admin_enabled: bool,
    pub(crate) minimize_to_tray_enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AutostartModeDto {
    Off,
    Elevated,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryStatsDto {
    pub(crate) standby_list_mb: u64,
    pub(crate) free_memory_mb: u64,
    pub(crate) total_memory_mb: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryPurgeConfigDto {
    pub(crate) master_enabled: bool,
    pub(crate) enable_standby_trigger: bool,
    pub(crate) standby_limit_mb: u64,
    pub(crate) enable_free_memory_trigger: bool,
    pub(crate) free_memory_limit_mb: u64,
    pub(crate) total_purges: u64,
    pub(crate) total_cleared_mb: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TimerResolutionDto {
    pub(crate) minimum_ms: f32,
    pub(crate) maximum_ms: f32,
    pub(crate) current_ms: f32,
    pub(crate) requested_ms: Option<f32>,
    pub(crate) enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ElevationStatusDto {
    pub(crate) status: String,
    pub(crate) message: String,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TimerResolutionState {
    pub(crate) requested_100ns: Option<u32>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryPurgeConfigState {
    pub(crate) master_enabled: bool,
    pub(crate) enable_standby_trigger: bool,
    pub(crate) standby_limit_mb: u64,
    pub(crate) enable_free_memory_trigger: bool,
    pub(crate) free_memory_limit_mb: u64,
}

impl Default for MemoryPurgeConfigState {
    fn default() -> Self {
        Self {
            master_enabled: false,
            enable_standby_trigger: false,
            standby_limit_mb: 1024,
            enable_free_memory_trigger: false,
            free_memory_limit_mb: 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppSettings {
    pub(crate) turbo_timer_enabled: bool,
    pub(crate) watchdog_enabled: bool,
    pub(crate) minimize_to_tray_enabled: bool,
    pub(crate) memory_purge_config: MemoryPurgeConfigState,
    #[serde(default)]
    pub(crate) optimization_desired: OptimizationDesiredState,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            turbo_timer_enabled: false,
            watchdog_enabled: true,
            minimize_to_tray_enabled: true,
            memory_purge_config: MemoryPurgeConfigState::default(),
            optimization_desired: OptimizationDesiredState::default(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct RuntimeControlState {
    pub(crate) watchdog_enabled: Arc<AtomicBool>,
    pub(crate) minimize_to_tray_enabled: Arc<AtomicBool>,
    pub(crate) exit_requested: Arc<AtomicBool>,
    pub(crate) timer_resolution: Arc<Mutex<TimerResolutionState>>,
    pub(crate) memory_purge_config: Arc<RwLock<MemoryPurgeConfigState>>,
    pub(crate) memory_purge_count: Arc<AtomicU64>,
    pub(crate) memory_purge_cleared_mb: Arc<AtomicU64>,
    pub(crate) process_sampler_snapshot: Arc<RwLock<ProcessSamplerSnapshot>>,
    pub(crate) process_delta_state: Arc<Mutex<ProcessDeltaState>>,
    pub(crate) optimization_desired: Arc<RwLock<OptimizationDesiredState>>,
    pub(crate) shutdown_tx: Arc<watch::Sender<bool>>,
}

impl Default for RuntimeControlState {
    fn default() -> Self {
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);
        Self {
            watchdog_enabled: Arc::new(AtomicBool::new(true)),
            minimize_to_tray_enabled: Arc::new(AtomicBool::new(true)),
            exit_requested: Arc::new(AtomicBool::new(false)),
            timer_resolution: Arc::new(Mutex::new(TimerResolutionState::default())),
            memory_purge_config: Arc::new(RwLock::new(MemoryPurgeConfigState::default())),
            memory_purge_count: Arc::new(AtomicU64::new(0)),
            memory_purge_cleared_mb: Arc::new(AtomicU64::new(0)),
            process_sampler_snapshot: Arc::new(RwLock::new(ProcessSamplerSnapshot::default())),
            process_delta_state: Arc::new(Mutex::new(ProcessDeltaState::default())),
            optimization_desired: Arc::new(RwLock::new(OptimizationDesiredState::default())),
            shutdown_tx: Arc::new(shutdown_tx),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(crate) struct SystemMemoryListInformation {
    pub(crate) zero_page_count: usize,
    pub(crate) free_page_count: usize,
    pub(crate) modified_page_count: usize,
    pub(crate) modified_no_write_page_count: usize,
    pub(crate) bad_page_count: usize,
    pub(crate) page_count_by_priority: [usize; 8],
    pub(crate) repurposed_pages_by_priority: [usize; 8],
    pub(crate) modified_page_count_page_file: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ElevatedActionPayload {
    pub(crate) action: ElevatedAction,
    pub(crate) priority: Option<PriorityClassDto>,
    pub(crate) pid: Option<u32>,
    pub(crate) pids: Option<Vec<u32>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ElevatedAction {
    SetProcessPriority,
    SetGroupPriority,
    KillProcess,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum AppError {
    #[error("Access denied while {context} for PID {pid}")]
    AccessDenied { pid: u32, context: &'static str },
    #[error("Windows API failed while {context} for PID {pid} (code {code})")]
    WinApi {
        pid: u32,
        context: &'static str,
        code: u32,
    },
    #[error("{0}")]
    Message(String),
}

pub(crate) struct OwnedHandle(pub(crate) HANDLE);

impl OwnedHandle {
    pub(crate) fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
