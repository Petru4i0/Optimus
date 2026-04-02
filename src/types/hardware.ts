export type MsiPriority = "undefined" | "low" | "normal" | "high";

export type PciDevice = {
  deviceId: string;
  displayName: string;
  readable: boolean;
  msiSupported: boolean;
  msiEnabled: boolean;
  priority: MsiPriority;
};

export type Driver = {
  publishedName: string;
  originalName: string;
  providerName: string;
  className: string;
  driverVersion: string;
  driverDate: string;
  safetyLevel?: string;
};

export type GhostDevice = {
  instanceId: string;
  deviceDescription: string;
  className: string;
  safetyLevel?: string;
};

export type MsiApplyDto = {
  deviceId: string;
  enable: boolean;
  priority: MsiPriority;
};

export type MsiBatchReportDto = {
  total: number;
  successful: number;
  failed: number;
  errors: string[];
};
