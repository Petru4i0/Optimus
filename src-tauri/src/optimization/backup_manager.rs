use crate::core::{fs, PathBuf};
use crate::settings_repo::atomic_write_json;
use crate::types::AppError;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tauri::Manager;

const BACKUP_DIR_NAME: &str = "backups";
const INTERNET_SNAPSHOT_FILE: &str = "snapshot_internet.json";
const TELEMETRY_SNAPSHOT_FILE: &str = "snapshot_telemetry.json";
const POWER_SNAPSHOT_FILE: &str = "snapshot_power.json";
const ADVANCED_SNAPSHOT_FILE: &str = "snapshot_advanced.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type", content = "value")]
pub(crate) enum SnapshotValue {
    Dword(u32),
    String(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegistryValueSnapshot {
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) existed_before: bool,
    pub(crate) previous_value: Option<SnapshotValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RegistryRestoreAction {
    Delete { path: String, name: String },
    SetDword { path: String, name: String, value: u32 },
    SetString { path: String, name: String, value: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceStateSnapshot {
    pub(crate) name: String,
    pub(crate) existed_before: bool,
    pub(crate) start_type: Option<u32>,
    pub(crate) was_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScheduledTaskSnapshot {
    pub(crate) task_name: String,
    pub(crate) existed_before: bool,
    pub(crate) was_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DnsInterfaceSnapshot {
    pub(crate) guid: String,
    pub(crate) interface_name: String,
    #[serde(default)]
    pub(crate) ipv4_servers: Vec<String>,
    #[serde(default)]
    pub(crate) ipv6_servers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HostsFileSnapshot {
    pub(crate) original_content: String,
    pub(crate) had_optimus_block: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PolicySecuritySnapshot {
    #[serde(default)]
    pub(crate) key_existed_before: bool,
    pub(crate) owner_sddl: Option<String>,
    pub(crate) dacl_sddl: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InternetSnapshot {
    #[serde(default)]
    pub(crate) registry_entries: Vec<RegistryValueSnapshot>,
    #[serde(default)]
    pub(crate) dns_interfaces: Vec<DnsInterfaceSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TelemetrySnapshot {
    #[serde(default)]
    pub(crate) registry_entries: Vec<RegistryValueSnapshot>,
    #[serde(default)]
    pub(crate) services: Vec<ServiceStateSnapshot>,
    #[serde(default)]
    pub(crate) scheduled_tasks: Vec<ScheduledTaskSnapshot>,
    #[serde(default)]
    pub(crate) hosts: HostsFileSnapshot,
    #[serde(default)]
    pub(crate) hosts_snapshot_captured: bool,
    #[serde(default)]
    pub(crate) policy_security: PolicySecuritySnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PowerSnapshot {
    pub(crate) active_plan_guid: Option<String>,
    #[serde(default)]
    pub(crate) registry_entries: Vec<RegistryValueSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BcdValueSnapshot {
    pub(crate) name: String,
    pub(crate) existed_before: bool,
    pub(crate) previous_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdvancedSnapshot {
    #[serde(default)]
    pub(crate) registry_entries: Vec<RegistryValueSnapshot>,
    #[serde(default)]
    pub(crate) bcd_entries: Vec<BcdValueSnapshot>,
}

fn backup_dir(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|e| AppError::Message(format!("failed to resolve local app data dir: {e}")))?;
    let backup_dir = root.join(BACKUP_DIR_NAME);
    fs::create_dir_all(&backup_dir)
        .map_err(|e| AppError::Message(format!("failed to create backup dir '{}': {e}", backup_dir.display())))?;
    Ok(backup_dir)
}

fn snapshot_path(app: &tauri::AppHandle, file_name: &str) -> Result<PathBuf, AppError> {
    Ok(backup_dir(app)?.join(file_name))
}

fn load_snapshot<T: DeserializeOwned>(
    app: &tauri::AppHandle,
    file_name: &str,
) -> Result<Option<T>, AppError> {
    let path = snapshot_path(app, file_name)?;
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| AppError::Message(format!("failed to read snapshot '{}': {e}", path.display())))?;
    if content.trim().is_empty() {
        return Ok(None);
    }
    let snapshot = serde_json::from_str::<T>(&content)
        .map_err(|e| AppError::Message(format!("failed to parse snapshot '{}': {e}", path.display())))?;
    Ok(Some(snapshot))
}

fn save_snapshot<T: Serialize>(
    app: &tauri::AppHandle,
    file_name: &str,
    snapshot: &T,
) -> Result<(), AppError> {
    let path = snapshot_path(app, file_name)?;
    let data = serde_json::to_string_pretty(snapshot)
        .map_err(|e| AppError::Message(format!("failed to serialize snapshot '{}': {e}", path.display())))?;
    atomic_write_json(&path, &data)
}

fn delete_snapshot(app: &tauri::AppHandle, file_name: &str) -> Result<(), AppError> {
    let path = snapshot_path(app, file_name)?;
    if !path.exists() {
        return Ok(());
    }
    fs::remove_file(&path)
        .map_err(|e| AppError::Message(format!("failed to remove snapshot '{}': {e}", path.display())))
}

pub(crate) fn build_registry_restore_actions(
    entries: &[RegistryValueSnapshot],
) -> Vec<RegistryRestoreAction> {
    entries
        .iter()
        .map(|entry| match (&entry.existed_before, &entry.previous_value) {
            (false, _) | (_, None) => RegistryRestoreAction::Delete {
                path: entry.path.clone(),
                name: entry.name.clone(),
            },
            (true, Some(SnapshotValue::Dword(value))) => RegistryRestoreAction::SetDword {
                path: entry.path.clone(),
                name: entry.name.clone(),
                value: *value,
            },
            (true, Some(SnapshotValue::String(value))) => RegistryRestoreAction::SetString {
                path: entry.path.clone(),
                name: entry.name.clone(),
                value: value.clone(),
            },
        })
        .collect()
}

fn merge_registry_entry(
    entries: &mut Vec<RegistryValueSnapshot>,
    candidate: RegistryValueSnapshot,
) {
    let exists = entries.iter().any(|entry| {
        entry.path.eq_ignore_ascii_case(&candidate.path)
            && entry.name.eq_ignore_ascii_case(&candidate.name)
    });
    if !exists {
        entries.push(candidate);
    }
}

pub(crate) fn load_internet_snapshot(
    app: &tauri::AppHandle,
) -> Result<Option<InternetSnapshot>, AppError> {
    load_snapshot(app, INTERNET_SNAPSHOT_FILE)
}

pub(crate) fn save_internet_snapshot(
    app: &tauri::AppHandle,
    snapshot: &InternetSnapshot,
) -> Result<(), AppError> {
    save_snapshot(app, INTERNET_SNAPSHOT_FILE, snapshot)
}

pub(crate) fn delete_internet_snapshot(app: &tauri::AppHandle) -> Result<(), AppError> {
    delete_snapshot(app, INTERNET_SNAPSHOT_FILE)
}

pub(crate) fn ensure_internet_registry_entry(
    app: &tauri::AppHandle,
    candidate: RegistryValueSnapshot,
) -> Result<(), AppError> {
    let mut snapshot = load_internet_snapshot(app)?.unwrap_or_default();
    merge_registry_entry(&mut snapshot.registry_entries, candidate);
    save_internet_snapshot(app, &snapshot)
}

pub(crate) fn ensure_dns_snapshot(
    app: &tauri::AppHandle,
    candidate: DnsInterfaceSnapshot,
) -> Result<(), AppError> {
    let mut snapshot = load_internet_snapshot(app)?.unwrap_or_default();
    let exists = snapshot
        .dns_interfaces
        .iter()
        .any(|entry| entry.guid.eq_ignore_ascii_case(&candidate.guid));
    if !exists {
        snapshot.dns_interfaces.push(candidate);
        save_internet_snapshot(app, &snapshot)?;
    }
    Ok(())
}

pub(crate) fn load_telemetry_snapshot(
    app: &tauri::AppHandle,
) -> Result<Option<TelemetrySnapshot>, AppError> {
    load_snapshot(app, TELEMETRY_SNAPSHOT_FILE)
}

pub(crate) fn save_telemetry_snapshot(
    app: &tauri::AppHandle,
    snapshot: &TelemetrySnapshot,
) -> Result<(), AppError> {
    save_snapshot(app, TELEMETRY_SNAPSHOT_FILE, snapshot)
}

pub(crate) fn delete_telemetry_snapshot(app: &tauri::AppHandle) -> Result<(), AppError> {
    delete_snapshot(app, TELEMETRY_SNAPSHOT_FILE)
}

pub(crate) fn ensure_telemetry_registry_entry(
    app: &tauri::AppHandle,
    candidate: RegistryValueSnapshot,
) -> Result<(), AppError> {
    let mut snapshot = load_telemetry_snapshot(app)?.unwrap_or_default();
    merge_registry_entry(&mut snapshot.registry_entries, candidate);
    save_telemetry_snapshot(app, &snapshot)
}

pub(crate) fn ensure_service_snapshot(
    app: &tauri::AppHandle,
    candidate: ServiceStateSnapshot,
) -> Result<(), AppError> {
    let mut snapshot = load_telemetry_snapshot(app)?.unwrap_or_default();
    let exists = snapshot
        .services
        .iter()
        .any(|service| service.name.eq_ignore_ascii_case(&candidate.name));
    if !exists {
        snapshot.services.push(candidate);
        save_telemetry_snapshot(app, &snapshot)?;
    }
    Ok(())
}

pub(crate) fn ensure_task_snapshot(
    app: &tauri::AppHandle,
    candidate: ScheduledTaskSnapshot,
) -> Result<(), AppError> {
    let mut snapshot = load_telemetry_snapshot(app)?.unwrap_or_default();
    let exists = snapshot
        .scheduled_tasks
        .iter()
        .any(|task| task.task_name.eq_ignore_ascii_case(&candidate.task_name));
    if !exists {
        snapshot.scheduled_tasks.push(candidate);
        save_telemetry_snapshot(app, &snapshot)?;
    }
    Ok(())
}

pub(crate) fn ensure_hosts_snapshot(
    app: &tauri::AppHandle,
    candidate: HostsFileSnapshot,
) -> Result<(), AppError> {
    let mut snapshot = load_telemetry_snapshot(app)?.unwrap_or_default();
    if !snapshot.hosts_snapshot_captured {
        let has_legacy_hosts_payload =
            !snapshot.hosts.original_content.is_empty() || snapshot.hosts.had_optimus_block;
        if !has_legacy_hosts_payload {
            snapshot.hosts = candidate;
        }
        snapshot.hosts_snapshot_captured = true;
        save_telemetry_snapshot(app, &snapshot)?;
    }
    Ok(())
}

pub(crate) fn save_policy_security_snapshot(
    app: &tauri::AppHandle,
    candidate: PolicySecuritySnapshot,
) -> Result<(), AppError> {
    let mut snapshot = load_telemetry_snapshot(app)?.unwrap_or_default();
    snapshot.policy_security = candidate;
    save_telemetry_snapshot(app, &snapshot)
}

pub(crate) fn load_power_snapshot(
    app: &tauri::AppHandle,
) -> Result<Option<PowerSnapshot>, AppError> {
    load_snapshot(app, POWER_SNAPSHOT_FILE)
}

pub(crate) fn save_power_snapshot(
    app: &tauri::AppHandle,
    snapshot: &PowerSnapshot,
) -> Result<(), AppError> {
    save_snapshot(app, POWER_SNAPSHOT_FILE, snapshot)
}

pub(crate) fn delete_power_snapshot(app: &tauri::AppHandle) -> Result<(), AppError> {
    delete_snapshot(app, POWER_SNAPSHOT_FILE)
}

pub(crate) fn ensure_power_registry_entry(
    app: &tauri::AppHandle,
    candidate: RegistryValueSnapshot,
) -> Result<(), AppError> {
    let mut snapshot = load_power_snapshot(app)?.unwrap_or_default();
    merge_registry_entry(&mut snapshot.registry_entries, candidate);
    save_power_snapshot(app, &snapshot)
}

pub(crate) fn ensure_power_active_plan(
    app: &tauri::AppHandle,
    guid: Option<String>,
) -> Result<(), AppError> {
    let mut snapshot = load_power_snapshot(app)?.unwrap_or_default();
    if snapshot.active_plan_guid.is_none() {
        snapshot.active_plan_guid = guid;
        save_power_snapshot(app, &snapshot)?;
    }
    Ok(())
}

pub(crate) fn load_advanced_snapshot(
    app: &tauri::AppHandle,
) -> Result<Option<AdvancedSnapshot>, AppError> {
    load_snapshot(app, ADVANCED_SNAPSHOT_FILE)
}

pub(crate) fn save_advanced_snapshot(
    app: &tauri::AppHandle,
    snapshot: &AdvancedSnapshot,
) -> Result<(), AppError> {
    save_snapshot(app, ADVANCED_SNAPSHOT_FILE, snapshot)
}

pub(crate) fn delete_advanced_snapshot(app: &tauri::AppHandle) -> Result<(), AppError> {
    delete_snapshot(app, ADVANCED_SNAPSHOT_FILE)
}

pub(crate) fn ensure_advanced_registry_entry(
    app: &tauri::AppHandle,
    candidate: RegistryValueSnapshot,
) -> Result<(), AppError> {
    let mut snapshot = load_advanced_snapshot(app)?.unwrap_or_default();
    merge_registry_entry(&mut snapshot.registry_entries, candidate);
    save_advanced_snapshot(app, &snapshot)
}

pub(crate) fn ensure_advanced_bcd_entry(
    app: &tauri::AppHandle,
    candidate: BcdValueSnapshot,
) -> Result<(), AppError> {
    let mut snapshot = load_advanced_snapshot(app)?.unwrap_or_default();
    let exists = snapshot
        .bcd_entries
        .iter()
        .any(|entry| entry.name.eq_ignore_ascii_case(&candidate.name));
    if !exists {
        snapshot.bcd_entries.push(candidate);
        save_advanced_snapshot(app, &snapshot)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::optimization::backup_manager::{
        build_registry_restore_actions, RegistryRestoreAction, RegistryValueSnapshot, SnapshotValue,
    };

    #[test]
    fn snapshot_restore_builds_exact_restore_actions() {
        let entries = vec![
            RegistryValueSnapshot {
                path: "HKLM\\Foo".into(),
                name: "Enabled".into(),
                existed_before: true,
                previous_value: Some(SnapshotValue::Dword(7)),
            },
            RegistryValueSnapshot {
                path: "HKLM\\Bar".into(),
                name: "Label".into(),
                existed_before: true,
                previous_value: Some(SnapshotValue::String("hello".into())),
            },
            RegistryValueSnapshot {
                path: "HKLM\\Baz".into(),
                name: "Missing".into(),
                existed_before: false,
                previous_value: None,
            },
        ];

        let actions = build_registry_restore_actions(&entries);
        assert_eq!(
            actions,
            vec![
                RegistryRestoreAction::SetDword {
                    path: "HKLM\\Foo".into(),
                    name: "Enabled".into(),
                    value: 7,
                },
                RegistryRestoreAction::SetString {
                    path: "HKLM\\Bar".into(),
                    name: "Label".into(),
                    value: "hello".into(),
                },
                RegistryRestoreAction::Delete {
                    path: "HKLM\\Baz".into(),
                    name: "Missing".into(),
                },
            ]
        );
    }
}
