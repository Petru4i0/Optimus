use crate::optimization::{toggle_advanced, toggle_net_sniper, toggle_power, toggle_telemetry};
use crate::types::{OptimizationStatusDto, RuntimeControlState};

#[tauri::command]
#[allow(non_snake_case)]
#[tracing::instrument(skip_all)]
pub(crate) fn optimization_telemetry_toggle(
    app: tauri::AppHandle,
    runtime: tauri::State<'_, RuntimeControlState>,
    subFeature: String,
    enabled: bool,
) -> super::IpcResult<OptimizationStatusDto> {
    super::ensure_admin()?;
    toggle_telemetry(&app, &runtime, subFeature.trim().to_lowercase().as_str(), enabled)
        .map_err(super::from_app_error)
}

#[tauri::command]
#[allow(non_snake_case)]
#[tracing::instrument(skip_all)]
pub(crate) fn optimization_net_sniper_toggle(
    app: tauri::AppHandle,
    runtime: tauri::State<'_, RuntimeControlState>,
    subFeature: String,
    enabled: bool,
) -> super::IpcResult<OptimizationStatusDto> {
    super::ensure_admin()?;
    toggle_net_sniper(&app, &runtime, subFeature.trim().to_lowercase().as_str(), enabled)
        .map_err(super::from_app_error)
}

#[tauri::command]
#[allow(non_snake_case)]
#[tracing::instrument(skip_all)]
pub(crate) fn optimization_power_toggle(
    app: tauri::AppHandle,
    runtime: tauri::State<'_, RuntimeControlState>,
    subFeature: String,
    enabled: bool,
) -> super::IpcResult<OptimizationStatusDto> {
    super::ensure_admin()?;
    toggle_power(&app, &runtime, subFeature.trim().to_lowercase().as_str(), enabled)
        .map_err(super::from_app_error)
}

#[tauri::command]
#[allow(non_snake_case)]
#[tracing::instrument(skip_all)]
pub(crate) fn optimization_advanced_toggle(
    app: tauri::AppHandle,
    runtime: tauri::State<'_, RuntimeControlState>,
    subFeature: String,
    enabled: bool,
) -> super::IpcResult<OptimizationStatusDto> {
    super::ensure_admin()?;
    toggle_advanced(&app, &runtime, subFeature.trim().to_lowercase().as_str(), enabled)
        .map_err(super::from_app_error)
}

#[tauri::command]
pub(crate) fn optimization_get_status(
    runtime: tauri::State<'_, RuntimeControlState>,
) -> super::IpcResult<OptimizationStatusDto> {
    super::collect_optimization_status(&runtime)
}
