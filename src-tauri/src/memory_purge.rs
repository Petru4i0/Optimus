use crate::*;

pub(crate) fn enable_profile_privilege() -> Result<(), AppError> {
    let mut token = HANDLE::default();
    unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        )
    }
    .map_err(|e| AppError::Message(format!("OpenProcessToken failed: {e}")))?;
    let token_handle = OwnedHandle(token);

    let mut luid = LUID::default();
    let privilege_name = to_wide("SeProfileSingleProcessPrivilege");
    unsafe {
        LookupPrivilegeValueW(PCWSTR::null(), PCWSTR(privilege_name.as_ptr()), &mut luid)
    }
    .map_err(|e| AppError::Message(format!("LookupPrivilegeValueW failed: {e}")))?;

    let privileges = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES {
            Luid: luid,
            Attributes: SE_PRIVILEGE_ENABLED,
        }],
    };

    unsafe {
        AdjustTokenPrivileges(
            token_handle.raw(),
            false,
            Some(&privileges),
            0,
            None,
            None,
        )
    }
    .map_err(|e| AppError::Message(format!("AdjustTokenPrivileges failed: {e}")))?;

    let last = last_error_code();
    if last == ERROR_NOT_ALL_ASSIGNED.0 {
        return Err(AppError::Message(
            "SeProfileSingleProcessPrivilege is not assigned to this token".to_owned(),
        ));
    }

    Ok(())
}

pub(crate) fn build_memory_purge_config_dto(
    runtime: &RuntimeControlState,
) -> Result<MemoryPurgeConfigDto, AppError> {
    let config = *runtime
        .memory_purge_config
        .read()
        .map_err(|_| AppError::Message("memory purge config lock poisoned".to_owned()))?;

    Ok(MemoryPurgeConfigDto {
        master_enabled: config.master_enabled,
        enable_standby_trigger: config.enable_standby_trigger,
        standby_limit_mb: config.standby_limit_mb,
        enable_free_memory_trigger: config.enable_free_memory_trigger,
        free_memory_limit_mb: config.free_memory_limit_mb,
        total_purges: runtime.memory_purge_count.load(Ordering::Relaxed),
    })
}

pub(crate) fn query_system_memory_list() -> Result<SystemMemoryListInformation, AppError> {
    let mut info = SystemMemoryListInformation::default();
    let mut return_length = 0u32;

    let status = unsafe {
        NtQuerySystemInformation(
            SYSTEM_MEMORY_LIST_INFORMATION_CLASS,
            (&mut info as *mut SystemMemoryListInformation).cast::<c_void>(),
            std::mem::size_of::<SystemMemoryListInformation>() as u32,
            &mut return_length,
        )
    };

    if !nt_success(status) {
        return Err(AppError::Message(format!(
            "NtQuerySystemInformation(SystemMemoryListInformation) failed with NTSTATUS 0x{status:08X}"
        )));
    }

    Ok(info)
}

pub(crate) fn read_memory_stats() -> Result<MemoryStatsDto, AppError> {
    let memory_list = query_system_memory_list()?;
    let standby_pages: u64 = memory_list
        .page_count_by_priority
        .iter()
        .map(|value| *value as u64)
        .sum();
    let free_pages = (memory_list.free_page_count as u64).saturating_add(memory_list.zero_page_count as u64);

    let mut sys_info = SYSTEM_INFO::default();
    unsafe {
        GetSystemInfo(&mut sys_info);
    }
    let page_size = u64::from(sys_info.dwPageSize);
    let standby_list_mb = standby_pages
        .saturating_mul(page_size)
        .saturating_div(1024 * 1024);
    let free_memory_mb = free_pages
        .saturating_mul(page_size)
        .saturating_div(1024 * 1024);
    let mut system = System::new();
    system.refresh_memory();
    let total_memory_mb = system.total_memory().saturating_div(1024 * 1024);

    Ok(MemoryStatsDto {
        standby_list_mb,
        free_memory_mb,
        total_memory_mb,
    })
}

pub(crate) fn run_standby_purge() -> Result<(), AppError> {
    let mut command = MEMORY_PURGE_STANDBY_LIST;
    let status = unsafe {
        NtSetSystemInformation(
            SYSTEM_MEMORY_LIST_INFORMATION_CLASS,
            (&mut command as *mut u32).cast::<c_void>(),
            std::mem::size_of::<u32>() as u32,
        )
    };

    if !nt_success(status) {
        if status == 0xC0000061u32 as i32 {
            return Err(AppError::Message(
                "Missing Admin Rights: Restart Optimus as Administrator to purge memory."
                    .to_owned(),
            ));
        }
        return Err(AppError::Message(format!(
            "NtSetSystemInformation(SystemMemoryListInformation) failed with NTSTATUS 0x{status:08X}"
        )));
    }

    Ok(())
}

pub(crate) fn should_run_memory_purge(config: &MemoryPurgeConfigState, stats: &MemoryStatsDto) -> bool {
    match (config.enable_standby_trigger, config.enable_free_memory_trigger) {
        (false, false) => false,
        (true, false) => stats.standby_list_mb > config.standby_limit_mb,
        (false, true) => stats.free_memory_mb < config.free_memory_limit_mb,
        (true, true) => {
            stats.standby_list_mb > config.standby_limit_mb
                && stats.free_memory_mb < config.free_memory_limit_mb
        }
    }
}

pub(crate) fn run_memory_purge_tick(runtime: &RuntimeControlState) -> Result<(), AppError> {
    let config = *runtime
        .memory_purge_config
        .read()
        .map_err(|_| AppError::Message("memory purge config lock poisoned".to_owned()))?;

    if !config.master_enabled {
        return Ok(());
    }

    if !config.enable_standby_trigger && !config.enable_free_memory_trigger {
        return Ok(());
    }

    let stats = read_memory_stats()?;
    if should_run_memory_purge(&config, &stats) {
        run_standby_purge()?;
        runtime.memory_purge_count.fetch_add(1, Ordering::Relaxed);
    }

    Ok(())
}
