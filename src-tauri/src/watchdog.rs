use crate::core::{error, warn, Arc, Duration, HashMap, HashSet, Mutex, Ordering};
use crate::memory_purge::run_memory_purge_tick;
use crate::process::{read_priority, read_sampler_snapshot, set_priority_for_pid};
use crate::settings_repo::{read_configs_from_disk, read_watchdog_config_from_disk};
use crate::types::{
    AppError, ConfigDto, PriorityClassDto, ProcessSamplerSnapshot, RuntimeControlState,
    WatchdogConfigDto,
};
fn watchdog_generation_key(configs: &[ConfigDto], watchdog: &WatchdogConfigDto) -> String {
    let mut canonical = String::new();

    let mut config_entries: Vec<(String, Vec<(String, PriorityClassDto)>)> = configs
        .iter()
        .map(|config| {
            let mut map_entries: Vec<(String, PriorityClassDto)> = config
                .config_map
                .iter()
                .map(|(app, priority)| (app.to_lowercase(), *priority))
                .collect();
            map_entries.sort_by(|a, b| a.0.cmp(&b.0));
            (config.name.to_lowercase(), map_entries)
        })
        .collect();
    config_entries.sort_by(|a, b| a.0.cmp(&b.0));

    for (config_name, targets) in config_entries {
        canonical.push_str("cfg:");
        canonical.push_str(&config_name);
        canonical.push('|');
        for (app_name, priority) in targets {
            canonical.push_str(&app_name);
            canonical.push('=');
            canonical.push_str(priority.label());
            canonical.push(';');
        }
        canonical.push('\n');
    }

    let mut trigger_entries: Vec<(&String, &crate::types::WatchdogTriggerMappingDto)> =
        watchdog.trigger_map.iter().collect();
    trigger_entries.sort_by(|a, b| {
        a.0.cmp(b.0)
            .then(a.1.config_name.cmp(&b.1.config_name))
            .then(a.1.icon.cmp(&b.1.icon))
    });
    for (trigger, mapping) in trigger_entries {
        canonical.push_str("trg:");
        canonical.push_str(trigger);
        canonical.push_str("->");
        canonical.push_str(&mapping.config_name.to_lowercase());
        canonical.push('\n');
    }

    let mut sticky_entries: Vec<(&String, &u8)> = watchdog.sticky_modes.iter().collect();
    sticky_entries.sort_by(|a, b| a.0.cmp(b.0));
    for (config_name, mode) in sticky_entries {
        canonical.push_str("stk:");
        canonical.push_str(&config_name.to_lowercase());
        canonical.push('=');
        canonical.push_str(&mode.to_string());
        canonical.push('\n');
    }

    blake3::hash(canonical.as_bytes()).to_hex().to_string()
}

pub(crate) fn enforce_config_on_running_processes(
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
                warn!(
                    "watchdog: skip pid {} for app '{}' in config '{}': access denied while reading priority",
                    pid, target_app, config.name
                );
                continue;
            }

            let target_raw = target_priority.to_windows_flag().0;
            let matches_target =
                current.class == Some(*target_priority) || current.raw == Some(target_raw);

            if !matches_target {
                if let Err(err) = set_priority_for_pid(*pid, *target_priority) {
                    warn!(
                        "watchdog: failed to apply priority '{}' to pid {} app '{}' in config '{}': {}",
                        target_priority.label(),
                        pid,
                        target_app,
                        config.name,
                        err
                    );
                }
            }
        }
    }
}

#[derive(Default)]
struct WatchdogTargets {
    trigger_apps: HashSet<String>,
    target_apps: HashSet<String>,
}

struct ScanIndexResult {
    app_pid_index: HashMap<String, Vec<u32>>,
    running_triggers: HashSet<String>,
}

#[derive(Default)]
struct WatchdogLoopState {
    active_triggers: HashSet<String>,
    watchdog_generation: Option<String>,
}

fn build_targets(
    watchdog: &WatchdogConfigDto,
    config_lookup: &HashMap<String, &ConfigDto>,
) -> WatchdogTargets {
    let mut tracked_config_keys: HashSet<String> = HashSet::new();
    for mapping in watchdog.trigger_map.values() {
        tracked_config_keys.insert(mapping.config_name.to_lowercase());
    }
    for (config_name, mode) in &watchdog.sticky_modes {
        if *mode > 0 {
            tracked_config_keys.insert(config_name.to_lowercase());
        }
    }

    let trigger_apps: HashSet<String> = watchdog.trigger_map.keys().cloned().collect();
    let mut target_apps: HashSet<String> = HashSet::new();
    for config_key in &tracked_config_keys {
        if let Some(config) = config_lookup.get(config_key).copied() {
            target_apps.extend(config.config_map.keys().map(|app| app.to_lowercase()));
        }
    }

    WatchdogTargets {
        trigger_apps,
        target_apps,
    }
}

fn scan_index(targets: &WatchdogTargets, snapshot: &ProcessSamplerSnapshot) -> ScanIndexResult {
    let mut app_pid_index: HashMap<String, Vec<u32>> = HashMap::new();
    let mut running_triggers: HashSet<String> = HashSet::new();

    for process in &snapshot.processes {
        let app_name_lower = &process.app_name_lower;

        if targets.trigger_apps.contains(app_name_lower) {
            running_triggers.insert(app_name_lower.clone());
        }

        if !targets.target_apps.contains(app_name_lower) {
            continue;
        }

        app_pid_index
            .entry(app_name_lower.clone())
            .or_default()
            .push(process.pid);
    }

    ScanIndexResult {
        app_pid_index,
        running_triggers,
    }
}

fn apply_enforcement(
    watchdog: &WatchdogConfigDto,
    config_lookup: &HashMap<String, &ConfigDto>,
    running_triggers: &HashSet<String>,
    app_pid_index: &HashMap<String, Vec<u32>>,
    active_triggers: &HashSet<String>,
) {
    let new_triggers: Vec<String> = running_triggers
        .difference(active_triggers)
        .cloned()
        .collect();

    let mut enforced_this_cycle = HashSet::new();

    for trigger in &new_triggers {
        let Some(mapping) = watchdog.trigger_map.get(trigger) else {
            continue;
        };
        let config_key = mapping.config_name.to_lowercase();
        let Some(config) = config_lookup.get(&config_key).copied() else {
            continue;
        };

        enforce_config_on_running_processes(config, app_pid_index);
        enforced_this_cycle.insert(config_key);
    }

    let mut triggers_by_config: HashMap<String, HashSet<String>> = HashMap::new();
    for (trigger_app, mapping) in &watchdog.trigger_map {
        triggers_by_config
            .entry(mapping.config_name.to_lowercase())
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

        enforce_config_on_running_processes(config, app_pid_index);
        enforced_this_cycle.insert(config_key);
    }
}

#[tracing::instrument(skip_all)]
pub(crate) fn run_watchdog_tick(
    app: &tauri::AppHandle,
    state: &RuntimeControlState,
    active_triggers: &mut HashSet<String>,
    watchdog_generation: &mut Option<String>,
) -> Result<(), AppError> {
    let configs = read_configs_from_disk(app)?;
    if configs.is_empty() {
        active_triggers.clear();
        *watchdog_generation = None;
        return Ok(());
    }

    let watchdog = read_watchdog_config_from_disk(app)?;
    let generation = watchdog_generation_key(&configs, &watchdog);
    if watchdog_generation.as_ref() != Some(&generation) {
        active_triggers.clear();
        *watchdog_generation = Some(generation);
    }

    let config_lookup: HashMap<String, &ConfigDto> = configs
        .iter()
        .map(|config| (config.name.to_lowercase(), config))
        .collect();

    if config_lookup.is_empty() {
        return Ok(());
    }

    let targets = build_targets(&watchdog, &config_lookup);
    if targets.trigger_apps.is_empty() && targets.target_apps.is_empty() {
        active_triggers.clear();
        return Ok(());
    }

    let snapshot = read_sampler_snapshot(state);
    let scan = scan_index(&targets, &snapshot);
    apply_enforcement(
        &watchdog,
        &config_lookup,
        &scan.running_triggers,
        &scan.app_pid_index,
        active_triggers,
    );

    *active_triggers = scan.running_triggers;
    Ok(())
}

pub(crate) fn spawn_watchdog_loop(app: tauri::AppHandle, state: RuntimeControlState) {
    tauri::async_runtime::spawn(async move {
        let loop_state = Arc::new(Mutex::new(WatchdogLoopState::default()));
        let mut shutdown_rx = state.shutdown_tx.subscribe();
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let app_for_tick = app.clone();
                    let state_for_tick = state.clone();
                    let loop_state_for_tick = loop_state.clone();
                    let watchdog_enabled = state.watchdog_enabled.load(Ordering::Relaxed);
                    let mut worker = tokio::task::spawn_blocking(move || -> Result<(), AppError> {
                        run_memory_purge_tick(&app_for_tick, &state_for_tick)?;
                        if watchdog_enabled {
                            let mut guard = match loop_state_for_tick.lock() {
                                Ok(guard) => guard,
                                Err(poisoned) => {
                                    warn!("watchdog loop state lock poisoned; recovering");
                                    poisoned.into_inner()
                                }
                            };
                            let WatchdogLoopState {
                                active_triggers,
                                watchdog_generation,
                            } = &mut *guard;
                            run_watchdog_tick(
                                &app_for_tick,
                                &state_for_tick,
                                active_triggers,
                                watchdog_generation,
                            )?;
                        }
                        Ok(())
                    });
                    tokio::select! {
                        worker_result = &mut worker => {
                            match worker_result {
                                Ok(Ok(())) => {}
                                Ok(Err(err)) => error!("Watchdog tick failed: {err}"),
                                Err(err) => error!("Watchdog worker join failed: {err}"),
                            }
                        }
                        _ = tokio::time::sleep(Duration::from_secs(30)) => {
                            worker.abort();
                            error!("Watchdog tick timed out after 30 seconds");
                        }
                    }
                }
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        break;
                    }
                }
            }
        }
    });
}

