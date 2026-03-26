#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use image::ImageFormat;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ffi::c_void;
use std::fs;
use std::io::{Cursor, Write};
use std::iter;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{ProcessesToUpdate, System};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, FilePath};
use tauri_plugin_autostart::ManagerExt as AutostartExt;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ACCESS_DENIED, ERROR_CANCELLED, ERROR_NOT_ALL_ASSIGNED,
    HANDLE, HWND, LUID,
};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBRUSH, HGDIOBJ,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetPriorityClass, OpenProcess, OpenProcessToken, SetPriorityClass,
    SetProcessInformation, TerminateProcess, ProcessPowerThrottling,
    ABOVE_NORMAL_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS,
    IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, PROCESS_ACCESS_RIGHTS,
    PROCESS_CREATION_FLAGS, PROCESS_POWER_THROTTLING_CURRENT_VERSION,
    PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION, PROCESS_TERMINATE,
    REALTIME_PRIORITY_CLASS,
};
use windows::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
use windows::Win32::Storage::FileSystem::{ReplaceFileW, REPLACEFILE_WRITE_THROUGH};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
    TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::UI::Shell::{
    ExtractIconExW, IsUserAnAdmin, ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, DrawIconEx, MessageBoxW, DI_NORMAL, HICON, MB_ICONERROR, MB_OK, SW_SHOWNORMAL,
};

mod autostart;
mod elevation;
mod ipc_commands;
mod memory_purge;
mod process;
mod settings_repo;
mod timer;
mod watchdog;

use autostart::*;
use elevation::*;
use memory_purge::*;
use process::*;
use settings_repo::*;
use timer::*;
use watchdog::*;

const ARG_ELEVATED_SESSION: &str = "--elevated-session";
const ARG_ELEVATED_PAYLOAD: &str = "--elevated-payload";
const ARG_APPLY_CONFIG: &str = "--apply-config";
const ICON_SIZE: i32 = 48;
const ICON_CACHE_MAX_ITEMS: usize = 500;
const ICON_COLLISION_GUARD_MAX_ITEMS: usize = 4096;
const TRAY_SHOW_ID: &str = "tray_show";
const TRAY_PURGE_ID: &str = "tray_purge_memory";
const TRAY_EXIT_ID: &str = "tray_exit";
const ELEVATED_AUTOSTART_TASK_NAME: &str = "OptimusAutoStart";
const SYSTEM_MEMORY_LIST_INFORMATION_CLASS: u32 = 80;
const MEMORY_PURGE_STANDBY_LIST: u32 = 4;
const PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION: u32 = 4;

static ICON_CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtQueryTimerResolution(
        MaximumTime: *mut u32,
        MinimumTime: *mut u32,
        CurrentTime: *mut u32,
    ) -> i32;
    fn NtSetTimerResolution(DesiredTime: u32, SetResolution: u8, CurrentTime: *mut u32) -> i32;
    fn NtQuerySystemInformation(
        SystemInformationClass: u32,
        SystemInformation: *mut c_void,
        SystemInformationLength: u32,
        ReturnLength: *mut u32,
    ) -> i32;
    fn NtSetSystemInformation(
        SystemInformationClass: u32,
        SystemInformation: *mut c_void,
        SystemInformationLength: u32,
    ) -> i32;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum PriorityClassDto {
    Realtime,
    High,
    AboveNormal,
    Normal,
    BelowNormal,
    Low,
}

impl PriorityClassDto {
    fn to_windows_flag(self) -> PROCESS_CREATION_FLAGS {
        match self {
            PriorityClassDto::Realtime => REALTIME_PRIORITY_CLASS,
            PriorityClassDto::High => HIGH_PRIORITY_CLASS,
            PriorityClassDto::AboveNormal => ABOVE_NORMAL_PRIORITY_CLASS,
            PriorityClassDto::Normal => NORMAL_PRIORITY_CLASS,
            PriorityClassDto::BelowNormal => BELOW_NORMAL_PRIORITY_CLASS,
            PriorityClassDto::Low => IDLE_PRIORITY_CLASS,
        }
    }

    fn from_windows_raw(raw: u32) -> Option<Self> {
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

    fn label(self) -> &'static str {
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
struct ProcessDto {
    pid: u32,
    memory_bytes: u64,
    priority: Option<PriorityClassDto>,
    priority_raw: Option<u32>,
    priority_label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProcessGroupDto {
    app_name: String,
    icon_key: String,
    icon_base64: Option<String>,
    total: usize,
    processes: Vec<ProcessDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProcessListResponse {
    groups: Vec<ProcessGroupDto>,
    needs_elevation: bool,
    is_elevated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplyResultDto {
    pid: u32,
    success: bool,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProcessPrioritySnapshotDto {
    pid: u32,
    priority: Option<PriorityClassDto>,
    priority_raw: Option<u32>,
    priority_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigDto {
    name: String,
    config_map: HashMap<String, PriorityClassDto>,
    updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct WatchdogConfigDto {
    trigger_map: HashMap<String, String>,
    sticky_modes: HashMap<String, u8>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct WatchdogConfigOnDisk {
    #[serde(default)]
    trigger_map: HashMap<String, String>,
    #[serde(default)]
    sticky_modes: HashMap<String, u8>,
    #[serde(default)]
    sticky_configs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeSettingsDto {
    watchdog_enabled: bool,
    autostart_enabled: bool,
    autostart_mode: AutostartModeDto,
    start_as_admin_enabled: bool,
    minimize_to_tray_enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
enum AutostartModeDto {
    Off,
    User,
    Elevated,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryStatsDto {
    standby_list_mb: u64,
    free_memory_mb: u64,
    total_memory_mb: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryPurgeConfigDto {
    master_enabled: bool,
    enable_standby_trigger: bool,
    standby_limit_mb: u64,
    enable_free_memory_trigger: bool,
    free_memory_limit_mb: u64,
    total_purges: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimerResolutionDto {
    minimum_ms: f32,
    maximum_ms: f32,
    current_ms: f32,
    requested_ms: Option<f32>,
    enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ElevationStatusDto {
    status: String,
    message: String,
}

#[derive(Clone, Copy, Debug, Default)]
struct TimerResolutionState {
    requested_100ns: Option<u32>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoryPurgeConfigState {
    master_enabled: bool,
    enable_standby_trigger: bool,
    standby_limit_mb: u64,
    enable_free_memory_trigger: bool,
    free_memory_limit_mb: u64,
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
struct AppSettings {
    turbo_timer_enabled: bool,
    watchdog_enabled: bool,
    minimize_to_tray_enabled: bool,
    memory_purge_config: MemoryPurgeConfigState,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            turbo_timer_enabled: false,
            watchdog_enabled: true,
            minimize_to_tray_enabled: true,
            memory_purge_config: MemoryPurgeConfigState::default(),
        }
    }
}

#[derive(Clone, Default)]
struct RuntimeControlState {
    watchdog_enabled: Arc<AtomicBool>,
    minimize_to_tray_enabled: Arc<AtomicBool>,
    exit_requested: Arc<AtomicBool>,
    timer_resolution: Arc<Mutex<TimerResolutionState>>,
    memory_purge_config: Arc<RwLock<MemoryPurgeConfigState>>,
    memory_purge_count: Arc<AtomicU64>,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct SystemMemoryListInformation {
    zero_page_count: usize,
    free_page_count: usize,
    modified_page_count: usize,
    modified_no_write_page_count: usize,
    bad_page_count: usize,
    page_count_by_priority: [usize; 8],
    repurposed_pages_by_priority: [usize; 8],
    modified_page_count_page_file: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ElevatedActionPayload {
    action: ElevatedAction,
    priority: Option<PriorityClassDto>,
    pid: Option<u32>,
    pids: Option<Vec<u32>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum ElevatedAction {
    SetProcessPriority,
    SetGroupPriority,
    KillProcess,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
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

struct OwnedHandle(HANDLE);

impl OwnedHandle {
    fn raw(&self) -> HANDLE {
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

#[derive(Debug)]
struct PriorityRead {
    class: Option<PriorityClassDto>,
    raw: Option<u32>,
    access_denied: bool,
    label: String,
}

#[derive(Debug)]
struct GroupAccumulator {
    icon_path: Option<PathBuf>,
    processes: Vec<ProcessDto>,
}

fn icon_cache() -> &'static Mutex<HashMap<String, String>> {
    ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn last_error_code() -> u32 {
    unsafe { GetLastError().0 }
}

fn is_access_denied(code: u32) -> bool {
    code == ERROR_ACCESS_DENIED.0
}

fn is_running_as_admin() -> bool {
    unsafe { IsUserAnAdmin().as_bool() }
}


fn escape_ps_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}

fn sanitize_shortcut_name(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        let invalid = matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*');
        if invalid || ch.is_control() {
            out.push('_');
        } else {
            out.push(ch);
        }
    }
    let trimmed = out.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        "Config".to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn show_main_window(app: &tauri::AppHandle) -> Result<(), AppError> {
    let Some(window) = app.get_webview_window("main") else {
        return Err(AppError::Message("main window was not found".to_owned()));
    };

    if window
        .is_minimized()
        .map_err(|e| AppError::Message(format!("failed to read window state: {e}")))?
    {
        window
            .unminimize()
            .map_err(|e| AppError::Message(format!("failed to restore window: {e}")))?;
    }

    window
        .show()
        .map_err(|e| AppError::Message(format!("failed to show window: {e}")))?;
    window
        .set_focus()
        .map_err(|e| AppError::Message(format!("failed to focus window: {e}")))?;
    Ok(())
}

fn dialog_file_path_to_path_buf(path: FilePath) -> Result<PathBuf, AppError> {
    match path {
        FilePath::Path(path) => Ok(path),
        FilePath::Url(url) => url.to_file_path().map_err(|_| {
            AppError::Message("selected location is not a local file path".to_owned())
        }),
    }
}

fn main() {
    let context = tauri::generate_context!();
    let app_identifier = context.config().identifier.clone();

    if let Some(config_name) = parse_apply_config_arg() {
        if let Err(err) = apply_config_headless(&config_name, &app_identifier) {
            error!("[headless] {err}");
        }
        std::process::exit(0);
    }

    apply_startup_elevated_payload();

    if let Err(err) = enable_profile_privilege() {
        error!("Failed to enable SeProfileSingleProcessPrivilege: {err}");
    }
    if let Err(err) = disable_process_power_throttling() {
        error!("Failed to disable process power throttling: {err}");
    }

    let runtime_state = RuntimeControlState::default();

    let run_result = tauri::Builder::default()
        .manage(runtime_state.clone())
        .setup({
            let runtime_state = runtime_state.clone();
            move |app| {
                info!("Optimus Core Engine started");
                let settings = match load_settings(&app.handle()) {
                    Ok(settings) => settings,
                    Err(err) => {
                        warn!("Failed to load settings.json: {err}");
                        AppSettings::default()
                    }
                };
                runtime_state
                    .watchdog_enabled
                    .store(settings.watchdog_enabled, Ordering::Relaxed);
                runtime_state.minimize_to_tray_enabled.store(
                    settings.minimize_to_tray_enabled,
                    Ordering::Relaxed,
                );
                let mut memory_purge_config = settings.memory_purge_config;
                if !is_running_as_admin() && memory_purge_config.master_enabled {
                    memory_purge_config.master_enabled = false;
                    warn!("Memory Purge Engine disabled at startup (administrator privileges required)");
                }
                if let Ok(mut config) = runtime_state.memory_purge_config.write() {
                    *config = memory_purge_config;
                } else {
                    error!("Failed to seed memory purge config from settings");
                }
                if settings.turbo_timer_enabled {
                    if let Err(err) = apply_timer_resolution_request(
                        &runtime_state,
                        Some(ms_to_hundred_ns(0.5)),
                    ) {
                        error!("Failed to apply startup turbo timer setting: {err}");
                    }
                }

                let show_item =
                    MenuItem::with_id(app, TRAY_SHOW_ID, "Open Optimus", true, None::<&str>)?;
                let purge_item = MenuItem::with_id(
                    app,
                    TRAY_PURGE_ID,
                    "Purge Memory Now",
                    true,
                    None::<&str>,
                )?;
                let exit_item =
                    MenuItem::with_id(app, TRAY_EXIT_ID, "Quit", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show_item, &purge_item, &exit_item])?;

                let show_id = show_item.id().clone();
                let purge_id = purge_item.id().clone();
                let exit_id = exit_item.id().clone();
                let runtime_state_for_menu = runtime_state.clone();

                let mut tray_builder = TrayIconBuilder::with_id("optimus-tray")
                    .menu(&menu)
                    .tooltip("Optimus");

                if let Some(icon) = app.default_window_icon().cloned() {
                    tray_builder = tray_builder.icon(icon);
                }

                tray_builder
                    .on_menu_event(move |app, event| {
                        if event.id == show_id {
                            let _ = show_main_window(app);
                        } else if event.id == purge_id {
                            match run_standby_purge() {
                                Ok(()) => {
                                    runtime_state_for_menu
                                        .memory_purge_count
                                        .fetch_add(1, Ordering::Relaxed);
                                    info!("Tray action: standby list purge completed");
                                }
                                Err(err) => {
                                    error!("Tray action: standby list purge failed: {err}");
                                }
                            }
                        } else if event.id == exit_id {
                            runtime_state_for_menu
                                .exit_requested
                                .store(true, Ordering::Relaxed);
                            app.exit(0);
                        }
                    })
                    .build(app)?;

                spawn_watchdog_loop(app.handle().clone(), runtime_state.clone());
                Ok(())
            }
        })
        .on_window_event({
            let runtime_state = runtime_state.clone();
            move |window, event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    if runtime_state.exit_requested.load(Ordering::Relaxed) {
                        return;
                    }

                    if runtime_state.minimize_to_tray_enabled.load(Ordering::Relaxed) {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
        })
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            ipc_commands::list_process_groups,
            ipc_commands::get_process_list_delta,
            ipc_commands::get_process_priority,
            ipc_commands::set_process_priority,
            ipc_commands::set_group_priority,
            ipc_commands::kill_process,
            ipc_commands::restart_as_administrator,
            ipc_commands::load_configs,
            ipc_commands::save_config,
            ipc_commands::delete_config,
            ipc_commands::export_config,
            ipc_commands::import_config,
            ipc_commands::load_watchdog_config,
            ipc_commands::upsert_watchdog_mapping,
            ipc_commands::remove_watchdog_mapping,
            ipc_commands::set_config_sticky,
            ipc_commands::get_app_settings,
            ipc_commands::get_runtime_settings,
            ipc_commands::get_current_timer_res,
            ipc_commands::set_timer_res,
            ipc_commands::get_memory_stats,
            ipc_commands::get_memory_purge_config,
            ipc_commands::set_memory_purge_config,
            ipc_commands::run_purge,
            ipc_commands::toggle_watchdog,
            ipc_commands::toggle_minimize_to_tray,
            ipc_commands::configure_autostart,
            ipc_commands::toggle_autostart,
            ipc_commands::create_desktop_shortcut
        ])
        .run(context);

    release_timer_resolution(&runtime_state);

    if let Err(err) = run_result {
        let message = format!("Failed to launch Optimus.\n\n{err}");
        error!("{message}");
        show_startup_error_dialog(&message);
    }
}

