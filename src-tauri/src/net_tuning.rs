use crate::core::warn;
use crate::types::{AppError, NetSniperStatusDto};

mod dns_tuning;
mod net_registry;

pub(crate) use dns_tuning::{
    capture_active_dns_snapshots, restore_cloudflare_dns_from_snapshot, set_cloudflare_dns,
};
pub(crate) use net_registry::{
    apply_advanced_net_tweaks, apply_interrupt_moderation, apply_tcp_tweaks,
    capture_active_tcp_registry_snapshots, capture_interrupt_moderation_snapshots,
    capture_registry_throttling_snapshot, check_interrupt_moderation_status,
    InterruptModerationStatus,
    restore_interrupt_moderation_from_snapshot, restore_registry_throttling_from_snapshot,
    restore_tcp_tweaks_from_snapshot,
};

fn is_not_found_error(err: &AppError) -> bool {
    let lower = err.to_string().to_lowercase();
    lower.contains("os error 2")
        || lower.contains("not found")
        || lower.contains("cannot find")
        || lower.contains("unable to find")
        || lower.contains("does not exist")
}

pub(crate) fn check_net_sniper_status() -> Result<NetSniperStatusDto, AppError> {
    let registry_status = match net_registry::check_net_registry_status() {
        Ok(value) => value,
        Err(err) => {
            warn!("failed to read net registry status: {}", err);
            return Ok(NetSniperStatusDto {
                verified: false,
                tcp_tweaks_applied: false,
                registry_throttling_applied: false,
                cloudflare_dns_applied: false,
                tcp_tweaks_readable: is_not_found_error(&err),
                registry_throttling_readable: is_not_found_error(&err),
                cloudflare_dns_readable: false,
                interfaces_total: 0,
                interfaces_tuned: 0,
                dns_interfaces_total: 0,
                dns_interfaces_tuned: 0,
            });
        }
    };

    let dns_status = dns_tuning::check_dns_status();
    let tcp_tweaks_applied = registry_status.tcp_tweaks_readable
        && registry_status.interfaces_total > 0
        && registry_status.interfaces_readable == registry_status.interfaces_total
        && registry_status.interfaces_tuned == registry_status.interfaces_total;
    let verified = tcp_tweaks_applied
        && registry_status.registry_throttling_applied
        && dns_status.cloudflare_dns_applied
        && registry_status.tcp_tweaks_readable
        && registry_status.registry_throttling_readable
        && dns_status.cloudflare_dns_readable;

    Ok(NetSniperStatusDto {
        verified,
        tcp_tweaks_applied,
        registry_throttling_applied: registry_status.registry_throttling_applied,
        cloudflare_dns_applied: dns_status.cloudflare_dns_applied,
        tcp_tweaks_readable: registry_status.tcp_tweaks_readable,
        registry_throttling_readable: registry_status.registry_throttling_readable,
        cloudflare_dns_readable: dns_status.cloudflare_dns_readable,
        interfaces_total: registry_status.interfaces_total,
        interfaces_tuned: registry_status.interfaces_tuned,
        dns_interfaces_total: dns_status.dns_interfaces_total,
        dns_interfaces_tuned: dns_status.dns_interfaces_tuned,
    })
}
