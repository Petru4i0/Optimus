use crate::core::Command;
use crate::optimization::backup_manager::{ensure_advanced_bcd_entry, BcdValueSnapshot};
use crate::types::AppError;
use crate::utils::registry_cli::output_text;
use std::os::windows::process::CommandExt;
use tauri::AppHandle;

const USE_PLATFORM_CLOCK: &str = "useplatformclock";
const DISABLE_DYNAMIC_TICK: &str = "disabledynamictick";

#[derive(Debug, Clone, Default)]
pub(crate) struct TimerOptimizationStatus {
    pub(crate) applied: bool,
    pub(crate) readable: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TimerBcdState {
    pub(crate) useplatformclock: Option<String>,
    pub(crate) disabledynamictick: Option<String>,
}

fn bcdedit(args: &[&str]) -> Result<std::process::Output, AppError> {
    Command::new("bcdedit")
        .creation_flags(0x08000000)
        .args(args)
        .output()
        .map_err(|e| AppError::Message(format!("failed to execute bcdedit {:?}: {e}", args)))
}

fn parse_bcd_state(text: &str) -> TimerBcdState {
    let mut state = TimerBcdState::default();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(key) = parts.next() else {
            continue;
        };
        let value = parts.collect::<Vec<_>>().join(" ");
        if value.is_empty() {
            continue;
        }

        match key.to_ascii_lowercase().as_str() {
            USE_PLATFORM_CLOCK => state.useplatformclock = Some(value.to_ascii_lowercase()),
            DISABLE_DYNAMIC_TICK => state.disabledynamictick = Some(value.to_ascii_lowercase()),
            _ => {}
        }
    }
    state
}

fn query_bcd_state() -> Result<TimerBcdState, AppError> {
    let output = bcdedit(&["/enum", "{current}"])?;
    let text = output_text(&output);
    if !output.status.success() {
        return Err(AppError::Message(format!(
            "bcdedit /enum {{current}} failed: {}",
            text.trim()
        )));
    }
    if text.trim().is_empty() {
        return Ok(TimerBcdState::default());
    }
    Ok(parse_bcd_state(&text))
}

fn set_bcd_value(name: &str, value: &str) -> Result<(), AppError> {
    let output = bcdedit(&["/set", name, value])?;
    if output.status.success() {
        return Ok(());
    }

    Err(AppError::Message(format!(
        "failed to set bcdedit {}={}: {}",
        name,
        value,
        output_text(&output)
    )))
}

fn delete_bcd_value(name: &str) -> Result<(), AppError> {
    let output = bcdedit(&["/deletevalue", name])?;
    if output.status.success() {
        return Ok(());
    }

    let current = query_bcd_state()?;
    let still_exists = match name {
        USE_PLATFORM_CLOCK => current.useplatformclock.is_some(),
        DISABLE_DYNAMIC_TICK => current.disabledynamictick.is_some(),
        _ => false,
    };
    if !still_exists {
        return Ok(());
    }

    Err(AppError::Message(format!(
        "failed to delete bcdedit value '{}': {}",
        name,
        output_text(&output)
    )))
}

pub(crate) fn restore_timer_defaults() -> Result<(), AppError> {
    delete_bcd_value(USE_PLATFORM_CLOCK)?;
    delete_bcd_value(DISABLE_DYNAMIC_TICK)?;
    Ok(())
}

pub(crate) fn capture_timer_bcd_snapshot(app: &AppHandle) -> Result<(), AppError> {
    let state = query_bcd_state()?;
    ensure_advanced_bcd_entry(
        app,
        BcdValueSnapshot {
            name: USE_PLATFORM_CLOCK.to_owned(),
            existed_before: state.useplatformclock.is_some(),
            previous_value: state.useplatformclock,
        },
    )?;
    ensure_advanced_bcd_entry(
        app,
        BcdValueSnapshot {
            name: DISABLE_DYNAMIC_TICK.to_owned(),
            existed_before: state.disabledynamictick.is_some(),
            previous_value: state.disabledynamictick,
        },
    )?;
    Ok(())
}

pub(crate) fn apply_hpet_dynamic_tick() -> Result<(), AppError> {
    delete_bcd_value(USE_PLATFORM_CLOCK)?;
    set_bcd_value(DISABLE_DYNAMIC_TICK, "yes")
}

pub(crate) fn restore_bcd_value(snapshot: &BcdValueSnapshot) -> Result<(), AppError> {
    if snapshot.existed_before {
        if let Some(value) = snapshot.previous_value.as_deref() {
            set_bcd_value(&snapshot.name, value)?;
        } else {
            delete_bcd_value(&snapshot.name)?;
        }
    } else {
        delete_bcd_value(&snapshot.name)?;
    }
    Ok(())
}

pub(crate) fn check_hpet_dynamic_tick_status() -> Result<TimerOptimizationStatus, AppError> {
    let state = query_bcd_state()?;
    Ok(TimerOptimizationStatus {
        applied: state.useplatformclock.is_none()
            && state
                .disabledynamictick
                .as_deref()
                .is_some_and(|value| value.eq_ignore_ascii_case("yes")),
        readable: true,
    })
}

#[cfg(test)]
mod tests {
    use crate::power::timers::parse_bcd_state;

    #[test]
    fn parses_hpet_and_dynamic_tick_lines() {
        let text = r#"
Windows Boot Loader
-------------------
identifier              {current}
useplatformclock        Yes
disabledynamictick      Yes
"#;
        let state = parse_bcd_state(text);
        assert_eq!(state.useplatformclock.as_deref(), Some("yes"));
        assert_eq!(state.disabledynamictick.as_deref(), Some("yes"));
    }

    #[test]
    fn missing_hpet_is_treated_as_absent() {
        let text = r#"
Windows Boot Loader
-------------------
identifier              {current}
disabledynamictick      No
"#;
        let state = parse_bcd_state(text);
        assert!(state.useplatformclock.is_none());
        assert_eq!(state.disabledynamictick.as_deref(), Some("no"));
    }

    #[test]
    fn empty_bcd_output_is_treated_as_default_state() {
        let state = parse_bcd_state("");
        assert!(state.useplatformclock.is_none());
        assert!(state.disabledynamictick.is_none());
    }
}
