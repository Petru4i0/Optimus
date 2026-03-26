use crate::*;

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

pub(crate) fn gather_process_groups() -> ProcessListResponse {
    gather_process_groups_with_known_icons(None)
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

pub(crate) fn gather_process_groups_with_known_icons(
    known_icon_keys: Option<&HashSet<String>>,
) -> ProcessListResponse {
    let mut system = System::new_all();
    system.refresh_processes(ProcessesToUpdate::All, true);

    let mut grouped: BTreeMap<String, GroupAccumulator> = BTreeMap::new();
    let mut needs_elevation = false;

    for (pid, process) in system.processes() {
        let exe_path = process.exe().map(PathBuf::from);
        let app_name = exe_path
            .as_deref()
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| process.name().to_string_lossy().into_owned());

        let priority = read_priority(pid.as_u32());
        if priority.access_denied {
            needs_elevation = true;
        }

        let entry = grouped.entry(app_name).or_insert_with(|| GroupAccumulator {
            icon_path: None,
            processes: Vec::new(),
        });

        if entry.icon_path.is_none() {
            entry.icon_path = exe_path;
        }

        entry.processes.push(ProcessDto {
            pid: pid.as_u32(),
            memory_bytes: process.memory(),
            priority: priority.class,
            priority_raw: priority.raw,
            priority_label: priority.label,
        });
    }

    let mut groups = Vec::with_capacity(grouped.len());
    for (app_name, mut group) in grouped {
        group.processes.sort_by_key(|proc| proc.pid);
        let fallback_pid = group.processes.first().map(|proc| proc.pid);
        let icon_identity = icon_identity(&app_name, group.icon_path.as_deref(), fallback_pid);
        let icon_key = icon_key_from_identity(&icon_identity);
        let should_include_icon = known_icon_keys
            .map(|keys| !keys.contains(&icon_key))
            .unwrap_or(true);
        let icon_base64 = if should_include_icon {
            group
                .icon_path
                .as_deref()
                .and_then(|path| extract_icon_base64(path).ok())
        } else {
            None
        };

        groups.push(ProcessGroupDto {
            total: group.processes.len(),
            app_name,
            icon_key,
            icon_base64,
            processes: group.processes,
        });
    }

    ProcessListResponse {
        groups,
        needs_elevation,
        is_elevated: is_running_as_admin(),
    }
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

pub(crate) fn app_name_lower_from_process(process: &sysinfo::Process) -> String {
    app_name_from_process(process).to_lowercase()
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

pub(crate) fn extract_icon_base64(path: &Path) -> Result<String, AppError> {
    let key = path.to_string_lossy().into_owned();

    if let Ok(cache) = icon_cache().lock() {
        if let Some(cached) = cache.get(&key) {
            return Ok(cached.clone());
        }
    }

    let rgba = extract_icon_rgba(path, ICON_SIZE)?;
    let image = image::RgbaImage::from_raw(ICON_SIZE as u32, ICON_SIZE as u32, rgba)
        .ok_or_else(|| AppError::Message("failed to build RGBA image".to_owned()))?;

    let mut cursor = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| AppError::Message(format!("failed to encode PNG: {e}")))?;

    let encoded = format!(
        "data:image/png;base64,{}",
        STANDARD.encode(cursor.into_inner())
    );
    if let Ok(mut cache) = icon_cache().lock() {
        if cache.len() >= ICON_CACHE_MAX_ITEMS {
            cache.clear();
            warn!("icon cache reached {} entries; cache cleared", ICON_CACHE_MAX_ITEMS);
        }
        cache.insert(key, encoded.clone());
    }

    Ok(encoded)
}

pub(crate) fn extract_icon_rgba(path: &Path, icon_size: i32) -> Result<Vec<u8>, AppError> {
    unsafe {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(iter::once(0))
            .collect();

        let mut icon = HICON::default();
        let extracted = ExtractIconExW(
            PCWSTR(wide.as_ptr()),
            0,
            Some(std::ptr::addr_of_mut!(icon)),
            None,
            1,
        );

        if extracted == 0 || icon.is_invalid() {
            return Err(AppError::Message(format!(
                "no icon extracted for {}",
                path.display()
            )));
        }

        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            let _ = DestroyIcon(icon);
            return Err(AppError::Message("CreateCompatibleDC failed".to_owned()));
        }

        let mut bits_ptr: *mut c_void = std::ptr::null_mut();
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: icon_size,
                biHeight: -icon_size,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0 as u32,
                ..Default::default()
            },
            ..Default::default()
        };

        let bitmap = match CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0) {
            Ok(bitmap) => bitmap,
            Err(_) => {
                let _ = DeleteDC(dc);
                let _ = DestroyIcon(icon);
                return Err(AppError::Message("CreateDIBSection failed".to_owned()));
            }
        };

        if bits_ptr.is_null() {
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
            let _ = DeleteDC(dc);
            let _ = DestroyIcon(icon);
            return Err(AppError::Message(
                "CreateDIBSection returned null".to_owned(),
            ));
        }

        let old = SelectObject(dc, HGDIOBJ(bitmap.0));

        let drew = DrawIconEx(
            dc,
            0,
            0,
            icon,
            icon_size,
            icon_size,
            0,
            HBRUSH(std::ptr::null_mut()),
            DI_NORMAL,
        )
        .is_ok();

        if !drew {
            if !old.is_invalid() {
                let _ = SelectObject(dc, old);
            }
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
            let _ = DeleteDC(dc);
            let _ = DestroyIcon(icon);
            return Err(AppError::Message("DrawIconEx failed".to_owned()));
        }

        let count = (icon_size * icon_size * 4) as usize;
        let bgra = std::slice::from_raw_parts(bits_ptr as *const u8, count);
        let mut rgba = Vec::with_capacity(count);
        for px in bgra.chunks_exact(4) {
            rgba.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
        }

        if !old.is_invalid() {
            let _ = SelectObject(dc, old);
        }

        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(dc);
        let _ = DestroyIcon(icon);

        Ok(rgba)
    }
}
