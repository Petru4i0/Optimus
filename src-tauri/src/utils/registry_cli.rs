use crate::core::{Command, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE, RegKey};
use crate::types::AppError;
use std::io;
use std::os::windows::process::CommandExt;
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};

fn io_error_to_app_error(context: &str, key: &str, error: io::Error) -> AppError {
    AppError::Message(format!("{} '{}' failed: {}", context, key, error))
}

fn is_not_found_io(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::NotFound || error.raw_os_error() == Some(2)
}

fn parse_hklm_subkey(key: &str) -> Result<String, AppError> {
    let trimmed = key.trim().trim_matches('\\');
    if let Some(subkey) = trimmed.strip_prefix("HKLM\\") {
        return Ok(subkey.to_owned());
    }
    if let Some(subkey) = trimmed.strip_prefix("HKEY_LOCAL_MACHINE\\") {
        return Ok(subkey.to_owned());
    }
    Err(AppError::Message(format!(
        "unsupported registry hive in key '{}'; only HKLM is supported",
        key
    )))
}

fn normalize_hklm_full(key: &str) -> Result<String, AppError> {
    Ok(format!("HKLM\\{}", parse_hklm_subkey(key)?))
}

fn open_hklm_read_raw(key: &str) -> Result<RegKey, io::Error> {
    let subkey = parse_hklm_subkey(key).map_err(|e| io::Error::other(e.to_string()))?;
    RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(&subkey, KEY_READ)
}

fn open_hklm_write_raw(key: &str) -> Result<RegKey, io::Error> {
    let subkey = parse_hklm_subkey(key).map_err(|e| io::Error::other(e.to_string()))?;
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .create_subkey_with_flags(&subkey, KEY_READ | KEY_WRITE)
        .map(|(k, _)| k)
}

pub(crate) fn output_text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

pub(crate) fn is_not_found_text(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("unable to find")
        || lower.contains("cannot find")
        || lower.contains("the system was unable to find")
        || lower.contains("the system cannot find")
        || lower.contains("error: the system cannot find")
        || lower.contains("does not exist")
        || lower.contains("not found")
        || lower.contains("?? ??????????")
        || lower.contains("?? ??????? ?????")
        || lower.contains("??????? ?? ??????? ?????")
        || lower.contains("system error: 2")
        || lower.contains("?????? ???????: 2")
}

pub(crate) fn run_command(
    program: &str,
    args: &[&str],
    context: &str,
) -> Result<std::process::Output, AppError> {
    Command::new(program)
        .creation_flags(0x08000000)
        .args(args)
        .output()
        .map_err(|e| AppError::Message(format!("failed to execute {context}: {e}")))
}

pub(crate) fn run_command_with_timeout(
    program: &str,
    args: &[&str],
    context: &str,
    timeout: Duration,
) -> Result<std::process::Output, AppError> {
    let mut child = Command::new(program)
        .creation_flags(0x08000000)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::Message(format!("failed to execute {context}: {e}")))?;
    let deadline = Instant::now() + timeout;

    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child.wait_with_output().map_err(|e| {
                    AppError::Message(format!("failed to collect output for {context}: {e}"))
                });
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(AppError::Message(format!(
                        "{context} timed out after {} seconds",
                        timeout.as_secs()
                    )));
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(AppError::Message(format!(
                    "failed while waiting for {context}: {e}"
                )));
            }
        }
    }
}

pub(crate) fn schtasks_command(args: &[&str]) -> Result<std::process::Output, AppError> {
    run_command("schtasks", args, &format!("schtasks {:?}", args))
}

pub(crate) fn netsh_command(args: &[&str]) -> Result<std::process::Output, AppError> {
    run_command("netsh", args, &format!("netsh {:?}", args))
}

pub(crate) fn reg_key_exists(key: &str) -> Result<bool, AppError> {
    match open_hklm_read_raw(key) {
        Ok(_) => Ok(true),
        Err(error) if is_not_found_io(&error) => Ok(false),
        Err(error) => Err(io_error_to_app_error("open_subkey_with_flags(KEY_READ)", key, error)),
    }
}

pub(crate) fn reg_set_dword_value(key: &str, value_name: &str, value: u32) -> Result<(), AppError> {
    let reg_key = open_hklm_write_raw(key)
        .map_err(|e| io_error_to_app_error("create_subkey_with_flags(KEY_WRITE)", key, e))?;
    reg_key
        .set_value(value_name, &value)
        .map_err(|e| io_error_to_app_error("set_value(REG_DWORD)", key, e))
}

pub(crate) fn reg_set_string_value(
    key: &str,
    value_name: &str,
    value: &str,
) -> Result<(), AppError> {
    let reg_key = open_hklm_write_raw(key)
        .map_err(|e| io_error_to_app_error("create_subkey_with_flags(KEY_WRITE)", key, e))?;
    reg_key
        .set_value(value_name, &value)
        .map_err(|e| io_error_to_app_error("set_value(REG_SZ)", key, e))
}

pub(crate) fn reg_create_key(key: &str) -> Result<(), AppError> {
    let _ = open_hklm_write_raw(key)
        .map_err(|e| io_error_to_app_error("create_subkey_with_flags(KEY_WRITE)", key, e))?;
    Ok(())
}

pub(crate) fn reg_delete_key_tree(key: &str) -> Result<(), AppError> {
    let subkey = parse_hklm_subkey(key)?;
    let mut parts = subkey.rsplitn(2, '\\');
    let child = parts
        .next()
        .ok_or_else(|| AppError::Message(format!("invalid registry key '{}'", key)))?;
    let parent = match parts.next() {
        Some(value) => value,
        None => return Ok(()),
    };

    let root = RegKey::predef(HKEY_LOCAL_MACHINE);
    let parent_key = match root.open_subkey_with_flags(parent, KEY_READ | KEY_WRITE) {
        Ok(value) => value,
        Err(error) if is_not_found_io(&error) => return Ok(()),
        Err(error) => {
            return Err(io_error_to_app_error(
                "open_subkey_with_flags(KEY_WRITE)",
                key,
                error,
            ));
        }
    };

    match parent_key.delete_subkey_all(child) {
        Ok(()) => Ok(()),
        Err(error) if is_not_found_io(&error) => Ok(()),
        Err(error) => Err(io_error_to_app_error("delete_subkey_all", key, error)),
    }
}

pub(crate) fn reg_delete_value(key: &str, value_name: &str) -> Result<(), AppError> {
    let reg_key = match open_hklm_write_raw(key) {
        Ok(value) => value,
        Err(error) if is_not_found_io(&error) => return Ok(()),
        Err(error) => {
            return Err(io_error_to_app_error(
                "create_subkey_with_flags(KEY_WRITE)",
                key,
                error,
            ));
        }
    };

    match reg_key.delete_value(value_name) {
        Ok(()) => Ok(()),
        Err(error) if is_not_found_io(&error) => Ok(()),
        Err(error) => Err(io_error_to_app_error("delete_value", key, error)),
    }
}

#[tracing::instrument(skip_all)]
pub(crate) fn reg_list_subkeys(key: &str) -> Result<Vec<String>, AppError> {
    let reg_key = match open_hklm_read_raw(key) {
        Ok(value) => value,
        Err(error) if is_not_found_io(&error) => return Ok(Vec::new()),
        Err(error) => {
            return Err(io_error_to_app_error(
                "open_subkey_with_flags(KEY_READ)",
                key,
                error,
            ));
        }
    };

    let normalized_root = normalize_hklm_full(key)?;
    let mut result = Vec::new();
    for item in reg_key.enum_keys() {
        let child = item.map_err(|e| io_error_to_app_error("enum_keys", key, e))?;
        result.push(format!("{}\\{}", normalized_root, child));
    }
    result.sort();
    result.dedup();
    Ok(result)
}

#[tracing::instrument(skip_all)]
pub(crate) fn reg_query_string_value(key: &str, value_name: &str) -> Result<Option<String>, AppError> {
    let reg_key = match open_hklm_read_raw(key) {
        Ok(value) => value,
        Err(error) if is_not_found_io(&error) => return Ok(None),
        Err(error) => {
            return Err(io_error_to_app_error(
                "open_subkey_with_flags(KEY_READ)",
                key,
                error,
            ));
        }
    };

    match reg_key.get_value::<String, _>(value_name) {
        Ok(value) => Ok(Some(value)),
        Err(error) if is_not_found_io(&error) => Ok(None),
        Err(error) => Err(io_error_to_app_error("get_value(String)", key, error)),
    }
}

#[tracing::instrument(skip_all)]
pub(crate) fn reg_query_dword_value(key: &str, value_name: &str) -> Result<Option<u32>, AppError> {
    let reg_key = match open_hklm_read_raw(key) {
        Ok(value) => value,
        Err(error) if is_not_found_io(&error) => return Ok(None),
        Err(error) => {
            return Err(io_error_to_app_error(
                "open_subkey_with_flags(KEY_READ)",
                key,
                error,
            ));
        }
    };

    match reg_key.get_value::<u32, _>(value_name) {
        Ok(value) => Ok(Some(value)),
        Err(error) if is_not_found_io(&error) => Ok(None),
        Err(error) => Err(io_error_to_app_error("get_value(DWORD)", key, error)),
    }
}

