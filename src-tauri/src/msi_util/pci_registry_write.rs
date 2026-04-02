use crate::types::{AppError, MsiPriorityDto};

use super::{key_exists, set_dword_value, PCI_ROOT};

pub(crate) fn set_msi_mode(
    device_id: &str,
    enable: bool,
    priority: MsiPriorityDto,
) -> Result<(), AppError> {
    let normalized = device_id.trim().trim_matches('\\');
    if normalized.is_empty() {
        return Err(AppError::Message("Device ID cannot be empty".to_owned()));
    }

    let instance_key = format!(r"{}\{}", PCI_ROOT, normalized);
    if !key_exists(&instance_key)? {
        return Err(AppError::Message(format!(
            "PCI device '{}' was not found",
            normalized
        )));
    }

    let msi_key = format!(
        r"{}\Device Parameters\Interrupt Management\MessageSignaledInterruptProperties",
        instance_key
    );
    let affinity_key = format!(
        r"{}\Device Parameters\Interrupt Management\Affinity Policy",
        instance_key
    );

    set_dword_value(&msi_key, "MSISupported", if enable { 1 } else { 0 })?;
    set_dword_value(&affinity_key, "DevicePriority", priority.to_registry())?;
    Ok(())
}
