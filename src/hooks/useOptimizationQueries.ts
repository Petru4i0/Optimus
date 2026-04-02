import { useQuery } from "@tanstack/react-query";
import { AppTab } from "../store/appStore";

type UseOptimizationQueriesOptions = {
  enabled: boolean;
  activeTab: AppTab;
  loadOptimizationStatus: (silent?: boolean) => Promise<void>;
};

export function useOptimizationQueries({
  enabled,
  activeTab,
  loadOptimizationStatus,
}: UseOptimizationQueriesOptions) {
  useQuery({
    queryKey: ["optimization", "status"],
    enabled: enabled && activeTab === "optimization",
    queryFn: async () => {
      await loadOptimizationStatus(true);
      return true;
    },
    refetchInterval: false,
    refetchOnWindowFocus: true,
  });
}
