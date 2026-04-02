import { useQuery } from "@tanstack/react-query";
import { AppTab } from "../store/appStore";

type UseHardwareQueriesOptions = {
  enabled: boolean;
  activeTab: AppTab;
  loadPciDevices: (silent?: boolean) => Promise<void>;
};

export function useHardwareQueries({ enabled, activeTab, loadPciDevices }: UseHardwareQueriesOptions) {
  useQuery({
    queryKey: ["hardware", "pci-devices"],
    queryFn: async () => {
      await loadPciDevices(true);
      return true;
    },
    enabled: enabled && activeTab === "engine",
    staleTime: 15_000,
    refetchInterval: false,
    refetchOnWindowFocus: false,
  });
}
