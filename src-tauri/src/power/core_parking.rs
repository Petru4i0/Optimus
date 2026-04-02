use crate::core::Command;
use crate::types::AppError;
use crate::utils::registry_cli::{reg_query_dword_value, reg_set_dword_value};
use std::os::windows::process::CommandExt;
pub(crate) const PROCESSOR_SETTINGS_SUBGROUP_GUID: &str = "54533251-82be-4824-96c1-47b60b740d00";
pub(crate) const CORE_PARKING_MIN_CORES_GUID: &str = "0cc5b647-c1df-4637-891a-dec35c318583";
const CORE_PARKING_DISABLED_VALUE: u32 = 100;
const POWER_SETTINGS_ROOT: &str = r"HKLM\SYSTEM\CurrentControlSet\Control\Power\PowerSettings";
const ATTRIBUTES_VALUE_NAME: &str = "Attributes";

fn powercfg(args: &[&str]) -> Result<std::process::Output, AppError> {
    Command::new("powercfg")
        .creation_flags(0x08000000)
        .args(args)
        .output()
        .map_err(|e| AppError::Message(format!("failed to execute powercfg {:?}: {e}", args)))
}

fn output_text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn extract_first_guid(text: &str) -> Option<String> {
    for token in text.split_whitespace() {
        let candidate = token.trim_matches(|c: char| matches!(c, '(' | ')' | '*' | ':'));
        if candidate.len() == 36
            && candidate
                .chars()
                .enumerate()
                .all(|(idx, ch)| matches!(idx, 8 | 13 | 18 | 23) && ch == '-' || ch.is_ascii_hexdigit())
        {
            return Some(candidate.to_lowercase());
        }
    }
    None
}

fn get_active_plan_guid() -> Result<Option<String>, AppError> {
    let output = powercfg(&["/GetActiveScheme"])?;
    if !output.status.success() {
        return Err(AppError::Message(format!(
            "failed to query active power plan: {}",
            output_text(&output)
        )));
    }

    for line in output_text(&output).lines() {
        if let Some(guid) = extract_first_guid(line) {
            return Ok(Some(guid));
        }
    }

    Ok(None)
}

pub(crate) fn core_parking_setting_key(active_guid: &str) -> String {
    format!(
        r"HKLM\SYSTEM\CurrentControlSet\Control\Power\User\PowerSchemes\{}\{}\{}",
        active_guid, PROCESSOR_SETTINGS_SUBGROUP_GUID, CORE_PARKING_MIN_CORES_GUID
    )
}

pub(crate) fn core_parking_attributes_key() -> String {
    format!(
        r"{POWER_SETTINGS_ROOT}\{}\{}",
        PROCESSOR_SETTINGS_SUBGROUP_GUID, CORE_PARKING_MIN_CORES_GUID
    )
}

pub(crate) fn query_core_parking_indices(
    active_guid: &str,
) -> Result<(Option<u32>, Option<u32>), AppError> {
    let setting_key = core_parking_setting_key(active_guid);
    Ok((
        reg_query_dword_value(&setting_key, "ACSettingIndex")?,
        reg_query_dword_value(&setting_key, "DCSettingIndex")?,
    ))
}

pub(crate) fn query_core_parking_attributes() -> Result<Option<u32>, AppError> {
    reg_query_dword_value(&core_parking_attributes_key(), ATTRIBUTES_VALUE_NAME)
}

fn set_core_parking_value(value: u32) -> Result<(), AppError> {
    let value_text = value.to_string();

    let set_ac = powercfg(&[
        "-setacvalueindex",
        "scheme_current",
        PROCESSOR_SETTINGS_SUBGROUP_GUID,
        CORE_PARKING_MIN_CORES_GUID,
        &value_text,
    ])?;
    if !set_ac.status.success() {
        return Err(AppError::Message(format!(
            "failed to set core parking AC value: {}",
            output_text(&set_ac)
        )));
    }

    let set_dc = powercfg(&[
        "-setdcvalueindex",
        "scheme_current",
        PROCESSOR_SETTINGS_SUBGROUP_GUID,
        CORE_PARKING_MIN_CORES_GUID,
        &value_text,
    ])?;
    if !set_dc.status.success() {
        return Err(AppError::Message(format!(
            "failed to set core parking DC value: {}",
            output_text(&set_dc)
        )));
    }

    let apply = powercfg(&["-setactive", "scheme_current"])?;
    if !apply.status.success() {
        return Err(AppError::Message(format!(
            "failed to apply core parking changes: {}",
            output_text(&apply)
        )));
    }

    Ok(())
}

pub(crate) fn disable_core_parking() -> Result<(), AppError> {
    reg_set_dword_value(&core_parking_attributes_key(), ATTRIBUTES_VALUE_NAME, 0)?;
    set_core_parking_value(CORE_PARKING_DISABLED_VALUE)
}

pub(crate) fn check_core_parking_disabled() -> Result<bool, AppError> {
    let Some(active_guid) = get_active_plan_guid()? else {
        return Ok(false);
    };

    let (ac, dc) = query_core_parking_indices(&active_guid)?;

    Ok(ac == Some(CORE_PARKING_DISABLED_VALUE) && dc == Some(CORE_PARKING_DISABLED_VALUE))
}

