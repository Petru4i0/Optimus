#![allow(unused_imports)]

use crate::core::is_running_as_admin;
use crate::optimization::collect_status;
use crate::types::{AppError, AppErrorEnvelope, OptimizationStatusDto, RuntimeControlState};

pub(crate) mod config_commands;
pub(crate) mod engine_commands;
pub(crate) mod optimization_commands;
pub(crate) mod process_commands;

const ADMIN_REQUIRED_ERROR: &str =
    "Administrator privileges required. Please restart Optimus as Admin.";

pub(crate) type IpcResult<T> = Result<T, AppErrorEnvelope>;

pub(crate) fn app_error(
    code: &str,
    message: impl Into<String>,
    requires_admin: bool,
    retryable: bool,
) -> AppErrorEnvelope {
    AppErrorEnvelope {
        code: code.to_owned(),
        message: message.into(),
        requires_admin,
        retryable,
    }
}

pub(crate) fn to_ipc_error<E: ToString>(error: E) -> AppErrorEnvelope {
    let message = error.to_string();
    if let Some(details) = message.strip_prefix("PROTECTED_BOOT_DRIVER:") {
        return app_error("PROTECTED_BOOT_DRIVER", details.trim(), false, false);
    }
    let lower = message.to_lowercase();
    if lower.contains("administrator privileges required")
        || lower.contains("access denied")
        || lower.contains("missing admin rights")
    {
        return app_error("ACCESS_DENIED", message, true, false);
    }
    if lower.contains("not found") || lower.contains("was not found") {
        return app_error("NOT_FOUND", message, false, false);
    }
    if lower.contains("cancelled") || lower.contains("canceled") {
        return app_error("CANCELLED", message, false, false);
    }
    app_error("INTERNAL", message, false, true)
}

pub(crate) fn from_app_error(error: AppError) -> AppErrorEnvelope {
    match error {
        AppError::AccessDenied { .. } => app_error("ACCESS_DENIED", error.to_string(), true, false),
        AppError::WinApi { code, .. } if code == 5 => {
            app_error("ACCESS_DENIED", error.to_string(), true, false)
        }
        AppError::WinApi { .. } => app_error("WINAPI_ERROR", error.to_string(), false, true),
        AppError::Message(message) => to_ipc_error(message),
    }
}

pub(crate) fn ensure_admin() -> IpcResult<()> {
    if is_running_as_admin() {
        Ok(())
    } else {
        Err(app_error("ACCESS_DENIED", ADMIN_REQUIRED_ERROR, true, false))
    }
}

pub(crate) fn collect_optimization_status(
    runtime: &RuntimeControlState,
) -> IpcResult<OptimizationStatusDto> {
    collect_status(runtime).map_err(from_app_error)
}

pub(crate) use config_commands::*;
pub(crate) use engine_commands::*;
pub(crate) use optimization_commands::*;
pub(crate) use process_commands::*;

#[cfg(test)]
mod tests {
    use crate::ipc::{from_app_error, to_ipc_error};
    use crate::types::AppError;

    #[test]
    fn maps_access_denied_error_to_admin_required_envelope() {
        let envelope = from_app_error(AppError::AccessDenied {
            pid: 77,
            context: "setting priority",
        });
        assert_eq!(envelope.code, "ACCESS_DENIED");
        assert!(envelope.requires_admin);
        assert!(!envelope.retryable);
    }

    #[test]
    fn maps_winapi_error_code_5_to_admin_required_envelope() {
        let envelope = from_app_error(AppError::WinApi {
            pid: 42,
            context: "open process",
            code: 5,
        });
        assert_eq!(envelope.code, "ACCESS_DENIED");
        assert!(envelope.requires_admin);
        assert!(!envelope.retryable);
    }

    #[test]
    fn maps_generic_error_to_retryable_internal_envelope() {
        let envelope = to_ipc_error("unexpected runtime failure");
        assert_eq!(envelope.code, "INTERNAL");
        assert!(!envelope.requires_admin);
        assert!(envelope.retryable);
    }

    #[test]
    fn maps_protected_boot_driver_message_to_specific_code() {
        let envelope = to_ipc_error(
            "PROTECTED_BOOT_DRIVER: force deletion blocked for 'oem1.inf' (class='System')",
        );
        assert_eq!(envelope.code, "PROTECTED_BOOT_DRIVER");
        assert!(!envelope.requires_admin);
        assert!(!envelope.retryable);
    }
}
