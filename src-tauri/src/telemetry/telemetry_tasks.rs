use crate::optimization::backup_manager::ScheduledTaskSnapshot;
use crate::types::AppError;
use crate::utils::registry_cli::{
    is_not_found_text, output_text, schtasks_command,
};

pub(crate) const TELEMETRY_TASKS: [&str; 5] = [
    r"\Microsoft\Windows\Application Experience\Microsoft Compatibility Appraiser",
    r"\Microsoft\Windows\Application Experience\ProgramDataUpdater",
    r"\Microsoft\Windows\Customer Experience Improvement Program\Consolidator",
    r"\Microsoft\Windows\Customer Experience Improvement Program\UsbCeip",
    r"\Microsoft\Windows\DiskDiagnostic\Microsoft-Windows-DiskDiagnosticDataCollector",
];

fn parse_csv_row(input: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        let ch = chars[i];
        if ch == '"' {
            if in_quotes && i + 1 < chars.len() && chars[i + 1] == '"' {
                current.push('"');
                i += 2;
                continue;
            }
            in_quotes = !in_quotes;
            i += 1;
            continue;
        }
        if ch == ',' && !in_quotes {
            fields.push(current.trim().to_owned());
            current.clear();
            i += 1;
            continue;
        }

        current.push(ch);
        i += 1;
    }
    fields.push(current.trim().to_owned());
    fields
}

fn query_task_disabled_xml(task_name: &str) -> Result<Option<bool>, AppError> {
    let output = schtasks_command(&["/query", "/tn", task_name, "/xml"])?;
    if !output.status.success() {
        let text = output_text(&output);
        if is_not_found_text(&text) {
            return Ok(Some(true));
        }
        return Err(AppError::Message(format!(
            "failed to query telemetry task xml '{task_name}': {text}"
        )));
    }

    let text = output_text(&output).to_lowercase();
    if text.contains("<enabled>false</enabled>") {
        return Ok(Some(true));
    }
    if text.contains("<enabled>true</enabled>") {
        return Ok(Some(false));
    }
    Ok(None)
}

fn set_task_enabled(task_name: &str, enabled: bool) -> Result<(), AppError> {
    let mode = if enabled { "/enable" } else { "/disable" };
    let output = schtasks_command(&["/change", "/tn", task_name, mode])?;

    if output.status.success() {
        return Ok(());
    }

    let text = output_text(&output);
    if is_not_found_text(&text) {
        return Ok(());
    }

    Err(AppError::Message(format!(
        "failed to change telemetry task '{task_name}': {text}"
    )))
}

fn query_task_disabled(task_name: &str) -> Result<bool, AppError> {
    let output = schtasks_command(&["/query", "/tn", task_name, "/v", "/fo", "csv", "/nh"])?;

    if !output.status.success() {
        let text = output_text(&output);
        if is_not_found_text(&text) {
            return Ok(true);
        }
        return Err(AppError::Message(format!(
            "failed to query telemetry task '{task_name}': {text}"
        )));
    }

    let csv_line = output_text(&output)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .to_owned();

    if !csv_line.is_empty() {
        let fields = parse_csv_row(&csv_line);
        for field in &fields {
            let value = field.to_lowercase();
            if value.contains("disabled") || value.contains("отключено") {
                return Ok(true);
            }
            if value.contains("ready")
                || value.contains("running")
                || value.contains("готово")
                || value.contains("выполняется")
            {
                return Ok(false);
            }
        }
    }

    if let Some(value) = query_task_disabled_xml(task_name)? {
        return Ok(value);
    }

    Ok(false)
}

pub(crate) fn capture_task_state(task_name: &str) -> Result<ScheduledTaskSnapshot, AppError> {
    let disabled = query_task_disabled(task_name)?;
    Ok(ScheduledTaskSnapshot {
        task_name: task_name.to_owned(),
        existed_before: true,
        was_enabled: !disabled,
    })
}

pub(crate) fn restore_task_state(snapshot: &ScheduledTaskSnapshot) -> Result<(), AppError> {
    if !snapshot.existed_before {
        return Ok(());
    }
    set_task_enabled(&snapshot.task_name, snapshot.was_enabled)
}

pub(crate) fn deep_purge_tasks() -> Result<(), AppError> {
    for task in TELEMETRY_TASKS {
        set_task_enabled(task, false)?;
    }
    Ok(())
}

pub(crate) fn check_tasks_status() -> Result<bool, AppError> {
    for task in TELEMETRY_TASKS {
        if !query_task_disabled(task)? {
            return Ok(false);
        }
    }
    Ok(true)
}
