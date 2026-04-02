#![allow(unused_imports)]

use crate::types::AppError;
use windows::Win32::Foundation::{GetLastError, ERROR_ACCESS_DENIED};
use windows::Win32::UI::Shell::IsUserAnAdmin;

pub(crate) use base64::engine::general_purpose::STANDARD;
pub(crate) use base64::Engine;
pub(crate) use image::ImageFormat;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::{BTreeMap, HashMap, HashSet};
pub(crate) use std::ffi::c_void;
pub(crate) use std::fs;
pub(crate) use std::io::{Cursor, Write};
pub(crate) use std::iter;
pub(crate) use std::os::windows::ffi::OsStrExt;
pub(crate) use std::path::{Path, PathBuf};
pub(crate) use std::process::Command;
pub(crate) use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
pub(crate) use std::sync::{Arc, Mutex, OnceLock, RwLock};
pub(crate) use std::time::Duration;
pub(crate) use std::time::{SystemTime, UNIX_EPOCH};
pub(crate) use sysinfo::{ProcessesToUpdate, System};
pub(crate) use tauri::menu::{Menu, MenuItem};
pub(crate) use tauri::tray::TrayIconBuilder;
pub(crate) use tauri::Manager;
pub(crate) use tauri_plugin_dialog::{DialogExt, FilePath};
pub(crate) use tokio::sync::watch;
pub(crate) use windows::core::PCWSTR;
pub(crate) use windows::Win32::Foundation::{
    CloseHandle, ERROR_CANCELLED, ERROR_NOT_ALL_ASSIGNED, HANDLE, HWND, LUID,
};
pub(crate) use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBRUSH, HGDIOBJ,
};
pub(crate) use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
    TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
pub(crate) use windows::Win32::Storage::FileSystem::{ReplaceFileW, REPLACEFILE_WRITE_THROUGH};
pub(crate) use windows::Win32::System::SystemInformation::{
    GetSystemInfo, GlobalMemoryStatusEx, MEMORYSTATUSEX, SYSTEM_INFO,
};
pub(crate) use windows::Win32::System::Threading::{
    GetCurrentProcess, GetPriorityClass, OpenProcess, OpenProcessToken, ProcessPowerThrottling,
    SetPriorityClass, SetProcessInformation, TerminateProcess,
    ABOVE_NORMAL_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS,
    IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, PROCESS_ACCESS_RIGHTS,
    PROCESS_CREATION_FLAGS, PROCESS_POWER_THROTTLING_CURRENT_VERSION,
    PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION, PROCESS_TERMINATE,
    REALTIME_PRIORITY_CLASS,
};
pub(crate) use windows::Win32::System::Services::{
    ChangeServiceConfigW, CloseServiceHandle, ControlService, OpenSCManagerW, OpenServiceW,
    QueryServiceConfigW, QueryServiceStatus, StartServiceW, SC_HANDLE, SC_MANAGER_CONNECT, SERVICE_AUTO_START, SERVICE_CHANGE_CONFIG,
    SERVICE_CONTROL_STOP, SERVICE_DEMAND_START, SERVICE_DISABLED, SERVICE_NO_CHANGE, SERVICE_QUERY_CONFIG,
    SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_START, SERVICE_STATUS, SERVICE_STOP, QUERY_SERVICE_CONFIGW,
};
pub(crate) use windows::Win32::UI::Shell::{
    ExtractIconExW, ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
};
pub(crate) use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, DrawIconEx, MessageBoxW, DI_NORMAL, HICON, MB_ICONERROR, MB_OK, SW_SHOWNORMAL,
};
pub(crate) use winreg::enums::{
    HKEY_LOCAL_MACHINE, KEY_READ, KEY_SET_VALUE, KEY_WRITE,
};
pub(crate) use winreg::RegKey;
pub(crate) use tracing::{error, info, warn};

pub(crate) const ARG_ELEVATED_SESSION: &str = "--elevated-session";
pub(crate) const ARG_ELEVATED_PAYLOAD: &str = "--elevated-payload";
pub(crate) const ARG_APPLY_CONFIG: &str = "--apply-config";
pub(crate) const ICON_SIZE: i32 = 48;
pub(crate) const ICON_CACHE_MAX_ITEMS: usize = 500;
pub(crate) const ICON_COLLISION_GUARD_MAX_ITEMS: usize = 4096;
pub(crate) const TRAY_SHOW_ID: &str = "tray_show";
pub(crate) const TRAY_PURGE_ID: &str = "tray_purge_memory";
pub(crate) const TRAY_EXIT_ID: &str = "tray_exit";
pub(crate) const ELEVATED_AUTOSTART_TASK_NAME: &str = "OptimusAutoStart";
pub(crate) const SYSTEM_MEMORY_LIST_INFORMATION_CLASS: u32 = 80;
pub(crate) const MEMORY_PURGE_STANDBY_LIST: u32 = 4;
pub(crate) const PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION: u32 = 4;

#[link(name = "ntdll")]
unsafe extern "system" {
    pub(crate) fn NtQueryTimerResolution(
        MaximumTime: *mut u32,
        MinimumTime: *mut u32,
        CurrentTime: *mut u32,
    ) -> i32;
    pub(crate) fn NtSetTimerResolution(
        DesiredTime: u32,
        SetResolution: u8,
        CurrentTime: *mut u32,
    ) -> i32;
    pub(crate) fn NtQuerySystemInformation(
        SystemInformationClass: u32,
        SystemInformation: *mut c_void,
        SystemInformationLength: u32,
        ReturnLength: *mut u32,
    ) -> i32;
    pub(crate) fn NtSetSystemInformation(
        SystemInformationClass: u32,
        SystemInformation: *mut c_void,
        SystemInformationLength: u32,
    ) -> i32;
}

pub(crate) fn last_error_code() -> u32 {
    unsafe { GetLastError().0 }
}

pub(crate) fn is_access_denied(code: u32) -> bool {
    code == ERROR_ACCESS_DENIED.0
}

pub(crate) fn is_running_as_admin() -> bool {
    unsafe { IsUserAnAdmin().as_bool() }
}

pub(crate) fn escape_ps_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}

pub(crate) fn sanitize_shortcut_name(value: &str) -> String {
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

pub(crate) fn show_main_window(app: &tauri::AppHandle) -> Result<(), AppError> {
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

pub(crate) fn dialog_file_path_to_path_buf(path: FilePath) -> Result<PathBuf, AppError> {
    match path {
        FilePath::Path(path) => Ok(path),
        FilePath::Url(url) => url
            .to_file_path()
            .map_err(|_| AppError::Message("selected location is not a local file path".to_owned())),
    }
}



