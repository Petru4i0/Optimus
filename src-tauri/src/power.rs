use crate::core::warn;
use crate::optimization::backup_manager::{
    ensure_power_active_plan, ensure_power_registry_entry, load_power_snapshot,
    RegistryValueSnapshot, SnapshotValue,
};
use crate::types::{AppError, PowerStatusDto};
use tauri::AppHandle;

mod core_parking;
mod powercfg_cli;
pub(crate) mod timers;
const BALANCED_POWER_PLAN_GUID: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";

pub(crate) use core_parking::disable_core_parking;
pub(crate) use powercfg_cli::enable_ultimate_performance;

fn is_not_found_error(err: &AppError) -> bool {
    let lower = err.to_string().to_lowercase();
    lower.contains("os error 2")
        || lower.contains("not found")
        || lower.contains("cannot find")
        || lower.contains("unable to find")
        || lower.contains("does not exist")
}

fn restore_power_defaults(feature: Option<&str>) -> Result<(), AppError> {
    match feature {
        Some("ultimate_plan") | Some("core_parking") | None => {
            powercfg_cli::set_active_plan_guid(BALANCED_POWER_PLAN_GUID)
        }
        Some(other) => Err(AppError::Message(format!(
            "unknown power snapshot restore feature '{other}'"
        ))),
    }
}

pub(crate) fn capture_power_snapshot_state(
    app: &AppHandle,
    feature: &str,
) -> Result<(), AppError> {
    let active_plan_guid = powercfg_cli::get_active_plan_guid()?;
    ensure_power_active_plan(app, active_plan_guid.clone())?;

    if matches!(feature, "core_parking" | "all") {
        let Some(active_guid) = active_plan_guid else {
            return Err(AppError::Message(
                "failed to capture core parking snapshot: no active power plan".to_owned(),
            ));
        };
        let (ac, dc) = core_parking::query_core_parking_indices(&active_guid)?;
        let attrs = core_parking::query_core_parking_attributes()?;
        let setting_key = core_parking::core_parking_setting_key(&active_guid);
        let attributes_key = core_parking::core_parking_attributes_key();
        for (name, value) in [
            ("ACSettingIndex", ac),
            ("DCSettingIndex", dc),
        ] {
            ensure_power_registry_entry(
                app,
                RegistryValueSnapshot {
                    path: setting_key.clone(),
                    name: name.to_owned(),
                    existed_before: value.is_some(),
                    previous_value: value.map(SnapshotValue::Dword),
                },
            )?;
        }
        ensure_power_registry_entry(
            app,
            RegistryValueSnapshot {
                path: attributes_key,
                name: "Attributes".to_owned(),
                existed_before: attrs.is_some(),
                previous_value: attrs.map(SnapshotValue::Dword),
            },
        )?;
    }

    Ok(())
}

pub(crate) fn restore_power_from_snapshot(
    app: &AppHandle,
    feature: Option<&str>,
) -> Result<(), AppError> {
    let snapshot = load_power_snapshot(app)?;
    if snapshot.is_none() {
        warn!(
            "power snapshot missing; restoring power defaults for feature {:?}",
            feature
        );
        return restore_power_defaults(feature);
    }

    match feature {
        Some("ultimate_plan") | Some("core_parking") | None => {
            powercfg_cli::set_active_plan_guid(BALANCED_POWER_PLAN_GUID)?
        }
        Some(other) => {
            return Err(AppError::Message(format!(
                "unknown power snapshot restore feature '{other}'"
            )))
        }
    }
    Ok(())
}

pub(crate) fn check_power_status() -> Result<PowerStatusDto, AppError> {
    let (ultimate_plan_active, ultimate_plan_readable) =
        match powercfg_cli::check_ultimate_performance_active() {
            Ok(value) => (value, true),
            Err(err) => {
                warn!("failed to read ultimate power plan status: {}", err);
                if is_not_found_error(&err) {
                    (false, true)
                } else {
                    (false, false)
                }
            }
        };
    let (core_parking_disabled, core_parking_readable) =
        match core_parking::check_core_parking_disabled() {
            Ok(value) => (value, true),
            Err(err) => {
                warn!("failed to read core parking status: {}", err);
                if is_not_found_error(&err) {
                    (false, true)
                } else {
                    (false, false)
                }
            }
        };

    Ok(PowerStatusDto {
        verified: ultimate_plan_readable
            && core_parking_readable
            && ultimate_plan_active
            && core_parking_disabled,
        ultimate_plan_active,
        core_parking_disabled,
        ultimate_plan_readable,
        core_parking_readable,
    })
}
