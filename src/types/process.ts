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

export type ToastKind = "success" | "error" | "info";

export type ToastMessage = {
  id: number;
  kind: ToastKind;
  message: string;
};

export type RuntimeSettings = {
  watchdogEnabled: boolean;
  autostartEnabled: boolean;
  minimizeToTrayEnabled: boolean;
};

export type ElevationStatus = {
  status: string;
  message: string;
};
