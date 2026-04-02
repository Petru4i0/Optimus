use crate::core::{
    iter, last_error_code, STANDARD, ARG_ELEVATED_PAYLOAD, ARG_ELEVATED_SESSION, ERROR_CANCELLED,
    HWND, MB_ICONERROR, MB_OK, MessageBoxW, PCWSTR, ShellExecuteExW, SHELLEXECUTEINFOW,
    SW_SHOWNORMAL, SEE_MASK_NOCLOSEPROCESS,
};
use crate::types::{AppError, ElevatedActionPayload, OwnedHandle};
use base64::Engine;
use std::os::windows::ffi::OsStrExt;
pub(crate) fn to_wide<S: AsRef<std::ffi::OsStr>>(value: S) -> Vec<u16> {
    value.as_ref().encode_wide().chain(iter::once(0)).collect()
}

pub(crate) fn show_startup_error_dialog(message: &str) {
    let title = to_wide("Optimus Startup Error");
   let body = to_wide(message);
    unsafe {
        let _ = MessageBoxW(
            Some(HWND(std::ptr::null_mut())),
            PCWSTR(body.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ElevationLaunchStatus {
    Launched,
    Cancelled,
}

pub(crate) fn launch_elevated(
    payload: Option<ElevatedActionPayload>,
) -> Result<ElevationLaunchStatus, AppError> {
    let exe = std::env::current_exe()
        .map_err(|e| AppError::Message(format!("failed to get current executable: {e}")))?;

    let mut params = String::from(ARG_ELEVATED_SESSION);
    if let Some(payload) = payload {
        let encoded = STANDARD
            .encode(serde_json::to_vec(&payload).map_err(|e| AppError::Message(e.to_string()))?);
        params.push(' ');
        params.push_str(ARG_ELEVATED_PAYLOAD);
        params.push(' ');
        params.push_str(&encoded);
    }

    let operation = to_wide("runas");
    let file = to_wide(exe.as_os_str());
    let parameters = to_wide(params);

    let mut shell_exec_info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        hwnd: HWND(std::ptr::null_mut()),
        lpVerb: PCWSTR(operation.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(parameters.as_ptr()),
        lpDirectory: PCWSTR::null(),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };

    if let Err(err) = unsafe { ShellExecuteExW(&mut shell_exec_info) } {
        let code = last_error_code();
        if code == ERROR_CANCELLED.0 {
            return Ok(ElevationLaunchStatus::Cancelled);
        }
        return Err(AppError::Message(format!(
            "failed to request UAC elevation (ShellExecuteExW error {code}): {err}"
        )));
    }

    if !shell_exec_info.hProcess.is_invalid() {
        let _handle = OwnedHandle(shell_exec_info.hProcess);
    }

    Ok(ElevationLaunchStatus::Launched)
}

