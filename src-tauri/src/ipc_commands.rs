use crate::*;

#[tauri::command]
pub(crate) async fn list_process_groups() -> Result<ProcessListResponse, String> {
    tauri::async_runtime::spawn_blocking(gather_process_groups)
        .await
        .map_err(|e| format!("background task error: {e}"))
}

#[tauri::command]
#[allow(non_snake_case)]
pub(crate) async fn get_process_list_delta(knownIconKeys: Vec<String>) -> Result<ProcessListResponse, String> {
    let known_icon_keys: HashSet<String> = knownIconKeys.into_iter().collect();
    tauri::async_runtime::spawn_blocking(move || {
        gather_process_groups_with_known_icons(Some(&known_icon_keys))
    })
    .await
    .map_err(|e| format!("background task error: {e}"))
}

#[tauri::command]
pub(crate) fn get_process_priority(pid: u32) -> Result<ProcessPrioritySnapshotDto, String> {
    let priority = read_priority(pid);
    Ok(ProcessPrioritySnapshotDto {
        pid,
        priority: priority.class,
        priority_raw: priority.raw,
        priority_label: priority.label,
    })
}

#[tauri::command]
pub(crate) fn load_configs(app: tauri::AppHandle) -> Result<Vec<ConfigDto>, String> {
    read_configs_from_disk(&app).map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(non_snake_case)]
pub(crate) fn save_config(
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
pub(crate) fn delete_config(app: tauri::AppHandle, name: String) -> Result<(), String> {
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
pub(crate) fn export_config(app: tauri::AppHandle, name: String) -> Result<(), String> {
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
pub(crate) fn import_config(app: tauri::AppHandle) -> Result<String, String> {
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
pub(crate) fn load_watchdog_config(app: tauri::AppHandle) -> Result<WatchdogConfigDto, String> {
    let (normalized, changed) = read_watchdog_config_with_migration(&app).map_err(|e| e.to_string())?;
    if changed {
        write_watchdog_config_to_disk(&app, &normalized).map_err(|e| e.to_string())?;
    }
    Ok(normalized)
}

#[tauri::command]
#[allow(non_snake_case)]
pub(crate) fn upsert_watchdog_mapping(
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
pub(crate) fn remove_watchdog_mapping(app: tauri::AppHandle, appName: String) -> Result<(), String> {
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
pub(crate) fn set_config_sticky(
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
pub(crate) fn get_runtime_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, RuntimeControlState>,
) -> Result<RuntimeSettingsDto, String> {
    let standard_autostart_enabled = app
        .autolaunch()
        .is_enabled()
        .map_err(|e| format!("failed to read autostart state: {e}"))?;
    let elevated_autostart_enabled = is_elevated_autostart_task_enabled()?;
    let autostart_mode = if elevated_autostart_enabled {
        AutostartModeDto::Elevated
    } else if standard_autostart_enabled {
        AutostartModeDto::User
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
pub(crate) fn get_app_settings(runtime: tauri::State<'_, RuntimeControlState>) -> Result<AppSettings, String> {
    snapshot_app_settings(&runtime).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn get_current_timer_res(
    runtime: tauri::State<'_, RuntimeControlState>,
) -> Result<TimerResolutionDto, String> {
    build_timer_resolution_dto(&runtime).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn set_timer_res(
    app: tauri::AppHandle,
    value: f32,
    runtime: tauri::State<'_, RuntimeControlState>,
) -> Result<TimerResolutionDto, String> {
    let request = if value <= 0.0 {
        None
    } else {
        Some(ms_to_hundred_ns(value))
    };
    let dto = apply_timer_resolution_request(&runtime, request).map_err(|e| e.to_string())?;
    save_runtime_settings(&app, &runtime).map_err(|e| e.to_string())?;
    Ok(dto)
}

#[tauri::command]
pub(crate) fn get_memory_stats() -> Result<MemoryStatsDto, String> {
    read_memory_stats().map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn get_memory_purge_config(
    runtime: tauri::State<'_, RuntimeControlState>,
) -> Result<MemoryPurgeConfigDto, String> {
    build_memory_purge_config_dto(&runtime).map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(non_snake_case)]
pub(crate) fn set_memory_purge_config(
    app: tauri::AppHandle,
    runtime: tauri::State<'_, RuntimeControlState>,
    masterEnabled: bool,
    enableStandbyTrigger: bool,
    standbyLimitMb: u64,
    enableFreeMemoryTrigger: bool,
    freeMemoryLimitMb: u64,
) -> Result<MemoryPurgeConfigDto, String> {
    {
        let mut config = runtime
            .memory_purge_config
            .write()
            .map_err(|_| "memory purge config lock poisoned".to_owned())?;
        config.master_enabled = masterEnabled;
        config.enable_standby_trigger = enableStandbyTrigger;
        config.standby_limit_mb = standbyLimitMb.max(1);
        config.enable_free_memory_trigger = enableFreeMemoryTrigger;
        config.free_memory_limit_mb = freeMemoryLimitMb.max(1);
    }

    save_runtime_settings(&app, &runtime).map_err(|e| e.to_string())?;
    build_memory_purge_config_dto(&runtime).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn run_purge(runtime: tauri::State<'_, RuntimeControlState>) -> Result<MemoryPurgeConfigDto, String> {
    run_standby_purge().map_err(|e| e.to_string())?;
    runtime.memory_purge_count.fetch_add(1, Ordering::Relaxed);
    build_memory_purge_config_dto(&runtime).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn toggle_watchdog(
    app: tauri::AppHandle,
    state: bool,
    runtime: tauri::State<'_, RuntimeControlState>,
) -> Result<(), String> {
    runtime.watchdog_enabled.store(state, Ordering::Relaxed);
    save_runtime_settings(&app, &runtime).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) fn toggle_minimize_to_tray(
    app: tauri::AppHandle,
    enabled: bool,
    runtime: tauri::State<'_, RuntimeControlState>,
) -> Result<(), String> {
    runtime
        .minimize_to_tray_enabled
        .store(enabled, Ordering::Relaxed);
    save_runtime_settings(&app, &runtime).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
#[allow(non_snake_case)]
pub(crate) fn configure_autostart(app: tauri::AppHandle, enabled: bool, asAdmin: bool) -> Result<(), String> {
    configure_autostart_impl(app, enabled, asAdmin)
}

#[tauri::command]
pub(crate) fn toggle_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    configure_autostart(app, enabled, false)
}

#[tauri::command]
#[allow(non_snake_case)]
pub(crate) fn create_desktop_shortcut(app: tauri::AppHandle, configName: String) -> Result<(), String> {
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
pub(crate) fn restart_as_administrator(app: tauri::AppHandle) -> Result<ElevationStatusDto, String> {
    match launch_elevated(None).map_err(|e| e.to_string())? {
        ElevationLaunchStatus::Launched => {
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
        ElevationLaunchStatus::Cancelled => Ok(ElevationStatusDto {
            status: "elevation_cancelled".to_owned(),
            message: "UAC request was cancelled.".to_owned(),
        }),
    }
}

#[tauri::command]
pub(crate) fn set_process_priority(
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
            match launch_elevated(Some(payload)).map_err(|e| e.to_string())? {
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
        Err(err) => Err(err.to_string()),
    }
}

#[tauri::command]
pub(crate) fn set_group_priority(
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
                return match launch_elevated(Some(payload)).map_err(|e| e.to_string())? {
                    ElevationLaunchStatus::Launched => {
                        app.exit(0);
                        Ok(vec![ApplyResultDto {
                            pid,
                            success: true,
                            message: "Restarting as administrator and retrying group action...".to_owned(),
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
pub(crate) fn kill_process(app: tauri::AppHandle, pid: u32) -> Result<ApplyResultDto, String> {
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
            match launch_elevated(Some(payload)).map_err(|e| e.to_string())? {
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
        Err(err) => Err(err.to_string()),
    }
}


