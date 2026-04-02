import { createContext, ReactNode, useContext } from "react";
import { PriorityClass, ProcessGroupDto } from "../types/process";

type ProcessActionContextValue = {
  onGroupPriorityChange: (appName: string, value: PriorityClass) => void;
  onApplyGroup: (group: ProcessGroupDto) => Promise<void>;
  onEndGroup: (group: ProcessGroupDto) => Promise<void>;
  onProcessPriorityChange: (pid: number, value: PriorityClass) => void;
  onApplyProcess: (appName: string, pid: number) => Promise<void>;
  onKillProcess: (appName: string, pid: number) => Promise<void>;
};

const ProcessActionContext = createContext<ProcessActionContextValue | null>(null);

export function ProcessActionProvider({
  value,
  children,
}: {
  value: ProcessActionContextValue;
  children: ReactNode;
}) {
  return <ProcessActionContext.Provider value={value}>{children}</ProcessActionContext.Provider>;
}

export function useProcessActions() {
  const context = useContext(ProcessActionContext);
  if (!context) {
    throw new Error("useProcessActions must be used inside ProcessActionProvider");
  }
  return context;
}

export type { ProcessActionContextValue };

