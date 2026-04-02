use crate::core::{error, warn, Command};
use crate::types::{AppError, DriverDto};
use crate::utils::pnputil_utils::{
    pnputil_command_with_timeout as shared_pnputil_command_with_timeout,
    pnputil_not_found_text as shared_not_found_text, pnputil_output_text as shared_output_text,
};
use crate::utils::registry_cli::run_command_with_timeout as shared_run_command_with_timeout;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::HashMap;
use std::os::windows::process::CommandExt;
use std::time::Duration;

const PROTECTED_DRIVER_CLASSES: &[&str] = &[
    "system",
    "computer",
    "processor",
    "volume",
    "volumesnapshot",
    "diskdrive",
    "hdc",
    "scsiadapter",
];

const PROTECTED_DRIVER_SERVICES: &[&str] = &[
    "acpi",
    "pci",
    "partmgr",
    "disk",
    "classpnp",
    "storahci",
    "stornvme",
    "volmgr",
    "volsnap",
    "mountmgr",
    "ndis",
    "vdrvroot",
    "fltmgr",
    "fileinfo",
];

const DELETE_COMMAND_TIMEOUT_SECS: u64 = 15;

#[derive(Default, Clone)]
struct DriverGuardInfo {
    class_name: String,
    service_name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CimDriverRecord {
    #[serde(default)]
    published_name: Option<String>,
    #[serde(default)]
    original_name: Option<String>,
    #[serde(default)]
    provider_name: Option<String>,
    #[serde(default)]
    class_name: Option<String>,
    #[serde(default)]
    driver_version: Option<String>,
    #[serde(default)]
    driver_date: Option<String>,
    #[serde(default)]
    service_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

#[derive(Default, Clone)]
struct DriverInventoryEntry {
    published_name: String,
    original_name: String,
    provider_name: String,
    class_name: String,
    driver_version: String,
    driver_date: String,
    service_name: String,
    safety_level: String,
}

fn pnputil_command_with_timeout(args: &[&str]) -> Result<std::process::Output, AppError> {
    shared_pnputil_command_with_timeout(args, Duration::from_secs(DELETE_COMMAND_TIMEOUT_SECS))
}

fn output_text(output: &std::process::Output) -> String {
    shared_output_text(output)
}

fn is_not_found_text(text: &str) -> bool {
    shared_not_found_text(text)
}

fn is_missing_oem_inf_text(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("oem inf")
        && (lower.contains("not")
            || lower.contains("installed")
            || lower.contains("specified file")
            || lower.contains("не")
            || lower.contains("установ"))
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

fn is_protected_boot_driver(info: &DriverGuardInfo) -> bool {
    let class_lower = info.class_name.trim().to_lowercase();
    if PROTECTED_DRIVER_CLASSES
        .iter()
        .any(|class_name| class_lower == *class_name)
    {
        return true;
    }

    let service_lower = info.service_name.trim().to_lowercase();
    PROTECTED_DRIVER_SERVICES
        .iter()
        .any(|service_name| service_lower == *service_name)
}

fn is_service_running(service_name: &str) -> Result<bool, AppError> {
    let normalized = service_name.trim();
    if normalized.is_empty() {
        return Ok(false);
    }

    let output = shared_run_command_with_timeout(
        "sc",
        &["query", normalized],
        &format!("sc query {}", normalized),
        Duration::from_secs(DELETE_COMMAND_TIMEOUT_SECS),
    )?;
    let text = output_text(&output);
    if !output.status.success() {
        if is_not_found_text(&text) {
            return Ok(false);
        }
        return Err(AppError::Message(format!(
            "failed to query service '{}' state: {}",
            normalized, text
        )));
    }

    let lower = text.to_lowercase();
    Ok(lower.contains("running"))
}

fn normalize_text(value: Option<String>) -> String {
    value.unwrap_or_default().trim().to_owned()
}

fn first_non_empty(target: &mut String, value: &str) {
    if target.is_empty() && !value.is_empty() {
        *target = value.to_owned();
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn evaluate_safety(class_name: &str, provider: &str, original_name: &str) -> String {
    let class_name = class_name.to_lowercase();
    let provider = provider.to_lowercase();
    let original_name = original_name.to_lowercase();

    const JUNK_KEYWORDS: &[&str] = &[
        "vpn",
        "tap-windows",
        "tunnelbear",
        "wireguard",
        "proton",
        "openvpn",
        "printer",
        "virtual",
        "iriun",
        "droidcam",
        "canon",
        "epson",
        "hp",
    ];
    const CRITICAL_PROVIDER_KEYWORDS: &[&str] = &[
        "intel",
        "advanced micro devices",
        "amd",
        "nvidia",
        "realtek",
        "qualcomm",
        "broadcom",
        "mediatek",
    ];
    const CRITICAL_CLASS_KEYWORDS: &[&str] = &[
        "display",
        "system",
        "scsiadapter",
        "hdc",
        "net",
        "bluetooth",
        "diskdrive",
        "processor",
    ];

    if contains_any(&provider, JUNK_KEYWORDS) || contains_any(&original_name, JUNK_KEYWORDS) {
        return "Junk".to_owned();
    }

    if contains_any(&provider, CRITICAL_PROVIDER_KEYWORDS)
        || contains_any(&class_name, CRITICAL_CLASS_KEYWORDS)
    {
        return "Critical".to_owned();
    }

    "Caution".to_owned()
}

fn build_inventory(records: Vec<CimDriverRecord>) -> Vec<DriverInventoryEntry> {
    let mut aggregated: HashMap<String, DriverInventoryEntry> = HashMap::new();

    for record in records {
        let published_name = normalize_text(record.published_name);
        if published_name.is_empty() {
            continue;
        }

        let provider_name = normalize_text(record.provider_name);
        if provider_name.to_ascii_lowercase().contains("microsoft") {
            continue;
        }

        let key = published_name.to_lowercase();
        let entry = aggregated.entry(key).or_insert_with(|| DriverInventoryEntry {
            published_name: published_name.clone(),
            ..DriverInventoryEntry::default()
        });
        first_non_empty(&mut entry.published_name, &published_name);

        let original_name = normalize_text(record.original_name);
        let class_name = normalize_text(record.class_name);
        let driver_version = normalize_text(record.driver_version);
        let driver_date = normalize_text(record.driver_date);
        let service_name = normalize_text(record.service_name);

        first_non_empty(&mut entry.original_name, &original_name);
        first_non_empty(&mut entry.provider_name, &provider_name);
        first_non_empty(&mut entry.class_name, &class_name);
        first_non_empty(&mut entry.driver_version, &driver_version);
        first_non_empty(&mut entry.driver_date, &driver_date);
        first_non_empty(&mut entry.service_name, &service_name);
        entry.safety_level =
            evaluate_safety(&entry.class_name, &entry.provider_name, &entry.original_name);
    }

    let mut result: Vec<DriverInventoryEntry> = aggregated.into_values().collect();
    result.sort_by(|a, b| a.published_name.to_lowercase().cmp(&b.published_name.to_lowercase()));
    result
}

fn collect_driver_inventory() -> Result<Vec<DriverInventoryEntry>, AppError> {
    let script = concat!(
        "$OutputEncoding = [System.Text.Encoding]::UTF8; try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {};",
        "$ErrorActionPreference='Stop';",
        "$svc = @{};",
        "Get-CimInstance Win32_PnPSignedDriver | ForEach-Object {",
        "  if ($_.InfName) {",
        "    $k = ($_.InfName.ToLower());",
        "    if (-not $svc.ContainsKey($k)) {",
        "      $svc[$k] = [pscustomobject]@{",
        "        ServiceName = ($_.Service);",
        "        ClassName = ($_.DriverClass);",
        "        DeviceName = if ($_.DeviceName) { ('' + $_.DeviceName) } elseif ($_.Description) { ('' + $_.Description) } else { '' };",
        "      };",
        "    }",
        "  }",
        "};",
        "$drivers = Get-WindowsDriver -Online | ForEach-Object {",
        "  $pub = ('' + $_.Driver);",
        "  $k = $pub.ToLower();",
        "  $svcInfo = $svc[$k];",
        "  [pscustomobject]@{",
        "    PublishedName = $pub;",
        "    OriginalName = if ($svcInfo -and $svcInfo.DeviceName) { ('' + $svcInfo.DeviceName) } elseif ($_.OriginalFileName) { ('' + $_.OriginalFileName) } elseif ($_.ClassDescription) { ('' + $_.ClassDescription) } else { $pub };",
        "    ProviderName = ('' + $_.ProviderName);",
        "    ClassName = if ($svcInfo -and $svcInfo.ClassName) { ('' + $svcInfo.ClassName) } elseif ($_.ClassName) { ('' + $_.ClassName) } else { ('' + $_.ClassDescription) };",
        "    DriverVersion = ('' + $_.Version);",
        "    DriverDate = if ($_.Date) { try { ([datetime]$_.Date).ToString('yyyy-MM-dd') } catch { '' } } else { '' };",
        "    ServiceName = if ($svcInfo) { ('' + $svcInfo.ServiceName) } else { '' };",
        "  }",
        "};",
        "$drivers | ConvertTo-Json -Depth 4 -Compress"
    );

    let output = run_powershell_scan(script, "collecting DriverStore inventory via Get-WindowsDriver")?;
    if !output.status.success() {
        return Err(AppError::Message(format!(
            "failed to collect driver inventory via DISM: {}",
            output_text(&output)
        )));
    }

    let records = parse_json_payload::<CimDriverRecord>(&output_text(&output))?;
    Ok(build_inventory(records))
}

fn collect_signed_driver_inventory() -> Result<Vec<DriverInventoryEntry>, AppError> {
    let script = concat!(
        "$OutputEncoding = [System.Text.Encoding]::UTF8; try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {};",
        "$ErrorActionPreference='Stop';",
        "$drivers = @(Get-CimInstance Win32_PnPSignedDriver | ForEach-Object {",
        "  [pscustomobject]@{",
        "    PublishedName = if ($_.InfName) { ('' + $_.InfName) } elseif ($_.DeviceID) { ('' + $_.DeviceID) } else { 'oem_unknown.inf' };",
        "    OriginalName = if ($_.DeviceName) { ('' + $_.DeviceName) } elseif ($_.Description) { ('' + $_.Description) } elseif ($_.DriverName) { ('' + $_.DriverName) } else { ('' + $_.InfName) };",
        "    ProviderName = ('' + $_.DriverProviderName);",
        "    ClassName = ('' + $_.DriverClass);",
        "    DriverVersion = ('' + $_.DriverVersion);",
        "    DriverDate = if ($_.DriverDate) { try { ([datetime]$_.DriverDate).ToString('yyyy-MM-dd') } catch { '' } } else { '' };",
        "    ServiceName = ('' + $_.Service);",
        "  }",
        "});",
        "$drivers | ConvertTo-Json -Depth 4 -Compress"
    );

    let output = run_powershell_scan(
        script,
        "collecting signed driver inventory via Win32_PnPSignedDriver",
    )?;
    if !output.status.success() {
        return Err(AppError::Message(format!(
            "failed to collect signed driver inventory via CIM: {}",
            output_text(&output)
        )));
    }

    let records = parse_json_payload::<CimDriverRecord>(&output_text(&output))?;
    Ok(build_inventory(records))
}

fn load_driver_inventory_with_fallback() -> Result<Vec<DriverInventoryEntry>, AppError> {
    match collect_driver_inventory() {
        Ok(inventory) if !inventory.is_empty() => Ok(inventory),
        Ok(_) => {
            warn!(
                "DriverStore DISM query returned zero records; falling back to Win32_PnPSignedDriver"
            );
            collect_signed_driver_inventory().map_err(|fallback_err| {
                error!(
                    "DriverStore fallback query failed after empty primary result: {}",
                    fallback_err
                );
                fallback_err
            })
        }
        Err(primary_err) => {
            error!("DriverStore primary query failed: {}", primary_err);
            collect_signed_driver_inventory().map_err(|fallback_err| {
                error!(
                    "DriverStore fallback query failed after primary error: {}",
                    fallback_err
                );
                AppError::Message(format!(
                    "Primary failed: {}, Fallback failed: {}",
                    primary_err, fallback_err
                ))
            })
        }
    }
}

fn find_driver_guard_info(
    inventory: &[DriverInventoryEntry],
    published_name: &str,
) -> Option<DriverGuardInfo> {
    let target = published_name.trim().to_lowercase();
    if target.is_empty() {
        return None;
    }

    inventory
        .iter()
        .find(|entry| entry.published_name.trim().eq_ignore_ascii_case(&target))
        .map(|entry| DriverGuardInfo {
            class_name: entry.class_name.clone(),
            service_name: entry.service_name.clone(),
        })
}

pub(crate) fn get_installed_drivers() -> Result<Vec<DriverDto>, AppError> {
    let inventory = load_driver_inventory_with_fallback()?;
    let drivers = inventory
        .into_iter()
        .map(|entry| DriverDto {
            published_name: entry.published_name,
            original_name: entry.original_name,
            provider_name: entry.provider_name,
            class_name: entry.class_name,
            driver_version: entry.driver_version,
            driver_date: entry.driver_date,
            safety_level: entry.safety_level,
        })
        .collect();
    Ok(drivers)
}

pub(crate) fn delete_driver(published_name: &str, force: bool) -> Result<(), AppError> {
    let normalized = published_name.trim();
    if normalized.is_empty() {
        return Err(AppError::Message("Published name cannot be empty".to_owned()));
    }

    if force {
        let inventory = load_driver_inventory_with_fallback()?;
        if let Some(guard_info) = find_driver_guard_info(&inventory, normalized) {
            let running = is_service_running(&guard_info.service_name)?;
            if is_protected_boot_driver(&guard_info) {
                return Err(AppError::Message(format!(
                    "PROTECTED_BOOT_DRIVER: force deletion blocked for '{}' (class='{}', service='{}', running={})",
                    normalized,
                    guard_info.class_name,
                    guard_info.service_name,
                    running
                )));
            }
        }
    }

    let mut args = vec!["/delete-driver", normalized, "/uninstall"];
    if force {
        args.push("/force");
    }
    let output = pnputil_command_with_timeout(&args)?;
    if output.status.success() {
        return Ok(());
    }

    let text = output_text(&output);
    if is_not_found_text(&text) {
        return Ok(());
    }
    if is_missing_oem_inf_text(&text) {
        warn!(
            "Driver already missing from disk for '{}': {}",
            normalized, text
        );
        return Ok(());
    }

    Err(AppError::Message(format!(
        "failed to delete driver '{}': {}",
        normalized, text
    )))
}

#[cfg(test)]
mod tests {
    use crate::driver_store::{
        build_inventory, find_driver_guard_info, is_protected_boot_driver, CimDriverRecord,
        DriverGuardInfo,
    };

    #[test]
    fn build_inventory_deduplicates_by_published_name() {
        let records = vec![
            CimDriverRecord {
                published_name: Some("oem14.inf".to_owned()),
                original_name: Some("nv_dispig.inf".to_owned()),
                provider_name: Some("NVIDIA".to_owned()),
                class_name: Some("Display".to_owned()),
                driver_version: Some("32.0.15.6603".to_owned()),
                driver_date: Some("2025-01-22".to_owned()),
                service_name: Some("nvlddmkm".to_owned()),
            },
            CimDriverRecord {
                published_name: Some("oem14.inf".to_owned()),
                original_name: Some(String::new()),
                provider_name: Some(String::new()),
                class_name: Some("Display".to_owned()),
                driver_version: Some("32.0.15.6603".to_owned()),
                driver_date: Some("2025-01-22".to_owned()),
                service_name: Some("nvlddmkm".to_owned()),
            },
        ];

        let parsed = build_inventory(records);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].published_name, "oem14.inf");
        assert_eq!(parsed[0].provider_name, "NVIDIA");
    }

    #[test]
    fn finds_guard_info_for_published_name_case_insensitive() {
        let records = vec![CimDriverRecord {
            published_name: Some("OEM2.inf".to_owned()),
            original_name: Some("netrtwlane.inf".to_owned()),
            provider_name: Some("Realtek".to_owned()),
            class_name: Some("Net".to_owned()),
            driver_version: Some("1.2.3.4".to_owned()),
            driver_date: Some("2024-12-31".to_owned()),
            service_name: Some("rtwlane".to_owned()),
        }];
        let inventory = build_inventory(records);
        let guard = find_driver_guard_info(&inventory, "oem2.inf");
        assert!(guard.is_some());
        assert_eq!(guard.as_ref().map(|item| item.class_name.as_str()), Some("Net"));
    }

    #[test]
    fn protected_boot_driver_detection_matches_class_and_service() {
        assert!(is_protected_boot_driver(&DriverGuardInfo {
            class_name: "System".to_owned(),
            service_name: "random".to_owned(),
        }));
        assert!(is_protected_boot_driver(&DriverGuardInfo {
            class_name: "Net".to_owned(),
            service_name: "acpi".to_owned(),
        }));
        assert!(!is_protected_boot_driver(&DriverGuardInfo {
            class_name: "Net".to_owned(),
            service_name: "rtwlane".to_owned(),
        }));
    }
}

