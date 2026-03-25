#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ffi::c_void;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{Cursor, Write};
use std::iter;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
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
    CloseHandle, GetLastError, ERROR_ACCESS_DENIED, HANDLE, HINSTANCE, HWND,
};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBRUSH, HGDIOBJ,
};
use windows::Win32::System::Threading::{
    GetPriorityClass, OpenProcess, SetPriorityClass, TerminateProcess, ABOVE_NORMAL_PRIORITY_CLASS,
    BELOW_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS, IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS,
    PROCESS_ACCESS_RIGHTS, PROCESS_CREATION_FLAGS, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_SET_INFORMATION, PROCESS_TERMINATE, REALTIME_PRIORITY_CLASS,
};
use windows::Win32::UI::Shell::{ExtractIconExW, IsUserAnAdmin, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, DrawIconEx, MessageBoxW, DI_NORMAL, HICON, MB_ICONERROR, MB_OK, SW_SHOWNORMAL,
};

const ARG_ELEVATED_SESSION: &str = "--elevated-session";
const ARG_ELEVATED_PAYLOAD: &str = "--elevated-payload";
const ARG_APPLY_CONFIG: &str = "--apply-config";
const ICON_SIZE: i32 = 48;
const APP_IDENTIFIER: &str = "com.petro.optimus";
const TRAY_SHOW_ID: &str = "tray_show";
const TRAY_EXIT_ID: &str = "tray_exit";

static ICON_CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
static CONFIG_IO_LOCK: OnceLock<RwLock<()>> = OnceLock::new();

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
    minimize_to_tray_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ElevationStatusDto {
    status: String,
    message: String,
}

#[derive(Clone, Default)]
struct RuntimeControlState {
    watchdog_enabled: Arc<AtomicBool>,
    minimize_to_tray_enabled: Arc<AtomicBool>,
    exit_requested: Arc<AtomicBool>,
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

fn config_io_lock() -> &'static RwLock<()> {
    CONFIG_IO_LOCK.get_or_init(|| RwLock::new(()))
}

fn atomic_write_json(path: &Path, data: &str) -> Result<(), AppError> {
    let file_name = path
        .file_name()
        .ok_or_else(|| AppError::Message(format!("invalid target path '{}'", path.display())))?;
    let mut tmp_name = file_name.to_os_string();
    tmp_name.push(".tmp");
    let tmp_path = path.with_file_name(tmp_name);

    let write_result = (|| -> Result<(), AppError> {
        let mut tmp_file = fs::File::create(&tmp_path).map_err(|e| {
            AppError::Message(format!(
                "failed to create temp file '{}': {e}",
                tmp_path.display()
            ))
        })?;
        tmp_file.write_all(data.as_bytes()).map_err(|e| {
            AppError::Message(format!(
                "failed to write temp file '{}': {e}",
                tmp_path.display()
            ))
        })?;
        tmp_file.sync_all().map_err(|e| {
            AppError::Message(format!(
                "failed to sync temp file '{}': {e}",
                tmp_path.display()
            ))
        })?;
        fs::rename(&tmp_path, path).map_err(|e| {
            AppError::Message(format!(
                "failed to atomically replace '{}' with '{}': {e}",
                path.display(),
                tmp_path.display()
            ))
        })?;
        Ok(())
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }

    write_result
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

fn open_process(
    pid: u32,
    access: PROCESS_ACCESS_RIGHTS,
    context: &'static str,
) -> Result<OwnedHandle, AppError> {
    unsafe {
        match OpenProcess(access, false, pid) {
            Ok(handle) => Ok(OwnedHandle(handle)),
            Err(_) => {
                let code = last_error_code();
                if is_access_denied(code) {
                    Err(AppError::AccessDenied { pid, context })
                } else {
                    Err(AppError::WinApi { pid, context, code })
                }
            }
        }
    }
}

fn read_priority(pid: u32) -> PriorityRead {
    let handle = match open_process(
        pid,
        PROCESS_QUERY_LIMITED_INFORMATION,
        "opening process for read",
    ) {
        Ok(handle) => handle,
        Err(AppError::AccessDenied { .. }) => {
            return PriorityRead {
                class: None,
                raw: None,
                access_denied: true,
                label: "Access denied".to_owned(),
            };
        }
        Err(err) => {
            return PriorityRead {
                class: None,
                raw: None,
                access_denied: false,
                label: err.to_string(),
            };
        }
    };

    let raw = unsafe { GetPriorityClass(handle.raw()) };
    if raw == 0 {
        let code = last_error_code();
        if is_access_denied(code) {
            return PriorityRead {
                class: None,
                raw: None,
                access_denied: true,
                label: "Access denied".to_owned(),
            };
        }

        return PriorityRead {
            class: None,
            raw: None,
            access_denied: false,
            label: format!("GetPriorityClass failed ({code})"),
        };
    }

    let class = PriorityClassDto::from_windows_raw(raw);
    PriorityRead {
        class,
        raw: Some(raw),
        access_denied: false,
        label: class
            .map(PriorityClassDto::label)
            .unwrap_or("Unknown")
            .to_owned(),
    }
}

fn set_priority_for_pid(pid: u32, priority: PriorityClassDto) -> Result<(), AppError> {
    let access = PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION;
    let handle = open_process(pid, access, "opening process for write")?;

    unsafe {
        match SetPriorityClass(handle.raw(), priority.to_windows_flag()) {
            Ok(()) => Ok(()),
            Err(_) => {
                let code = last_error_code();
                if is_access_denied(code) {
                    Err(AppError::AccessDenied {
                        pid,
                        context: "setting priority",
                    })
                } else {
                    Err(AppError::WinApi {
                        pid,
                        context: "setting priority",
                        code,
                    })
                }
            }
        }
    }
}

fn kill_process_by_pid(pid: u32) -> Result<(), AppError> {
    let access = PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION;
    let handle = open_process(pid, access, "opening process for terminate")?;

    unsafe {
        match TerminateProcess(handle.raw(), 1) {
            Ok(()) => Ok(()),
            Err(_) => {
                let code = last_error_code();
                if is_access_denied(code) {
                    Err(AppError::AccessDenied {
                        pid,
                        context: "terminating process",
                    })
                } else {
                    Err(AppError::WinApi {
                        pid,
                        context: "terminating process",
                        code,
                    })
                }
            }
        }
    }
}

fn gather_process_groups() -> ProcessListResponse {
    gather_process_groups_with_known_icons(None)
}

fn icon_identity(app_name: &str, icon_path: Option<&Path>) -> String {
    match icon_path {
        Some(path) => format!("exe:{}", path.to_string_lossy().to_lowercase()),
        None => format!("app:{}", app_name.trim().to_lowercase()),
    }
}

fn icon_key_from_identity(identity: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    identity.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn gather_process_groups_with_known_icons(
    known_icon_keys: Option<&HashSet<String>>,
) -> ProcessListResponse {
    let mut system = System::new_all();
    system.refresh_processes(ProcessesToUpdate::All, true);

    let mut grouped: BTreeMap<String, GroupAccumulator> = BTreeMap::new();
    let mut needs_elevation = false;

    for (pid, process) in system.processes() {
        let exe_path = process.exe().map(PathBuf::from);
        let app_name = exe_path
            .as_deref()
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| process.name().to_string_lossy().into_owned());

        let priority = read_priority(pid.as_u32());
        if priority.access_denied {
            needs_elevation = true;
        }

        let entry = grouped.entry(app_name).or_insert_with(|| GroupAccumulator {
            icon_path: None,
            processes: Vec::new(),
        });

        if entry.icon_path.is_none() {
            entry.icon_path = exe_path;
        }

        entry.processes.push(ProcessDto {
            pid: pid.as_u32(),
            memory_bytes: process.memory(),
            priority: priority.class,
            priority_raw: priority.raw,
            priority_label: priority.label,
        });
    }

    let mut groups = Vec::with_capacity(grouped.len());
    for (app_name, mut group) in grouped {
        group.processes.sort_by_key(|proc| proc.pid);
        let icon_identity = icon_identity(&app_name, group.icon_path.as_deref());
        let icon_key = icon_key_from_identity(&icon_identity);
        let should_include_icon = known_icon_keys
            .map(|keys| !keys.contains(&icon_key))
            .unwrap_or(true);
        let icon_base64 = if should_include_icon {
            group
                .icon_path
                .as_deref()
                .and_then(|path| extract_icon_base64(path).ok())
        } else {
            None
        };

        groups.push(ProcessGroupDto {
            total: group.processes.len(),
            app_name,
            icon_key,
            icon_base64,
            processes: group.processes,
        });
    }

    ProcessListResponse {
        groups,
        needs_elevation,
        is_elevated: is_running_as_admin(),
    }
}

fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn config_storage_dir(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::Message(format!("failed to resolve app config dir: {e}")))?;
    let storage_dir = config_dir.join("configs");
    fs::create_dir_all(&storage_dir)
        .map_err(|e| AppError::Message(format!("failed to create config dir: {e}")))?;
    Ok(storage_dir)
}

fn configs_file_path(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    Ok(config_storage_dir(app)?.join("configs.json"))
}

fn watchdog_file_path(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    Ok(config_storage_dir(app)?.join("config_watchdog.json"))
}

fn read_configs_from_disk(app: &tauri::AppHandle) -> Result<Vec<ConfigDto>, AppError> {
    let _guard = config_io_lock()
        .read()
        .map_err(|_| AppError::Message("config I/O lock poisoned".to_owned()))?;
    let file_path = configs_file_path(app)?;
    read_configs_from_path(&file_path)
}

fn read_configs_from_path(file_path: &Path) -> Result<Vec<ConfigDto>, AppError> {
    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&file_path)
        .map_err(|e| AppError::Message(format!("failed to read configs: {e}")))?;

    serde_json::from_str::<Vec<ConfigDto>>(&content)
        .map_err(|e| AppError::Message(format!("failed to parse configs: {e}")))
}

fn read_watchdog_config_from_disk(app: &tauri::AppHandle) -> Result<WatchdogConfigDto, AppError> {
    let _guard = config_io_lock()
        .read()
        .map_err(|_| AppError::Message("config I/O lock poisoned".to_owned()))?;
    let file_path = watchdog_file_path(app)?;
    if !file_path.exists() {
        return Ok(WatchdogConfigDto::default());
    }

    let content = fs::read_to_string(&file_path)
        .map_err(|e| AppError::Message(format!("failed to read watchdog config: {e}")))?;

    let parsed = parse_watchdog_config(&content)
        .map_err(|e| AppError::Message(format!("failed to parse watchdog config: {e}")))?;
    let (normalized, _) = normalize_watchdog_config(parsed);
    Ok(normalized)
}

fn write_watchdog_config_to_disk(
    app: &tauri::AppHandle,
    config: &WatchdogConfigDto,
) -> Result<(), AppError> {
    let _guard = config_io_lock()
        .write()
        .map_err(|_| AppError::Message("config I/O lock poisoned".to_owned()))?;
    let file_path = watchdog_file_path(app)?;
    let data = serde_json::to_string_pretty(config)
        .map_err(|e| AppError::Message(format!("failed to serialize watchdog config: {e}")))?;
    atomic_write_json(&file_path, &data)
}

fn normalize_watchdog_config(config: WatchdogConfigDto) -> (WatchdogConfigDto, bool) {
    let original = config.clone();
    let mut trigger_entries: Vec<(String, String)> = config.trigger_map.into_iter().collect();
    trigger_entries.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    let mut trigger_map = HashMap::new();
    for (key, config_name) in trigger_entries {
        let normalized_key = key.trim().to_lowercase();
        let normalized_config_name = config_name.trim().to_owned();
        if normalized_key.is_empty() || normalized_config_name.is_empty() {
            continue;
        }
        // Deterministic last-write-wins for collisions after normalization.
        trigger_map.insert(normalized_key, normalized_config_name);
    }

    let mut sticky_modes = HashMap::new();
    let mut sticky_mode_entries: Vec<(String, u8)> = config.sticky_modes.into_iter().collect();
    sticky_mode_entries.sort_by(|a, b| a.0.cmp(&b.0));

    for (config_name, mode) in sticky_mode_entries {
        let normalized_name = config_name.trim().to_owned();
        if normalized_name.is_empty() {
            continue;
        }
        let normalized_mode = match mode {
            1 | 2 => mode,
            _ => 0,
        };
        sticky_modes.insert(normalized_name.to_lowercase(), normalized_mode);
    }

    let normalized = WatchdogConfigDto {
        trigger_map,
        sticky_modes,
    };
    let changed = normalized.trigger_map != original.trigger_map
        || normalized.sticky_modes != original.sticky_modes;
    (normalized, changed)
}

fn parse_watchdog_config(content: &str) -> Result<WatchdogConfigDto, serde_json::Error> {
    let on_disk: WatchdogConfigOnDisk = serde_json::from_str(content)?;
    let mut sticky_modes = on_disk.sticky_modes;

    // Legacy migration: sticky_configs -> sticky_modes(mode=1).
    for config_name in on_disk.sticky_configs {
        let normalized_name = config_name.trim().to_lowercase();
        if normalized_name.is_empty() {
            continue;
        }
        sticky_modes.entry(normalized_name).or_insert(1);
    }

    Ok(WatchdogConfigDto {
        trigger_map: on_disk.trigger_map,
        sticky_modes,
    })
}

fn headless_configs_file_path() -> Result<PathBuf, AppError> {
    let base_dir = std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("LOCALAPPDATA"))
        .ok_or_else(|| AppError::Message("failed to resolve APPDATA/LOCALAPPDATA".to_owned()))?;
    let storage_dir = PathBuf::from(base_dir).join(APP_IDENTIFIER).join("configs");
    fs::create_dir_all(&storage_dir)
        .map_err(|e| AppError::Message(format!("failed to create config dir: {e}")))?;
    Ok(storage_dir.join("configs.json"))
}

fn parse_apply_config_arg() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let idx = args.iter().position(|arg| arg == ARG_APPLY_CONFIG)?;
    args.get(idx + 1).cloned()
}

fn app_name_from_process(process: &sysinfo::Process) -> String {
    process
        .exe()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| process.name().to_string_lossy().into_owned())
}

fn apply_config_headless(config_name: &str) -> Result<(), AppError> {
    let configs_path = headless_configs_file_path()?;
    let configs = read_configs_from_path(&configs_path)?;
    let Some(config) = configs
        .iter()
        .find(|item| item.name.eq_ignore_ascii_case(config_name))
    else {
        return Err(AppError::Message(format!(
            "config '{config_name}' was not found"
        )));
    };

    let mut system = System::new_all();
    system.refresh_processes(ProcessesToUpdate::All, true);

    let mut matched = 0usize;
    let mut applied = 0usize;
    let mut failed = 0usize;

    for (pid, process) in system.processes() {
        let app_name = app_name_from_process(process);
        let Some(priority) = config.config_map.get(&app_name).copied() else {
            continue;
        };

        matched += 1;
        match set_priority_for_pid(pid.as_u32(), priority) {
            Ok(()) => applied += 1,
            Err(err) => {
                failed += 1;
                eprintln!(
                    "[headless] failed to apply '{app_name}' for pid {}: {err}",
                    pid.as_u32()
                );
            }
        }
    }

    println!(
        "[headless] config '{}' done: matched={}, applied={}, failed={}",
        config.name, matched, applied, failed
    );
    Ok(())
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

fn enforce_config_on_running_processes(
    config: &ConfigDto,
    app_pid_index: &HashMap<String, Vec<u32>>,
) {
    for (target_app, target_priority) in &config.config_map {
        let target_app_lower = target_app.to_lowercase();
        let Some(pids) = app_pid_index.get(&target_app_lower) else {
            continue;
        };

        for pid in pids {
            let current = read_priority(*pid);
            if current.access_denied {
                continue;
            }

            let target_raw = target_priority.to_windows_flag().0;
            let matches_target = current.class == Some(*target_priority) || current.raw == Some(target_raw);

            if !matches_target {
                let _ = set_priority_for_pid(*pid, *target_priority);
            }
        }
    }
}

fn run_watchdog_tick(
    app: &tauri::AppHandle,
    active_triggers: &mut HashSet<String>,
) -> Result<(), AppError> {
    let configs = read_configs_from_disk(app)?;
    if configs.is_empty() {
        active_triggers.clear();
        return Ok(());
    }
    let watchdog = read_watchdog_config_from_disk(app)?;

    let config_lookup: HashMap<String, &ConfigDto> = configs
        .iter()
        .map(|config| (config.name.to_lowercase(), config))
        .collect();

    if config_lookup.is_empty() {
        return Ok(());
    }

    let mut system = System::new_all();
    system.refresh_processes(ProcessesToUpdate::All, true);

    let mut app_pid_index: HashMap<String, Vec<u32>> = HashMap::new();
    for (pid, process) in system.processes() {
        app_pid_index
            .entry(app_name_from_process(process).to_lowercase())
            .or_default()
            .push(pid.as_u32());
    }
    let running_apps: HashSet<String> = app_pid_index.keys().cloned().collect();

    let running_triggers: HashSet<String> = watchdog
        .trigger_map
        .keys()
        .filter(|app_name| running_apps.contains(*app_name))
        .cloned()
        .collect();

    let new_triggers: Vec<String> = running_triggers
        .difference(active_triggers)
        .cloned()
        .collect();

    let mut enforced_this_cycle = HashSet::new();

    for trigger in &new_triggers {
        let Some(config_name) = watchdog.trigger_map.get(trigger) else {
            continue;
        };
        let config_key = config_name.to_lowercase();
        let Some(config) = config_lookup.get(&config_key).copied() else {
            continue;
        };

        enforce_config_on_running_processes(config, &app_pid_index);
        enforced_this_cycle.insert(config_key);
    }

    let mut triggers_by_config: HashMap<String, HashSet<String>> = HashMap::new();
    for (trigger_app, config_name) in &watchdog.trigger_map {
        triggers_by_config
            .entry(config_name.to_lowercase())
            .or_default()
            .insert(trigger_app.to_owned());
    }

    for (sticky_config_name, mode) in &watchdog.sticky_modes {
        let config_key = sticky_config_name.to_lowercase();
        if enforced_this_cycle.contains(&config_key) {
            continue;
        }

        if *mode == 0 {
            continue;
        }

        let Some(config) = config_lookup.get(&config_key).copied() else {
            continue;
        };

        let should_enforce = match mode {
            1 => true,
            2 => match triggers_by_config.get(&config_key) {
                Some(mapped_triggers) if !mapped_triggers.is_empty() => mapped_triggers
                    .iter()
                    .any(|trigger| running_triggers.contains(trigger)),
                _ => false,
            },
            _ => false,
        };

        if !should_enforce {
            continue;
        }

        enforce_config_on_running_processes(config, &app_pid_index);
        enforced_this_cycle.insert(config_key);
    }

    *active_triggers = running_triggers;

    Ok(())
}

fn spawn_watchdog_loop(app: tauri::AppHandle, state: RuntimeControlState) {
    thread::spawn(move || {
        let mut active_triggers: HashSet<String> = HashSet::new();
        loop {
            if state.watchdog_enabled.load(Ordering::Relaxed) {
                if let Err(err) = run_watchdog_tick(&app, &mut active_triggers) {
                    eprintln!("Watchdog tick failed: {err}");
                }
            }
            thread::sleep(Duration::from_secs(5));
        }
    });
}

fn dialog_file_path_to_path_buf(path: FilePath) -> Result<PathBuf, AppError> {
    match path {
        FilePath::Path(path) => Ok(path),
        FilePath::Url(url) => url.to_file_path().map_err(|_| {
            AppError::Message("selected location is not a local file path".to_owned())
        }),
    }
}

fn write_configs_to_disk(app: &tauri::AppHandle, configs: &[ConfigDto]) -> Result<(), AppError> {
    let _guard = config_io_lock()
        .write()
        .map_err(|_| AppError::Message("config I/O lock poisoned".to_owned()))?;
    let file_path = configs_file_path(app)?;
    let data = serde_json::to_string_pretty(configs)
        .map_err(|e| AppError::Message(format!("failed to serialize configs: {e}")))?;
    atomic_write_json(&file_path, &data)
}

fn to_wide<S: AsRef<std::ffi::OsStr>>(value: S) -> Vec<u16> {
    value.as_ref().encode_wide().chain(iter::once(0)).collect()
}

fn show_startup_error_dialog(message: &str) {
    let title = to_wide("Optimus Startup Error");
    let body = to_wide(message);
    unsafe {
        let _ = MessageBoxW(
            HWND(std::ptr::null_mut()),
            PCWSTR(body.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

fn launch_elevated(payload: Option<ElevatedActionPayload>) -> Result<(), AppError> {
    let exe = std::env::current_exe()
        .map_err(|e| AppError::Message(format!("failed to get current executable: {e}")))?;

    let mut params = String::from(ARG_ELEVATED_SESSION);
    if let Some(payload) = payload {
        let encoded = STANDARD
            .encode(serde_json::to_vec(&payload).map_err(|e| AppError::Message(e.to_string()))?);
        params.push(' ');
        params.push_str(ARG_ELEVATED_PAYLOAD);
        params.push(' ');
        params.push_str(&encoded);
    }

    let operation = to_wide("runas");
    let file = to_wide(exe.as_os_str());
    let parameters = to_wide(params);

    let result: HINSTANCE = unsafe {
        ShellExecuteW(
            HWND(std::ptr::null_mut()),
            PCWSTR(operation.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR(parameters.as_ptr()),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    if result.0 as isize <= 32 {
        return Err(AppError::Message(format!(
            "failed to request UAC elevation (ShellExecuteW code {})",
            result.0 as isize
        )));
    }

    Ok(())
}

fn extract_icon_base64(path: &Path) -> Result<String, AppError> {
    let key = path.to_string_lossy().into_owned();

    if let Ok(cache) = icon_cache().lock() {
        if let Some(cached) = cache.get(&key) {
            return Ok(cached.clone());
        }
    }

    let rgba = extract_icon_rgba(path, ICON_SIZE)?;
    let image = image::RgbaImage::from_raw(ICON_SIZE as u32, ICON_SIZE as u32, rgba)
        .ok_or_else(|| AppError::Message("failed to build RGBA image".to_owned()))?;

    let mut cursor = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| AppError::Message(format!("failed to encode PNG: {e}")))?;

    let encoded = format!(
        "data:image/png;base64,{}",
        STANDARD.encode(cursor.into_inner())
    );
    if let Ok(mut cache) = icon_cache().lock() {
        cache.insert(key, encoded.clone());
    }

    Ok(encoded)
}

fn extract_icon_rgba(path: &Path, icon_size: i32) -> Result<Vec<u8>, AppError> {
    unsafe {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(iter::once(0))
            .collect();

        let mut icon = HICON::default();
        let extracted = ExtractIconExW(
            PCWSTR(wide.as_ptr()),
            0,
            Some(std::ptr::addr_of_mut!(icon)),
            None,
            1,
        );

        if extracted == 0 || icon.is_invalid() {
            return Err(AppError::Message(format!(
                "no icon extracted for {}",
                path.display()
            )));
        }

        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            let _ = DestroyIcon(icon);
            return Err(AppError::Message("CreateCompatibleDC failed".to_owned()));
        }

        let mut bits_ptr: *mut c_void = std::ptr::null_mut();
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: icon_size,
                biHeight: -icon_size,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0 as u32,
                ..Default::default()
            },
            ..Default::default()
        };

        let bitmap = match CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0) {
            Ok(bitmap) => bitmap,
            Err(_) => {
                let _ = DeleteDC(dc);
                let _ = DestroyIcon(icon);
                return Err(AppError::Message("CreateDIBSection failed".to_owned()));
            }
        };

        if bits_ptr.is_null() {
            let _ = DeleteDC(dc);
            let _ = DestroyIcon(icon);
            return Err(AppError::Message(
                "CreateDIBSection returned null".to_owned(),
            ));
        }

        let old = SelectObject(dc, HGDIOBJ(bitmap.0));

        let drew = DrawIconEx(
            dc,
            0,
            0,
            icon,
            icon_size,
            icon_size,
            0,
            HBRUSH(std::ptr::null_mut()),
            DI_NORMAL,
        )
        .is_ok();

        if !drew {
            if !old.is_invalid() {
                let _ = SelectObject(dc, old);
            }
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
            let _ = DeleteDC(dc);
            let _ = DestroyIcon(icon);
            return Err(AppError::Message("DrawIconEx failed".to_owned()));
        }

        let count = (icon_size * icon_size * 4) as usize;
        let bgra = std::slice::from_raw_parts(bits_ptr as *const u8, count);
        let mut rgba = Vec::with_capacity(count);
        for px in bgra.chunks_exact(4) {
            rgba.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
        }

        if !old.is_invalid() {
            let _ = SelectObject(dc, old);
        }

        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(dc);
        let _ = DestroyIcon(icon);

        Ok(rgba)
    }
}

fn decode_elevated_payload_from_args() -> Option<ElevatedActionPayload> {
    let args: Vec<String> = std::env::args().collect();
    let payload_index = args.iter().position(|arg| arg == ARG_ELEVATED_PAYLOAD)?;
    let encoded = args.get(payload_index + 1)?;
    let raw = STANDARD.decode(encoded).ok()?;
    serde_json::from_slice::<ElevatedActionPayload>(&raw).ok()
}

fn apply_startup_elevated_payload() {
    let Some(payload) = decode_elevated_payload_from_args() else {
        return;
    };

    match payload.action {
        ElevatedAction::SetProcessPriority => {
            if let (Some(pid), Some(priority)) = (payload.pid, payload.priority) {
                if let Err(err) = set_priority_for_pid(pid, priority) {
                    eprintln!("[elevated-startup] failed to set pid {pid}: {err}");
                }
            }
        }
        ElevatedAction::SetGroupPriority => {
            if let (Some(pids), Some(priority)) = (payload.pids, payload.priority) {
                for pid in pids {
                    if let Err(err) = set_priority_for_pid(pid, priority) {
                        eprintln!("[elevated-startup] failed to set pid {pid}: {err}");
                    }
                }
            }
        }
        ElevatedAction::KillProcess => {
            if let Some(pid) = payload.pid {
                if let Err(err) = kill_process_by_pid(pid) {
                    eprintln!("[elevated-startup] failed to kill pid {pid}: {err}");
                }
            }
        }
    }
}

#[tauri::command]
async fn list_process_groups() -> Result<ProcessListResponse, String> {
    tauri::async_runtime::spawn_blocking(gather_process_groups)
        .await
        .map_err(|e| format!("background task error: {e}"))
}

#[tauri::command]
#[allow(non_snake_case)]
async fn get_process_list_delta(knownIconKeys: Vec<String>) -> Result<ProcessListResponse, String> {
    let known_icon_keys: HashSet<String> = knownIconKeys.into_iter().collect();
    tauri::async_runtime::spawn_blocking(move || {
        gather_process_groups_with_known_icons(Some(&known_icon_keys))
    })
    .await
    .map_err(|e| format!("background task error: {e}"))
}

#[tauri::command]
fn get_process_priority(pid: u32) -> Result<ProcessPrioritySnapshotDto, String> {
    let priority = read_priority(pid);
    Ok(ProcessPrioritySnapshotDto {
        pid,
        priority: priority.class,
        priority_raw: priority.raw,
        priority_label: priority.label,
    })
}

#[tauri::command]
fn load_configs(app: tauri::AppHandle) -> Result<Vec<ConfigDto>, String> {
    read_configs_from_disk(&app).map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(non_snake_case)]
fn save_config(
    app: tauri::AppHandle,
    name: String,
    configMap: HashMap<String, PriorityClassDto>,
) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Config name cannot be empty".to_owned());
    }
    let config_map = configMap;

    let mut configs = read_configs_from_disk(&app).map_err(|e| e.to_string())?;
    let timestamp = current_unix_timestamp();

    if let Some(existing) = configs
        .iter_mut()
        .find(|config| config.name.eq_ignore_ascii_case(trimmed))
    {
        existing.name = trimmed.to_owned();
        existing.config_map = config_map;
        existing.updated_at = timestamp;
    } else {
        configs.push(ConfigDto {
            name: trimmed.to_owned(),
            config_map,
            updated_at: timestamp,
        });
    }

    configs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    write_configs_to_disk(&app, &configs).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_config(app: tauri::AppHandle, name: String) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Config name cannot be empty".to_owned());
    }

    let mut configs = read_configs_from_disk(&app).map_err(|e| e.to_string())?;
    let before_len = configs.len();
    configs.retain(|config| !config.name.eq_ignore_ascii_case(trimmed));

    if configs.len() == before_len {
        return Err(format!("Config '{trimmed}' was not found"));
    }

    configs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    write_configs_to_disk(&app, &configs).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_config(app: tauri::AppHandle, name: String) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Config name cannot be empty".to_owned());
    }

    let configs = read_configs_from_disk(&app).map_err(|e| e.to_string())?;
    let config = configs
        .into_iter()
        .find(|item| item.name.eq_ignore_ascii_case(trimmed))
        .ok_or_else(|| format!("Config '{trimmed}' was not found"))?;

    let default_name = format!("{}_Config.json", sanitize_shortcut_name(&config.name));
    let Some(file_path) = app
        .dialog()
        .file()
        .add_filter("Config JSON", &["json"])
        .set_file_name(default_name)
        .blocking_save_file()
    else {
        return Err("Export cancelled".to_owned());
    };

    let output_path = dialog_file_path_to_path_buf(file_path).map_err(|e| e.to_string())?;
    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("failed to serialize config: {e}"))?;
    fs::write(&output_path, content)
        .map_err(|e| format!("failed to write file '{}': {e}", output_path.display()))?;
    Ok(())
}

#[tauri::command]
fn import_config(app: tauri::AppHandle) -> Result<String, String> {
    let Some(file_path) = app
        .dialog()
        .file()
        .add_filter("Config JSON", &["json"])
        .blocking_pick_file()
    else {
        return Err("Import cancelled".to_owned());
    };

    let import_path = dialog_file_path_to_path_buf(file_path).map_err(|e| e.to_string())?;
    let content = fs::read_to_string(&import_path)
        .map_err(|e| format!("failed to read file '{}': {e}", import_path.display()))?;
    let mut imported = serde_json::from_str::<ConfigDto>(&content)
        .map_err(|e| format!("invalid config file format: {e}"))?;

    let trimmed_name = imported.name.trim();
    if trimmed_name.is_empty() {
        return Err("Imported config has an empty name".to_owned());
    }
    imported.name = trimmed_name.to_owned();
    imported.updated_at = current_unix_timestamp();

    let mut configs = read_configs_from_disk(&app).map_err(|e| e.to_string())?;
    if let Some(existing) = configs
        .iter_mut()
        .find(|item| item.name.eq_ignore_ascii_case(&imported.name))
    {
        *existing = imported.clone();
    } else {
        configs.push(imported.clone());
    }

    configs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    write_configs_to_disk(&app, &configs).map_err(|e| e.to_string())?;

    Ok(imported.name)
}

#[tauri::command]
fn load_watchdog_config(app: tauri::AppHandle) -> Result<WatchdogConfigDto, String> {
    let file_path = watchdog_file_path(&app).map_err(|e| e.to_string())?;
    if !file_path.exists() {
        return Ok(WatchdogConfigDto::default());
    }

    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("failed to read watchdog config: {e}"))?;
    let parsed = parse_watchdog_config(&content)
        .map_err(|e| format!("failed to parse watchdog config: {e}"))?;
    let (normalized, changed) = normalize_watchdog_config(parsed);
    if changed {
        write_watchdog_config_to_disk(&app, &normalized).map_err(|e| e.to_string())?;
    }
    Ok(normalized)
}

#[tauri::command]
#[allow(non_snake_case)]
fn upsert_watchdog_mapping(
    app: tauri::AppHandle,
    appName: String,
    configName: String,
) -> Result<(), String> {
    let app_name_trimmed = appName.trim().to_lowercase();
    if app_name_trimmed.is_empty() {
        return Err("Application name cannot be empty".to_owned());
    }

    let config_name_trimmed = configName.trim();
    if config_name_trimmed.is_empty() {
        return Err("Config name cannot be empty".to_owned());
    }

    let mut watchdog = read_watchdog_config_from_disk(&app).map_err(|e| e.to_string())?;
    watchdog
        .trigger_map
        .insert(app_name_trimmed, config_name_trimmed.to_owned());
    write_watchdog_config_to_disk(&app, &watchdog).map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(non_snake_case)]
fn remove_watchdog_mapping(app: tauri::AppHandle, appName: String) -> Result<(), String> {
    let app_name_trimmed = appName.trim().to_lowercase();
    if app_name_trimmed.is_empty() {
        return Err("Application name cannot be empty".to_owned());
    }

    let mut watchdog = read_watchdog_config_from_disk(&app).map_err(|e| e.to_string())?;
    watchdog.trigger_map.remove(&app_name_trimmed);
    write_watchdog_config_to_disk(&app, &watchdog).map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(non_snake_case)]
fn set_config_sticky(
    app: tauri::AppHandle,
    configName: String,
    mode: u8,
) -> Result<(), String> {
    let config_name_trimmed = configName.trim();
    if config_name_trimmed.is_empty() {
        return Err("Config name cannot be empty".to_owned());
    }

    if mode > 2 {
        return Err("Live mode must be 0, 1, or 2".to_owned());
    }

    let mut watchdog = read_watchdog_config_from_disk(&app).map_err(|e| e.to_string())?;
    let key = config_name_trimmed.to_lowercase();
    watchdog.sticky_modes.insert(key, mode);

    write_watchdog_config_to_disk(&app, &watchdog).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_runtime_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, RuntimeControlState>,
) -> Result<RuntimeSettingsDto, String> {
    let autostart_enabled = app
        .autolaunch()
        .is_enabled()
        .map_err(|e| format!("failed to read autostart state: {e}"))?;

    Ok(RuntimeSettingsDto {
        watchdog_enabled: state.watchdog_enabled.load(Ordering::Relaxed),
        autostart_enabled,
        minimize_to_tray_enabled: state.minimize_to_tray_enabled.load(Ordering::Relaxed),
    })
}

#[tauri::command]
fn toggle_watchdog(state: bool, runtime: tauri::State<'_, RuntimeControlState>) -> Result<(), String> {
    runtime.watchdog_enabled.store(state, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
fn toggle_minimize_to_tray(
    enabled: bool,
    runtime: tauri::State<'_, RuntimeControlState>,
) -> Result<(), String> {
    runtime
        .minimize_to_tray_enabled
        .store(enabled, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
fn toggle_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enabled {
        manager
            .enable()
            .map_err(|e| format!("failed to enable autostart: {e}"))?;
    } else {
        manager
            .disable()
            .map_err(|e| format!("failed to disable autostart: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
#[allow(non_snake_case)]
fn create_desktop_shortcut(app: tauri::AppHandle, configName: String) -> Result<(), String> {
    let trimmed = configName.trim();
    if trimmed.is_empty() {
        return Err("Config name cannot be empty".to_owned());
    }

    let _ = app;
    let exe = std::env::current_exe()
        .map_err(|e| format!("failed to resolve current executable: {e}"))?;
    let working_dir = exe
        .parent()
        .ok_or_else(|| "failed to resolve executable parent directory".to_owned())?;

    let config_arg = trimmed.replace('"', "\\\"");
    let shortcut_name = format!("Optimus - {}.lnk", sanitize_shortcut_name(trimmed));
    let shortcut_name = escape_ps_single_quoted(&shortcut_name);
    let exe_escaped = escape_ps_single_quoted(&exe.to_string_lossy());
    let workdir_escaped = escape_ps_single_quoted(&working_dir.to_string_lossy());
    let args_escaped = escape_ps_single_quoted(&format!("--apply-config \"{config_arg}\""));

    let script = format!(
        "$desktop=[Environment]::GetFolderPath('Desktop'); \
         $path=Join-Path $desktop '{shortcut_name}'; \
         $shell=New-Object -ComObject WScript.Shell; \
         $lnk=$shell.CreateShortcut($path); \
         $lnk.TargetPath='{exe_escaped}'; \
         $lnk.Arguments='{args_escaped}'; \
         $lnk.WorkingDirectory='{workdir_escaped}'; \
         $lnk.IconLocation='{exe_escaped},0'; \
         $lnk.Save();"
    );

    let output = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(script)
        .output()
        .map_err(|e| format!("failed to execute PowerShell: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "shortcut creation failed (status {}): {}{}{}",
            output.status,
            stderr,
            if !stderr.is_empty() && !stdout.is_empty() {
                " | "
            } else {
                ""
            },
            stdout
        ));
    }

    Ok(())
}

#[tauri::command]
fn restart_as_administrator(app: tauri::AppHandle) -> Result<ElevationStatusDto, String> {
    launch_elevated(None).map_err(|e| e.to_string())?;
    let app_handle = app.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(250));
        app_handle.exit(0);
    });
    Ok(ElevationStatusDto {
        status: "elevation_pending".to_owned(),
        message: "Waiting for Windows UAC confirmation...".to_owned(),
    })
}

#[tauri::command]
fn set_process_priority(
    app: tauri::AppHandle,
    pid: u32,
    priority: PriorityClassDto,
) -> Result<ApplyResultDto, String> {
    match set_priority_for_pid(pid, priority) {
        Ok(()) => Ok(ApplyResultDto {
            pid,
            success: true,
            message: format!("Priority set to {}", priority.label()),
        }),
        Err(AppError::AccessDenied { .. }) if !is_running_as_admin() => {
            let payload = ElevatedActionPayload {
                action: ElevatedAction::SetProcessPriority,
                priority: Some(priority),
                pid: Some(pid),
                pids: None,
            };
            launch_elevated(Some(payload)).map_err(|e| e.to_string())?;
            app.exit(0);
            Ok(ApplyResultDto {
                pid,
                success: true,
                message: "Restarting as administrator and retrying...".to_owned(),
            })
        }
        Err(err) => Err(err.to_string()),
    }
}

#[tauri::command]
fn set_group_priority(
    app: tauri::AppHandle,
    pids: Vec<u32>,
    priority: PriorityClassDto,
) -> Result<Vec<ApplyResultDto>, String> {
    let mut results = Vec::with_capacity(pids.len());

    for &pid in &pids {
        match set_priority_for_pid(pid, priority) {
            Ok(()) => results.push(ApplyResultDto {
                pid,
                success: true,
                message: format!("Priority set to {}", priority.label()),
            }),
            Err(AppError::AccessDenied { .. }) if !is_running_as_admin() => {
                let payload = ElevatedActionPayload {
                    action: ElevatedAction::SetGroupPriority,
                    priority: Some(priority),
                    pid: None,
                    pids: Some(pids),
                };
                launch_elevated(Some(payload)).map_err(|e| e.to_string())?;
                app.exit(0);
                return Ok(vec![ApplyResultDto {
                    pid,
                    success: true,
                    message: "Restarting as administrator and retrying group action...".to_owned(),
                }]);
            }
            Err(err) => {
                results.push(ApplyResultDto {
                    pid,
                    success: false,
                    message: err.to_string(),
                });
            }
        }
    }

    Ok(results)
}

#[tauri::command]
fn kill_process(app: tauri::AppHandle, pid: u32) -> Result<ApplyResultDto, String> {
    match kill_process_by_pid(pid) {
        Ok(()) => Ok(ApplyResultDto {
            pid,
            success: true,
            message: "Process terminated".to_owned(),
        }),
        Err(AppError::AccessDenied { .. }) if !is_running_as_admin() => {
            let payload = ElevatedActionPayload {
                action: ElevatedAction::KillProcess,
                priority: None,
                pid: Some(pid),
                pids: None,
            };
            launch_elevated(Some(payload)).map_err(|e| e.to_string())?;
            app.exit(0);
            Ok(ApplyResultDto {
                pid,
                success: true,
                message: "Restarting as administrator and retrying termination...".to_owned(),
            })
        }
        Err(err) => Err(err.to_string()),
    }
}

fn main() {
    if let Some(config_name) = parse_apply_config_arg() {
        if let Err(err) = apply_config_headless(&config_name) {
            eprintln!("[headless] {err}");
        }
        std::process::exit(0);
    }

    apply_startup_elevated_payload();

    let runtime_state = RuntimeControlState::default();
    runtime_state.watchdog_enabled.store(true, Ordering::Relaxed);

    let run_result = tauri::Builder::default()
        .manage(runtime_state.clone())
        .setup({
            let runtime_state = runtime_state.clone();
            move |app| {
                let show_item =
                    MenuItem::with_id(app, TRAY_SHOW_ID, "Show Optimus", true, None::<&str>)?;
                let exit_item =
                    MenuItem::with_id(app, TRAY_EXIT_ID, "Exit", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show_item, &exit_item])?;

                let show_id = show_item.id().clone();
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
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_process_groups,
            get_process_list_delta,
            get_process_priority,
            set_process_priority,
            set_group_priority,
            kill_process,
            restart_as_administrator,
            load_configs,
            save_config,
            delete_config,
            export_config,
            import_config,
            load_watchdog_config,
            upsert_watchdog_mapping,
            remove_watchdog_mapping,
            set_config_sticky,
            get_runtime_settings,
            toggle_watchdog,
            toggle_minimize_to_tray,
            toggle_autostart,
            create_desktop_shortcut
        ])
        .run(tauri::generate_context!());

    if let Err(err) = run_result {
        let message = format!("Failed to launch Optimus.\n\n{err}");
        eprintln!("{message}");
        show_startup_error_dialog(&message);
    }
}

