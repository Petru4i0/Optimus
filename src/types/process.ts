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

export type ProcessRowDto = {
  pid: number;
  appName: string;
  iconKey: string;
  memoryBytes: number;
  priority: PriorityClass | null;
  priorityRaw: number | null;
  priorityLabel: string;
};

export type ProcessDeltaPayload = {
  sequence: number;
  added: ProcessRowDto[];
  updated: ProcessRowDto[];
  removed: number[];
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
