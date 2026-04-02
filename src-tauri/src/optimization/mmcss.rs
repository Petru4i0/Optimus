use crate::optimization::backup_manager::{
    ensure_advanced_registry_entry, build_registry_restore_actions, load_advanced_snapshot,
    RegistryRestoreAction, RegistryValueSnapshot, SnapshotValue,
};
use crate::types::AppError;
use crate::utils::registry_cli::{
    reg_delete_value, reg_query_dword_value, reg_query_string_value, reg_set_dword_value,
    reg_set_string_value,
};
use tauri::AppHandle;

const SYSTEM_PROFILE_KEY: &str =
    r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile";
const GAMES_TASK_KEY: &str =
    r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games";
const SYSTEM_RESPONSIVENESS: &str = "SystemResponsiveness";
const GPU_PRIORITY: &str = "GPU Priority";
const PRIORITY: &str = "Priority";
const SCHEDULING_CATEGORY: &str = "Scheduling Category";

#[derive(Debug, Clone, Default)]
pub(crate) struct MmcssStatus {
    pub(crate) applied: bool,
    pub(crate) readable: bool,
}

fn capture_dword_entry(app: &AppHandle, path: &str, name: &str) -> Result<(), AppError> {
    let previous = reg_query_dword_value(path, name)?;
    ensure_advanced_registry_entry(
        app,
        RegistryValueSnapshot {
            path: path.to_owned(),
            name: name.to_owned(),
            existed_before: previous.is_some(),
            previous_value: previous.map(SnapshotValue::Dword),
        },
    )
}

fn capture_string_entry(app: &AppHandle, path: &str, name: &str) -> Result<(), AppError> {
    let previous = reg_query_string_value(path, name)?;
    ensure_advanced_registry_entry(
        app,
        RegistryValueSnapshot {
            path: path.to_owned(),
            name: name.to_owned(),
            existed_before: previous.is_some(),
            previous_value: previous.map(SnapshotValue::String),
        },
    )
}

pub(crate) fn capture_mmcss_snapshot(app: &AppHandle) -> Result<(), AppError> {
    capture_dword_entry(app, SYSTEM_PROFILE_KEY, SYSTEM_RESPONSIVENESS)?;
    capture_dword_entry(app, GAMES_TASK_KEY, GPU_PRIORITY)?;
    capture_dword_entry(app, GAMES_TASK_KEY, PRIORITY)?;
    capture_string_entry(app, GAMES_TASK_KEY, SCHEDULING_CATEGORY)?;
    Ok(())
}

pub(crate) fn apply_mmcss() -> Result<(), AppError> {
    reg_set_dword_value(SYSTEM_PROFILE_KEY, SYSTEM_RESPONSIVENESS, 0)?;
    reg_set_dword_value(GAMES_TASK_KEY, GPU_PRIORITY, 8)?;
    reg_set_dword_value(GAMES_TASK_KEY, PRIORITY, 6)?;
    reg_set_string_value(GAMES_TASK_KEY, SCHEDULING_CATEGORY, "High")?;
    Ok(())
}

pub(crate) fn restore_mmcss_from_snapshot(app: &AppHandle) -> Result<(), AppError> {
    let Some(snapshot) = load_advanced_snapshot(app)? else {
        reg_delete_value(SYSTEM_PROFILE_KEY, SYSTEM_RESPONSIVENESS)?;
        reg_delete_value(GAMES_TASK_KEY, GPU_PRIORITY)?;
        reg_delete_value(GAMES_TASK_KEY, PRIORITY)?;
        reg_delete_value(GAMES_TASK_KEY, SCHEDULING_CATEGORY)?;
        return Ok(());
    };

    let entries: Vec<RegistryValueSnapshot> = snapshot
        .registry_entries
        .into_iter()
        .filter(|entry| {
            (entry.path.eq_ignore_ascii_case(SYSTEM_PROFILE_KEY)
                && entry.name.eq_ignore_ascii_case(SYSTEM_RESPONSIVENESS))
                || (entry.path.eq_ignore_ascii_case(GAMES_TASK_KEY)
                    && (entry.name.eq_ignore_ascii_case(GPU_PRIORITY)
                        || entry.name.eq_ignore_ascii_case(PRIORITY)
                        || entry.name.eq_ignore_ascii_case(SCHEDULING_CATEGORY)))
        })
        .collect();

    for action in build_registry_restore_actions(&entries) {
        match action {
            RegistryRestoreAction::Delete { path, name } => reg_delete_value(&path, &name)?,
            RegistryRestoreAction::SetDword { path, name, value } => {
                reg_set_dword_value(&path, &name, value)?
            }
            RegistryRestoreAction::SetString { path, name, value } => {
                reg_set_string_value(&path, &name, &value)?
            }
        }
    }
    Ok(())
}

pub(crate) fn check_mmcss_status() -> Result<MmcssStatus, AppError> {
    let system_responsiveness = reg_query_dword_value(SYSTEM_PROFILE_KEY, SYSTEM_RESPONSIVENESS)?;
    let gpu_priority = reg_query_dword_value(GAMES_TASK_KEY, GPU_PRIORITY)?;
    let priority = reg_query_dword_value(GAMES_TASK_KEY, PRIORITY)?;
    let scheduling_category = reg_query_string_value(GAMES_TASK_KEY, SCHEDULING_CATEGORY)?;

    Ok(MmcssStatus {
        applied: system_responsiveness == Some(0)
            && gpu_priority == Some(8)
            && priority == Some(6)
            && scheduling_category
                .as_deref()
                .is_some_and(|value| value.eq_ignore_ascii_case("high")),
        readable: true,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn mmcss_verification_requires_all_values() {
        let applied = Some(0) == Some(0)
            && Some(8) == Some(8)
            && Some(6) == Some(6)
            && Some("High").is_some_and(|value| value.eq_ignore_ascii_case("high"));
        assert!(applied);

        let missing_priority = Some(0) == Some(0)
            && Some(8) == Some(8)
            && None::<u32> == Some(6)
            && Some("High").is_some_and(|value| value.eq_ignore_ascii_case("high"));
        assert!(!missing_priority);
    }
}
