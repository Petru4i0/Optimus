export type PriorityClass =
  | "realtime"
  | "high"
  | "aboveNormal"
  | "normal"
  | "belowNormal"
  | "low";

export type PriorityOption = {
  label: string;
  value: PriorityClass;
};

export type ProcessDto = {
  pid: number;
  memoryBytes: number;
  priority: PriorityClass | null;
  priorityRaw: number | null;
  priorityLabel: string;
};

export type ProcessGroupDto = {
  appName: string;
  iconKey: string;
  iconBase64: string | null;
  total: number;
  processes: ProcessDto[];
};

export type ProcessListResponse = {
  groups: ProcessGroupDto[];
  needsElevation: boolean;
  isElevated: boolean;
};

export type ApplyResultDto = {
  pid: number;
  success: boolean;
  message: string;
};

export type ProcessPrioritySnapshot = {
  pid: number;
  priority: PriorityClass | null;
  priorityRaw: number | null;
  priorityLabel: string;
};

export type Config = {
  name: string;
  configMap: Record<string, PriorityClass>;
  updatedAt: number;
};

export type WatchdogConfig = {
  triggerMap: Record<string, string>;
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
  autostartMode: "off" | "user" | "elevated";
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
};
