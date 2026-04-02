use crate::autostart::{configure_autostart_impl, is_elevated_autostart_task_enabled};
use crate::core::Ordering;
use crate::elevation::{launch_elevated, ElevationLaunchStatus};
use crate::memory_purge::{
    build_memory_purge_config_dto, read_memory_stats, run_standby_purge_with_telemetry,
};
use crate::settings_repo::{save_runtime_settings, snapshot_app_settings};
use crate::timer::{apply_timer_resolution_request, build_timer_resolution_dto, ms_to_hundred_ns};
use crate::types::{
    AppSettings, AutostartModeDto, ElevationStatusDto, MemoryPurgeConfigDto, MemoryStatsDto,
    RuntimeControlState, RuntimeSettingsDto, TimerResolutionDto,
};
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

const APP_COMPAT_LAYERS_KEY: &str =
    "Software\\Microsoft\\Windows NT\\CurrentVersion\\AppCompatFlags\\Layers";
const RUN_AS_ADMIN_LAYER_VALUE: &str = "~ RUNASADMIN";

#[tauri::command]
pub(crate) fn engine_get_runtime_settings(
    _app: tauri::AppHandle,
    state: tauri::State<'_, RuntimeControlState>,
) -> super::IpcResult<RuntimeSettingsDto> {
    let elevated_autostart_enabled =
        is_elevated_autostart_task_enabled().map_err(super::to_ipc_error)?;
    let autostart_mode = if elevated_autostart_enabled {
        AutostartModeDto::Elevated
    } else {
        AutostartModeDto::Off
    };

    Ok(RuntimeSettingsDto {
        watchdog_enabled: state.watchdog_enabled.load(Ordering::Relaxed),
        autostart_enabled: !matches!(autostart_mode, AutostartModeDto::Off),
        autostart_mode,
        start_as_admin_enabled: matches!(autostart_mode, AutostartModeDto::Elevated),
        minimize_to_tray_enabled: state.minimize_to_tray_enabled.load(Ordering::Relaxed),
    })
}

#[tauri::command]
pub(crate) fn engine_get_app_settings(
    runtime: tauri::State<'_, RuntimeControlState>,
) -> super::IpcResult<AppSettings> {
    snapshot_app_settings(&runtime).map_err(super::from_app_error)
}

#[tauri::command]
pub(crate) fn engine_timer_get_status(
    runtime: tauri::State<'_, RuntimeControlState>,
) -> super::IpcResult<TimerResolutionDto> {
    build_timer_resolution_dto(&runtime).map_err(super::from_app_error)
}

#[tauri::command]
pub(crate) fn engine_timer_set(
    app: tauri::AppHandle,
    value: f32,
    runtime: tauri::State<'_, RuntimeControlState>,
) -> super::IpcResult<TimerResolutionDto> {
    let request = if value <= 0.0 {
        None
    } else {
        Some(ms_to_hundred_ns(value))
    };
    let dto = apply_timer_resolution_request(&runtime, request).map_err(super::from_app_error)?;
    save_runtime_settings(&app, &runtime).map_err(super::from_app_error)?;
    Ok(dto)
}

#[tauri::command]
pub(crate) fn engine_memory_get_stats() -> super::IpcResult<MemoryStatsDto> {
    read_memory_stats().map_err(super::from_app_error)
}

#[tauri::command]
pub(crate) fn engine_memory_get_config(
    runtime: tauri::State<'_, RuntimeControlState>,
) -> super::IpcResult<MemoryPurgeConfigDto> {
    build_memory_purge_config_dto(&runtime).map_err(super::from_app_error)
}

#[tauri::command]
#[allow(non_snake_case)]
pub(crate) fn engine_memory_set_config(
    app: tauri::AppHandle,
    runtime: tauri::State<'_, RuntimeControlState>,
    masterEnabled: bool,
    enableStandbyTrigger: bool,
    standbyLimitMb: u64,
    enableFreeMemoryTrigger: bool,
    freeMemoryLimitMb: u64,
) -> super::IpcResult<MemoryPurgeConfigDto> {
    {
        let mut config = runtime
            .memory_purge_config
            .write()
            .map_err(|_| super::app_error("INTERNAL", "memory purge config lock poisoned", false, true))?;
        config.master_enabled = masterEnabled;
        config.enable_standby_trigger = enableStandbyTrigger;
        config.standby_limit_mb = standbyLimitMb.max(1);
        config.enable_free_memory_trigger = enableFreeMemoryTrigger;
        config.free_memory_limit_mb = freeMemoryLimitMb.max(1);
    }

    save_runtime_settings(&app, &runtime).map_err(super::from_app_error)?;
    build_memory_purge_config_dto(&runtime).map_err(super::from_app_error)
}

#[tauri::command]
#[tracing::instrument(skip_all)]
pub(crate) fn engine_memory_purge(
    runtime: tauri::State<'_, RuntimeControlState>,
) -> super::IpcResult<MemoryPurgeConfigDto> {
    run_standby_purge_with_telemetry(runtime.inner()).map_err(super::from_app_error)?;
    build_memory_purge_config_dto(&runtime).map_err(super::from_app_error)
}

#[tauri::command]
pub(crate) fn engine_watchdog_set_enabled(
    app: tauri::AppHandle,
    state: bool,
    runtime: tauri::State<'_, RuntimeControlState>,
) -> super::IpcResult<()> {
    runtime.watchdog_enabled.store(state, Ordering::Relaxed);
    save_runtime_settings(&app, &runtime).map_err(super::from_app_error)?;
    Ok(())
}

#[tauri::command]
pub(crate) fn engine_tray_set_minimize(
    app: tauri::AppHandle,
    enabled: bool,
    runtime: tauri::State<'_, RuntimeControlState>,
) -> super::IpcResult<()> {
    runtime
        .minimize_to_tray_enabled
        .store(enabled, Ordering::Relaxed);
    save_runtime_settings(&app, &runtime).map_err(super::from_app_error)?;
    Ok(())
}

#[tauri::command]
#[allow(non_snake_case)]
pub(crate) fn engine_autostart_configure(
    app: tauri::AppHandle,
    enabled: bool,
    asAdmin: bool,
) -> super::IpcResult<()> {
    configure_autostart_impl(app, enabled, asAdmin).map_err(super::to_ipc_error)
}

#[tauri::command]
pub(crate) fn engine_autostart_toggle(
    app: tauri::AppHandle,
    enabled: bool,
) -> super::IpcResult<()> {
    configure_autostart_impl(app, enabled, true).map_err(super::to_ipc_error)
}

#[tauri::command]
pub(crate) fn set_run_as_admin(enable: bool) -> super::IpcResult<()> {
    let exe_path = std::env::current_exe()
        .map_err(super::to_ipc_error)?
        .to_string_lossy()
        .into_owned();

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (layers, _) = hkcu
        .create_subkey(APP_COMPAT_LAYERS_KEY)
        .map_err(super::to_ipc_error)?;

    if enable {
        layers
            .set_value(&exe_path, &RUN_AS_ADMIN_LAYER_VALUE)
            .map_err(super::to_ipc_error)?;
        return Ok(());
    }

    match layers.delete_value(&exe_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(super::to_ipc_error(error)),
    }
}

#[tauri::command]
pub(crate) fn engine_elevation_restart(
    app: tauri::AppHandle,
) -> super::IpcResult<ElevationStatusDto> {
    match launch_elevated(None).map_err(super::from_app_error)? {
        ElevationLaunchStatus::Launched => {
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                app_handle.exit(0);
            });
            Ok(ElevationStatusDto {
                status: "elevation_pending".to_owned(),
                message: "Waiting for Windows UAC confirmation...".to_owned(),
            })
        }
        ElevationLaunchStatus::Cancelled => Ok(ElevationStatusDto {
            status: "elevation_cancelled".to_owned(),
            message: "UAC request was cancelled.".to_owned(),
        }),
    }
}
