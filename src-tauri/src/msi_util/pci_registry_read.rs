use crate::core::warn;
use crate::types::{AppError, MsiPriorityDto, PciDeviceDto};

use super::{list_subkeys, query_dword_value, query_string_value, PCI_ROOT};

fn pci_device_id_from_key(instance_key: &str) -> String {
    instance_key
        .strip_prefix(&format!("{PCI_ROOT}\\"))
        .unwrap_or(instance_key)
        .to_owned()
}

fn split_trailing_args(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim();
    if !trimmed.ends_with(')') {
        return None;
    }

    let mut depth = 0i32;
    for (idx, ch) in trimmed.char_indices().rev() {
        if ch == ')' {
            depth += 1;
        } else if ch == '(' {
            depth -= 1;
            if depth == 0 {
                let left = trimmed[..idx].trim_end();
                let right = &trimmed[idx..];
                return Some((left, right));
            }
        }
    }

    None
}

fn parse_args(args_block: &str) -> Option<Vec<String>> {
    if !args_block.starts_with('(') || !args_block.ends_with(')') {
        return None;
    }
    let inner = args_block[1..args_block.len().saturating_sub(1)].trim();
    if inner.is_empty() {
        return Some(Vec::new());
    }

    let mut args = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut in_quotes = false;

    for ch in inner.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                current.push(ch);
            }
            '(' if !in_quotes => {
                depth += 1;
                current.push(ch);
            }
            ')' if !in_quotes && depth > 0 => {
                depth -= 1;
                current.push(ch);
            }
            ',' if !in_quotes && depth == 0 => {
                args.push(current.trim().to_owned());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        args.push(current.trim().to_owned());
    }

    Some(args)
}

fn remove_placeholder_tokens(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0usize;
    while index < chars.len() {
        if chars[index] == '%' {
            let mut cursor = index + 1;
            let mut saw_digit = false;
            while cursor < chars.len() && chars[cursor].is_ascii_digit() {
                saw_digit = true;
                cursor += 1;
            }
            if saw_digit {
                index = cursor;
                continue;
            }
        }
        output.push(chars[index]);
        index += 1;
    }
    output
}

fn collapse_spaces(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn replace_placeholders(template: &str, args: &[String]) -> String {
    let chars: Vec<char> = template.chars().collect();
    let mut output = String::with_capacity(template.len());
    let mut index = 0usize;
    while index < chars.len() {
        if chars[index] == '%' {
            let mut cursor = index + 1;
            let mut digits = String::new();
            while cursor < chars.len() && chars[cursor].is_ascii_digit() {
                digits.push(chars[cursor]);
                cursor += 1;
            }

            if !digits.is_empty() {
                let replacement = digits
                    .parse::<usize>()
                    .ok()
                    .and_then(|position| position.checked_sub(1))
                    .and_then(|position| args.get(position))
                    .cloned()
                    .unwrap_or_default();
                output.push_str(&replacement);
                index = cursor;
                continue;
            }
        }

        output.push(chars[index]);
        index += 1;
    }

    collapse_spaces(output.trim())
}

fn sanitize_device_desc(raw: &str) -> String {
    let mut normalized = raw.trim();
    if normalized.is_empty() {
        return String::new();
    }

    if normalized.starts_with('@') {
        if let Some((_, tail)) = normalized.split_once(';') {
            normalized = tail.trim();
        }
    }

    if let Some((base_text, args_block)) = split_trailing_args(normalized) {
        if let Some(args) = parse_args(args_block) {
            let base = base_text.trim_end_matches(';').trim();
            let replaced = replace_placeholders(base, &args);
            if !replaced.is_empty() {
                return replaced;
            }
        }
    }

    let fallback_base = normalized
        .split_once(';')
        .map(|(head, _)| head)
        .unwrap_or(normalized);
    collapse_spaces(remove_placeholder_tokens(fallback_base).trim())
}

fn mark_unreadable_on_error<T>(
    result: Result<Option<T>, AppError>,
    readable: &mut bool,
    context: &str,
    device_id: &str,
) -> Option<T> {
    match result {
        Ok(value) => value,
        Err(error) => {
            *readable = false;
            warn!("MSI read warning ({context}) for '{device_id}': {error}");
            None
        }
    }
}

#[tracing::instrument(skip_all)]
pub(crate) fn get_pci_devices() -> Result<Vec<PciDeviceDto>, AppError> {
    let vendor_keys = match list_subkeys(PCI_ROOT) {
        Ok(keys) => keys,
        Err(error) => {
            warn!("MSI root enumeration failed at '{}': {}", PCI_ROOT, error);
            return Ok(Vec::new());
        }
    };
    let mut devices = Vec::new();

    for vendor_key in vendor_keys {
        let instance_keys = match list_subkeys(&vendor_key) {
            Ok(keys) => keys,
            Err(error) => {
                warn!("Skipping unreadable PCI vendor key '{}': {}", vendor_key, error);
                continue;
            }
        };
        for instance_key in instance_keys {
            let device_id = pci_device_id_from_key(&instance_key);
            let mut readable = true;

            let friendly = mark_unreadable_on_error(
                query_string_value(&instance_key, "FriendlyName"),
                &mut readable,
                "FriendlyName",
                &device_id,
            );
            let desc = mark_unreadable_on_error(
                query_string_value(&instance_key, "DeviceDesc"),
                &mut readable,
                "DeviceDesc",
                &device_id,
            )
            .map(|raw| sanitize_device_desc(&raw));
            let display_name = friendly.or(desc).unwrap_or_else(|| device_id.clone());

            let msi_key = format!(
                r"{}\Device Parameters\Interrupt Management\MessageSignaledInterruptProperties",
                instance_key
            );
            let affinity_key = format!(
                r"{}\Device Parameters\Interrupt Management\Affinity Policy",
                instance_key
            );

            let msi_value = mark_unreadable_on_error(
                query_dword_value(&msi_key, "MSISupported"),
                &mut readable,
                "MSISupported",
                &device_id,
            );
            let priority_value = mark_unreadable_on_error(
                query_dword_value(&affinity_key, "DevicePriority"),
                &mut readable,
                "DevicePriority",
                &device_id,
            );

            devices.push(PciDeviceDto {
                device_id,
                display_name,
                readable,
                msi_supported: msi_value.is_some(),
                msi_enabled: msi_value == Some(1),
                priority: MsiPriorityDto::from_registry(priority_value.unwrap_or(0)),
            });
        }
    }

    devices.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));
    Ok(devices)
}

#[cfg(test)]
mod tests {
    use crate::msi_util::pci_registry_read::sanitize_device_desc;

    #[test]
    fn sanitize_device_desc_extracts_tail_segment() {
        let raw = "@machine.inf,%pci\\cc_0600_desc%;PCI standard host CPU bridge";
        assert_eq!(sanitize_device_desc(raw), "PCI standard host CPU bridge");
    }

    #[test]
    fn sanitize_device_desc_trims_whitespace_without_semicolon() {
        assert_eq!(sanitize_device_desc("  Realtek PCIe GBE Family Controller  "), "Realtek PCIe GBE Family Controller");
    }

    #[test]
    fn sanitize_device_desc_expands_localized_template_arguments() {
        let raw = "@System32\\drivers\\usbxhci.sys,#1073807361;%1 USB %2 eXtensible Host Controller - %3 (Microsoft);(Intel(R),3.0,1.0)";
        assert_eq!(
            sanitize_device_desc(raw),
            "Intel(R) USB 3.0 eXtensible Host Controller - 1.0 (Microsoft)"
        );
    }
}
