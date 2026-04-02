import { invoke } from "@tauri-apps/api/core";
import { useCallback, useState } from "react";
import { Config, ToastKind, WatchdogConfig } from "../types/config";
import { parseIpcError } from "../types/ipc";
import { ApplyResultDto, PriorityClass } from "../types/process";

type PushToast = (kind: ToastKind, message: string) => void;
type RefreshProcesses = (silent?: boolean) => Promise<void>;

export function useConfigCrud(
  pushToast: PushToast,
  groups: { appName: string; processes: { pid: number }[] }[],
  refreshProcesses: RefreshProcesses,
  equalsIgnoreCase: (a: string, b: string) => boolean,
) {
  const [configs, setConfigs] = useState<Config[]>([]);
  const [watchdogConfig, setWatchdogConfig] = useState<WatchdogConfig>({
    triggerMap: {},
    stickyModes: {},
  });

  const loadConfigs = useCallback(async () => {
    const loaded = await invoke<Config[]>("config_load_configs");
    setConfigs(loaded);
  }, []);

  const loadWatchdog = useCallback(async () => {
    const loaded = await invoke<WatchdogConfig>("config_watchdog_load");
    setWatchdogConfig(loaded);
  }, []);

  const saveConfig = useCallback(
    async (name: string, configMap: Record<string, PriorityClass>) => {
      await invoke("config_save", { name, configMap });
      await loadConfigs();
    },
    [loadConfigs],
  );

  const onApplyConfig = useCallback(
    async (config: Config) => {
      let applied = 0;
      let failed = 0;
      let skipped = 0;

      const entries = Object.entries(config.configMap);
      for (const [appName, priority] of entries) {
        const group = groups.find((item) => equalsIgnoreCase(item.appName, appName));
        if (!group) {
          skipped += 1;
          continue;
        }

        const tasks = group.processes.map((process) =>
          invoke<ApplyResultDto>("process_set_priority", {
            pid: process.pid,
            priority,
          }),
        );

        const settled = await Promise.allSettled(tasks);
        for (const result of settled) {
          if (result.status === "fulfilled" && result.value.success) {
            applied += 1;
          } else {
            failed += 1;
          }
        }
      }

      if (failed > 0) {
        pushToast("info", `${config.name}: applied ${applied}, skipped ${skipped}, failed ${failed}`);
      } else {
        pushToast("success", `${config.name}: applied ${applied}, skipped ${skipped}`);
      }

      await refreshProcesses(true);
    },
    [equalsIgnoreCase, groups, pushToast, refreshProcesses],
  );

  const onDeleteConfig = useCallback(
    async (name: string) => {
      try {
        await invoke("config_delete", { name });
        pushToast("success", `Config '${name}' deleted`);
        await loadConfigs();
        await loadWatchdog();
      } catch (invokeError) {
        pushToast("error", parseIpcError(invokeError).message);
      }
    },
    [loadConfigs, loadWatchdog, pushToast],
  );

  const onExportConfig = useCallback(
    async (name: string) => {
      try {
        await invoke("config_export", { name });
        pushToast("success", `Config '${name}' exported`);
      } catch (invokeError) {
        pushToast("error", parseIpcError(invokeError).message);
      }
    },
    [pushToast],
  );

  const onImportConfig = useCallback(async () => {
    try {
      const imported = await invoke<string>("config_import");
      pushToast("success", `Imported config '${imported}'`);
      await loadConfigs();
    } catch (invokeError) {
      pushToast("error", parseIpcError(invokeError).message);
    }
  }, [loadConfigs, pushToast]);

  const onCreateShortcut = useCallback(
    async (configName: string) => {
      try {
        await invoke("config_create_desktop_shortcut", { configName });
        pushToast("success", `Shortcut created for '${configName}'`);
      } catch (invokeError) {
        pushToast("error", parseIpcError(invokeError).message);
      }
    },
    [pushToast],
  );

  const onSetSticky = useCallback(
    async (configName: string, mode: 0 | 1 | 2) => {
      try {
        await invoke("config_set_sticky_mode", { configName, mode });
        await loadWatchdog();
        const modeLabel = mode === 0 ? "off" : mode === 1 ? "always" : "smart";
        pushToast("success", `${configName}: live ${modeLabel}`);
      } catch (invokeError) {
        pushToast("error", parseIpcError(invokeError).message);
      }
    },
    [loadWatchdog, pushToast],
  );

  const onAddWatchMapping = useCallback(
    async (appName: string, configName: string, icon?: string | null) => {
      try {
        await invoke("config_watchdog_upsert_mapping", {
          appName,
          configName,
          icon: icon ?? null,
        });
        await loadWatchdog();
        pushToast("success", `Mapping saved: ${appName} -> ${configName}`);
      } catch (invokeError) {
        pushToast("error", parseIpcError(invokeError).message);
      }
    },
    [loadWatchdog, pushToast],
  );

  const onRemoveWatchMapping = useCallback(
    async (appName: string) => {
      try {
        await invoke("config_watchdog_remove_mapping", { appName });
        await loadWatchdog();
        pushToast("success", `Mapping removed: ${appName}`);
      } catch (invokeError) {
        pushToast("error", parseIpcError(invokeError).message);
      }
    },
    [loadWatchdog, pushToast],
  );

  return {
    configs,
    watchdogConfig,
    loadConfigs,
    loadWatchdog,
    saveConfig,
    onApplyConfig,
    onDeleteConfig,
    onExportConfig,
    onImportConfig,
    onCreateShortcut,
    onSetSticky,
    onAddWatchMapping,
    onRemoveWatchMapping,
  };
}
