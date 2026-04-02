use crate::core::{error, STANDARD, ARG_ELEVATED_PAYLOAD};
use crate::process::{kill_process_by_pid, set_priority_for_pid};
use crate::types::{ElevatedAction, ElevatedActionPayload};
use base64::Engine;
pub(crate) fn decode_elevated_payload_from_args() -> Option<ElevatedActionPayload> {
    let args: Vec<String> = std::env::args().collect();
    let payload_index = args.iter().position(|arg| arg == ARG_ELEVATED_PAYLOAD)?;
    let encoded = args.get(payload_index + 1)?;
    let raw = STANDARD.decode(encoded).ok()?;
    serde_json::from_slice::<ElevatedActionPayload>(&raw).ok()
}

pub(crate) fn apply_startup_elevated_payload() {
    let Some(payload) = decode_elevated_payload_from_args() else {
        return;
    };

    match payload.action {
        ElevatedAction::SetProcessPriority => {
            if let (Some(pid), Some(priority)) = (payload.pid, payload.priority) {
                if let Err(err) = set_priority_for_pid(pid, priority) {
                    error!("[elevated-startup] failed to set pid {pid}: {err}");
                }
            }
        }
        ElevatedAction::SetGroupPriority => {
            if let (Some(pids), Some(priority)) = (payload.pids, payload.priority) {
                for pid in pids {
                    if let Err(err) = set_priority_for_pid(pid, priority) {
                        error!("[elevated-startup] failed to set pid {pid}: {err}");
                    }
                }
            }
        }
        ElevatedAction::KillProcess => {
            if let Some(pid) = payload.pid {
                if let Err(err) = kill_process_by_pid(pid) {
                    error!("[elevated-startup] failed to kill pid {pid}: {err}");
                }
            }
        }
    }
}

