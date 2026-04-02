use crate::core::{
    error, info, is_access_denied, is_running_as_admin, last_error_code, warn, Duration, HashMap,
    Mutex, OnceLock, Path, PathBuf, ProcessesToUpdate, System, UNIX_EPOCH,
    ARG_APPLY_CONFIG, ICON_COLLISION_GUARD_MAX_ITEMS, GetPriorityClass, OpenProcess,
    SetPriorityClass, TerminateProcess, PROCESS_ACCESS_RIGHTS, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_SET_INFORMATION, PROCESS_TERMINATE,
};
use crate::settings_repo::{headless_configs_file_path, read_configs_from_path};
use crate::types::{
    AppError, OwnedHandle, PriorityClassDto, PriorityRead, ProcessDeltaPayload, ProcessRowDto,
    ProcessSamplerSnapshot, RuntimeControlState, SampledProcess,
};
pub(crate) fn open_process(
    pid: u32,
    access: PROCESS_ACCESS_RIGHTS,
    context: &'static str,
) -> Result<OwnedHandle, AppError> {
    unsafe {
        match OpenProcess(access, false, pid) {
            Ok(handle) => Ok(OwnedHandle(handle)),
            Err(_) => {
                let code = last_error_code();
                if is_access_denied(code) {
                    Err(AppError::AccessDenied { pid, context })
                } else {
                    Err(AppError::WinApi { pid, context, code })
                }
            }
        }
    }
}

pub(crate) fn read_priority(pid: u32) -> PriorityRead {
    let handle = match open_process(
        pid,
        PROCESS_QUERY_LIMITED_INFORMATION,
        "opening process for read",
    ) {
        Ok(handle) => handle,
        Err(AppError::AccessDenied { .. }) => {
            return PriorityRead {
                class: None,
                raw: None,
                access_denied: true,
                label: "Access denied".to_owned(),
            };
        }
        Err(err) => {
            return PriorityRead {
                class: None,
                raw: None,
                access_denied: false,
                label: err.to_string(),
            };
        }
    };

    let raw = unsafe { GetPriorityClass(handle.raw()) };
    if raw == 0 {
        let code = last_error_code();
        if is_access_denied(code) {
            return PriorityRead {
                class: None,
                raw: None,
                access_denied: true,
                label: "Access denied".to_owned(),
            };
        }

        return PriorityRead {
            class: None,
            raw: None,
            access_denied: false,
            label: format!("GetPriorityClass failed ({code})"),
        };
    }

    let class = PriorityClassDto::from_windows_raw(raw);
    PriorityRead {
        class,
        raw: Some(raw),
        access_denied: false,
        label: class
            .map(PriorityClassDto::label)
            .unwrap_or("Unknown")
            .to_owned(),
    }
}

pub(crate) fn set_priority_for_pid(pid: u32, priority: PriorityClassDto) -> Result<(), AppError> {
    let access = PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION;
    let handle = open_process(pid, access, "opening process for write")?;

    unsafe {
        match SetPriorityClass(handle.raw(), priority.to_windows_flag()) {
            Ok(()) => Ok(()),
            Err(_) => {
                let code = last_error_code();
                if is_access_denied(code) {
                    Err(AppError::AccessDenied {
                        pid,
                        context: "setting priority",
                    })
                } else {
                    Err(AppError::WinApi {
                        pid,
                        context: "setting priority",
                        code,
                    })
                }
            }
        }
    }
}

pub(crate) fn kill_process_by_pid(pid: u32) -> Result<(), AppError> {
    let access = PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION;
    let handle = open_process(pid, access, "opening process for terminate")?;

    unsafe {
        match TerminateProcess(handle.raw(), 1) {
            Ok(()) => Ok(()),
            Err(_) => {
                let code = last_error_code();
                if is_access_denied(code) {
                    Err(AppError::AccessDenied {
                        pid,
                        context: "terminating process",
                    })
                } else {
                    Err(AppError::WinApi {
                        pid,
                        context: "terminating process",
                        code,
                    })
                }
            }
        }
    }
}

fn build_sampler_snapshot(system: &System) -> ProcessSamplerSnapshot {
    let mut processes = Vec::with_capacity(system.processes().len());
    for (pid, process) in system.processes() {
        let exe_path = process.exe().map(PathBuf::from);
        let app_name = exe_path
            .as_deref()
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| process.name().to_string_lossy().into_owned());

        processes.push(SampledProcess {
            pid: pid.as_u32(),
            app_name_lower: app_name.to_lowercase(),
            app_name,
            exe_path,
            memory_bytes: process.memory(),
        });
    }

    ProcessSamplerSnapshot { processes }
}

fn write_sampler_snapshot(state: &RuntimeControlState, snapshot: ProcessSamplerSnapshot) {
    match state.process_sampler_snapshot.write() {
        Ok(mut guard) => {
            *guard = snapshot;
        }
        Err(poisoned) => {
            warn!("process sampler snapshot lock poisoned; recovering state");
            let mut guard = poisoned.into_inner();
            *guard = snapshot;
        }
    }
}

pub(crate) fn read_sampler_snapshot(state: &RuntimeControlState) -> ProcessSamplerSnapshot {
    match state.process_sampler_snapshot.read() {
        Ok(snapshot) => snapshot.clone(),
        Err(_) => ProcessSamplerSnapshot::default(),
    }
}

pub(crate) fn spawn_process_sampler_loop(state: RuntimeControlState) {
    tauri::async_runtime::spawn(async move {
        let mut system = System::new_all();
        let mut shutdown_rx = state.shutdown_tx.subscribe();
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        system.refresh_processes(ProcessesToUpdate::All, true);
        write_sampler_snapshot(&state, build_sampler_snapshot(&system));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    system.refresh_processes(ProcessesToUpdate::All, true);
                    write_sampler_snapshot(&state, build_sampler_snapshot(&system));
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

fn icon_collision_guard() -> &'static Mutex<HashMap<String, String>> {
    static ICON_COLLISION_GUARD: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    ICON_COLLISION_GUARD.get_or_init(|| Mutex::new(HashMap::new()))
}

fn blake3_key_16(input: &str) -> String {
    let digest = blake3::hash(input.as_bytes());
    digest.to_hex().to_string()[..16].to_owned()
}

pub(crate) fn icon_identity(
    app_name: &str,
    icon_path: Option<&Path>,
    fallback_pid: Option<u32>,
) -> String {
    match icon_path {
        Some(path) => {
            let normalized_path = path.to_string_lossy().to_lowercase();
            let (file_size, modified_ns) = match std::fs::metadata(path) {
                Ok(metadata) => {
                    let size = metadata.len();
                    let modified = metadata
                        .modified()
                        .ok()
                        .and_then(|ts| ts.duration_since(UNIX_EPOCH).ok())
                        .map(|dur| dur.as_nanos().to_string())
                        .unwrap_or_else(|| "na".to_owned());
                    (size.to_string(), modified)
                }
                Err(_) => ("na".to_owned(), "na".to_owned()),
            };
            format!("exe|{normalized_path}|{file_size}|{modified_ns}")
        }
        None => {
            let app = app_name.trim().to_lowercase();
            let pid = fallback_pid.unwrap_or(0);
            format!("weak|app:{app}|pid:{pid}")
        }
    }
}

pub(crate) fn icon_key_from_identity(identity: &str) -> String {
    // Deterministic fast hash key for IPC delta cache. On theoretical collisions,
    // re-salt with a numeric suffix until the key slot maps to this identity.
    let mut attempt = 0u32;
    loop {
        let salted = if attempt == 0 {
            identity.to_owned()
        } else {
            format!("{identity}#{attempt}")
        };
        let key = blake3_key_16(&salted);

        let Ok(mut guard) = icon_collision_guard().lock() else {
            return key;
        };

        if guard.len() >= ICON_COLLISION_GUARD_MAX_ITEMS {
            guard.clear();
            warn!(
                "icon collision guard reached {} entries; cache cleared",
                ICON_COLLISION_GUARD_MAX_ITEMS
            );
        }

        match guard.get(&key) {
            Some(existing) if existing == identity => return key,
            Some(_) => {
                attempt = attempt.saturating_add(1);
            }
            None => {
                guard.insert(key.clone(), identity.to_owned());
                return key;
            }
        }
    }
}

fn build_process_rows_from_snapshot(
    snapshot: &ProcessSamplerSnapshot,
) -> (
    HashMap<u32, ProcessRowDto>,
    HashMap<String, PathBuf>,
    bool,
) {
    let mut rows = HashMap::with_capacity(snapshot.processes.len());
    let mut icon_sources = HashMap::new();
    let mut needs_elevation = false;

    for process in &snapshot.processes {
        let priority = read_priority(process.pid);
        if priority.access_denied {
            needs_elevation = true;
        }

        let identity = icon_identity(
            &process.app_name,
            process.exe_path.as_deref(),
            Some(process.pid),
        );
        let icon_key = icon_key_from_identity(&identity);
        if let Some(path) = process.exe_path.clone() {
            icon_sources.insert(icon_key.clone(), path);
        }

        rows.insert(
            process.pid,
            ProcessRowDto {
                pid: process.pid,
                app_name: process.app_name.clone(),
                icon_key,
                memory_bytes: process.memory_bytes,
                priority: priority.class,
                priority_raw: priority.raw,
                priority_label: priority.label,
            },
        );
    }

    (rows, icon_sources, needs_elevation)
}

fn diff_process_rows(
    last_rows: &HashMap<u32, ProcessRowDto>,
    current_rows: &HashMap<u32, ProcessRowDto>,
) -> (Vec<ProcessRowDto>, Vec<ProcessRowDto>, Vec<u32>) {
    let mut added = Vec::new();
    let mut updated = Vec::new();
    let mut removed = Vec::new();

    for (pid, row) in current_rows {
        match last_rows.get(pid) {
            None => added.push(row.clone()),
            Some(previous) if previous != row => updated.push(row.clone()),
            _ => {}
        }
    }

    for pid in last_rows.keys() {
        if !current_rows.contains_key(pid) {
            removed.push(*pid);
        }
    }

    added.sort_by_key(|item| item.pid);
    updated.sort_by_key(|item| item.pid);
    removed.sort_unstable();
    (added, updated, removed)
}

#[tracing::instrument(skip_all)]
pub(crate) fn compute_process_delta(runtime: &RuntimeControlState) -> ProcessDeltaPayload {
    let snapshot = read_sampler_snapshot(runtime);
    let (current_rows, icon_sources, needs_elevation) = build_process_rows_from_snapshot(&snapshot);
    let mut state = match runtime.process_delta_state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            warn!("process delta state lock poisoned; recovering state");
            poisoned.into_inner()
        }
    };
    let sequence = state.sequence.wrapping_add(1);
    let (added, updated, removed) = diff_process_rows(&state.last_rows, &current_rows);
    state.last_rows = current_rows;
    state.icon_sources = icon_sources;
    state.sequence = sequence;

    ProcessDeltaPayload {
        sequence,
        added,
        updated,
        removed,
        needs_elevation,
        is_elevated: is_running_as_admin(),
    }
}

pub(crate) fn resolve_icon_path_for_key(
    runtime: &RuntimeControlState,
    icon_key: &str,
) -> Option<PathBuf> {
    if let Ok(state) = runtime.process_delta_state.lock() {
        if let Some(path) = state.icon_sources.get(icon_key) {
            return Some(path.clone());
        }
    }

    let snapshot = read_sampler_snapshot(runtime);
    for process in snapshot.processes {
        let identity = icon_identity(
            &process.app_name,
            process.exe_path.as_deref(),
            Some(process.pid),
        );
        let key = icon_key_from_identity(&identity);
        if key == icon_key {
            return process.exe_path;
        }
    }

    None
}

pub(crate) fn parse_apply_config_arg() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let idx = args.iter().position(|arg| arg == ARG_APPLY_CONFIG)?;
    args.get(idx + 1).cloned()
}

pub(crate) fn app_name_from_process(process: &sysinfo::Process) -> String {
    process
        .exe()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| process.name().to_string_lossy().into_owned())
}

pub(crate) fn apply_config_headless(config_name: &str, app_identifier: &str) -> Result<(), AppError> {
    let configs_path = headless_configs_file_path(app_identifier)?;
    let configs = read_configs_from_path(&configs_path)?;
    let Some(config) = configs
        .iter()
        .find(|item| item.name.eq_ignore_ascii_case(config_name))
    else {
        return Err(AppError::Message(format!(
            "config '{config_name}' was not found"
        )));
    };

    let mut system = System::new_all();
    system.refresh_processes(ProcessesToUpdate::All, true);

    let mut matched = 0usize;
    let mut applied = 0usize;
    let mut failed = 0usize;

    for (pid, process) in system.processes() {
        let app_name = app_name_from_process(process);
        let Some(priority) = config.config_map.get(&app_name).copied() else {
            continue;
        };

        matched += 1;
        match set_priority_for_pid(pid.as_u32(), priority) {
            Ok(()) => applied += 1,
            Err(err) => {
                failed += 1;
                error!(
                    "[headless] failed to apply '{app_name}' for pid {}: {err}",
                    pid.as_u32()
                );
            }
        }
    }

    info!(
        "[headless] config '{}' done: matched={}, applied={}, failed={}",
        config.name, matched, applied, failed
    );
    Ok(())
}

pub(crate) fn extract_icon_png_bytes(path: &Path) -> Result<Vec<u8>, AppError> {
    crate::icon_service::extract_icon_png_bytes(path)
}

#[cfg(test)]
mod tests {
    use crate::process::{compute_process_delta, diff_process_rows};
    use crate::types::{ProcessRowDto, ProcessSamplerSnapshot, RuntimeControlState, SampledProcess};
    use std::collections::HashMap;

    fn row(pid: u32, memory_bytes: u64, priority_label: &str) -> ProcessRowDto {
        ProcessRowDto {
            pid,
            app_name: "game.exe".to_owned(),
            icon_key: "icon".to_owned(),
            memory_bytes,
            priority: None,
            priority_raw: None,
            priority_label: priority_label.to_owned(),
        }
    }

    #[test]
    fn diff_process_rows_detects_added_updated_removed() {
        let mut last_rows = HashMap::new();
        last_rows.insert(1, row(1, 100, "Normal"));
        last_rows.insert(2, row(2, 200, "Normal"));

        let mut current_rows = HashMap::new();
        current_rows.insert(2, row(2, 250, "High"));
        current_rows.insert(3, row(3, 300, "Normal"));

        let (added, updated, removed) = diff_process_rows(&last_rows, &current_rows);
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].pid, 3);
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].pid, 2);
        assert_eq!(removed, vec![1]);
    }

    #[test]
    fn diff_process_rows_ignores_unchanged_rows() {
        let mut last_rows = HashMap::new();
        let snapshot = row(10, 512, "Normal");
        last_rows.insert(10, snapshot.clone());

        let mut current_rows = HashMap::new();
        current_rows.insert(10, snapshot);

        let (added, updated, removed) = diff_process_rows(&last_rows, &current_rows);
        assert!(added.is_empty());
        assert!(updated.is_empty());
        assert!(removed.is_empty());
    }

    #[test]
    fn compute_process_delta_recovers_poisoned_lock_and_keeps_sequence_moving() {
        let runtime = RuntimeControlState::default();
        {
            let mut sampler = runtime
                .process_sampler_snapshot
                .write()
                .expect("sampler snapshot write lock");
            *sampler = ProcessSamplerSnapshot {
                processes: vec![SampledProcess {
                    pid: std::process::id(),
                    app_name: "test.exe".to_owned(),
                    app_name_lower: "test.exe".to_owned(),
                    exe_path: None,
                    memory_bytes: 0,
                }],
            };
        }

        {
            let poisoned = runtime.process_delta_state.clone();
            let _ = std::panic::catch_unwind(move || {
                let _guard = poisoned.lock().expect("delta lock");
                panic!("poison process delta lock");
            });
        }

        let first = compute_process_delta(&runtime);
        let second = compute_process_delta(&runtime);
        assert!(second.sequence > first.sequence);
    }
}

