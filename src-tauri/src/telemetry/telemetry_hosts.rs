use crate::core::{error, fs, Path, PCWSTR, ReplaceFileW, REPLACEFILE_WRITE_THROUGH, Write};
use crate::elevation::to_wide;
use crate::optimization::backup_manager::HostsFileSnapshot;
use crate::types::AppError;
use std::io;

const HOSTS_PATH: &str = r"C:\Windows\System32\drivers\etc\hosts";
const HOSTS_START_MARKER: &str = "# OPTIMUS START";
const HOSTS_END_MARKER: &str = "# OPTIMUS END";
const TELEMETRY_DOMAINS: [&str; 6] = [
    "vortex.data.microsoft.com",
    "settings-win.data.microsoft.com",
    "watson.telemetry.microsoft.com",
    "telemetry.microsoft.com",
    "vortex-win.data.microsoft.com",
    "oca.telemetry.microsoft.com",
];

fn hosts_path() -> &'static Path {
    Path::new(HOSTS_PATH)
}

fn is_permission_denied_io(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::PermissionDenied || error.raw_os_error() == Some(5)
}

pub(crate) fn read_hosts_content() -> Result<String, AppError> {
    let path = hosts_path();
    if !path.exists() {
        return Ok(String::new());
    }

    match fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(error) if is_permission_denied_io(&error) => {
            error!("AV blocked hosts modification: read denied for '{}'", path.display());
            Ok(String::new())
        }
        Err(error) => Err(AppError::Message(format!("failed to read hosts file: {error}"))),
    }
}

fn atomic_write_hosts(content: &str) -> Result<(), AppError> {
    let path = hosts_path();
    let parent = path
        .parent()
        .ok_or_else(|| AppError::Message("hosts path has no parent".to_owned()))?;
    let temp_path = parent.join("hosts.optimus.tmp");

    {
        let mut file = match fs::File::create(&temp_path) {
            Ok(file) => file,
            Err(error) if is_permission_denied_io(&error) => {
                error!(
                    "AV blocked hosts modification: create denied for '{}'",
                    temp_path.display()
                );
                return Ok(());
            }
            Err(error) => {
                return Err(AppError::Message(format!(
                    "failed to create temporary hosts file '{}': {error}",
                    temp_path.display()
                )));
            }
        };
        if let Err(error) = file.write_all(content.as_bytes()) {
            if is_permission_denied_io(&error) {
                error!(
                    "AV blocked hosts modification: write denied for '{}'",
                    temp_path.display()
                );
                let _ = fs::remove_file(&temp_path);
                return Ok(());
            }
            return Err(AppError::Message(format!(
                "failed to write temporary hosts file '{}': {error}",
                temp_path.display()
            )));
        }
        if let Err(error) = file.sync_all() {
            if is_permission_denied_io(&error) {
                error!(
                    "AV blocked hosts modification: sync denied for '{}'",
                    temp_path.display()
                );
                let _ = fs::remove_file(&temp_path);
                return Ok(());
            }
            return Err(AppError::Message(format!(
                "failed to sync temporary hosts file '{}': {error}",
                temp_path.display()
            )));
        }
    }

    if path.exists() {
        let replaced = unsafe {
            ReplaceFileW(
                PCWSTR(to_wide(path.as_os_str()).as_ptr()),
                PCWSTR(to_wide(temp_path.as_os_str()).as_ptr()),
                PCWSTR::null(),
                REPLACEFILE_WRITE_THROUGH,
                None,
                None,
            )
        }
        .is_ok();

        if !replaced {
            let replace_error = io::Error::last_os_error();
            let _ = fs::remove_file(&temp_path);
            if is_permission_denied_io(&replace_error) {
                error!(
                    "AV blocked hosts modification: replace denied for '{}'",
                    path.display()
                );
                return Ok(());
            }
            return Err(AppError::Message(format!(
                "failed to atomically replace '{}' with '{}'",
                path.display(),
                temp_path.display()
            )));
        }
    } else {
        if let Err(e) = fs::rename(&temp_path, path) {
            let _ = fs::remove_file(&temp_path);
            if is_permission_denied_io(&e) {
                error!(
                    "AV blocked hosts modification: move denied for '{}'",
                    path.display()
                );
                return Ok(());
            }
            return Err(AppError::Message(format!(
                "failed to move '{}' into '{}': {e}",
                temp_path.display(),
                path.display()
            )));
        }
    }

    Ok(())
}

fn build_telemetry_block() -> String {
    let mut lines = Vec::with_capacity(TELEMETRY_DOMAINS.len() + 2);
    lines.push(HOSTS_START_MARKER.to_owned());
    lines.extend(
        TELEMETRY_DOMAINS
            .iter()
            .map(|domain| format!("0.0.0.0 {domain}")),
    );
    lines.push(HOSTS_END_MARKER.to_owned());
    lines.join("\n")
}

fn strip_optimus_block(content: &str) -> (String, bool) {
    let mut lines = Vec::new();
    let mut inside_block = false;
    let mut found_block = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case(HOSTS_START_MARKER) {
            inside_block = true;
            found_block = true;
            continue;
        }
        if trimmed.eq_ignore_ascii_case(HOSTS_END_MARKER) {
            inside_block = false;
            continue;
        }
        if !inside_block {
            lines.push(line);
        }
    }

    let mut merged = lines.join("\n");
    if !merged.is_empty() && !merged.ends_with('\n') {
        merged.push('\n');
    }
    (merged, found_block)
}

pub(crate) fn merge_hosts(existing: &str, telemetry_enabled: bool) -> String {
    let (mut stripped, _) = strip_optimus_block(existing);
    if telemetry_enabled {
        if !stripped.is_empty() && !stripped.ends_with('\n') {
            stripped.push('\n');
        }
        stripped.push_str(&build_telemetry_block());
        stripped.push('\n');
    }
    stripped
}

pub(crate) fn capture_hosts_snapshot() -> Result<HostsFileSnapshot, AppError> {
    let original_content = read_hosts_content()?;
    let (_, had_optimus_block) = strip_optimus_block(&original_content);
    Ok(HostsFileSnapshot {
        original_content,
        had_optimus_block,
    })
}

pub(crate) fn restore_hosts_snapshot(snapshot: &HostsFileSnapshot) -> Result<(), AppError> {
    atomic_write_hosts(&snapshot.original_content)
}

pub(crate) fn block_telemetry_hosts() -> Result<(), AppError> {
    let merged = merge_hosts(&read_hosts_content()?, true);
    atomic_write_hosts(&merged)
}

pub(crate) fn check_hosts_status() -> Result<bool, AppError> {
    let content = read_hosts_content()?;
    let lowered = content.to_lowercase();
    if !lowered.contains(&HOSTS_START_MARKER.to_lowercase())
        || !lowered.contains(&HOSTS_END_MARKER.to_lowercase())
    {
        return Ok(false);
    }

    for domain in TELEMETRY_DOMAINS {
        let required = format!("0.0.0.0 {}", domain.to_lowercase());
        if !lowered.contains(&required) {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use crate::telemetry::telemetry_hosts::merge_hosts;

    #[test]
    fn merge_hosts_replaces_existing_optimus_block_without_touching_user_lines() {
        let original = "127.0.0.1 localhost\n# OPTIMUS START\n0.0.0.0 old.example\n# OPTIMUS END\n192.168.0.1 router\n";
        let merged = merge_hosts(original, true);
        assert!(merged.contains("127.0.0.1 localhost"));
        assert!(merged.contains("192.168.0.1 router"));
        assert!(!merged.contains("old.example"));
        assert!(merged.contains("# OPTIMUS START"));
        assert!(merged.contains("0.0.0.0 vortex.data.microsoft.com"));
    }

    #[test]
    fn merge_hosts_removes_optimus_block_on_disable() {
        let original = "127.0.0.1 localhost\n# OPTIMUS START\n0.0.0.0 vortex.data.microsoft.com\n# OPTIMUS END\n";
        let merged = merge_hosts(original, false);
        assert_eq!(merged, "127.0.0.1 localhost\n");
    }
}
