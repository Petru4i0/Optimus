use crate::core::{
    dialog_file_path_to_path_buf, escape_ps_single_quoted, fs,
    sanitize_shortcut_name, Command,
};
use crate::settings_repo::{
    current_unix_timestamp, read_configs_from_disk, read_watchdog_config_from_disk,
    read_watchdog_config_with_migration, write_configs_to_disk, write_watchdog_config_to_disk,
};
use crate::types::{ConfigDto, PriorityClassDto, WatchdogConfigDto, WatchdogTriggerMappingDto};
use std::collections::HashMap;
use std::os::windows::process::CommandExt;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub(crate) fn config_load_configs(app: tauri::AppHandle) -> super::IpcResult<Vec<ConfigDto>> {
    read_configs_from_disk(&app).map_err(super::from_app_error)
}

#[tauri::command]
#[allow(non_snake_case)]
pub(crate) fn config_save(
    app: tauri::AppHandle,
    name: String,
    configMap: HashMap<String, PriorityClassDto>,
) -> super::IpcResult<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(super::app_error(
            "VALIDATION_ERROR",
            "Config name cannot be empty",
            false,
            false,
        ));
    }
    let config_map = configMap;

    let mut configs = read_configs_from_disk(&app).map_err(super::from_app_error)?;
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
    write_configs_to_disk(&app, &configs).map_err(super::from_app_error)
}

#[tauri::command]
pub(crate) fn config_delete(app: tauri::AppHandle, name: String) -> super::IpcResult<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(super::app_error(
            "VALIDATION_ERROR",
            "Config name cannot be empty",
            false,
            false,
        ));
    }

    let mut configs = read_configs_from_disk(&app).map_err(super::from_app_error)?;
    let before_len = configs.len();
    configs.retain(|config| !config.name.eq_ignore_ascii_case(trimmed));

    if configs.len() == before_len {
        return Err(super::app_error(
            "NOT_FOUND",
            format!("Config '{trimmed}' was not found"),
            false,
            false,
        ));
    }

    configs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    write_configs_to_disk(&app, &configs).map_err(super::from_app_error)
}

#[tauri::command]
pub(crate) fn config_export(app: tauri::AppHandle, name: String) -> super::IpcResult<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(super::app_error(
            "VALIDATION_ERROR",
            "Config name cannot be empty",
            false,
            false,
        ));
    }

    let configs = read_configs_from_disk(&app).map_err(super::from_app_error)?;
    let config = configs
        .into_iter()
        .find(|item| item.name.eq_ignore_ascii_case(trimmed))
        .ok_or_else(|| {
            super::app_error(
                "NOT_FOUND",
                format!("Config '{trimmed}' was not found"),
                false,
                false,
            )
        })?;

    let default_name = format!("{}_Config.json", sanitize_shortcut_name(&config.name));
    let Some(file_path) = app
        .dialog()
        .file()
        .add_filter("Config JSON", &["json"])
        .set_file_name(default_name)
        .blocking_save_file()
    else {
        return Err(super::app_error("CANCELLED", "Export cancelled", false, false));
    };

    let output_path = dialog_file_path_to_path_buf(file_path).map_err(super::from_app_error)?;
    let content = serde_json::to_string_pretty(&config).map_err(super::to_ipc_error)?;
    fs::write(&output_path, content).map_err(super::to_ipc_error)?;
    Ok(())
}

#[tauri::command]
pub(crate) fn config_import(app: tauri::AppHandle) -> super::IpcResult<String> {
    let Some(file_path) = app
        .dialog()
        .file()
        .add_filter("Config JSON", &["json"])
        .blocking_pick_file()
    else {
        return Err(super::app_error("CANCELLED", "Import cancelled", false, false));
    };

    let import_path = dialog_file_path_to_path_buf(file_path).map_err(super::from_app_error)?;
    let content = fs::read_to_string(&import_path).map_err(super::to_ipc_error)?;
    let mut imported = serde_json::from_str::<ConfigDto>(&content).map_err(super::to_ipc_error)?;

    let trimmed_name = imported.name.trim();
    if trimmed_name.is_empty() {
        return Err(super::app_error(
            "VALIDATION_ERROR",
            "Imported config has an empty name",
            false,
            false,
        ));
    }
    imported.name = trimmed_name.to_owned();
    imported.updated_at = current_unix_timestamp();

    let mut configs = read_configs_from_disk(&app).map_err(super::from_app_error)?;
    if let Some(existing) = configs
        .iter_mut()
        .find(|item| item.name.eq_ignore_ascii_case(&imported.name))
    {
        *existing = imported.clone();
    } else {
        configs.push(imported.clone());
    }

    configs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    write_configs_to_disk(&app, &configs).map_err(super::from_app_error)?;

    Ok(imported.name)
}

#[tauri::command]
pub(crate) fn config_watchdog_load(app: tauri::AppHandle) -> super::IpcResult<WatchdogConfigDto> {
    let (normalized, changed) = read_watchdog_config_with_migration(&app).map_err(super::from_app_error)?;
    if changed {
        write_watchdog_config_to_disk(&app, &normalized).map_err(super::from_app_error)?;
    }
    Ok(normalized)
}

#[tauri::command]
#[allow(non_snake_case)]
pub(crate) fn config_watchdog_upsert_mapping(
    app: tauri::AppHandle,
    appName: String,
    configName: String,
    icon: Option<String>,
) -> super::IpcResult<()> {
    let app_name_trimmed = appName.trim().to_lowercase();
    if app_name_trimmed.is_empty() {
        return Err(super::app_error(
            "VALIDATION_ERROR",
            "Application name cannot be empty",
            false,
            false,
        ));
    }

    let config_name_trimmed = configName.trim();
    if config_name_trimmed.is_empty() {
        return Err(super::app_error(
            "VALIDATION_ERROR",
            "Config name cannot be empty",
            false,
            false,
        ));
    }

    let mut watchdog = read_watchdog_config_from_disk(&app).map_err(super::from_app_error)?;
    watchdog.trigger_map.insert(
        app_name_trimmed,
        WatchdogTriggerMappingDto {
            config_name: config_name_trimmed.to_owned(),
            icon: icon
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned),
        },
    );
    write_watchdog_config_to_disk(&app, &watchdog).map_err(super::from_app_error)
}

#[tauri::command]
#[allow(non_snake_case)]
pub(crate) fn config_watchdog_remove_mapping(
    app: tauri::AppHandle,
    appName: String,
) -> super::IpcResult<()> {
    let app_name_trimmed = appName.trim().to_lowercase();
    if app_name_trimmed.is_empty() {
        return Err(super::app_error(
            "VALIDATION_ERROR",
            "Application name cannot be empty",
            false,
            false,
        ));
    }

    let mut watchdog = read_watchdog_config_from_disk(&app).map_err(super::from_app_error)?;
    watchdog.trigger_map.remove(&app_name_trimmed);
    write_watchdog_config_to_disk(&app, &watchdog).map_err(super::from_app_error)
}

#[tauri::command]
#[allow(non_snake_case)]
pub(crate) fn config_set_sticky_mode(
    app: tauri::AppHandle,
    configName: String,
    mode: u8,
) -> super::IpcResult<()> {
    let config_name_trimmed = configName.trim();
    if config_name_trimmed.is_empty() {
        return Err(super::app_error(
            "VALIDATION_ERROR",
            "Config name cannot be empty",
            false,
            false,
        ));
    }

    if mode > 2 {
        return Err(super::app_error(
            "VALIDATION_ERROR",
            "Live mode must be 0, 1, or 2",
            false,
            false,
        ));
    }

    let mut watchdog = read_watchdog_config_from_disk(&app).map_err(super::from_app_error)?;
    let key = config_name_trimmed.to_lowercase();
    watchdog.sticky_modes.insert(key, mode);

    write_watchdog_config_to_disk(&app, &watchdog).map_err(super::from_app_error)
}

#[tauri::command]
#[allow(non_snake_case)]
pub(crate) fn config_create_desktop_shortcut(
    app: tauri::AppHandle,
    configName: String,
) -> super::IpcResult<()> {
    let trimmed = configName.trim();
    if trimmed.is_empty() {
        return Err(super::app_error(
            "VALIDATION_ERROR",
            "Config name cannot be empty",
            false,
            false,
        ));
    }

    let _ = app;
    let exe = std::env::current_exe().map_err(super::to_ipc_error)?;
    let working_dir = exe.parent().ok_or_else(|| {
        super::app_error(
            "INTERNAL",
            "failed to resolve executable parent directory",
            false,
            true,
        )
    })?;

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
        .creation_flags(0x08000000)
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(script)
        .output()
        .map_err(super::to_ipc_error)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(super::app_error(
            "INTERNAL",
            format!(
                "shortcut creation failed (status {}): {}{}{}",
                output.status,
                stderr,
                if !stderr.is_empty() && !stdout.is_empty() {
                    " | "
                } else {
                    ""
                },
                stdout
            ),
            false,
            false,
        ));
    }

    Ok(())
}
