import { invoke } from "@tauri-apps/api/core";
import { useCallback, useMemo, useState } from "react";
import {
  ApplyResultDto,
  Config,
  PriorityClass,
  ProcessGroupDto,
  ToastKind,
  WatchdogConfig,
} from "../types/process";

type PushToast = (kind: ToastKind, message: string) => void;

type RefreshProcesses = (silent?: boolean) => Promise<void>;

function equalsIgnoreCase(a: string, b: string) {
  return a.toLowerCase() === b.toLowerCase();
}

function hasConfigName(configs: Config[], name: string) {
  return configs.some((config) => equalsIgnoreCase(config.name, name));
}

export function useConfigManager(
  pushToast: PushToast,
  groups: ProcessGroupDto[],
  refreshProcesses: RefreshProcesses,
) {
  const [configs, setConfigs] = useState<Config[]>([]);
  const [homeSavedOpen, setHomeSavedOpen] = useState(false);
  const [settingsSavedOpen, setSettingsSavedOpen] = useState(true);

  const [builderName, setBuilderName] = useState("");
  const [builderTargetApp, setBuilderTargetApp] = useState("");
  const [builderTargetPriority, setBuilderTargetPriority] = useState<PriorityClass>("normal");
  const [builderTargets, setBuilderTargets] = useState<Record<string, PriorityClass>>({});

  const [watchdogConfig, setWatchdogConfig] = useState<WatchdogConfig>({
    triggerMap: {},
    stickyModes: {},
  });
  const [watchTriggerApp, setWatchTriggerApp] = useState("");
  const [watchConfigName, setWatchConfigName] = useState("");

  const loadConfigs = useCallback(async () => {
    const loaded = await invoke<Config[]>("load_configs");
    setConfigs(loaded);
  }, []);

  const loadWatchdog = useCallback(async () => {
    const loaded = await invoke<WatchdogConfig>("load_watchdog_config");
    setWatchdogConfig(loaded);
  }, []);

  const onAddBuilderTarget = useCallback(() => {
    const appName = builderTargetApp.trim();
    if (!appName) {
      pushToast("error", "Select target app");
      return;
    }

    setBuilderTargets((prev) => ({ ...prev, [appName]: builderTargetPriority }));
    pushToast("success", `${appName} mapped to ${builderTargetPriority}`);
  }, [builderTargetApp, builderTargetPriority, pushToast]);

  const onRemoveBuilderTarget = useCallback((appName: string) => {
    setBuilderTargets((prev) => {
      const { [appName]: _removed, ...rest } = prev;
      return rest;
    });
  }, []);

  const onSaveConfig = useCallback(async () => {
    const name = builderName.trim();
    if (!name) {
      pushToast("error", "Enter config name");
      return;
    }

    if (Object.keys(builderTargets).length === 0) {
      pushToast("error", "Add at least one target");
      return;
    }

    try {
      await invoke("save_config", { name, configMap: builderTargets });
      pushToast("success", `Config '${name}' saved`);
      setBuilderName("");
      setBuilderTargets({});
      await loadConfigs();
    } catch (invokeError) {
      const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
      pushToast("error", message);
    }
  }, [builderName, builderTargets, loadConfigs, pushToast]);

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
          invoke<ApplyResultDto>("set_process_priority", {
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
    [groups, pushToast, refreshProcesses],
  );

  const onDeleteConfig = useCallback(
    async (name: string) => {
      try {
        await invoke("delete_config", { name });
        pushToast("success", `Config '${name}' deleted`);
        await loadConfigs();
        await loadWatchdog();
      } catch (invokeError) {
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      }
    },
    [loadConfigs, loadWatchdog, pushToast],
  );

  const onExportConfig = useCallback(
    async (name: string) => {
      try {
        await invoke("export_config", { name });
        pushToast("success", `Config '${name}' exported`);
      } catch (invokeError) {
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      }
    },
    [pushToast],
  );

  const onImportConfig = useCallback(async () => {
    try {
      const imported = await invoke<string>("import_config");
      pushToast("success", `Imported config '${imported}'`);
      await loadConfigs();
    } catch (invokeError) {
      const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
      pushToast("error", message);
    }
  }, [loadConfigs, pushToast]);

  const onCreateShortcut = useCallback(
    async (configName: string) => {
      try {
        await invoke("create_desktop_shortcut", { configName });
        pushToast("success", `Shortcut created for '${configName}'`);
      } catch (invokeError) {
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      }
    },
    [pushToast],
  );

  const onSetSticky = useCallback(
    async (configName: string, mode: 0 | 1 | 2) => {
      try {
        await invoke("set_config_sticky", { configName, mode });
        await loadWatchdog();
        const modeLabel = mode === 0 ? "off" : mode === 1 ? "always" : "smart";
        pushToast("success", `${configName}: live ${modeLabel}`);
      } catch (invokeError) {
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      }
    },
    [loadWatchdog, pushToast],
  );

  const onAddWatchMapping = useCallback(async () => {
    const appName = watchTriggerApp.trim();
    const configName = watchConfigName.trim();

    if (!appName) {
      pushToast("error", "Select trigger application");
      return;
    }

    if (!configName) {
      pushToast("error", "Select a config");
      return;
    }

    if (!hasConfigName(configs, configName)) {
      pushToast("error", "Selected config no longer exists");
      return;
    }

    try {
      await invoke("upsert_watchdog_mapping", { appName, configName });
      setWatchTriggerApp("");
      await loadWatchdog();
      pushToast("success", `Mapping saved: ${appName} -> ${configName}`);
    } catch (invokeError) {
      const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
      pushToast("error", message);
    }
  }, [configs, loadWatchdog, pushToast, watchConfigName, watchTriggerApp]);

  const onRemoveWatchMapping = useCallback(
    async (appName: string) => {
      try {
        await invoke("remove_watchdog_mapping", { appName });
        await loadWatchdog();
        pushToast("success", `Mapping removed: ${appName}`);
      } catch (invokeError) {
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      }
    },
    [loadWatchdog, pushToast],
  );

  const sortedMappings = useMemo(
    () =>
      Object.entries(watchdogConfig.triggerMap).sort((a, b) =>
        a[0].toLowerCase().localeCompare(b[0].toLowerCase()),
      ),
    [watchdogConfig.triggerMap],
  );

  const appIconLookup = useMemo(() => {
    const map = new Map<string, string | null>();
    for (const group of groups) {
      map.set(group.appName.toLowerCase(), group.iconBase64);
    }
    return map;
  }, [groups]);

  return {
    configs,
    homeSavedOpen,
    settingsSavedOpen,
    setHomeSavedOpen,
    setSettingsSavedOpen,
    builderName,
    setBuilderName,
    builderTargetApp,
    setBuilderTargetApp,
    builderTargetPriority,
    setBuilderTargetPriority,
    builderTargets,
    watchTriggerApp,
    setWatchTriggerApp,
    watchConfigName,
    setWatchConfigName,
    watchdogConfig,
    loadConfigs,
    loadWatchdog,
    onAddBuilderTarget,
    onRemoveBuilderTarget,
    onSaveConfig,
    onApplyConfig,
    onDeleteConfig,
    onExportConfig,
    onImportConfig,
    onCreateShortcut,
    onSetSticky,
    onAddWatchMapping,
    onRemoveWatchMapping,
    sortedMappings,
    appIconLookup,
    equalsIgnoreCase,
  };
}
