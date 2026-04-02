import { useQuery } from "@tanstack/react-query";

type UseConfigQueriesOptions = {
  enabled: boolean;
  loadConfigs: () => Promise<void>;
  loadWatchdog: () => Promise<void>;
};

export function useConfigQueries({ enabled, loadConfigs, loadWatchdog }: UseConfigQueriesOptions) {
  useQuery({
    queryKey: ["config", "configs"],
    enabled,
    queryFn: async () => {
      await loadConfigs();
      return true;
    },
    staleTime: Infinity,
    gcTime: Infinity,
  });

  useQuery({
    queryKey: ["config", "watchdog"],
    enabled,
    queryFn: async () => {
      await loadWatchdog();
      return true;
    },
    staleTime: Infinity,
    gcTime: Infinity,
  });
}
