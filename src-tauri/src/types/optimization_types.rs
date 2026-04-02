use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TelemetryDesiredState {
    pub(crate) services: bool,
    pub(crate) registry_policies: bool,
    pub(crate) scheduled_tasks: bool,
    pub(crate) hosts_block: bool,
}

impl TelemetryDesiredState {
    pub(crate) fn any_enabled(&self) -> bool {
        self.services || self.registry_policies || self.scheduled_tasks || self.hosts_block
    }

}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NetSniperDesiredState {
    pub(crate) tcp_tweaks: bool,
    pub(crate) registry_throttling: bool,
    pub(crate) cloudflare_dns: bool,
}

impl NetSniperDesiredState {
    pub(crate) fn any_enabled(&self) -> bool {
        self.tcp_tweaks || self.registry_throttling || self.cloudflare_dns
    }

}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PowerDesiredState {
    pub(crate) ultimate_plan: bool,
    pub(crate) core_parking: bool,
}

impl PowerDesiredState {
    pub(crate) fn any_enabled(&self) -> bool {
        self.ultimate_plan || self.core_parking
    }

}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdvancedDesiredState {
    pub(crate) hpet_dynamic_tick: bool,
    pub(crate) interrupt_moderation: bool,
    pub(crate) mmcss: bool,
}

impl AdvancedDesiredState {
    pub(crate) fn any_enabled(&self) -> bool {
        self.hpet_dynamic_tick || self.interrupt_moderation || self.mmcss
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OptimizationDesiredState {
    pub(crate) telemetry: TelemetryDesiredState,
    pub(crate) net_sniper: NetSniperDesiredState,
    pub(crate) power_mode: PowerDesiredState,
    pub(crate) advanced: AdvancedDesiredState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TelemetryStatusDto {
    pub(crate) verified: bool,
    pub(crate) services_disabled: bool,
    pub(crate) registry_policies_disabled: bool,
    pub(crate) scheduled_tasks_disabled: bool,
    pub(crate) hosts_blocked: bool,
    pub(crate) services_readable: bool,
    pub(crate) registry_policies_readable: bool,
    pub(crate) scheduled_tasks_readable: bool,
    pub(crate) hosts_readable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NetSniperStatusDto {
    pub(crate) verified: bool,
    pub(crate) tcp_tweaks_applied: bool,
    pub(crate) registry_throttling_applied: bool,
    pub(crate) cloudflare_dns_applied: bool,
    pub(crate) tcp_tweaks_readable: bool,
    pub(crate) registry_throttling_readable: bool,
    pub(crate) cloudflare_dns_readable: bool,
    pub(crate) interfaces_total: usize,
    pub(crate) interfaces_tuned: usize,
    pub(crate) dns_interfaces_total: usize,
    pub(crate) dns_interfaces_tuned: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PowerStatusDto {
    pub(crate) verified: bool,
    pub(crate) ultimate_plan_active: bool,
    pub(crate) core_parking_disabled: bool,
    pub(crate) ultimate_plan_readable: bool,
    pub(crate) core_parking_readable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdvancedStatusDto {
    pub(crate) verified: bool,
    pub(crate) hpet_dynamic_tick_applied: bool,
    pub(crate) interrupt_moderation_applied: bool,
    pub(crate) mmcss_applied: bool,
    pub(crate) hpet_dynamic_tick_readable: bool,
    pub(crate) interrupt_moderation_readable: bool,
    pub(crate) mmcss_readable: bool,
    pub(crate) interrupt_moderation_adapters_total: usize,
    pub(crate) interrupt_moderation_adapters_tuned: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OptimizationStatusDto {
    pub(crate) telemetry: TelemetryStatusDto,
    pub(crate) net_sniper: NetSniperStatusDto,
    pub(crate) power_mode: PowerStatusDto,
    pub(crate) advanced: AdvancedStatusDto,
}
