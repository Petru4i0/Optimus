use crate::core::{error, info, warn, Command, HKEY_LOCAL_MACHINE, KEY_READ, RegKey};
use crate::types::{AppError, GhostDeviceDto};
use crate::utils::pnputil_utils::{
    pnputil_command as shared_pnputil_command, pnputil_not_found_text as shared_not_found_text,
    pnputil_output_text as shared_output_text,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::io;
use std::os::windows::process::CommandExt;

const ENUM_ROOT_SUBKEY: &str = r"SYSTEM\CurrentControlSet\Enum";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CimGhostRecord {
    #[serde(default)]
    instance_id: Option<String>,
    #[serde(default)]
    device_description: Option<String>,
    #[serde(default)]
    class_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

fn pnputil_command(args: &[&str]) -> Result<std::process::Output, AppError> {
    shared_pnputil_command(args)
}

fn output_text(output: &std::process::Output) -> String {
    shared_output_text(output)
}

fn is_not_found_text(text: &str) -> bool {
    shared_not_found_text(text)
}

fn parse_json_payload<T: DeserializeOwned>(payload: &str) -> Result<Vec<T>, AppError> {
    let trimmed = payload.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if trimmed.eq_ignore_ascii_case("null") || trimmed == "[]" {
        return Ok(Vec::new());
    }

    match serde_json::from_str::<OneOrMany<T>>(trimmed)
        .map_err(|err| AppError::Message(format!("failed to parse CIM JSON payload: {err}")))?
    {
        OneOrMany::One(item) => Ok(vec![item]),
        OneOrMany::Many(items) => Ok(items),
    }
}

fn run_powershell_scan(script: &str, context: &str) -> Result<std::process::Output, AppError> {
    Command::new("powershell")
        .creation_flags(0x08000000)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .output()
        .map_err(|e| AppError::Message(format!("failed to execute {context}: {e}")))
}

fn normalize_text(value: Option<String>) -> String {
    value.unwrap_or_default().trim().to_owned()
}

fn is_not_found_io(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::NotFound || error.raw_os_error() == Some(2)
}

fn normalize_instance_id(instance_id: &str) -> String {
    instance_id.trim().trim_matches('\\').replace('/', "\\")
}

fn evaluate_safety(instance_id: &str, class_name: &str, device_description: &str) -> String {
    let instance_lower = instance_id.trim().to_lowercase();
    let class_lower = class_name.trim().to_lowercase();
    if instance_lower.starts_with("sw\\") || instance_lower.starts_with("root\\") || class_lower == "system" {
        return "Critical".to_owned();
    }

    let description_lower = device_description.trim().to_lowercase();
    if description_lower.contains("unknown")
        || description_lower.contains("failed")
        || description_lower.contains("error")
        || description_lower.contains("unrecognized")
    {
        return "Junk".to_owned();
    }

    "Caution".to_owned()
}

fn cleanup_removed_instance_registry(instance_id: &str) {
    let normalized_instance = normalize_instance_id(instance_id);
    if normalized_instance.is_empty() {
        return;
    }

    let target_subkey = format!(r"{}\{}", ENUM_ROOT_SUBKEY, normalized_instance);
    info!("ghost cleanup target: HKLM\\{}", target_subkey);

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    match hklm.delete_subkey_all(&target_subkey) {
        Ok(()) => info!("ghost cleanup deleted: HKLM\\{}", target_subkey),
        Err(error) if is_not_found_io(&error) => {
            info!("ghost cleanup skipped missing key: HKLM\\{}", target_subkey);
        }
        Err(error) => {
            warn!(
                "ghost cleanup failed for HKLM\\{}: {}",
                target_subkey, error
            );
            return;
        }
    }

    let mut ancestors = Vec::new();
    let mut cursor = normalized_instance.as_str();
    while let Some((parent, _)) = cursor.rsplit_once('\\') {
        if parent.is_empty() {
            break;
        }
        ancestors.push(parent.to_owned());
        cursor = parent;
    }

    for ancestor in ancestors {
        let full_ancestor = format!(r"{}\{}", ENUM_ROOT_SUBKEY, ancestor);
        info!("ghost cleanup ancestor check: HKLM\\{}", full_ancestor);

        let ancestor_key = match hklm.open_subkey_with_flags(&full_ancestor, KEY_READ) {
            Ok(key) => key,
            Err(error) if is_not_found_io(&error) => continue,
            Err(error) => {
                warn!(
                    "ghost cleanup failed to open ancestor HKLM\\{}: {}",
                    full_ancestor, error
                );
                continue;
            }
        };

        match ancestor_key.enum_keys().next() {
            None => match hklm.delete_subkey(&full_ancestor) {
                Ok(()) => info!("ghost cleanup removed empty ancestor: HKLM\\{}", full_ancestor),
                Err(error) if is_not_found_io(&error) => {}
                Err(error) => warn!(
                    "ghost cleanup failed removing ancestor HKLM\\{}: {}",
                    full_ancestor, error
                ),
            },
            Some(Ok(_)) => break,
            Some(Err(error)) => {
                warn!(
                    "ghost cleanup failed enumerating ancestor HKLM\\{}: {}",
                    full_ancestor, error
                );
                break;
            }
        }
    }
}

fn build_ghost_devices(records: Vec<CimGhostRecord>) -> Vec<GhostDeviceDto> {
    let mut result = Vec::new();
    for record in records {
        let instance_id = normalize_text(record.instance_id);
        if instance_id.is_empty() {
            continue;
        }

        let device_description = normalize_text(record.device_description);
        let class_name = normalize_text(record.class_name);
        let safety_level = evaluate_safety(&instance_id, &class_name, &device_description);

        result.push(GhostDeviceDto {
            instance_id,
            device_description,
            class_name,
            safety_level,
        });
    }

    result.sort_by(|a, b| a.instance_id.to_lowercase().cmp(&b.instance_id.to_lowercase()));
    result
}

fn collect_ghost_records(script: &str, context: &str) -> Result<Vec<CimGhostRecord>, AppError> {
    let output = run_powershell_scan(script, context)?;
    if !output.status.success() {
        return Err(AppError::Message(format!(
            "{context} failed: {}",
            output_text(&output)
        )));
    }

    parse_json_payload::<CimGhostRecord>(&output_text(&output))
}

pub(crate) fn get_ghost_devices() -> Result<Vec<GhostDeviceDto>, AppError> {
    let primary_script = concat!(
        "$OutputEncoding = [System.Text.Encoding]::UTF8; try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {};",
        "$ErrorActionPreference='Stop';",
        "$ghosts = @(Get-CimInstance Win32_PnPEntity | Where-Object { $_.ConfigManagerErrorCode -eq 45 } | Select-Object ",
        "@{Name='InstanceId';Expression={$_.PNPDeviceID}},",
        "@{Name='DeviceDescription';Expression={$_.Name}},",
        "@{Name='ClassName';Expression={$_.PNPClass}});",
        "$ghosts | ConvertTo-Json -Depth 4 -Compress"
    );

    let fallback_script = concat!(
        "$OutputEncoding = [System.Text.Encoding]::UTF8; try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {};",
        "$ErrorActionPreference='Stop';",
        "$ghosts = @(Get-PnpDevice -PresentOnly:$false | Where-Object { -not $_.Present } | Select-Object ",
        "@{Name='InstanceId';Expression={$_.InstanceId}},",
        "@{Name='DeviceDescription';Expression={$_.FriendlyName}},",
        "@{Name='ClassName';Expression={$_.Class}});",
        "$ghosts | ConvertTo-Json -Depth 4 -Compress"
    );

    let records = match collect_ghost_records(
        primary_script,
        "collecting Win32_PnPEntity disconnected inventory",
    ) {
        Ok(records) if !records.is_empty() => records,
        Ok(_) => {
            warn!(
                "Ghost device primary query returned zero records; falling back to Get-PnpDevice"
            );
            collect_ghost_records(
                fallback_script,
                "collecting disconnected devices via Get-PnpDevice fallback",
            )
            .map_err(|fallback_err| {
                error!(
                    "Ghost device fallback query failed after empty primary result: {}",
                    fallback_err
                );
                fallback_err
            })?
        }
        Err(primary_err) => {
            error!("Ghost device primary query failed: {}", primary_err);
            collect_ghost_records(
                fallback_script,
                "collecting disconnected devices via Get-PnpDevice fallback",
            )
            .map_err(|fallback_err| {
                error!(
                    "Ghost device fallback query failed after primary error: {}",
                    fallback_err
                );
                AppError::Message(format!(
                    "Primary failed: {}, Fallback failed: {}",
                    primary_err, fallback_err
                ))
            })?
        }
    };
    Ok(build_ghost_devices(records))
}

pub(crate) fn remove_ghost_device(instance_id: &str, force: bool) -> Result<(), AppError> {
    let normalized = instance_id.trim();
    if normalized.is_empty() {
        return Err(AppError::Message("Instance ID cannot be empty".to_owned()));
    }

    let output = if force {
        let forced = pnputil_command(&["/remove-device", normalized, "/force"])?;
        if forced.status.success() {
            forced
        } else {
            warn!(
                "pnputil /remove-device /force failed for '{}', retrying without /force",
                normalized
            );
            pnputil_command(&["/remove-device", normalized])?
        }
    } else {
        pnputil_command(&["/remove-device", normalized])?
    };

    if output.status.success() {
        cleanup_removed_instance_registry(normalized);
        return Ok(());
    }

    let text = output_text(&output);
    if is_not_found_text(&text) {
        cleanup_removed_instance_registry(normalized);
        return Ok(());
    }

    Err(AppError::Message(format!(
        "failed to remove ghost device '{}': {}",
        normalized, text
    )))
}

#[cfg(test)]
mod tests {
    use crate::ghost_devices::{build_ghost_devices, evaluate_safety, CimGhostRecord};

    #[test]
    fn marks_system_or_software_enumerated_devices_as_critical() {
        let records = vec![
            CimGhostRecord {
                instance_id: Some("ROOT\\USB\\0000".to_owned()),
                device_description: Some("PCI Express Root Port".to_owned()),
                class_name: Some("SoftwareDevice".to_owned()),
            },
            CimGhostRecord {
                instance_id: Some("USB\\VID_046D&PID_C534\\6&2B5D2AF4&0&3".to_owned()),
                device_description: Some("USB Receiver".to_owned()),
                class_name: Some("HIDClass".to_owned()),
            },
        ];
        let parsed = build_ghost_devices(records);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].safety_level, "Critical");
        assert_eq!(parsed[1].safety_level, "Caution");
    }

    #[test]
    fn marks_broken_descriptions_as_junk() {
        assert_eq!(
            evaluate_safety(r"USB\VID_1234&PID_5678", "USB", "Unknown USB device"),
            "Junk"
        );
        assert_eq!(
            evaluate_safety(r"USB\VID_1234&PID_5678", "USB", "Normal Device"),
            "Caution"
        );
    }
}

