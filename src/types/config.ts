import { PriorityClass } from "./process";
import { AdvancedStatus, NetSniperStatus, PowerStatus, TelemetryStatus } from "./engine";

export type Config = {
  name: string;
  configMap: Record<string, PriorityClass>;
  updatedAt: number;
};

export type TriggerMapping = {
  configName: string;
  icon?: string;
};

export type WatchdogConfig = {
  triggerMap: Record<string, TriggerMapping>;
  stickyModes: Record<string, number>;
};

export type ToastKind = "success" | "error" | "info" | "warning";

export type ToastMessage = {
  id: number;
  kind: ToastKind;
  message: string;
};

export type RuntimeSettings = {
  watchdogEnabled: boolean;
  autostartEnabled: boolean;
  startAsAdminEnabled: boolean;
  autostartMode: "off" | "elevated";
  minimizeToTrayEnabled: boolean;
};

export type AppSettings = {
  turboTimerEnabled: boolean;
  watchdogEnabled: boolean;
  minimizeToTrayEnabled: boolean;
  memoryPurgeConfig: {
    masterEnabled: boolean;
    enableStandbyTrigger: boolean;
    standbyLimitMb: number;
    enableFreeMemoryTrigger: boolean;
    freeMemoryLimitMb: number;
  };
};

export type ElevationStatus = {
  status: string;
  message: string;
};

export type OptimizationStatus = {
  telemetry: TelemetryStatus;
  netSniper: NetSniperStatus;
  powerMode: PowerStatus;
  advanced: AdvancedStatus;
};
