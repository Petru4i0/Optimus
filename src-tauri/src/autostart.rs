use crate::*;

pub(crate) fn set_elevated_autostart_task(enabled: bool) -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|e| format!("failed to resolve current executable: {e}"))?;
    let exe_arg = format!("\"{}\"", exe.to_string_lossy());

    let output = if enabled {
        Command::new("schtasks")
            .arg("/create")
            .arg("/tn")
            .arg(ELEVATED_AUTOSTART_TASK_NAME)
            .arg("/tr")
            .arg(exe_arg)
            .arg("/sc")
            .arg("onlogon")
            .arg("/rl")
            .arg("highest")
            .arg("/f")
            .output()
            .map_err(|e| format!("failed to execute schtasks /create: {e}"))?
    } else {
        Command::new("schtasks")
            .arg("/delete")
            .arg("/tn")
            .arg(ELEVATED_AUTOSTART_TASK_NAME)
            .arg("/f")
            .output()
            .map_err(|e| format!("failed to execute schtasks /delete: {e}"))?
    };

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !enabled && (stderr.contains("cannot find") || stderr.contains("ERROR: The system cannot find")) {
        return Ok(());
    }

    Err(format!(
        "schtasks command failed (status {}): {}{}{}",
        output.status,
        stderr,
        if !stderr.is_empty() && !stdout.is_empty() { " | " } else { "" },
        stdout
    ))
}

pub(crate) fn is_elevated_autostart_task_enabled() -> Result<bool, String> {
    let output = Command::new("schtasks")
        .arg("/query")
        .arg("/tn")
        .arg(ELEVATED_AUTOSTART_TASK_NAME)
        .output()
        .map_err(|e| format!("failed to execute schtasks /query: {e}"))?;
    Ok(output.status.success())
}

pub(crate) fn configure_autostart_impl(app: tauri::AppHandle, enabled: bool, as_admin: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enabled {
        if as_admin {
            manager
                .disable()
                .map_err(|e| format!("failed to disable standard autostart: {e}"))?;
            set_elevated_autostart_task(true)?;
        } else {
            set_elevated_autostart_task(false)?;
            manager
                .enable()
                .map_err(|e| format!("failed to enable autostart: {e}"))?;
        }
    } else {
        manager
            .disable()
            .map_err(|e| format!("failed to disable autostart: {e}"))?;
        set_elevated_autostart_task(false)?;
    }
    Ok(())
}
