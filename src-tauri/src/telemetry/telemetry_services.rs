use crate::core::{
    iter, last_error_code, CloseServiceHandle, OpenSCManagerW, OpenServiceW, PCWSTR,
    ChangeServiceConfigW, ControlService, QueryServiceConfigW, QueryServiceStatus, StartServiceW,
    QUERY_SERVICE_CONFIGW, SC_HANDLE,
    SC_MANAGER_CONNECT, SERVICE_CHANGE_CONFIG, SERVICE_CONTROL_STOP,
    SERVICE_DISABLED, SERVICE_NO_CHANGE, SERVICE_QUERY_CONFIG, SERVICE_QUERY_STATUS,
    SERVICE_RUNNING, SERVICE_START, SERVICE_STATUS, SERVICE_STOP, OsStrExt,
};
use crate::optimization::backup_manager::ServiceStateSnapshot;
use crate::types::AppError;

pub(crate) const TELEMETRY_SERVICE_NAMES: [&str; 2] = ["DiagTrack", "dmwappushservice"];
const ERROR_SERVICE_DOES_NOT_EXIST_CODE: u32 = 1060;
const ERROR_SERVICE_CANNOT_ACCEPT_CTRL_CODE: u32 = 1061;
const ERROR_SERVICE_NOT_ACTIVE_CODE: u32 = 1062;
const ERROR_SERVICE_ALREADY_RUNNING_CODE: u32 = 1056;

struct OwnedServiceHandle(SC_HANDLE);

impl OwnedServiceHandle {
    fn raw(&self) -> SC_HANDLE {
        self.0
    }
}

impl Drop for OwnedServiceHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseServiceHandle(self.0);
        }
    }
}

fn to_wide_local(value: &str) -> Vec<u16> {
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(iter::once(0))
        .collect()
}

fn open_service(service: &str, access: u32) -> Result<Option<OwnedServiceHandle>, AppError> {
    let scm = unsafe { OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT) }
        .map_err(|e| AppError::Message(format!("OpenSCManagerW failed: {e}")))?;
    let scm_handle = OwnedServiceHandle(scm);

    let service_name = to_wide_local(service);
    match unsafe { OpenServiceW(scm_handle.raw(), PCWSTR(service_name.as_ptr()), access) } {
        Ok(handle) => Ok(Some(OwnedServiceHandle(handle))),
        Err(_) => {
            let code = last_error_code();
            if code == ERROR_SERVICE_DOES_NOT_EXIST_CODE {
                Ok(None)
            } else {
                Err(AppError::Message(format!(
                    "OpenServiceW failed for '{}': {}",
                    service, code
                )))
            }
        }
    }
}

fn stop_service(service: &str) -> Result<(), AppError> {
    let Some(service_handle) = open_service(service, SERVICE_STOP | SERVICE_QUERY_STATUS)? else {
        return Ok(());
    };

    let mut status = SERVICE_STATUS::default();
    match unsafe { ControlService(service_handle.raw(), SERVICE_CONTROL_STOP, &mut status) } {
        Ok(()) => Ok(()),
        Err(_) => {
            let code = last_error_code();
            if code == ERROR_SERVICE_NOT_ACTIVE_CODE || code == ERROR_SERVICE_CANNOT_ACCEPT_CTRL_CODE {
                Ok(())
            } else {
                Err(AppError::Message(format!(
                    "failed to stop service '{}': {}",
                    service, code
                )))
            }
        }
    }
}

fn set_service_start_mode(
    service: &str,
    mode: windows::Win32::System::Services::SERVICE_START_TYPE,
) -> Result<(), AppError> {
    let Some(service_handle) = open_service(service, SERVICE_CHANGE_CONFIG)? else {
        return Ok(());
    };

    unsafe {
        ChangeServiceConfigW(
            service_handle.raw(),
            windows::Win32::System::Services::ENUM_SERVICE_TYPE(SERVICE_NO_CHANGE),
            mode,
            windows::Win32::System::Services::SERVICE_ERROR(SERVICE_NO_CHANGE),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }
    .map_err(|e| AppError::Message(format!("ChangeServiceConfigW failed for '{}': {e}", service)))
}

fn query_service_config_disabled(service: &str) -> Result<bool, AppError> {
    let Some(service_handle) = open_service(service, SERVICE_QUERY_CONFIG)? else {
        return Ok(true);
    };

    let mut bytes_needed = 0u32;
    let _ = unsafe { QueryServiceConfigW(service_handle.raw(), None, 0, &mut bytes_needed) };
    if bytes_needed == 0 {
        let code = last_error_code();
        return Err(AppError::Message(format!(
            "QueryServiceConfigW size query failed for '{}': {}",
            service, code
        )));
    }

    let mut buffer = vec![0u8; bytes_needed as usize];
    let config_ptr = buffer.as_mut_ptr() as *mut QUERY_SERVICE_CONFIGW;

    unsafe {
        QueryServiceConfigW(
            service_handle.raw(),
            Some(config_ptr),
            bytes_needed,
            &mut bytes_needed,
        )
    }
    .map_err(|e| AppError::Message(format!("QueryServiceConfigW failed for '{}': {e}", service)))?;

    let start_type = unsafe { (*config_ptr).dwStartType };
    Ok(start_type == SERVICE_DISABLED)
}

pub(crate) fn apply_telemetry_services_hard_kill() -> Result<(), AppError> {
    for service in TELEMETRY_SERVICE_NAMES {
        stop_service(service)?;
        set_service_start_mode(service, SERVICE_DISABLED)?;
    }
    Ok(())
}

fn query_service_start_type(service: &str) -> Result<Option<u32>, AppError> {
    let Some(service_handle) = open_service(service, SERVICE_QUERY_CONFIG)? else {
        return Ok(None);
    };

    let mut bytes_needed = 0u32;
    let _ = unsafe { QueryServiceConfigW(service_handle.raw(), None, 0, &mut bytes_needed) };
    if bytes_needed == 0 {
        let code = last_error_code();
        return Err(AppError::Message(format!(
            "QueryServiceConfigW size query failed for '{}': {}",
            service, code
        )));
    }

    let mut buffer = vec![0u8; bytes_needed as usize];
    let config_ptr = buffer.as_mut_ptr() as *mut QUERY_SERVICE_CONFIGW;
    unsafe {
        QueryServiceConfigW(
            service_handle.raw(),
            Some(config_ptr),
            bytes_needed,
            &mut bytes_needed,
        )
    }
    .map_err(|e| AppError::Message(format!("QueryServiceConfigW failed for '{}': {e}", service)))?;

    Ok(Some(unsafe { (*config_ptr).dwStartType.0 }))
}

fn query_service_running(service: &str) -> Result<bool, AppError> {
    let Some(service_handle) = open_service(service, SERVICE_QUERY_STATUS)? else {
        return Ok(false);
    };
    let mut status = SERVICE_STATUS::default();
    unsafe { QueryServiceStatus(service_handle.raw(), &mut status) }
        .map_err(|e| AppError::Message(format!("QueryServiceStatus failed for '{}': {e}", service)))?;
    Ok(status.dwCurrentState == SERVICE_RUNNING)
}

pub(crate) fn capture_service_state(service: &str) -> Result<ServiceStateSnapshot, AppError> {
    let start_type = query_service_start_type(service)?;
    let existed_before = start_type.is_some();
    let was_running = if existed_before {
        query_service_running(service)?
    } else {
        false
    };
    Ok(ServiceStateSnapshot {
        name: service.to_owned(),
        existed_before,
        start_type,
        was_running,
    })
}

pub(crate) fn restore_service_state(snapshot: &ServiceStateSnapshot) -> Result<(), AppError> {
    if !snapshot.existed_before {
        return Ok(());
    }

    if let Some(start_type) = snapshot.start_type {
        set_service_start_mode(
            &snapshot.name,
            windows::Win32::System::Services::SERVICE_START_TYPE(start_type),
        )?;
    }

    if snapshot.was_running {
        if let Some(service_handle) = open_service(&snapshot.name, SERVICE_START)? {
            match unsafe { StartServiceW(service_handle.raw(), None) } {
                Ok(()) => {}
                Err(_) => {
                    let code = last_error_code();
                    if code != ERROR_SERVICE_ALREADY_RUNNING_CODE {
                        return Err(AppError::Message(format!(
                            "StartServiceW failed for '{}': {}",
                            snapshot.name, code
                        )));
                    }
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn check_services_status() -> Result<bool, AppError> {
    let diag = match query_service_config_disabled(TELEMETRY_SERVICE_NAMES[0]) {
        Ok(value) => value,
        Err(err) => {
            tracing::warn!(
                "failed to query telemetry service '{}' status, treating as not optimized: {}",
                TELEMETRY_SERVICE_NAMES[0],
                err
            );
            false
        }
    };
    let dm = match query_service_config_disabled(TELEMETRY_SERVICE_NAMES[1]) {
        Ok(value) => value,
        Err(err) => {
            tracing::warn!(
                "failed to query telemetry service '{}' status, treating as not optimized: {}",
                TELEMETRY_SERVICE_NAMES[1],
                err
            );
            false
        }
    };
    Ok(diag && dm)
}
