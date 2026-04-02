use crate::core::{
    error, fs, Duration, OnceLock, Path, PathBuf, PCWSTR, ReplaceFileW,
    REPLACEFILE_WRITE_THROUGH, RwLock, SystemTime, UNIX_EPOCH, Write,
};
use crate::elevation::to_wide;
use crate::types::{AppError, AppSettings, ConfigDto, WatchdogConfigDto};
use tauri::Manager;
static CONFIG_IO_LOCK: OnceLock<RwLock<()>> = OnceLock::new();
static SETTINGS_WRITE_TX: OnceLock<tokio::sync::mpsc::UnboundedSender<SettingsWriteMessage>> =
    OnceLock::new();
const SETTINGS_WRITE_DEBOUNCE_MS: u64 = 1500;

enum SettingsWriteMessage {
    Update { path: PathBuf, data: String },
    Flush(tokio::sync::oneshot::Sender<Result<(), String>>),
    Shutdown,
}

pub(crate) fn config_io_lock() -> &'static RwLock<()> {
    CONFIG_IO_LOCK.get_or_init(|| RwLock::new(()))
}

fn write_settings_json_immediate(path: &Path, data: &str) -> Result<(), AppError> {
    let _guard = config_io_lock()
        .write()
        .map_err(|_| AppError::Message("config I/O lock poisoned".to_owned()))?;
    atomic_write_json(path, data)
}

async fn settings_write_behind_loop(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<SettingsWriteMessage>,
) {
    let mut pending: Option<(PathBuf, String)> = None;
    let debounce = Duration::from_millis(SETTINGS_WRITE_DEBOUNCE_MS);
    let mut deadline: Option<tokio::time::Instant> = None;

    loop {
        if let Some(next_flush_at) = deadline {
            tokio::select! {
                _ = tokio::time::sleep_until(next_flush_at) => {
                    if let Some((path, data)) = pending.take() {
                        if let Err(err) = write_settings_json_immediate(&path, &data) {
                            error!("settings write-behind flush failed: {err}");
                        }
                    }
                    deadline = None;
                }
                maybe_message = rx.recv() => {
                    let Some(message) = maybe_message else {
                        break;
                    };
                    match message {
                        SettingsWriteMessage::Update { path, data } => {
                            pending = Some((path, data));
                            deadline = Some(tokio::time::Instant::now() + debounce);
                        }
                        SettingsWriteMessage::Flush(reply_tx) => {
                            let result = if let Some((path, data)) = pending.take() {
                                write_settings_json_immediate(&path, &data).map_err(|err| err.to_string())
                            } else {
                                Ok(())
                            };
                            let _ = reply_tx.send(result);
                            deadline = None;
                        }
                        SettingsWriteMessage::Shutdown => {
                            if let Some((path, data)) = pending.take() {
                                if let Err(err) = write_settings_json_immediate(&path, &data) {
                                    error!("settings write-behind shutdown flush failed: {err}");
                                }
                            }
                            break;
                        }
                    }
                }
            }
        } else {
            let Some(message) = rx.recv().await else {
                break;
            };
            match message {
                SettingsWriteMessage::Update { path, data } => {
                    pending = Some((path, data));
                    deadline = Some(tokio::time::Instant::now() + debounce);
                }
                SettingsWriteMessage::Flush(reply_tx) => {
                    let _ = reply_tx.send(Ok(()));
                }
                SettingsWriteMessage::Shutdown => break,
            }
        }
    }
}

fn settings_write_tx() -> &'static tokio::sync::mpsc::UnboundedSender<SettingsWriteMessage> {
    SETTINGS_WRITE_TX.get_or_init(|| {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tauri::async_runtime::spawn(settings_write_behind_loop(rx));
        tx
    })
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
            let _ = fs::remove_file(&temp_path);
            return Err(AppError::Message(format!(
                "failed to atomically replace '{}' with '{}'",
                path.display(),
                temp_path.display()
            )));
        }
    } else {
        if let Err(e) = fs::rename(&temp_path, path) {
            let _ = fs::remove_file(&temp_path);
            return Err(AppError::Message(format!(
                "failed to rename temp file '{}' to '{}': {e}",
                temp_path.display(),
                path.display()
            )));
        }
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
    let file_path = settings_file_path(app)?;
    let data = serde_json::to_string_pretty(settings)
        .map_err(|e| AppError::Message(format!("failed to serialize settings: {e}")))?;
    settings_write_tx()
        .send(SettingsWriteMessage::Update {
            path: file_path,
            data,
        })
        .map_err(|_| AppError::Message("settings write-behind channel closed".to_owned()))
}

pub(crate) fn flush_settings_write_behind() -> Result<(), AppError> {
    let Some(tx) = SETTINGS_WRITE_TX.get() else {
        return Ok(());
    };

    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    tx.send(SettingsWriteMessage::Flush(reply_tx))
        .map_err(|_| AppError::Message("settings write-behind channel closed".to_owned()))?;

    tauri::async_runtime::block_on(async {
        reply_rx
            .await
            .map_err(|_| AppError::Message("settings flush response dropped".to_owned()))?
            .map_err(AppError::Message)
    })
}

pub(crate) fn shutdown_settings_write_behind() {
    if let Some(tx) = SETTINGS_WRITE_TX.get() {
        let _ = tx.send(SettingsWriteMessage::Shutdown);
    }
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

    let parsed = super::projection::parse_watchdog_config(&content)
        .map_err(|e| AppError::Message(format!("failed to parse watchdog config: {e}")))?;
    let (normalized, changed) = super::projection::normalize_watchdog_config(parsed);
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

