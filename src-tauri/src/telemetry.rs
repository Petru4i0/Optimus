use crate::core::warn;
use crate::elevation::to_wide;
use crate::optimization::backup_manager::{
    build_registry_restore_actions, ensure_hosts_snapshot, ensure_service_snapshot,
    ensure_task_snapshot, ensure_telemetry_registry_entry, load_telemetry_snapshot,
    save_policy_security_snapshot, HostsFileSnapshot, PolicySecuritySnapshot,
    RegistryRestoreAction, RegistryValueSnapshot, ScheduledTaskSnapshot, ServiceStateSnapshot,
    SnapshotValue,
};
use crate::types::{AppError, OwnedHandle, TelemetryStatusDto};
use crate::utils::registry_cli::{
    reg_create_key, reg_delete_key_tree, reg_delete_value, reg_key_exists, reg_query_dword_value,
    reg_set_dword_value,
};
use std::ptr::null_mut;
use std::io;
use tauri::AppHandle;
use crate::core::{
    last_error_code, AdjustTokenPrivileges, ERROR_NOT_ALL_ASSIGNED, GetCurrentProcess, HANDLE,
    HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE, LookupPrivilegeValueW, LUID, LUID_AND_ATTRIBUTES, OpenProcessToken, PCWSTR, RegKey,
    SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_INVALID_OWNER, GENERIC_ALL, HLOCAL, LocalFree, WIN32_ERROR,
};
use windows::Win32::Security::Authorization::{
    BuildTrusteeWithSidW, ConvertSecurityDescriptorToStringSecurityDescriptorW,
    ConvertStringSecurityDescriptorToSecurityDescriptorW, GetNamedSecurityInfoW,
    SetEntriesInAclW, SetNamedSecurityInfoW, EXPLICIT_ACCESS_W, GRANT_ACCESS, SE_REGISTRY_KEY,
};
use windows::Win32::Security::{
    CreateWellKnownSid, GetSecurityDescriptorDacl, GetSecurityDescriptorOwner,
    DACL_SECURITY_INFORMATION, OBJECT_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION,
    PSECURITY_DESCRIPTOR, PSID, SECURITY_MAX_SID_SIZE, WinBuiltinAdministratorsSid,
    SUB_CONTAINERS_AND_OBJECTS_INHERIT,
};

mod telemetry_hosts;
mod telemetry_services;
mod telemetry_tasks;

pub(crate) use telemetry_hosts::{
    block_telemetry_hosts, capture_hosts_snapshot, check_hosts_status, restore_hosts_snapshot,
};
pub(crate) use telemetry_services::{
    apply_telemetry_services_hard_kill, capture_service_state, check_services_status,
    restore_service_state, TELEMETRY_SERVICE_NAMES,
};
pub(crate) use telemetry_tasks::{
    capture_task_state, check_tasks_status, deep_purge_tasks, restore_task_state,
    TELEMETRY_TASKS,
};

const TELEMETRY_POLICY_KEY: &str =
    r"HKLM\SOFTWARE\Policies\Microsoft\Windows\DataCollection";
const TELEMETRY_POLICY_SUBKEY: &str =
    r"SOFTWARE\Policies\Microsoft\Windows\DataCollection";
const TELEMETRY_POLICY_AUTH_PATH: &str =
    r"MACHINE\SOFTWARE\Policies\Microsoft\Windows\DataCollection";
const TELEMETRY_POLICY_VALUE: &str = "AllowTelemetry";
const SECURITY_DESCRIPTOR_REVISION: u32 = 1;

fn is_not_found_error(err: &AppError) -> bool {
    let lower = err.to_string().to_lowercase();
    lower.contains("os error 2")
        || lower.contains("not found")
        || lower.contains("cannot find")
        || lower.contains("unable to find")
        || lower.contains("does not exist")
}

fn enable_named_privilege(name: &str) -> Result<(), AppError> {
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
    let privilege_name = to_wide(name);
    unsafe {
        LookupPrivilegeValueW(PCWSTR::null(), PCWSTR(privilege_name.as_ptr()), &mut luid)
    }
    .map_err(|e| AppError::Message(format!("LookupPrivilegeValueW failed for {name}: {e}")))?;

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
    .map_err(|e| AppError::Message(format!("AdjustTokenPrivileges failed for {name}: {e}")))?;

    if last_error_code() == ERROR_NOT_ALL_ASSIGNED.0 {
        return Err(AppError::Message(format!(
            "{name} is not assigned to this token"
        )));
    }

    Ok(())
}

fn enable_policy_takeover_privileges() -> Result<(), AppError> {
    enable_named_privilege("SeTakeOwnershipPrivilege")?;
    enable_named_privilege("SeRestorePrivilege")?;
    Ok(())
}

fn security_object_name() -> Vec<u16> {
    to_wide(TELEMETRY_POLICY_AUTH_PATH)
}

fn free_local_ptr(ptr: *mut core::ffi::c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = LocalFree(Some(HLOCAL(ptr)));
        }
    }
}

fn pwstr_to_owned_and_free(value: windows::core::PWSTR) -> String {
    if value.is_null() {
        return String::new();
    }

    let text = unsafe { value.to_string() }.unwrap_or_default();
    free_local_ptr(value.0.cast());
    text
}

fn query_policy_security_snapshot() -> Result<PolicySecuritySnapshot, AppError> {
    let key_existed_before = reg_key_exists(TELEMETRY_POLICY_KEY)?;
    if !key_existed_before {
        return Ok(PolicySecuritySnapshot {
            key_existed_before: false,
            owner_sddl: None,
            dacl_sddl: None,
        });
    }

    let object_name = security_object_name();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let result = unsafe {
        GetNamedSecurityInfoW(
            PCWSTR(object_name.as_ptr()),
            SE_REGISTRY_KEY,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            None,
            None,
            None,
            None,
            &mut descriptor,
        )
    };

    if result != WIN32_ERROR(0) {
        if result.0 == 2 {
            return Ok(PolicySecuritySnapshot {
                key_existed_before: false,
                owner_sddl: None,
                dacl_sddl: None,
            });
        }
        return Err(AppError::Message(format!(
            "GetNamedSecurityInfoW failed for telemetry policy key: {}",
            result.0
        )));
    }

    let mut owner_sddl = windows::core::PWSTR::null();
    unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            descriptor,
            SECURITY_DESCRIPTOR_REVISION,
            OWNER_SECURITY_INFORMATION,
            &mut owner_sddl,
            None,
        )
    }
    .map_err(|e| {
        free_local_ptr(descriptor.0);
        AppError::Message(format!(
            "ConvertSecurityDescriptorToStringSecurityDescriptorW(owner) failed: {e}"
        ))
    })?;

    let mut dacl_sddl = windows::core::PWSTR::null();
    unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            descriptor,
            SECURITY_DESCRIPTOR_REVISION,
            DACL_SECURITY_INFORMATION,
            &mut dacl_sddl,
            None,
        )
    }
    .map_err(|e| {
        free_local_ptr(owner_sddl.0.cast());
        free_local_ptr(descriptor.0);
        AppError::Message(format!(
            "ConvertSecurityDescriptorToStringSecurityDescriptorW(dacl) failed: {e}"
        ))
    })?;

    let snapshot = PolicySecuritySnapshot {
        key_existed_before,
        owner_sddl: Some(pwstr_to_owned_and_free(owner_sddl)),
        dacl_sddl: Some(pwstr_to_owned_and_free(dacl_sddl)),
    };
    free_local_ptr(descriptor.0);
    Ok(snapshot)
}

fn builtin_admin_sid() -> Result<Vec<u8>, AppError> {
    let mut sid = vec![0u8; SECURITY_MAX_SID_SIZE as usize];
    let mut sid_len = sid.len() as u32;
    unsafe {
        CreateWellKnownSid(
            WinBuiltinAdministratorsSid,
            None,
            Some(PSID(sid.as_mut_ptr().cast())),
            &mut sid_len,
        )
    }
    .map_err(|e| AppError::Message(format!("CreateWellKnownSid failed: {e}")))?;
    sid.truncate(sid_len as usize);
    Ok(sid)
}

fn takeover_policy_key_security() -> Result<(), AppError> {
    enable_policy_takeover_privileges()?;
    reg_create_key(TELEMETRY_POLICY_KEY)?;
    let admin_sid = builtin_admin_sid()?;
    let object_name = security_object_name();
    let admin_psid = PSID(admin_sid.as_ptr() as *mut _);

    let owner_result = unsafe {
        SetNamedSecurityInfoW(
            PCWSTR(object_name.as_ptr()),
            SE_REGISTRY_KEY,
            OWNER_SECURITY_INFORMATION,
            Some(admin_psid),
            None,
            None,
            None,
        )
    };
    if owner_result != WIN32_ERROR(0) {
        return Err(AppError::Message(format!(
            "SetNamedSecurityInfoW(owner) failed for telemetry policy key: {}",
            owner_result.0
        )));
    }

    let mut current_dacl = null_mut();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let query_result = unsafe {
        GetNamedSecurityInfoW(
            PCWSTR(object_name.as_ptr()),
            SE_REGISTRY_KEY,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(&mut current_dacl),
            None,
            &mut descriptor,
        )
    };
    if query_result != WIN32_ERROR(0) {
        return Err(AppError::Message(format!(
            "GetNamedSecurityInfoW(dacl) failed for telemetry policy key: {}",
            query_result.0
        )));
    }

    let mut explicit_access = EXPLICIT_ACCESS_W::default();
    explicit_access.grfAccessPermissions = GENERIC_ALL.0;
    explicit_access.grfAccessMode = GRANT_ACCESS;
    explicit_access.grfInheritance = SUB_CONTAINERS_AND_OBJECTS_INHERIT;
    unsafe {
        BuildTrusteeWithSidW(&mut explicit_access.Trustee, Some(admin_psid));
    }

    let mut new_dacl = null_mut();
    let acl_result = unsafe { SetEntriesInAclW(Some(&[explicit_access]), Some(current_dacl), &mut new_dacl) };
    if acl_result != WIN32_ERROR(0) {
        free_local_ptr(descriptor.0);
        return Err(AppError::Message(format!(
            "SetEntriesInAclW failed for telemetry policy key: {}",
            acl_result.0
        )));
    }

    let dacl_result = unsafe {
        SetNamedSecurityInfoW(
            PCWSTR(object_name.as_ptr()),
            SE_REGISTRY_KEY,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(new_dacl),
            None,
        )
    };
    free_local_ptr(new_dacl.cast());
    free_local_ptr(descriptor.0);
    if dacl_result != WIN32_ERROR(0) {
        return Err(AppError::Message(format!(
            "SetNamedSecurityInfoW(dacl) failed for telemetry policy key: {}",
            dacl_result.0
        )));
    }

    Ok(())
}

fn restore_policy_key_security(snapshot: &PolicySecuritySnapshot) -> Result<(), AppError> {
    if !snapshot.key_existed_before {
        return Ok(());
    }

    let mut combined = String::new();
    if let Some(owner) = &snapshot.owner_sddl {
        combined.push_str(owner);
    }
    if let Some(dacl) = &snapshot.dacl_sddl {
        combined.push_str(dacl);
    }
    if combined.trim().is_empty() {
        return Ok(());
    }

    let sddl = to_wide(&combined);
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(sddl.as_ptr()),
            SECURITY_DESCRIPTOR_REVISION,
            &mut descriptor,
            None,
        )
    }
    .map_err(|e| AppError::Message(format!(
        "ConvertStringSecurityDescriptorToSecurityDescriptorW failed: {e}"
    )))?;

    let mut owner = PSID::default();
    let mut owner_defaulted = false.into();
    unsafe { GetSecurityDescriptorOwner(descriptor, &mut owner, &mut owner_defaulted) }
        .map_err(|e| {
            free_local_ptr(descriptor.0);
            AppError::Message(format!("GetSecurityDescriptorOwner failed: {e}"))
        })?;

    let mut dacl_present = false.into();
    let mut dacl_defaulted = false.into();
    let mut dacl = null_mut();
    unsafe { GetSecurityDescriptorDacl(descriptor, &mut dacl_present, &mut dacl, &mut dacl_defaulted) }
        .map_err(|e| {
            free_local_ptr(descriptor.0);
            AppError::Message(format!("GetSecurityDescriptorDacl failed: {e}"))
        })?;

    let object_name = security_object_name();
    let mut info = OBJECT_SECURITY_INFORMATION(0);
    let owner_ptr = if snapshot.owner_sddl.is_some() {
        info |= OWNER_SECURITY_INFORMATION;
        Some(owner)
    } else {
        None
    };
    let dacl_ptr = if snapshot.dacl_sddl.is_some() {
        info |= DACL_SECURITY_INFORMATION;
        Some(dacl as *const _)
    } else {
        None
    };

    let result = unsafe {
        SetNamedSecurityInfoW(
            PCWSTR(object_name.as_ptr()),
            SE_REGISTRY_KEY,
            info,
            owner_ptr,
            None,
            dacl_ptr,
            None,
        )
    };
    free_local_ptr(descriptor.0);
    if result != WIN32_ERROR(0) {
        if result == ERROR_INVALID_OWNER || result == ERROR_ACCESS_DENIED {
            warn!(
                "telemetry policy security restore skipped due to OS permissions: {}",
                result.0
            );
            return Ok(());
        }
        return Err(AppError::Message(format!(
            "SetNamedSecurityInfoW(restore) failed for telemetry policy key: {}",
            result.0
        )));
    }

    Ok(())
}

fn set_allow_telemetry_disabled() -> Result<(), AppError> {
    match write_allow_telemetry_disabled_native() {
        Ok(()) => Ok(()),
        Err(error)
            if error.kind() == io::ErrorKind::PermissionDenied
                || error.raw_os_error() == Some(ERROR_ACCESS_DENIED.0 as i32) =>
        {
            takeover_policy_key_security()?;
            write_allow_telemetry_disabled_native().map_err(|retry_error| {
                AppError::Message(format!(
                    "failed to write telemetry policy value '{}' after takeover: {}",
                    TELEMETRY_POLICY_VALUE, retry_error
                ))
            })
        }
        Err(error) => Err(AppError::Message(format!(
            "failed to write telemetry policy value '{}': {}",
            TELEMETRY_POLICY_VALUE, error
        ))),
    }
}

fn write_allow_telemetry_disabled_native() -> io::Result<()> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let (policy_key, _) = hklm.create_subkey_with_flags(TELEMETRY_POLICY_SUBKEY, KEY_READ | KEY_WRITE)?;
    policy_key.set_value(TELEMETRY_POLICY_VALUE, &0u32)
}

fn query_allow_telemetry_disabled() -> Result<bool, AppError> {
    Ok(reg_query_dword_value(TELEMETRY_POLICY_KEY, TELEMETRY_POLICY_VALUE)? == Some(0))
}

pub(crate) fn apply_telemetry_registry_policies() -> Result<(), AppError> {
    set_allow_telemetry_disabled()
}

pub(crate) fn capture_telemetry_policy_snapshot(app: &AppHandle) -> Result<(), AppError> {
    let previous = reg_query_dword_value(TELEMETRY_POLICY_KEY, TELEMETRY_POLICY_VALUE)?;
    ensure_telemetry_registry_entry(
        app,
        RegistryValueSnapshot {
            path: TELEMETRY_POLICY_KEY.to_owned(),
            name: TELEMETRY_POLICY_VALUE.to_owned(),
            existed_before: previous.is_some(),
            previous_value: previous.map(SnapshotValue::Dword),
        },
    )?;
    save_policy_security_snapshot(app, query_policy_security_snapshot()?)
}

pub(crate) fn capture_telemetry_services_snapshot(app: &AppHandle) -> Result<(), AppError> {
    for service in TELEMETRY_SERVICE_NAMES {
        ensure_service_snapshot(app, capture_service_state(service)?)?;
    }
    Ok(())
}

pub(crate) fn capture_telemetry_tasks_snapshot(app: &AppHandle) -> Result<(), AppError> {
    for task in TELEMETRY_TASKS {
        ensure_task_snapshot(app, capture_task_state(task)?)?;
    }
    Ok(())
}

pub(crate) fn capture_telemetry_hosts_snapshot(app: &AppHandle) -> Result<(), AppError> {
    ensure_hosts_snapshot(app, capture_hosts_snapshot()?)
}

fn restore_services_to_default_state() -> Result<(), AppError> {
    for service_name in TELEMETRY_SERVICE_NAMES {
        restore_service_state(&ServiceStateSnapshot {
            name: service_name.to_owned(),
            existed_before: true,
            start_type: Some(2),
            was_running: true,
        })?;
    }
    Ok(())
}

fn restore_registry_policy_to_default_state() -> Result<(), AppError> {
    reg_set_dword_value(TELEMETRY_POLICY_KEY, TELEMETRY_POLICY_VALUE, 1)
}

fn restore_tasks_to_default_state() -> Result<(), AppError> {
    for task_name in TELEMETRY_TASKS {
        restore_task_state(&ScheduledTaskSnapshot {
            task_name: task_name.to_owned(),
            existed_before: true,
            was_enabled: true,
        })?;
    }
    Ok(())
}

fn restore_hosts_to_default_state() -> Result<(), AppError> {
    let current_hosts = telemetry_hosts::read_hosts_content()?;
    let restored_content = telemetry_hosts::merge_hosts(&current_hosts, false);
    restore_hosts_snapshot(&HostsFileSnapshot {
        original_content: restored_content,
        had_optimus_block: false,
    })
}

fn restore_telemetry_defaults(feature: Option<&str>) -> Result<(), AppError> {
    match feature {
        Some("services") => restore_services_to_default_state(),
        Some("registry_policies") => restore_registry_policy_to_default_state(),
        Some("scheduled_tasks") => restore_tasks_to_default_state(),
        Some("hosts_block") => restore_hosts_to_default_state(),
        None => {
            restore_services_to_default_state()?;
            restore_registry_policy_to_default_state()?;
            restore_tasks_to_default_state()?;
            restore_hosts_to_default_state()
        }
        Some(other) => Err(AppError::Message(format!(
            "unknown telemetry snapshot restore feature '{other}'"
        ))),
    }
}

pub(crate) fn restore_telemetry_from_snapshot(
    app: &AppHandle,
    feature: Option<&str>,
) -> Result<(), AppError> {
    let Some(snapshot) = load_telemetry_snapshot(app)? else {
        warn!(
            "telemetry snapshot missing; restoring telemetry defaults for feature {:?}",
            feature
        );
        return restore_telemetry_defaults(feature);
    };

    match feature {
        Some("services") => {
            for service in &snapshot.services {
                restore_service_state(service)?;
            }
        }
        Some("registry_policies") => {
            for action in build_registry_restore_actions(&snapshot.registry_entries) {
                match action {
                    RegistryRestoreAction::Delete { path, name } => reg_delete_value(&path, &name)?,
                    RegistryRestoreAction::SetDword { path, name, value } => {
                        reg_set_dword_value(&path, &name, value)?
                    }
                    RegistryRestoreAction::SetString { .. } => {
                        return Err(AppError::Message(
                            "unexpected string restore action for telemetry policy".to_owned(),
                        ))
                    }
                }
            }
            if !snapshot.policy_security.key_existed_before {
                reg_delete_key_tree(TELEMETRY_POLICY_KEY)?;
            }
            restore_policy_key_security(&snapshot.policy_security)?;
        }
        Some("scheduled_tasks") => {
            for task in &snapshot.scheduled_tasks {
                restore_task_state(task)?;
            }
        }
        Some("hosts_block") => restore_hosts_snapshot(&snapshot.hosts)?,
        None => {
            for service in &snapshot.services {
                restore_service_state(service)?;
            }
            for action in build_registry_restore_actions(&snapshot.registry_entries) {
                match action {
                    RegistryRestoreAction::Delete { path, name } => reg_delete_value(&path, &name)?,
                    RegistryRestoreAction::SetDword { path, name, value } => {
                        reg_set_dword_value(&path, &name, value)?
                    }
                    RegistryRestoreAction::SetString { .. } => {
                        return Err(AppError::Message(
                            "unexpected string restore action for telemetry policy".to_owned(),
                        ))
                    }
                }
            }
            if !snapshot.policy_security.key_existed_before {
                reg_delete_key_tree(TELEMETRY_POLICY_KEY)?;
            }
            restore_policy_key_security(&snapshot.policy_security)?;
            for task in &snapshot.scheduled_tasks {
                restore_task_state(task)?;
            }
            restore_hosts_snapshot(&snapshot.hosts)?;
        }
        Some(other) => {
            return Err(AppError::Message(format!(
                "unknown telemetry snapshot restore feature '{other}'"
            )))
        }
    }

    Ok(())
}

fn check_registry_policy_status() -> Result<bool, AppError> {
    query_allow_telemetry_disabled()
}

pub(crate) fn check_telemetry_status() -> Result<TelemetryStatusDto, AppError> {
    let (services_disabled, services_readable) = match check_services_status() {
        Ok(value) => (value, true),
        Err(err) => {
            warn!("failed to read telemetry service status: {}", err);
            (false, true)
        }
    };
    let (registry_policies_disabled, registry_policies_readable) =
        match check_registry_policy_status() {
            Ok(value) => (value, true),
            Err(err) => {
                warn!("failed to read telemetry registry policy status: {}", err);
                if is_not_found_error(&err) {
                    (false, true)
                } else {
                    (false, false)
                }
            }
        };
    let (scheduled_tasks_disabled, scheduled_tasks_readable) = match check_tasks_status() {
        Ok(value) => (value, true),
        Err(err) => {
            warn!("failed to read telemetry task status: {}", err);
            if is_not_found_error(&err) {
                (false, true)
            } else {
                (false, false)
            }
        }
    };
    let (hosts_blocked, hosts_readable) = match check_hosts_status() {
        Ok(value) => (value, true),
        Err(err) => {
            warn!("failed to read telemetry hosts status: {}", err);
            (false, false)
        }
    };
    let verified = services_readable
        && registry_policies_readable
        && scheduled_tasks_readable
        && hosts_readable
        && services_disabled
        && registry_policies_disabled
        && scheduled_tasks_disabled
        && hosts_blocked;

    Ok(TelemetryStatusDto {
        verified,
        services_disabled,
        registry_policies_disabled,
        scheduled_tasks_disabled,
        hosts_blocked,
        services_readable,
        registry_policies_readable,
        scheduled_tasks_readable,
        hosts_readable,
    })
}

