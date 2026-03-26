use crate::*;

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

    let mut trigger_entries: Vec<(&String, &String)> = watchdog.trigger_map.iter().collect();
    trigger_entries.sort_by(|a, b| a.0.cmp(b.0).then(a.1.cmp(b.1)));
    for (trigger, config_name) in trigger_entries {
        canonical.push_str("trg:");
        canonical.push_str(trigger);
        canonical.push_str("->");
        canonical.push_str(&config_name.to_lowercase());
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
            let matches_target = current.class == Some(*target_priority) || current.raw == Some(target_raw);

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

pub(crate) fn run_watchdog_tick(
    app: &tauri::AppHandle,
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

    // Sniper mode:
    // 1) precompute only trigger/config targets that can influence this tick;
    // 2) while scanning processes, index only matching apps.
    let mut tracked_config_keys: HashSet<String> = HashSet::new();
    for config_name in watchdog.trigger_map.values() {
        tracked_config_keys.insert(config_name.to_lowercase());
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

    if trigger_apps.is_empty() && target_apps.is_empty() {
        active_triggers.clear();
        return Ok(());
    }

    let mut system = System::new_all();
    system.refresh_processes(ProcessesToUpdate::All, true);

    let mut app_pid_index: HashMap<String, Vec<u32>> = HashMap::new();
    let mut running_triggers: HashSet<String> = HashSet::new();
    for (pid, process) in system.processes() {
        let app_name_lower = app_name_lower_from_process(process);

        if trigger_apps.contains(&app_name_lower) {
            running_triggers.insert(app_name_lower.clone());
        }

        if !target_apps.contains(&app_name_lower) {
            continue;
        }

        app_pid_index
            .entry(app_name_lower)
            .or_default()
            .push(pid.as_u32());
    }

    let new_triggers: Vec<String> = running_triggers
        .difference(active_triggers)
        .cloned()
        .collect();

    let mut enforced_this_cycle = HashSet::new();

    for trigger in &new_triggers {
        let Some(config_name) = watchdog.trigger_map.get(trigger) else {
            continue;
        };
        let config_key = config_name.to_lowercase();
        let Some(config) = config_lookup.get(&config_key).copied() else {
            continue;
        };

        enforce_config_on_running_processes(config, &app_pid_index);
        enforced_this_cycle.insert(config_key);
    }

    let mut triggers_by_config: HashMap<String, HashSet<String>> = HashMap::new();
    for (trigger_app, config_name) in &watchdog.trigger_map {
        triggers_by_config
            .entry(config_name.to_lowercase())
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

        enforce_config_on_running_processes(config, &app_pid_index);
        enforced_this_cycle.insert(config_key);
    }

    *active_triggers = running_triggers;

    Ok(())
}

pub(crate) fn spawn_watchdog_loop(app: tauri::AppHandle, state: RuntimeControlState) {
    thread::spawn(move || {
        let mut active_triggers: HashSet<String> = HashSet::new();
        let mut watchdog_generation: Option<String> = None;
        loop {
            if let Err(err) = run_memory_purge_tick(&state) {
                error!("Memory purge tick failed: {err}");
            }

            if state.watchdog_enabled.load(Ordering::Relaxed) {
                if let Err(err) =
                    run_watchdog_tick(&app, &mut active_triggers, &mut watchdog_generation)
                {
                    error!("Watchdog tick failed: {err}");
                }
            }
            thread::sleep(Duration::from_secs(5));
        }
    });
}
