use crate::core::{error, warn};
use crate::net_tuning::net_registry::list_active_interface_targets;
use crate::optimization::backup_manager::{ensure_dns_snapshot, load_internet_snapshot, DnsInterfaceSnapshot};
use crate::types::AppError;
use crate::utils::registry_cli::{netsh_command, output_text as command_output_to_text};
use std::net::IpAddr;
use std::str::FromStr;
use tauri::AppHandle;

const CLOUDFLARE_PRIMARY_DNS_V4: &str = "1.1.1.1";
const CLOUDFLARE_SECONDARY_DNS_V4: &str = "1.0.0.1";
const CLOUDFLARE_PRIMARY_DNS_V6: &str = "2606:4700:4700::1111";
const CLOUDFLARE_SECONDARY_DNS_V6: &str = "2606:4700:4700::1001";

#[derive(Debug, Clone, Copy)]
pub(crate) struct DnsStatus {
    pub(crate) dns_interfaces_total: usize,
    pub(crate) dns_interfaces_tuned: usize,
    pub(crate) cloudflare_dns_applied: bool,
    pub(crate) cloudflare_dns_readable: bool,
}

fn normalize_ip_token(token: &str) -> Option<String> {
    let cleaned = token
        .trim()
        .trim_matches(|ch: char| matches!(ch, '[' | ']' | '(' | ')' | ',' | ';'));
    IpAddr::from_str(cleaned).ok().map(|ip| ip.to_string())
}

fn query_interface_dns_servers(family: &str, interface_name: &str) -> Result<Vec<String>, AppError> {
    let output = netsh_command(&[
        "interface",
        family,
        "show",
        "dnsservers",
        &format!("name={interface_name}"),
    ])?;
    if !output.status.success() {
        return Err(AppError::Message(format!(
            "failed to query DNS on '{}': {}",
            interface_name,
            command_output_to_text(&output)
        )));
    }

    let mut servers = Vec::new();
    for token in command_output_to_text(&output).split_whitespace() {
        if let Some(ip) = normalize_ip_token(token) {
            if !servers.iter().any(|existing| existing == &ip) {
                servers.push(ip);
            }
        }
    }
    Ok(servers)
}

fn set_dns_servers(family: &str, interface_name: &str, servers: &[&str]) -> Result<(), AppError> {
    if servers.is_empty() {
        return reset_dns_servers(family, interface_name);
    }

    let set_primary = match netsh_command(&[
        "interface",
        family,
        "set",
        "dnsservers",
        &format!("name={interface_name}"),
        "static",
        servers[0],
        "primary",
    ]) {
        Ok(output) => output,
        Err(err) => {
            error!(
                "AV blocked DNS modification: failed to launch netsh for {} on '{}': {}",
                family, interface_name, err
            );
            return Ok(());
        }
    };
    if !set_primary.status.success() {
        error!(
            "failed to set primary {} DNS on '{}': {}",
            family,
            interface_name,
            command_output_to_text(&set_primary)
        );
        return Ok(());
    }

    for (index, server) in servers.iter().enumerate().skip(1) {
        let add_secondary = match netsh_command(&[
            "interface",
            family,
            "add",
            "dnsservers",
            &format!("name={interface_name}"),
            &format!("address={server}"),
            &format!("index={}", index + 1),
        ]) {
            Ok(output) => output,
            Err(err) => {
                error!(
                    "AV blocked DNS modification: failed to launch netsh for {} secondary DNS on '{}': {}",
                    family, interface_name, err
                );
                return Ok(());
            }
        };
        if !add_secondary.status.success() {
            error!(
                "failed to add secondary {} DNS on '{}': {}",
                family,
                interface_name,
                command_output_to_text(&add_secondary)
            );
            return Ok(());
        }
    }

    Ok(())
}

fn reset_dns_servers(family: &str, interface_name: &str) -> Result<(), AppError> {
    let output = match netsh_command(&[
        "interface",
        family,
        "set",
        "dnsservers",
        &format!("name={interface_name}"),
        "source=dhcp",
    ]) {
        Ok(output) => output,
        Err(err) => {
            error!(
                "AV blocked DNS rollback: failed to launch netsh for {} on '{}': {}",
                family, interface_name, err
            );
            return Ok(());
        }
    };
    if output.status.success() {
        return Ok(());
    }

    error!(
        "failed to rollback {} DNS on '{}': {}",
        family,
        interface_name,
        command_output_to_text(&output)
    );
    Ok(())
}

fn interface_has_cloudflare_dns(interface_name: &str) -> Result<bool, AppError> {
    let ipv4 = query_interface_dns_servers("ipv4", interface_name)?;
    let ipv6 = query_interface_dns_servers("ipv6", interface_name)?;
    let ipv4_ok = ipv4.contains(&CLOUDFLARE_PRIMARY_DNS_V4.to_owned())
        && ipv4.contains(&CLOUDFLARE_SECONDARY_DNS_V4.to_owned());
    let ipv6_ok = ipv6.contains(&CLOUDFLARE_PRIMARY_DNS_V6.to_owned())
        && ipv6.contains(&CLOUDFLARE_SECONDARY_DNS_V6.to_owned());
    Ok(ipv4_ok && ipv6_ok)
}

pub(crate) fn capture_active_dns_snapshots(app: &AppHandle) -> Result<(), AppError> {
    for target in list_active_interface_targets()? {
        let ipv4_servers = match query_interface_dns_servers("ipv4", &target.interface_name) {
            Ok(servers) => servers,
            Err(err) => {
                warn!(
                    "failed to capture IPv4 DNS snapshot for '{}': {}",
                    target.interface_name, err
                );
                continue;
            }
        };
        let ipv6_servers = match query_interface_dns_servers("ipv6", &target.interface_name) {
            Ok(servers) => servers,
            Err(err) => {
                warn!(
                    "failed to capture IPv6 DNS snapshot for '{}': {}",
                    target.interface_name, err
                );
                continue;
            }
        };
        ensure_dns_snapshot(
            app,
            DnsInterfaceSnapshot {
                guid: target.guid,
                interface_name: target.interface_name,
                ipv4_servers,
                ipv6_servers,
            },
        )?;
    }
    Ok(())
}

pub(crate) fn restore_cloudflare_dns_from_snapshot(app: &AppHandle) -> Result<(), AppError> {
    let Some(snapshot) = load_internet_snapshot(app)? else {
        warn!("internet snapshot missing; restoring DNS settings to DHCP defaults");
        for target in list_active_interface_targets()? {
            reset_dns_servers("ipv4", &target.interface_name)?;
            reset_dns_servers("ipv6", &target.interface_name)?;
        }
        return Ok(());
    };

    for dns in snapshot.dns_interfaces {
        if dns.ipv4_servers.is_empty() {
            reset_dns_servers("ipv4", &dns.interface_name)?;
        } else {
            let refs: Vec<&str> = dns.ipv4_servers.iter().map(String::as_str).collect();
            set_dns_servers("ipv4", &dns.interface_name, &refs)?;
        }

        if dns.ipv6_servers.is_empty() {
            reset_dns_servers("ipv6", &dns.interface_name)?;
        } else {
            let refs: Vec<&str> = dns.ipv6_servers.iter().map(String::as_str).collect();
            set_dns_servers("ipv6", &dns.interface_name, &refs)?;
        }
    }
    Ok(())
}

pub(crate) fn set_cloudflare_dns() -> Result<(), AppError> {
    let targets = list_active_interface_targets()?;
    for target in &targets {
        if let Err(err) = set_dns_servers(
            "ipv4",
            &target.interface_name,
            &[CLOUDFLARE_PRIMARY_DNS_V4, CLOUDFLARE_SECONDARY_DNS_V4],
        ) {
            error!(
                "failed to apply IPv4 Cloudflare DNS on '{}': {}",
                target.interface_name, err
            );
        }
        if let Err(err) = set_dns_servers(
            "ipv6",
            &target.interface_name,
            &[CLOUDFLARE_PRIMARY_DNS_V6, CLOUDFLARE_SECONDARY_DNS_V6],
        ) {
            error!(
                "failed to apply IPv6 Cloudflare DNS on '{}': {}",
                target.interface_name, err
            );
        }
    }

    Ok(())
}

pub(crate) fn check_dns_status() -> DnsStatus {
    let active_interfaces = match list_active_interface_targets() {
        Ok(value) => value,
        Err(err) => {
            warn!("failed to list active interfaces for DNS status: {}", err);
            return DnsStatus {
                dns_interfaces_total: 0,
                dns_interfaces_tuned: 0,
                cloudflare_dns_applied: false,
                cloudflare_dns_readable: false,
            };
        }
    };
    let mut dns_tuned_count = 0usize;
    let mut readable_count = 0usize;

    for interface in &active_interfaces {
        match interface_has_cloudflare_dns(&interface.interface_name) {
            Ok(has_cloudflare_dns) => {
                readable_count += 1;
                if has_cloudflare_dns {
                    dns_tuned_count += 1;
                }
            }
            Err(err) => {
                warn!(
                    "failed to query DNS status for interface '{}': {}",
                    interface.interface_name, err
                );
            }
        }
    }
    let cloudflare_dns_readable =
        active_interfaces.is_empty() || readable_count == active_interfaces.len();

    DnsStatus {
        dns_interfaces_total: active_interfaces.len(),
        dns_interfaces_tuned: dns_tuned_count,
        cloudflare_dns_applied: cloudflare_dns_readable
            && dns_tuned_count == active_interfaces.len(),
        cloudflare_dns_readable,
    }
}
