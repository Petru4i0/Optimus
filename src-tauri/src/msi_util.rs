use crate::types::AppError;
use crate::utils::registry_cli::{
    reg_key_exists as shared_reg_key_exists, reg_list_subkeys as shared_reg_list_subkeys,
    reg_query_dword_value as shared_reg_query_dword_value,
    reg_query_string_value as shared_reg_query_string_value,
    reg_set_dword_value as shared_reg_set_dword_value,
};

mod pci_registry_read;
mod pci_registry_write;

pub(crate) use pci_registry_read::get_pci_devices;
pub(crate) use pci_registry_write::set_msi_mode;

pub(super) const PCI_ROOT: &str = r"HKLM\SYSTEM\CurrentControlSet\Enum\PCI";

pub(super) fn list_subkeys(key: &str) -> Result<Vec<String>, AppError> {
    shared_reg_list_subkeys(key)
}

pub(super) fn query_string_value(key: &str, value_name: &str) -> Result<Option<String>, AppError> {
    shared_reg_query_string_value(key, value_name)
}

pub(super) fn query_dword_value(key: &str, value_name: &str) -> Result<Option<u32>, AppError> {
    shared_reg_query_dword_value(key, value_name)
}

pub(super) fn key_exists(key: &str) -> Result<bool, AppError> {
    shared_reg_key_exists(key)
}

pub(super) fn set_dword_value(key: &str, value_name: &str, value: u32) -> Result<(), AppError> {
    shared_reg_set_dword_value(key, value_name, value)
}
