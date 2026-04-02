use crate::core::{Command, OnceLock, RwLock};
use crate::types::AppError;
use crate::utils::registry_cli::output_text;
use std::collections::HashSet;
use std::os::windows::process::CommandExt;
pub(crate) const ULTIMATE_PERFORMANCE_BASE_GUID: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";

#[derive(Debug, Clone)]
struct PowerPlanEntry {
    guid: String,
}

fn ultimate_plan_guid_cache() -> &'static RwLock<Option<String>> {
    static ULTIMATE_PLAN_GUID_CACHE: OnceLock<RwLock<Option<String>>> = OnceLock::new();
    ULTIMATE_PLAN_GUID_CACHE.get_or_init(|| RwLock::new(None))
}

fn powercfg(args: &[&str]) -> Result<std::process::Output, AppError> {
    Command::new("powercfg")
        .creation_flags(0x08000000)
        .args(args)
        .output()
        .map_err(|e| AppError::Message(format!("failed to execute powercfg {:?}: {e}", args)))
}

fn is_guid(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    for (idx, byte) in bytes.iter().enumerate() {
        let is_dash = matches!(idx, 8 | 13 | 18 | 23);
        if is_dash {
            if *byte != b'-' {
                return false;
            }
        } else if !byte.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

fn extract_first_guid(text: &str) -> Option<String> {
    for token in text.split_whitespace() {
        let candidate = token.trim_matches(|c: char| matches!(c, '(' | ')' | '*' | ':'));
        if is_guid(candidate) {
            return Some(candidate.to_lowercase());
        }
    }
    None
}

fn parse_plan_line(line: &str) -> Option<PowerPlanEntry> {
    let guid = extract_first_guid(line)?;
    Some(PowerPlanEntry { guid })
}

fn list_power_plans() -> Result<Vec<PowerPlanEntry>, AppError> {
    let output = powercfg(&["/L"])?;
    if !output.status.success() {
        return Err(AppError::Message(format!(
            "failed to list power plans: {}",
            output_text(&output)
        )));
    }

    let mut plans = Vec::new();
    for line in output_text(&output).lines() {
        if let Some(entry) = parse_plan_line(line) {
            plans.push(entry);
        }
    }
    Ok(plans)
}

fn get_active_plan() -> Result<Option<PowerPlanEntry>, AppError> {
    let output = powercfg(&["/GetActiveScheme"])?;
    if !output.status.success() {
        return Err(AppError::Message(format!(
            "failed to query active power plan: {}",
            output_text(&output)
        )));
    }

    for line in output_text(&output).lines() {
        if let Some(entry) = parse_plan_line(line) {
            return Ok(Some(entry));
        }
    }
    Ok(None)
}

pub(crate) fn set_active_plan_guid(guid: &str) -> Result<(), AppError> {
    let output = powercfg(&["/SETACTIVE", guid])?;
    if output.status.success() {
        return Ok(());
    }

    Err(AppError::Message(format!(
        "failed to set active power plan '{}': {}",
        guid,
        output_text(&output)
    )))
}

pub(crate) fn enable_ultimate_performance() -> Result<(), AppError> {
    let plans = list_power_plans()?;
    let existing_guids: HashSet<String> = plans.iter().map(|plan| plan.guid.clone()).collect();

    let target_guid = if existing_guids.contains(ULTIMATE_PERFORMANCE_BASE_GUID) {
        ULTIMATE_PERFORMANCE_BASE_GUID.to_owned()
    } else {
        let duplicate = powercfg(&["-duplicatescheme", ULTIMATE_PERFORMANCE_BASE_GUID])?;
        if !duplicate.status.success() {
            return Err(AppError::Message(format!(
                "failed to duplicate ultimate performance scheme: {}",
                output_text(&duplicate)
            )));
        }

        if let Some(guid) = extract_first_guid(&output_text(&duplicate)) {
            guid.to_lowercase()
        } else {
            list_power_plans()?
                .into_iter()
                .map(|plan| plan.guid.to_lowercase())
                .find(|guid| !existing_guids.contains(guid))
                .or_else(|| Some(ULTIMATE_PERFORMANCE_BASE_GUID.to_owned()))
                .ok_or_else(|| {
                    AppError::Message(
                        "failed to resolve ultimate performance power plan GUID".to_owned(),
                    )
                })?
        }
    };

    if let Ok(mut guard) = ultimate_plan_guid_cache().write() {
        *guard = Some(target_guid.clone());
    }

    set_active_plan_guid(&target_guid)
}

pub(crate) fn check_ultimate_performance_active() -> Result<bool, AppError> {
    let Some(active) = get_active_plan()?.map(|plan| plan.guid.to_lowercase()) else {
        return Ok(false);
    };

    if active == ULTIMATE_PERFORMANCE_BASE_GUID {
        return Ok(true);
    }

    let cached = ultimate_plan_guid_cache()
        .read()
        .ok()
        .and_then(|guard| guard.clone());
    Ok(cached.is_some_and(|guid| guid == active))
}

pub(crate) fn get_active_plan_guid() -> Result<Option<String>, AppError> {
    Ok(get_active_plan()?.map(|plan| plan.guid.to_lowercase()))
}

