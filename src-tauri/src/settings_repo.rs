use crate::*;

static CONFIG_IO_LOCK: OnceLock<RwLock<()>> = OnceLock::new();

pub(crate) fn config_io_lock() -> &'static RwLock<()> {
    CONFIG_IO_LOCK.get_or_init(|| RwLock::new(()))
}

pub(crate) fn atomic_write_json(path: &Path, data: &str) -> Result<(), AppError> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::Message(format!("path '{}' has no parent", path.display())))?;
    fs::create_dir_all(parent)
        .map_err(|e| AppError::Message(format!("failed to create directory '{}': {e}", parent.display())))?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::Message(format!("invalid filename '{}': not UTF-8", path.display())))?;

    let temp_path = parent.join(format!("{}.tmp", file_name));

    {
        let mut file = fs::File::create(&temp_path).map_err(|e| {
            AppError::Message(format!(
                "failed to create temp file '{}': {e}",
                temp_path.display()
            ))
        })?;
        file.write_all(data.as_bytes()).map_err(|e| {
            AppError::Message(format!("failed to write temp file '{}': {e}", temp_path.display()))
        })?;
        file.sync_all().map_err(|e| {
            AppError::Message(format!("failed to sync temp file '{}': {e}", temp_path.display()))
        })?;
    }

    if path.exists() {
        // Atomic replace on Windows without delete-then-rename gap.
        let replaced = unsafe {
            ReplaceFileW(
                PCWSTR(to_wide(path.as_os_str()).as_ptr()),
                PCWSTR(to_wide(temp_path.as_os_str()).as_ptr()),
                PCWSTR::null(),
                REPLACEFILE_WRITE_THROUGH,
                None,
                None,
            )
        }
        .is_ok();

        if !replaced {
            return Err(AppError::Message(format!(
                "failed to atomically replace '{}' with '{}'",
                path.display(),
                temp_path.display()
            )));
        }
    } else {
        fs::rename(&temp_path, path).map_err(|e| {
            AppError::Message(format!(
                "failed to rename temp file '{}' to '{}': {e}",
                temp_path.display(),
                path.display()
            ))
        })?;
    }

    Ok(())
}

pub(crate) fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

pub(crate) fn config_storage_dir(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::Message(format!("failed to resolve app config dir: {e}")))?;
    let storage_dir = config_dir.join("configs");
    fs::create_dir_all(&storage_dir)
        .map_err(|e| AppError::Message(format!("failed to create config dir: {e}")))?;
    Ok(storage_dir)
}

pub(crate) fn configs_file_path(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    Ok(config_storage_dir(app)?.join("configs.json"))
}

pub(crate) fn watchdog_file_path(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    Ok(config_storage_dir(app)?.join("config_watchdog.json"))
}

pub(crate) fn settings_file_path(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    Ok(config_storage_dir(app)?.join("settings.json"))
}

pub(crate) fn load_settings(app: &tauri::AppHandle) -> Result<AppSettings, AppError> {
    let _guard = config_io_lock()
        .read()
        .map_err(|_| AppError::Message("config I/O lock poisoned".to_owned()))?;
    let file_path = settings_file_path(app)?;
    if !file_path.exists() {
        return Ok(AppSettings::default());
    }

    let content = fs::read_to_string(&file_path)
        .map_err(|e| AppError::Message(format!("failed to read settings: {e}")))?;

    serde_json::from_str::<AppSettings>(&content)
        .map_err(|e| AppError::Message(format!("failed to parse settings: {e}")))
}

pub(crate) fn save_settings(app: &tauri::AppHandle, settings: &AppSettings) -> Result<(), AppError> {
    let _guard = config_io_lock()
        .write()
        .map_err(|_| AppError::Message("config I/O lock poisoned".to_owned()))?;
    let file_path = settings_file_path(app)?;
    let data = serde_json::to_string_pretty(settings)
        .map_err(|e| AppError::Message(format!("failed to serialize settings: {e}")))?;
    atomic_write_json(&file_path, &data)
}

pub(crate) fn snapshot_app_settings(runtime: &RuntimeControlState) -> Result<AppSettings, AppError> {
    let turbo_timer_enabled = runtime
        .timer_resolution
        .lock()
        .map_err(|_| AppError::Message("timer state lock poisoned".to_owned()))?
        .requested_100ns
        .is_some();
    let memory_purge_config = *runtime
        .memory_purge_config
        .read()
        .map_err(|_| AppError::Message("memory purge config lock poisoned".to_owned()))?;

    Ok(AppSettings {
        turbo_timer_enabled,
        watchdog_enabled: runtime.watchdog_enabled.load(Ordering::Relaxed),
        minimize_to_tray_enabled: runtime.minimize_to_tray_enabled.load(Ordering::Relaxed),
        memory_purge_config,
    })
}

pub(crate) fn save_runtime_settings(
    app: &tauri::AppHandle,
    runtime: &RuntimeControlState,
) -> Result<(), AppError> {
    let settings = snapshot_app_settings(runtime)?;
    save_settings(app, &settings)
}

pub(crate) fn read_configs_from_disk(app: &tauri::AppHandle) -> Result<Vec<ConfigDto>, AppError> {
    let _guard = config_io_lock()
        .read()
        .map_err(|_| AppError::Message("config I/O lock poisoned".to_owned()))?;
    let file_path = configs_file_path(app)?;
    read_configs_from_path(&file_path)
}

pub(crate) fn read_configs_from_path(file_path: &Path) -> Result<Vec<ConfigDto>, AppError> {
    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&file_path)
        .map_err(|e| AppError::Message(format!("failed to read configs: {e}")))?;

    serde_json::from_str::<Vec<ConfigDto>>(&content)
        .map_err(|e| AppError::Message(format!("failed to parse configs: {e}")))
}

pub(crate) fn read_watchdog_config_from_disk(app: &tauri::AppHandle) -> Result<WatchdogConfigDto, AppError> {
    let (normalized, _) = read_watchdog_config_with_migration(app)?;
    Ok(normalized)
}

pub(crate) fn read_watchdog_config_with_migration(
    app: &tauri::AppHandle,
) -> Result<(WatchdogConfigDto, bool), AppError> {
    let _guard = config_io_lock()
        .read()
        .map_err(|_| AppError::Message("config I/O lock poisoned".to_owned()))?;
    let file_path = watchdog_file_path(app)?;
    if !file_path.exists() {
        return Ok((WatchdogConfigDto::default(), false));
    }

    let content = fs::read_to_string(&file_path)
        .map_err(|e| AppError::Message(format!("failed to read watchdog config: {e}")))?;

    let parsed = parse_watchdog_config(&content)
        .map_err(|e| AppError::Message(format!("failed to parse watchdog config: {e}")))?;
    let (normalized, changed) = normalize_watchdog_config(parsed);
    Ok((normalized, changed))
}

pub(crate) fn write_watchdog_config_to_disk(
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

pub(crate) fn normalize_watchdog_config(config: WatchdogConfigDto) -> (WatchdogConfigDto, bool) {
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

pub(crate) fn parse_watchdog_config(content: &str) -> Result<WatchdogConfigDto, serde_json::Error> {
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

pub(crate) fn headless_configs_file_path(app_identifier: &str) -> Result<PathBuf, AppError> {
    let base_dir = std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("LOCALAPPDATA"))
        .ok_or_else(|| AppError::Message("failed to resolve APPDATA/LOCALAPPDATA".to_owned()))?;
    let identifier = app_identifier.trim();
    if identifier.is_empty() {
        return Err(AppError::Message(
            "application identifier cannot be empty for headless path resolution".to_owned(),
        ));
    }
    let storage_dir = PathBuf::from(base_dir).join(identifier).join("configs");
    fs::create_dir_all(&storage_dir)
        .map_err(|e| AppError::Message(format!("failed to create config dir: {e}")))?;
    Ok(storage_dir.join("configs.json"))
}

pub(crate) fn write_configs_to_disk(app: &tauri::AppHandle, configs: &[ConfigDto]) -> Result<(), AppError> {
    let _guard = config_io_lock()
        .write()
        .map_err(|_| AppError::Message("config I/O lock poisoned".to_owned()))?;
    let file_path = configs_file_path(app)?;
    let data = serde_json::to_string_pretty(configs)
        .map_err(|e| AppError::Message(format!("failed to serialize configs: {e}")))?;
    atomic_write_json(&file_path, &data)
}
