export type TimerResolutionStatus = {
  minimumMs: number;
  maximumMs: number;
  currentMs: number;
  requestedMs: number | null;
  enabled: boolean;
};

export type MemoryStats = {
  standbyListMb: number;
  freeMemoryMb: number;
  totalMemoryMb: number;
};

export type MemoryPurgeConfig = {
  masterEnabled: boolean;
  enableStandbyTrigger: boolean;
  standbyLimitMb: number;
  enableFreeMemoryTrigger: boolean;
  freeMemoryLimitMb: number;
  totalPurges: number;
  totalClearedMb: number;
};

export type TelemetryStatus = {
  verified: boolean;
  servicesDisabled: boolean;
  registryPoliciesDisabled: boolean;
  scheduledTasksDisabled: boolean;
  hostsBlocked: boolean;
  servicesReadable: boolean;
  registryPoliciesReadable: boolean;
  scheduledTasksReadable: boolean;
  hostsReadable: boolean;
};

export type NetSniperStatus = {
  verified: boolean;
  tcpTweaksApplied: boolean;
  registryThrottlingApplied: boolean;
  cloudflareDnsApplied: boolean;
  tcpTweaksReadable: boolean;
  registryThrottlingReadable: boolean;
  cloudflareDnsReadable: boolean;
  interfacesTotal: number;
  interfacesTuned: number;
  dnsInterfacesTotal: number;
  dnsInterfacesTuned: number;
};

export type PowerStatus = {
  verified: boolean;
  ultimatePlanActive: boolean;
  coreParkingDisabled: boolean;
  ultimatePlanReadable: boolean;
  coreParkingReadable: boolean;
};

export type AdvancedStatus = {
  verified: boolean;
  hpetDynamicTickApplied: boolean;
  interruptModerationApplied: boolean;
  mmcssApplied: boolean;
  hpetDynamicTickReadable: boolean;
  interruptModerationReadable: boolean;
  mmcssReadable: boolean;
  interruptModerationAdaptersTotal: number;
  interruptModerationAdaptersTuned: number;
};

export type TelemetrySubFeature =
  | "all"
  | "services"
  | "registry_policies"
  | "scheduled_tasks"
  | "hosts_block";

export type NetSniperSubFeature =
  | "all"
  | "tcp_tweaks"
  | "registry_throttling"
  | "cloudflare_dns";

export type PowerSubFeature = "all" | "ultimate_plan" | "core_parking";

export type AdvancedSubFeature =
  | "all"
  | "hpet_dynamic_tick"
  | "interrupt_moderation"
  | "mmcss";
