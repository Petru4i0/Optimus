use crate::core::is_running_as_admin;
use crate::elevation::{launch_elevated, ElevationLaunchStatus};
use crate::process::{
    compute_process_delta, extract_icon_png_bytes, kill_process_by_pid, read_priority,
    resolve_icon_path_for_key, set_priority_for_pid,
};
use crate::types::{
    AppError, ApplyResultDto, DriverDto, ElevatedAction, ElevatedActionPayload,
    GhostDeviceDto, IconBinaryDto, MsiApplyDto, MsiBatchReportDto, PciDeviceDto, PriorityClassDto,
    ProcessDeltaPayload, ProcessPrioritySnapshotDto, RuntimeControlState,
};

#[tauri::command]
pub(crate) fn check_is_admin() -> super::IpcResult<bool> {
    Ok(is_running_as_admin())
}

#[tauri::command]
pub(crate) async fn run_deep_purge(
    config: Option<crate::cleaner::DeepPurgeConfig>,
) -> super::IpcResult<u64> {
    super::ensure_admin()?;
    let effective_config = config.unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || crate::cleaner::run_deep_purge(effective_config))
        .await
        .map_err(super::to_ipc_error)?
        .map_err(super::from_app_error)
}

#[tauri::command]
#[tracing::instrument(skip_all)]
pub(crate) async fn process_get_delta(
    runtime: tauri::State<'_, RuntimeControlState>,
) -> super::IpcResult<ProcessDeltaPayload> {
    let runtime_state = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || compute_process_delta(&runtime_state))
        .await
        .map_err(super::to_ipc_error)
}

#[tauri::command]
#[allow(non_snake_case)]
#[tracing::instrument(skip_all)]
pub(crate) async fn icon_get_png(
    iconKey: String,
    runtime: tauri::State<'_, RuntimeControlState>,
) -> super::IpcResult<IconBinaryDto> {
    let icon_key = iconKey.trim();
    if icon_key.is_empty() {
        return Err(super::app_error(
            "VALIDATION_ERROR",
            "iconKey cannot be empty",
            false,
            false,
        ));
    }

    let runtime_state = runtime.inner().clone();
    let icon_key_owned = icon_key.to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        let icon_path = resolve_icon_path_for_key(&runtime_state, &icon_key_owned).ok_or_else(|| {
            AppError::Message(format!("Icon source for key '{}' was not found", icon_key_owned))
        })?;
        let bytes = extract_icon_png_bytes(&icon_path)?;
        Ok::<IconBinaryDto, AppError>(IconBinaryDto {
            icon_key: icon_key_owned,
            content_type: "image/png".to_owned(),
            bytes,
        })
    })
    .await
    .map_err(super::to_ipc_error)?
    .map_err(super::from_app_error)
}

#[tauri::command]
pub(crate) fn process_get_priority(pid: u32) -> super::IpcResult<ProcessPrioritySnapshotDto> {
    let priority = read_priority(pid);
    Ok(ProcessPrioritySnapshotDto {
        pid,
        priority: priority.class,
        priority_raw: priority.raw,
        priority_label: priority.label,
    })
}

#[tauri::command]
pub(crate) fn process_set_priority(
    app: tauri::AppHandle,
    pid: u32,
    priority: PriorityClassDto,
) -> super::IpcResult<ApplyResultDto> {
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
            match launch_elevated(Some(payload)).map_err(super::from_app_error)? {
                ElevationLaunchStatus::Launched => {
                    app.exit(0);
                    Ok(ApplyResultDto {
                        pid,
                        success: true,
                        message: "Restarting as administrator and retrying...".to_owned(),
                    })
                }
                ElevationLaunchStatus::Cancelled => Ok(ApplyResultDto {
                    pid,
                    success: false,
                    message: "UAC request was cancelled.".to_owned(),
                }),
            }
        }
        Err(err) => Err(super::from_app_error(err)),
    }
}

#[tauri::command]
pub(crate) fn process_set_group_priority(
    app: tauri::AppHandle,
    pids: Vec<u32>,
    priority: PriorityClassDto,
) -> super::IpcResult<Vec<ApplyResultDto>> {
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
                return match launch_elevated(Some(payload)).map_err(super::from_app_error)? {
                    ElevationLaunchStatus::Launched => {
                        app.exit(0);
                        Ok(vec![ApplyResultDto {
                            pid,
                            success: true,
                            message: "Restarting as administrator and retrying group action..."
                                .to_owned(),
                        }])
                    }
                    ElevationLaunchStatus::Cancelled => Ok(vec![ApplyResultDto {
                        pid,
                        success: false,
                        message: "UAC request was cancelled.".to_owned(),
                    }]),
                };
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
pub(crate) fn process_kill(app: tauri::AppHandle, pid: u32) -> super::IpcResult<ApplyResultDto> {
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
            match launch_elevated(Some(payload)).map_err(super::from_app_error)? {
                ElevationLaunchStatus::Launched => {
                    app.exit(0);
                    Ok(ApplyResultDto {
                        pid,
                        success: true,
                        message: "Restarting as administrator and retrying termination...".to_owned(),
                    })
                }
                ElevationLaunchStatus::Cancelled => Ok(ApplyResultDto {
                    pid,
                    success: false,
                    message: "UAC request was cancelled.".to_owned(),
                }),
            }
        }
        Err(err) => Err(super::from_app_error(err)),
    }
}

#[tauri::command]
pub(crate) async fn hardware_msi_list() -> super::IpcResult<Vec<PciDeviceDto>> {
    tauri::async_runtime::spawn_blocking(crate::msi_util::get_pci_devices)
        .await
        .map_err(super::to_ipc_error)?
        .map_err(super::from_app_error)
}

#[tauri::command]
#[tracing::instrument(skip_all)]
pub(crate) fn hardware_msi_apply_batch(payload: Vec<MsiApplyDto>) -> super::IpcResult<MsiBatchReportDto> {
    super::ensure_admin()?;

    let total = payload.len() as i32;
    let mut successful = 0i32;
    let mut failed = 0i32;
    let mut errors = Vec::new();

    for item in payload {
        match crate::msi_util::set_msi_mode(&item.device_id, item.enable, item.priority) {
            Ok(()) => successful += 1,
            Err(err) => {
                failed += 1;
                errors.push(format!("{}: {}", item.device_id, err));
            }
        }
    }

    Ok(MsiBatchReportDto {
        total,
        successful,
        failed,
        errors,
    })
}

#[tauri::command]
pub(crate) async fn hardware_driver_list() -> super::IpcResult<Vec<DriverDto>> {
    tauri::async_runtime::spawn_blocking(crate::driver_store::get_installed_drivers)
        .await
        .map_err(super::to_ipc_error)?
        .map_err(super::from_app_error)
}

#[tauri::command]
#[allow(non_snake_case)]
pub(crate) async fn hardware_driver_delete(
    publishedName: String,
    force: bool,
) -> super::IpcResult<()> {
    super::ensure_admin()?;
    tauri::async_runtime::spawn_blocking(move || crate::driver_store::delete_driver(&publishedName, force))
        .await
        .map_err(super::to_ipc_error)?
        .map_err(super::from_app_error)
}

#[tauri::command]
pub(crate) async fn hardware_ghost_list() -> super::IpcResult<Vec<GhostDeviceDto>> {
    tauri::async_runtime::spawn_blocking(crate::ghost_devices::get_ghost_devices)
        .await
        .map_err(super::to_ipc_error)?
        .map_err(super::from_app_error)
}

#[tauri::command]
#[allow(non_snake_case)]
pub(crate) fn hardware_ghost_remove(instanceId: String, force: bool) -> super::IpcResult<()> {
    super::ensure_admin()?;
    crate::ghost_devices::remove_ghost_device(&instanceId, force).map_err(super::from_app_error)
}
