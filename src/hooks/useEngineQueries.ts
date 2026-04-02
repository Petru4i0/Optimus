import { useQuery } from "@tanstack/react-query";
import { AppTab } from "../store/appStore";

type UseEngineQueriesOptions = {
  enabled: boolean;
  activeTab: AppTab;
  loadAppSettings: (silent?: boolean) => Promise<void>;
  loadRuntimeSettings: () => Promise<void>;
  loadTurboTimerStatus: (silent?: boolean) => Promise<void>;
  refreshMemoryStats: (silent?: boolean) => Promise<unknown>;
};

export function useEngineQueries({
  enabled,
  activeTab,
  loadAppSettings,
  loadRuntimeSettings,
  loadTurboTimerStatus,
  refreshMemoryStats,
}: UseEngineQueriesOptions) {
  useQuery({
    queryKey: ["engine", "app-settings"],
    enabled,
    queryFn: async () => {
      await loadAppSettings(false);
      return true;
    },
    staleTime: Infinity,
    gcTime: Infinity,
  });

  useQuery({
    queryKey: ["engine", "runtime-settings"],
    enabled,
    queryFn: async () => {
      await loadRuntimeSettings();
      return true;
    },
    staleTime: 30_000,
  });

  useQuery({
    queryKey: ["engine", "timer-status"],
    enabled: enabled && activeTab === "engine",
    queryFn: async () => {
      await loadTurboTimerStatus(true);
      return true;
    },
    refetchInterval: enabled && activeTab === "engine" ? 1000 : false,
    refetchIntervalInBackground: false,
  });

  useQuery({
    queryKey: ["engine", "memory-stats"],
    enabled: enabled && activeTab === "engine",
    queryFn: async () => {
      await refreshMemoryStats(true);
      return true;
    },
    refetchInterval: enabled && activeTab === "engine" ? 2000 : false,
    refetchIntervalInBackground: false,
  });
}
