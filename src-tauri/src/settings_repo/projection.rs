use crate::core::Ordering;
use crate::types::{
    AppError, AppSettings, RuntimeControlState, WatchdogConfigDto, WatchdogConfigOnDisk,
    WatchdogTriggerMappingDto, WatchdogTriggerMappingOnDisk,
};
use std::collections::HashMap;
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
        optimization_desired: *runtime
            .optimization_desired
            .read()
            .map_err(|_| AppError::Message("optimization desired state lock poisoned".to_owned()))?,
    })
}

pub(crate) fn save_runtime_settings(
    app: &tauri::AppHandle,
    runtime: &RuntimeControlState,
) -> Result<(), AppError> {
    let settings = snapshot_app_settings(runtime)?;
    super::io::save_settings(app, &settings)
}

pub(crate) fn normalize_watchdog_config(config: WatchdogConfigDto) -> (WatchdogConfigDto, bool) {
    let original = config.clone();
    let mut trigger_entries: Vec<(String, WatchdogTriggerMappingDto)> =
        config.trigger_map.into_iter().collect();
    trigger_entries.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.config_name.cmp(&b.1.config_name))
            .then(a.1.icon.cmp(&b.1.icon))
    });

    let mut trigger_map = HashMap::new();
    for (key, mapping) in trigger_entries {
        let normalized_key = key.trim().to_lowercase();
        let normalized_config_name = mapping.config_name.trim().to_owned();
        if normalized_key.is_empty() || normalized_config_name.is_empty() {
            continue;
        }
        let normalized_icon = mapping
            .icon
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        trigger_map.insert(
            normalized_key,
            WatchdogTriggerMappingDto {
                config_name: normalized_config_name,
                icon: normalized_icon,
            },
        );
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

    for config_name in on_disk.sticky_configs {
        let normalized_name = config_name.trim().to_lowercase();
        if normalized_name.is_empty() {
            continue;
        }
        sticky_modes.entry(normalized_name).or_insert(1);
    }

    let trigger_map = on_disk
        .trigger_map
        .into_iter()
        .map(|(app_name, mapping)| {
            let normalized = match mapping {
                WatchdogTriggerMappingOnDisk::LegacyConfigName(config_name) => {
                    WatchdogTriggerMappingDto {
                        config_name,
                        icon: None,
                    }
                }
                WatchdogTriggerMappingOnDisk::Mapping(value) => value,
            };
            (app_name, normalized)
        })
        .collect();

    Ok(WatchdogConfigDto {
        trigger_map,
        sticky_modes,
    })
}

