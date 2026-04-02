use crate::core::warn;
use crate::optimization::backup_manager::{
    build_registry_restore_actions, ensure_advanced_registry_entry,
    ensure_internet_registry_entry, load_advanced_snapshot, load_internet_snapshot,
    RegistryRestoreAction, RegistryValueSnapshot, SnapshotValue,
};
use crate::types::AppError;
use crate::utils::registry_cli::{
    reg_delete_value, reg_list_subkeys, reg_query_dword_value, reg_query_string_value,
    reg_set_dword_value, reg_set_string_value,
};
use std::ffi::CStr;
use tauri::AppHandle;
use windows::Win32::Foundation::ERROR_BUFFER_OVERFLOW;
use windows::Win32::NetworkManagement::IpHelper::{
    GetAdaptersAddresses, GAA_FLAG_INCLUDE_PREFIX, IF_TYPE_ETHERNET_CSMACD, IF_TYPE_IEEE80211,
    IP_ADAPTER_ADDRESSES_LH,
};

const INTERFACES_ROOT: &str = r"HKLM\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces";
const SYSTEM_PROFILE_KEY: &str =
    r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile";
const ADAPTER_CLASS_ROOT: &str =
    r"HKLM\SYSTEM\CurrentControlSet\Control\Class\{4d36e972-e325-11ce-bfc1-08002be10318}";
const TCP_ACK_FREQUENCY: &str = "TcpAckFrequency";
const TCP_NO_DELAY: &str = "TcpNoDelay";
const INTERRUPT_MODERATION: &str = "*InterruptModeration";
const NETCFG_INSTANCE_ID: &str = "NetCfgInstanceId";
const NETWORK_THROTTLING_INDEX: &str = "NetworkThrottlingIndex";
const SYSTEM_RESPONSIVENESS: &str = "SystemResponsiveness";
const NETWORK_THROTTLING_DISABLED: u32 = 0xFFFF_FFFF;
const SYSTEM_RESPONSIVENESS_GAMING: u32 = 0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveInterfaceTarget {
    pub(crate) guid: String,
    pub(crate) interface_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InterruptModerationTarget {
    pub(crate) guid: String,
    pub(crate) interface_name: String,
    pub(crate) registry_key: String,
    pub(crate) value_kind: InterruptModerationValueKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InterruptModerationValueKind {
    Dword,
    String,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct InterruptModerationStatus {
    pub(crate) adapters_total: usize,
    pub(crate) adapters_tuned: usize,
    pub(crate) readable: bool,
    pub(crate) applied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InterruptModerationProbe {
    guid: String,
    interface_name: String,
    registry_key: String,
    current_value: Option<SnapshotValue>,
    value_kind: Option<InterruptModerationValueKind>,
    readable: bool,
}

#[derive(Debug, Clone, Default)]
struct InterruptModerationQuery {
    value: Option<SnapshotValue>,
    value_kind: Option<InterruptModerationValueKind>,
    readable: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct NetRegistryStatus {
    pub(crate) interfaces_total: usize,
    pub(crate) interfaces_readable: usize,
    pub(crate) interfaces_tuned: usize,
    pub(crate) tcp_tweaks_readable: bool,
    pub(crate) registry_throttling_applied: bool,
    pub(crate) registry_throttling_readable: bool,
}

fn interface_registry_key(guid: &str) -> String {
    format!(r"{INTERFACES_ROOT}\{guid}")
}

fn normalize_guid(adapter_name: &str) -> String {
    let trimmed = adapter_name.trim().trim_matches('{').trim_matches('}');
    format!("{{{trimmed}}}")
}

fn pstr_to_string(ptr: windows::core::PSTR) -> String {
    if ptr.0.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr.0 as *const i8) }
        .to_string_lossy()
        .into_owned()
}

fn pwstr_to_string(ptr: windows::core::PWSTR) -> String {
    if ptr.0.is_null() {
        return String::new();
    }

    unsafe {
        let mut len = 0usize;
        while *ptr.0.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr.0, len))
    }
}

fn is_supported_live_interface(adapter: &IP_ADAPTER_ADDRESSES_LH) -> bool {
    adapter.OperStatus.0 == 1
        && matches!(adapter.IfType, IF_TYPE_ETHERNET_CSMACD | IF_TYPE_IEEE80211)
        && adapter.PhysicalAddressLength > 0
}

pub(crate) fn list_active_interface_targets() -> Result<Vec<ActiveInterfaceTarget>, AppError> {
    let mut out_buf_len = 15_000u32;

    loop {
        let mut buffer = vec![0u8; out_buf_len as usize];
        let result = unsafe {
            GetAdaptersAddresses(
                0,
                GAA_FLAG_INCLUDE_PREFIX,
                None,
                Some(buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH),
                &mut out_buf_len,
            )
        };

        if result == ERROR_BUFFER_OVERFLOW.0 {
            continue;
        }
        if result != 0 {
            return Err(AppError::Message(format!(
                "GetAdaptersAddresses failed: {result}"
            )));
        }

        let mut targets = Vec::new();
        let mut current = buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
        while !current.is_null() {
            let adapter = unsafe { &*current };
            if is_supported_live_interface(adapter) {
                let guid = normalize_guid(&pstr_to_string(adapter.AdapterName));
                let interface_name = pwstr_to_string(adapter.FriendlyName).trim().to_owned();
                if !guid.is_empty() && !interface_name.is_empty() {
                    targets.push(ActiveInterfaceTarget { guid, interface_name });
                }
            }
            current = unsafe { (*current).Next };
        }

        targets.sort_by(|a, b| a.interface_name.cmp(&b.interface_name).then(a.guid.cmp(&b.guid)));
        targets.dedup_by(|a, b| a.guid.eq_ignore_ascii_case(&b.guid));
        return Ok(targets);
    }
}

fn set_dword(key: &str, value_name: &str, value: u32) -> Result<(), AppError> {
    reg_set_dword_value(key, value_name, value)
}

fn delete_value(key: &str, value_name: &str) -> Result<(), AppError> {
    reg_delete_value(key, value_name)
}

fn query_dword(key: &str, value_name: &str) -> Result<Option<u32>, AppError> {
    reg_query_dword_value(key, value_name)
}

fn query_string(key: &str, value_name: &str) -> Result<Option<String>, AppError> {
    reg_query_string_value(key, value_name)
}

fn set_string(key: &str, value_name: &str, value: &str) -> Result<(), AppError> {
    reg_set_string_value(key, value_name, value)
}

fn is_not_found_error(err: &AppError) -> bool {
    let lower = err.to_string().to_lowercase();
    lower.contains("os error 2")
        || lower.contains("not found")
        || lower.contains("cannot find")
        || lower.contains("unable to find")
        || lower.contains("does not exist")
}

fn parse_interrupt_moderation_string(value: &str) -> Option<u32> {
    value.trim().parse::<u32>().ok()
}

fn is_adapter_instance_registry_key(key: &str) -> bool {
    let Some(leaf) = key.rsplit('\\').next() else {
        return false;
    };
    leaf.len() == 4 && leaf.bytes().all(|byte| byte.is_ascii_digit())
}

fn list_adapter_instance_keys() -> Result<Vec<String>, AppError> {
    let mut keys: Vec<String> = reg_list_subkeys(ADAPTER_CLASS_ROOT)?
        .into_iter()
        .filter(|key| is_adapter_instance_registry_key(key))
        .collect();
    keys.sort();
    keys.dedup();
    Ok(keys)
}

fn query_interrupt_moderation_value(key: &str) -> InterruptModerationQuery {
    let mut had_error = false;
    match query_dword(key, INTERRUPT_MODERATION) {
        Ok(Some(value)) => {
            return InterruptModerationQuery {
                value: Some(SnapshotValue::Dword(value)),
                value_kind: Some(InterruptModerationValueKind::Dword),
                readable: true,
            };
        }
        Ok(None) => {}
        Err(err) => {
            had_error = true;
            warn!(
                "failed to read interrupt moderation as DWORD on '{}', trying string fallback: {}",
                key, err
            );
        }
    }

    match query_string(key, INTERRUPT_MODERATION) {
        Ok(Some(value)) => {
            let normalized = value.trim().to_owned();
            if normalized.is_empty() {
                return InterruptModerationQuery {
                    value: None,
                    value_kind: None,
                    readable: !had_error,
                };
            }

            if parse_interrupt_moderation_string(&normalized).is_some() {
                return InterruptModerationQuery {
                    value: Some(SnapshotValue::String(normalized)),
                    value_kind: Some(InterruptModerationValueKind::String),
                    readable: true,
                };
            }

            warn!(
                "failed to parse interrupt moderation string value on '{}': '{}'",
                key, normalized
            );
            InterruptModerationQuery {
                value: Some(SnapshotValue::String(normalized)),
                value_kind: Some(InterruptModerationValueKind::String),
                readable: true,
            }
        }
        Ok(None) => InterruptModerationQuery {
            value: None,
            value_kind: None,
            readable: !had_error,
        },
        Err(err) => {
            warn!(
                "failed to read interrupt moderation as string on '{}': {}",
                key, err
            );
            InterruptModerationQuery {
                value: None,
                value_kind: None,
                readable: false,
            }
        }
    }
}

fn normalize_compare_guid(value: &str) -> String {
    value
        .trim()
        .trim_matches('{')
        .trim_matches('}')
        .to_ascii_lowercase()
}

#[cfg(test)]
fn build_interrupt_moderation_targets_from_pairs(
    active_targets: &[ActiveInterfaceTarget],
    class_keys: &[(String, Option<String>, Option<u32>, Option<String>)],
) -> Vec<InterruptModerationTarget> {
    let mut targets = Vec::new();
    for active in active_targets {
        let expected = normalize_compare_guid(&active.guid);
        for (registry_key, netcfg_id, dword_value, string_value) in class_keys {
            let Some(netcfg_id) = netcfg_id.as_ref() else {
                continue;
            };
            if normalize_compare_guid(netcfg_id) != expected {
                continue;
            }

            let value_kind = if dword_value.is_some() {
                Some(InterruptModerationValueKind::Dword)
            } else if string_value.is_some() {
                Some(InterruptModerationValueKind::String)
            } else {
                None
            };

            if let Some(value_kind) = value_kind {
                targets.push(InterruptModerationTarget {
                    guid: active.guid.clone(),
                    interface_name: active.interface_name.clone(),
                    registry_key: registry_key.clone(),
                    value_kind,
                });
            }
        }
    }
    targets.sort_by(|a, b| a.interface_name.cmp(&b.interface_name).then(a.guid.cmp(&b.guid)));
    targets.dedup_by(|a, b| a.registry_key.eq_ignore_ascii_case(&b.registry_key));
    targets
}

fn probe_interrupt_moderation_targets() -> Result<Vec<InterruptModerationProbe>, AppError> {
    let active_targets = list_active_interface_targets()?;
    if active_targets.is_empty() {
        return Ok(Vec::new());
    }

    let mut probes = Vec::new();
    for key in list_adapter_instance_keys()? {
        let netcfg_id = match query_string(&key, NETCFG_INSTANCE_ID) {
            Ok(value) => value,
            Err(err) => {
                warn!(
                    "failed to read adapter instance id '{}' for interrupt moderation probe: {}",
                    key, err
                );
                continue;
            }
        };
        let Some(netcfg_id) = netcfg_id else {
            continue;
        };

        for active in &active_targets {
            if normalize_compare_guid(&netcfg_id) != normalize_compare_guid(&active.guid) {
                continue;
            }

            let query = query_interrupt_moderation_value(&key);
            if query.value_kind.is_none() && query.readable {
                continue;
            }

            probes.push(InterruptModerationProbe {
                guid: active.guid.clone(),
                interface_name: active.interface_name.clone(),
                registry_key: key.clone(),
                current_value: query.value,
                value_kind: query.value_kind,
                readable: query.readable,
            });
        }
    }

    probes.sort_by(|a, b| a.interface_name.cmp(&b.interface_name).then(a.guid.cmp(&b.guid)));
    probes.dedup_by(|a, b| a.registry_key.eq_ignore_ascii_case(&b.registry_key));
    Ok(probes)
}

fn list_interrupt_moderation_targets() -> Result<Vec<InterruptModerationTarget>, AppError> {
    let mut targets = Vec::new();
    for probe in probe_interrupt_moderation_targets()? {
        let Some(value_kind) = probe.value_kind else {
            continue;
        };
        if !probe.readable {
            continue;
        }

        targets.push(InterruptModerationTarget {
            guid: probe.guid,
            interface_name: probe.interface_name,
            registry_key: probe.registry_key,
            value_kind,
        });
    }
    Ok(targets)
}

fn compute_interrupt_moderation_status(
    probes: &[InterruptModerationProbe],
) -> InterruptModerationStatus {
    if probes.is_empty() {
        return InterruptModerationStatus::default();
    }

    let adapters_total = probes.len();
    let mut readable_targets = 0usize;
    let mut adapters_tuned = 0usize;

    for probe in probes {
        if !probe.readable {
            continue;
        }

        readable_targets += 1;
        let is_disabled = match (&probe.value_kind, &probe.current_value) {
            (Some(InterruptModerationValueKind::Dword), Some(SnapshotValue::Dword(value))) => {
                *value == 0
            }
            (Some(InterruptModerationValueKind::String), Some(SnapshotValue::String(value))) => {
                value.trim() == "0"
            }
            (Some(InterruptModerationValueKind::String), Some(SnapshotValue::Dword(value))) => {
                *value == 0
            }
            _ => false,
        };
        if is_disabled {
            adapters_tuned += 1;
        }
    }

    let readable = adapters_total > 0 && readable_targets == adapters_total;
    InterruptModerationStatus {
        adapters_total,
        adapters_tuned,
        readable,
        applied: readable && adapters_tuned == adapters_total,
    }
}

pub(crate) fn capture_active_tcp_registry_snapshots(app: &AppHandle) -> Result<(), AppError> {
    for target in list_active_interface_targets()? {
        let key = interface_registry_key(&target.guid);
        for value_name in [TCP_ACK_FREQUENCY, TCP_NO_DELAY] {
            let previous = query_dword(&key, value_name)?;
            ensure_internet_registry_entry(
                app,
                RegistryValueSnapshot {
                    path: key.clone(),
                    name: value_name.to_owned(),
                    existed_before: previous.is_some(),
                    previous_value: previous.map(SnapshotValue::Dword),
                },
            )?;
        }
    }
    Ok(())
}

pub(crate) fn capture_registry_throttling_snapshot(app: &AppHandle) -> Result<(), AppError> {
    for value_name in [NETWORK_THROTTLING_INDEX, SYSTEM_RESPONSIVENESS] {
        let previous = query_dword(SYSTEM_PROFILE_KEY, value_name)?;
        ensure_internet_registry_entry(
            app,
            RegistryValueSnapshot {
                path: SYSTEM_PROFILE_KEY.to_owned(),
                name: value_name.to_owned(),
                existed_before: previous.is_some(),
                previous_value: previous.map(SnapshotValue::Dword),
            },
        )?;
    }
    Ok(())
}

pub(crate) fn capture_interrupt_moderation_snapshots(app: &AppHandle) -> Result<(), AppError> {
    for target in list_interrupt_moderation_targets()? {
        let previous_value = query_interrupt_moderation_value(&target.registry_key).value;

        ensure_advanced_registry_entry(
            app,
            RegistryValueSnapshot {
                path: target.registry_key.clone(),
                name: INTERRUPT_MODERATION.to_owned(),
                existed_before: previous_value.is_some(),
                previous_value,
            },
        )?;
    }
    Ok(())
}

fn restore_tcp_tweaks_defaults() -> Result<(), AppError> {
    for target in list_active_interface_targets()? {
        let key = interface_registry_key(&target.guid);
        if let Err(err) = delete_value(&key, TCP_ACK_FREQUENCY) {
            warn!(
                "failed to delete fallback TCP tweak '{}' on '{}': {}",
                TCP_ACK_FREQUENCY, key, err
            );
        }
        if let Err(err) = delete_value(&key, TCP_NO_DELAY) {
            warn!(
                "failed to delete fallback TCP tweak '{}' on '{}': {}",
                TCP_NO_DELAY, key, err
            );
        }
    }
    Ok(())
}

fn restore_registry_throttling_defaults() -> Result<(), AppError> {
    delete_value(SYSTEM_PROFILE_KEY, NETWORK_THROTTLING_INDEX)?;
    delete_value(SYSTEM_PROFILE_KEY, SYSTEM_RESPONSIVENESS)?;
    Ok(())
}

fn restore_interrupt_moderation_defaults() -> Result<(), AppError> {
    for target in list_interrupt_moderation_targets()? {
        let result = match target.value_kind {
            InterruptModerationValueKind::Dword => {
                set_dword(&target.registry_key, INTERRUPT_MODERATION, 1)
            }
            InterruptModerationValueKind::String => {
                set_string(&target.registry_key, INTERRUPT_MODERATION, "1")
            }
        };
        if let Err(err) = result {
            warn!(
                "failed to restore fallback interrupt moderation on '{}': {}",
                target.registry_key, err
            );
        }
    }
    Ok(())
}

pub(crate) fn restore_tcp_tweaks_from_snapshot(app: &AppHandle) -> Result<(), AppError> {
    let Some(snapshot) = load_internet_snapshot(app)? else {
        warn!("internet snapshot missing; restoring TCP tweaks to defaults");
        return restore_tcp_tweaks_defaults();
    };

    let entries: Vec<RegistryValueSnapshot> = snapshot
        .registry_entries
        .into_iter()
        .filter(|entry| {
            entry.name.eq_ignore_ascii_case(TCP_ACK_FREQUENCY)
                || entry.name.eq_ignore_ascii_case(TCP_NO_DELAY)
        })
        .collect();

    for action in build_registry_restore_actions(&entries) {
        match action {
            RegistryRestoreAction::Delete { path, name } => {
                delete_value(&path, &name)?;
            }
            RegistryRestoreAction::SetDword { path, name, value } => {
                set_dword(&path, &name, value)?;
            }
            RegistryRestoreAction::SetString { .. } => {
                return Err(AppError::Message(
                    "unexpected string restore action for TCP tweak snapshot".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn restore_registry_throttling_from_snapshot(app: &AppHandle) -> Result<(), AppError> {
    let Some(snapshot) = load_internet_snapshot(app)? else {
        warn!("internet snapshot missing; restoring throttling values to defaults");
        return restore_registry_throttling_defaults();
    };

    let entries: Vec<RegistryValueSnapshot> = snapshot
        .registry_entries
        .into_iter()
        .filter(|entry| {
            entry.name.eq_ignore_ascii_case(NETWORK_THROTTLING_INDEX)
                || entry.name.eq_ignore_ascii_case(SYSTEM_RESPONSIVENESS)
        })
        .collect();

    for action in build_registry_restore_actions(&entries) {
        match action {
            RegistryRestoreAction::Delete { path, name } => delete_value(&path, &name)?,
            RegistryRestoreAction::SetDword { path, name, value } => {
                set_dword(&path, &name, value)?
            }
            RegistryRestoreAction::SetString { .. } => {
                return Err(AppError::Message(
                    "unexpected string restore action for throttling snapshot".to_owned(),
                ))
            }
        }
    }
    Ok(())
}

pub(crate) fn restore_interrupt_moderation_from_snapshot(app: &AppHandle) -> Result<(), AppError> {
    let Some(snapshot) = load_advanced_snapshot(app)? else {
        warn!("advanced snapshot missing; restoring interrupt moderation to defaults");
        return restore_interrupt_moderation_defaults();
    };

    let entries: Vec<RegistryValueSnapshot> = snapshot
        .registry_entries
        .into_iter()
        .filter(|entry| entry.name.eq_ignore_ascii_case(INTERRUPT_MODERATION))
        .collect();

    for action in build_registry_restore_actions(&entries) {
        match action {
            RegistryRestoreAction::Delete { path, name } => delete_value(&path, &name)?,
            RegistryRestoreAction::SetDword { path, name, value } => set_dword(&path, &name, value)?,
            RegistryRestoreAction::SetString { path, name, value } => set_string(&path, &name, &value)?,
        }
    }
    Ok(())
}

pub(crate) fn apply_tcp_tweaks() -> Result<(), AppError> {
    let interfaces = list_active_interface_targets()?;
    if interfaces.is_empty() {
        return Err(AppError::Message(
            "No active hardware network interfaces found.".to_owned(),
        ));
    }

    for target in &interfaces {
        let key = interface_registry_key(&target.guid);
        if let Err(err) = set_dword(&key, TCP_ACK_FREQUENCY, 1) {
            warn!(
                "failed to set {}=1 (REG_DWORD) on '{}': {}",
                TCP_ACK_FREQUENCY, key, err
            );
        }
        if let Err(err) = set_dword(&key, TCP_NO_DELAY, 1) {
            warn!(
                "failed to set {}=1 (REG_DWORD) on '{}': {}",
                TCP_NO_DELAY, key, err
            );
        }
    }

    Ok(())
}

pub(crate) fn apply_advanced_net_tweaks() -> Result<(), AppError> {
    set_dword(
        SYSTEM_PROFILE_KEY,
        NETWORK_THROTTLING_INDEX,
        NETWORK_THROTTLING_DISABLED,
    )?;
    set_dword(
        SYSTEM_PROFILE_KEY,
        SYSTEM_RESPONSIVENESS,
        SYSTEM_RESPONSIVENESS_GAMING,
    )?;
    Ok(())
}

pub(crate) fn apply_interrupt_moderation() -> Result<(), AppError> {
    let targets = list_interrupt_moderation_targets()?;
    if targets.is_empty() {
        return Err(AppError::Message(
            "No active adapters with interrupt moderation settings found.".to_owned(),
        ));
    }

    for target in &targets {
        let result = match target.value_kind {
            InterruptModerationValueKind::Dword => set_dword(&target.registry_key, INTERRUPT_MODERATION, 0),
            InterruptModerationValueKind::String => {
                set_string(&target.registry_key, INTERRUPT_MODERATION, "0")
            }
        };
        if let Err(err) = result {
            warn!(
                "failed to set interrupt moderation on '{}': {}",
                target.registry_key, err
            );
        }
    }

    Ok(())
}

pub(crate) fn check_interrupt_moderation_status() -> Result<InterruptModerationStatus, AppError> {
    Ok(compute_interrupt_moderation_status(
        &probe_interrupt_moderation_targets()?,
    ))
}

pub(crate) fn check_net_registry_status() -> Result<NetRegistryStatus, AppError> {
    let interfaces = list_active_interface_targets()?;
    let interfaces_total = interfaces.len();
    let mut interfaces_tuned = 0usize;
    let mut interfaces_readable = 0usize;

    for target in &interfaces {
        let key = interface_registry_key(&target.guid);
        let ack_result = query_dword(&key, TCP_ACK_FREQUENCY);
        let nodelay_result = query_dword(&key, TCP_NO_DELAY);
        match (ack_result, nodelay_result) {
            (Ok(ack), Ok(nodelay)) => {
                interfaces_readable += 1;
                if ack == Some(1) && nodelay == Some(1) {
                    interfaces_tuned += 1;
                }
            }
            (ack_result, nodelay_result) => {
                let ack_not_found = ack_result
                    .as_ref()
                    .err()
                    .is_some_and(is_not_found_error);
                let nodelay_not_found = nodelay_result
                    .as_ref()
                    .err()
                    .is_some_and(is_not_found_error);

                if ack_not_found || nodelay_not_found {
                    interfaces_readable += 1;
                    continue;
                }

                let ack_err = ack_result.err();
                let nodelay_err = nodelay_result.err();
                warn!(
                    "failed to read TCP tweak values for interface '{}': ack={:?}, nodelay={:?}",
                    key, ack_err, nodelay_err
                );
            }
        }
    }

    let (network_throttling_value, network_throttling_readable) =
        match query_dword(SYSTEM_PROFILE_KEY, NETWORK_THROTTLING_INDEX) {
            Ok(value) => (value, true),
            Err(err) if is_not_found_error(&err) => (None, true),
            Err(err) => {
                warn!(
                    "failed to read registry throttling value '{}': {}",
                    NETWORK_THROTTLING_INDEX, err
                );
                (None, false)
            }
        };
    let (system_responsiveness_value, system_responsiveness_readable) =
        match query_dword(SYSTEM_PROFILE_KEY, SYSTEM_RESPONSIVENESS) {
            Ok(value) => (value, true),
            Err(err) if is_not_found_error(&err) => (None, true),
            Err(err) => {
                warn!(
                    "failed to read registry throttling value '{}': {}",
                    SYSTEM_RESPONSIVENESS, err
                );
                (None, false)
            }
        };
    let registry_throttling_readable =
        network_throttling_readable && system_responsiveness_readable;
    let network_throttling_ok = network_throttling_value == Some(NETWORK_THROTTLING_DISABLED);
    let system_responsiveness_ok = system_responsiveness_value == Some(SYSTEM_RESPONSIVENESS_GAMING);

    Ok(NetRegistryStatus {
        interfaces_total,
        interfaces_readable,
        interfaces_tuned,
        tcp_tweaks_readable: interfaces_total == 0 || interfaces_readable == interfaces_total,
        registry_throttling_applied: registry_throttling_readable
            && network_throttling_ok
            && system_responsiveness_ok,
        registry_throttling_readable,
    })
}

#[cfg(test)]
mod tests {
    use crate::net_tuning::net_registry::{
        build_interrupt_moderation_targets_from_pairs, compute_interrupt_moderation_status,
        is_adapter_instance_registry_key, normalize_guid, ActiveInterfaceTarget,
        InterruptModerationProbe, InterruptModerationValueKind,
    };
    use crate::optimization::backup_manager::SnapshotValue;

    #[test]
    fn normalizes_adapter_guid_for_registry_lookup() {
        assert_eq!(
            normalize_guid("{12345678-1234-1234-1234-1234567890ab}"),
            "{12345678-1234-1234-1234-1234567890ab}"
        );
        assert_eq!(
            normalize_guid("12345678-1234-1234-1234-1234567890ab"),
            "{12345678-1234-1234-1234-1234567890ab}"
        );
    }

    #[test]
    fn matches_active_adapter_to_class_registry_key() {
        let active = vec![ActiveInterfaceTarget {
            guid: "{12345678-1234-1234-1234-1234567890ab}".into(),
            interface_name: "Ethernet".into(),
        }];
        let class_keys = vec![
            (
                r"HKLM\System\Class\0000".to_string(),
                Some("{12345678-1234-1234-1234-1234567890ab}".to_string()),
                None,
                Some("0".to_string()),
            ),
            (
                r"HKLM\System\Class\0001".to_string(),
                Some("{aaaaaaaa-1234-1234-1234-1234567890ab}".to_string()),
                Some(0),
                None,
            ),
        ];

        let targets = build_interrupt_moderation_targets_from_pairs(&active, &class_keys);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].registry_key, r"HKLM\System\Class\0000");
        assert_eq!(targets[0].value_kind, InterruptModerationValueKind::String);
    }

    #[test]
    fn adapter_instance_filter_skips_properties_branch() {
        assert!(is_adapter_instance_registry_key(
            r"HKLM\SYSTEM\CurrentControlSet\Control\Class\{guid}\0001"
        ));
        assert!(!is_adapter_instance_registry_key(
            r"HKLM\SYSTEM\CurrentControlSet\Control\Class\{guid}\Properties"
        ));
        assert!(!is_adapter_instance_registry_key(
            r"HKLM\SYSTEM\CurrentControlSet\Control\Class\{guid}\00A1"
        ));
    }

    #[test]
    fn interrupt_moderation_status_marks_unreadable_probe_without_poisoning_totals() {
        let probes = vec![InterruptModerationProbe {
            guid: "{12345678-1234-1234-1234-1234567890ab}".into(),
            interface_name: "Ethernet".into(),
            registry_key: r"HKLM\System\Class\0000".into(),
            current_value: None,
            value_kind: None,
            readable: false,
        }];

        let status = compute_interrupt_moderation_status(&probes);
        assert_eq!(status.adapters_total, 1);
        assert_eq!(status.adapters_tuned, 0);
        assert!(!status.readable);
        assert!(!status.applied);
    }

    #[test]
    fn interrupt_moderation_status_requires_all_targets_disabled() {
        let probes = vec![
            InterruptModerationProbe {
                guid: "{12345678-1234-1234-1234-1234567890ab}".into(),
                interface_name: "Ethernet".into(),
                registry_key: r"HKLM\System\Class\0000".into(),
                current_value: Some(SnapshotValue::String("0".into())),
                value_kind: Some(InterruptModerationValueKind::String),
                readable: true,
            },
            InterruptModerationProbe {
                guid: "{aaaaaaaa-1234-1234-1234-1234567890ab}".into(),
                interface_name: "Wi-Fi".into(),
                registry_key: r"HKLM\System\Class\0001".into(),
                current_value: Some(SnapshotValue::Dword(1)),
                value_kind: Some(InterruptModerationValueKind::Dword),
                readable: true,
            },
        ];

        let status = compute_interrupt_moderation_status(&probes);
        assert_eq!(status.adapters_total, 2);
        assert_eq!(status.adapters_tuned, 1);
        assert!(status.readable);
        assert!(!status.applied);
    }
}
