
import { invoke } from "@tauri-apps/api/core";
import { type Dispatch, type SetStateAction, useCallback, useEffect, useMemo, useRef, useState } from "react";
import AppIcon from "./components/AppIcon";
import AppPickerDropdown, { type AppPickerOption } from "./components/AppPickerDropdown";
import Layout from "./components/Layout";
import ModeSelector from "./components/ModeSelector";
import ProcessGroupCard from "./components/ProcessGroupCard";
import Toasts from "./components/Toasts";
import {
  ApplyResultDto,
  Config,
  ElevationStatus,
  PriorityClass,
  PriorityOption,
  ProcessGroupDto,
  ProcessListResponse,
  ProcessPrioritySnapshot,
  RuntimeSettings,
  ToastKind,
  ToastMessage,
  WatchdogConfig,
} from "./types/process";

const PRIORITY_OPTIONS: PriorityOption[] = [
  { label: "Realtime", value: "realtime" },
  { label: "High", value: "high" },
  { label: "Above Normal", value: "aboveNormal" },
  { label: "Normal", value: "normal" },
  { label: "Below Normal", value: "belowNormal" },
  { label: "Low", value: "low" },
];

function equalsIgnoreCase(a: string, b: string) {
  return a.toLowerCase() === b.toLowerCase();
}

function formatUpdatedAt(timestamp: number) {
  const date = new Date(timestamp * 1000);
  if (Number.isNaN(date.getTime())) {
    return "Unknown";
  }
  return date.toLocaleString();
}

function defaultGroupPriority(group: ProcessGroupDto): PriorityClass {
  return group.processes[0]?.priority ?? "normal";
}

function hasConfigName(configs: Config[], name: string) {
  return configs.some((config) => equalsIgnoreCase(config.name, name));
}

export default function App() {
  const [activeTab, setActiveTab] = useState<"home" | "settings">("home");
  const [searchQuery, setSearchQuery] = useState("");

  const [groups, setGroups] = useState<ProcessGroupDto[]>([]);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [needsElevation, setNeedsElevation] = useState(false);
  const [isElevated, setIsElevated] = useState(false);
  const [lastSync, setLastSync] = useState<number | null>(null);

  const [groupPriority, setGroupPriority] = useState<Record<string, PriorityClass>>({});
  const [pidPriority, setPidPriority] = useState<Record<number, PriorityClass>>({});

  const [applyingGroup, setApplyingGroup] = useState<Record<string, boolean>>({});
  const [endingGroup, setEndingGroup] = useState<Record<string, boolean>>({});
  const [applyingPid, setApplyingPid] = useState<Record<number, boolean>>({});
  const [killingPid, setKillingPid] = useState<Record<number, boolean>>({});

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

  const [watchdogEnabled, setWatchdogEnabled] = useState(true);
  const [autostartEnabled, setAutostartEnabled] = useState(false);
  const [minimizeToTrayEnabled, setMinimizeToTrayEnabled] = useState(true);
  const [elevationPending, setElevationPending] = useState(false);

  const [toasts, setToasts] = useState<ToastMessage[]>([]);
  const toastId = useRef(0);
  const toastTimersRef = useRef<Map<number, number>>(new Map());
  const iconCacheRef = useRef<Map<string, string>>(new Map());
  const knownIconKeysRef = useRef<Set<string>>(new Set());

  const filteredGroups = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    if (!query) {
      return groups;
    }
    return groups.filter((group) => group.appName.toLowerCase().includes(query));
  }, [groups, searchQuery]);

  const totalProcesses = useMemo(
    () => groups.reduce((sum, group) => sum + group.total, 0),
    [groups],
  );

  const appPickerOptions = useMemo<AppPickerOption[]>(
    () =>
      groups
        .map((group) => ({ appName: group.appName, iconBase64: group.iconBase64 }))
        .sort((a, b) => a.appName.localeCompare(b.appName)),
    [groups],
  );

  const pushToast = useCallback((kind: ToastKind, message: string) => {
    toastId.current += 1;
    const id = toastId.current;
    setToasts((prev) => [...prev, { id, kind, message }]);
    const timeoutId = window.setTimeout(() => {
      setToasts((prev) => prev.filter((item) => item.id !== id));
      toastTimersRef.current.delete(id);
    }, 4200);
    toastTimersRef.current.set(id, timeoutId);
  }, []);

  const dismissToast = useCallback((id: number) => {
    const timeoutId = toastTimersRef.current.get(id);
    if (timeoutId !== undefined) {
      window.clearTimeout(timeoutId);
      toastTimersRef.current.delete(id);
    }
    setToasts((prev) => prev.filter((item) => item.id !== id));
  }, []);

  useEffect(() => {
    return () => {
      for (const timeoutId of toastTimersRef.current.values()) {
        window.clearTimeout(timeoutId);
      }
      toastTimersRef.current.clear();
    };
  }, []);

  const loadConfigs = useCallback(async () => {
    const loaded = await invoke<Config[]>("load_configs");
    setConfigs(loaded);
  }, []);

  const loadWatchdog = useCallback(async () => {
    const loaded = await invoke<WatchdogConfig>("load_watchdog_config");
    setWatchdogConfig(loaded);
  }, []);

  const loadRuntimeSettings = useCallback(async () => {
    const runtime = await invoke<RuntimeSettings>("get_runtime_settings");
    setWatchdogEnabled(runtime.watchdogEnabled);
    setAutostartEnabled(runtime.autostartEnabled);
    setMinimizeToTrayEnabled(runtime.minimizeToTrayEnabled);
  }, []);

  const refreshProcesses = useCallback(
    async (silent = false) => {
      if (!silent) {
        setRefreshing(true);
      }

      try {
        const response = await invoke<ProcessListResponse>("get_process_list_delta", {
          knownIconKeys: Array.from(knownIconKeysRef.current),
        });
        const nextGroups = response.groups.map((group) => {
          knownIconKeysRef.current.add(group.iconKey);

          if (group.iconBase64) {
            iconCacheRef.current.set(group.iconKey, group.iconBase64);
            return group;
          }

          const cached = iconCacheRef.current.get(group.iconKey);
          if (!cached) {
            return group;
          }

          return {
            ...group,
            iconBase64: cached,
          };
        });

        setGroups(nextGroups);
        setNeedsElevation(response.needsElevation);
        setIsElevated(response.isElevated);
        setLastSync(Date.now());
        setError(null);

        setGroupPriority((prev) => {
          const next: Record<string, PriorityClass> = {};
          for (const group of nextGroups) {
            next[group.appName] = prev[group.appName] ?? defaultGroupPriority(group);
          }
          return next;
        });

        setPidPriority((prev) => {
          const next: Record<number, PriorityClass> = {};
          for (const group of nextGroups) {
            for (const process of group.processes) {
              next[process.pid] = prev[process.pid] ?? process.priority ?? "normal";
            }
          }
          return next;
        });
      } catch (invokeError) {
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        setError(message);
        if (!silent) {
          pushToast("error", message);
        }
      } finally {
        if (!silent) {
          setRefreshing(false);
        }
      }
    },
    [pushToast],
  );

  const onRefreshRequested = useCallback(() => {
    void refreshProcesses();
  }, [refreshProcesses]);

  useEffect(() => {
    void refreshProcesses();
    void loadConfigs().catch((e) => {
      pushToast("error", e instanceof Error ? e.message : String(e));
    });
    void loadWatchdog().catch((e) => {
      pushToast("error", e instanceof Error ? e.message : String(e));
    });
    void loadRuntimeSettings().catch((e) => {
      pushToast("error", e instanceof Error ? e.message : String(e));
    });
  }, [loadConfigs, loadRuntimeSettings, loadWatchdog, pushToast, refreshProcesses]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      void refreshProcesses(true);
    }, 3000);

    return () => {
      window.clearInterval(timer);
    };
  }, [refreshProcesses]);

  const onRequestElevation = useCallback(async () => {
    setElevationPending(true);
    try {
      const status = await invoke<ElevationStatus>("restart_as_administrator");
      if (status.status === "elevation_pending") {
        pushToast("info", status.message);
      } else {
        setElevationPending(false);
      }
    } catch (invokeError) {
      setElevationPending(false);
      const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
      pushToast("error", message);
    }
  }, [pushToast]);

  const onRequestElevationClick = useCallback(() => {
    void onRequestElevation();
  }, [onRequestElevation]);

  const onGroupPriorityChange = useCallback((appName: string, value: PriorityClass) => {
    setGroupPriority((prev) => ({ ...prev, [appName]: value }));
  }, []);

  const onProcessPriorityChange = useCallback((pid: number, value: PriorityClass) => {
    setPidPriority((prev) => ({ ...prev, [pid]: value }));
  }, []);

  const patchPidPriority = useCallback((snapshot: ProcessPrioritySnapshot) => {
    setGroups((prev) =>
      prev.map((group) => ({
        ...group,
        processes: group.processes.map((process) => {
          if (process.pid !== snapshot.pid) {
            return process;
          }
          return {
            ...process,
            priority: snapshot.priority,
            priorityRaw: snapshot.priorityRaw,
            priorityLabel: snapshot.priorityLabel,
          };
        }),
      })),
    );

    if (snapshot.priority) {
      setPidPriority((prev) => ({ ...prev, [snapshot.pid]: snapshot.priority as PriorityClass }));
    }
  }, []);

  const onApplyProcess = useCallback(
    async (_appName: string, pid: number) => {
      const selected = pidPriority[pid] ?? "normal";
      setApplyingPid((prev) => ({ ...prev, [pid]: true }));

      try {
        const result = await invoke<ApplyResultDto>("set_process_priority", {
          pid,
          priority: selected,
        });

        if (!result.success) {
          pushToast("error", result.message);
          return;
        }

        pushToast("success", `PID ${pid}: ${result.message}`);

        const snapshot = await invoke<ProcessPrioritySnapshot>("get_process_priority", { pid });
        patchPidPriority(snapshot);
      } catch (invokeError) {
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      } finally {
        setApplyingPid((prev) => {
          const { [pid]: _removed, ...rest } = prev;
          return rest;
        });
      }
    },
    [patchPidPriority, pidPriority, pushToast],
  );

  const onApplyGroup = useCallback(
    async (group: ProcessGroupDto) => {
      const selected = groupPriority[group.appName] ?? defaultGroupPriority(group);
      setApplyingGroup((prev) => ({ ...prev, [group.appName]: true }));

      try {
        const results = await invoke<ApplyResultDto[]>("set_group_priority", {
          pids: group.processes.map((process) => process.pid),
          priority: selected,
        });

        const successCount = results.filter((result) => result.success).length;
        const failed = results.length - successCount;

        if (failed > 0) {
          pushToast("info", `Group ${group.appName}: applied ${successCount}, failed ${failed}`);
        } else {
          pushToast("success", `Group ${group.appName}: applied ${successCount}`);
        }

        await refreshProcesses(true);
      } catch (invokeError) {
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      } finally {
        setApplyingGroup((prev) => {
          const { [group.appName]: _removed, ...rest } = prev;
          return rest;
        });
      }
    },
    [groupPriority, pushToast, refreshProcesses],
  );

  const onKillProcess = useCallback(
    async (_appName: string, pid: number) => {
      setKillingPid((prev) => ({ ...prev, [pid]: true }));

      try {
        const result = await invoke<ApplyResultDto>("kill_process", { pid });
        if (!result.success) {
          pushToast("error", result.message);
          return;
        }

        pushToast("success", `PID ${pid}: ${result.message}`);
        await refreshProcesses(true);
      } catch (invokeError) {
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      } finally {
        setKillingPid((prev) => {
          const { [pid]: _removed, ...rest } = prev;
          return rest;
        });
      }
    },
    [pushToast, refreshProcesses],
  );

  const onEndGroup = useCallback(
    async (group: ProcessGroupDto) => {
      setEndingGroup((prev) => ({ ...prev, [group.appName]: true }));

      try {
        const tasks = group.processes.map((process) =>
          invoke<ApplyResultDto>("kill_process", { pid: process.pid }),
        );
        const settled = await Promise.allSettled(tasks);

        let successCount = 0;
        let failedCount = 0;

        for (const item of settled) {
          if (item.status === "fulfilled" && item.value.success) {
            successCount += 1;
          } else {
            failedCount += 1;
          }
        }

        if (failedCount > 0) {
          pushToast("info", `${group.appName}: terminated ${successCount}, failed ${failedCount}`);
        } else {
          pushToast("success", `${group.appName}: terminated ${successCount}`);
        }

        await refreshProcesses(true);
      } catch (invokeError) {
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      } finally {
        setEndingGroup((prev) => {
          const { [group.appName]: _removed, ...rest } = prev;
          return rest;
        });
      }
    },
    [pushToast, refreshProcesses],
  );

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

  const onToggleWatchdog = useCallback(
    async (enabled: boolean) => {
      setWatchdogEnabled(enabled);
      try {
        await invoke("toggle_watchdog", { state: enabled });
      } catch (invokeError) {
        setWatchdogEnabled((prev) => !prev);
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      }
    },
    [pushToast],
  );

  const onToggleAutostart = useCallback(
    async (enabled: boolean) => {
      setAutostartEnabled(enabled);
      try {
        await invoke("toggle_autostart", { enabled });
      } catch (invokeError) {
        setAutostartEnabled((prev) => !prev);
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      }
    },
    [pushToast],
  );

  const onToggleMinimizeToTray = useCallback(
    async (enabled: boolean) => {
      setMinimizeToTrayEnabled(enabled);
      try {
        await invoke("toggle_minimize_to_tray", { enabled });
      } catch (invokeError) {
        setMinimizeToTrayEnabled((prev) => !prev);
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      }
    },
    [pushToast],
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

  const renderToggleSwitch = (
    checked: boolean,
    onToggle: (next: boolean) => void,
    ariaLabel: string,
  ) => (
    <label className="relative inline-flex h-6 w-11 cursor-pointer items-center">
      <input
        type="checkbox"
        className="peer sr-only"
        checked={checked}
        onChange={(event) => onToggle(event.target.checked)}
        aria-label={ariaLabel}
      />
      <span className="absolute inset-0 rounded-full border border-zinc-600 bg-zinc-800/80 transition-colors peer-checked:border-zinc-300 peer-checked:bg-zinc-200/20" />
      <span className="absolute left-[2px] top-[2px] h-5 w-5 rounded-full bg-zinc-300 transition-transform peer-checked:translate-x-5 peer-checked:bg-white" />
    </label>
  );

  const renderSavedConfigsSection = (
    isOpen: boolean,
    setOpen: Dispatch<SetStateAction<boolean>>,
    sectionId: "home" | "settings",
  ) => (
    <section className="glass-card rounded-2xl p-4">
      <div className="flex flex-wrap items-center gap-3">
        <button
          className="inline-flex h-8 w-8 items-center justify-center rounded-md border border-zinc-700 bg-zinc-900/60 text-zinc-200 transition hover:border-zinc-400 hover:text-zinc-100"
          onClick={() => setOpen((prev) => !prev)}
          aria-label={isOpen ? "Collapse saved configs" : "Expand saved configs"}
        >
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.9"
            className={`h-5 w-5 transition-transform duration-200 ${isOpen ? "rotate-180" : ""}`}
          >
            <path d="M6 9l6 6 6-6" />
          </svg>
        </button>

        <div>
          <h2 className="text-lg font-semibold text-zinc-100">Saved Configs</h2>
          <p className="text-xs text-zinc-400">Manage reusable app-priority configurations.</p>
        </div>

        <div className="ml-auto flex flex-wrap items-center gap-2">
          <button className="btn-primary" onClick={() => void onImportConfig()}>
            Import Config
          </button>
        </div>
      </div>

      {isOpen ? (
        <div className="mt-4 space-y-2 border-t border-zinc-700/70 pt-4">
          {configs.length === 0 ? (
            <div className="rounded-xl border border-zinc-700 bg-zinc-900/50 px-3 py-3 text-sm text-zinc-400">
              No saved configs yet.
            </div>
          ) : (
            configs.map((config) => {
              const storedModeRaw =
                watchdogConfig.stickyModes[config.name.toLowerCase()] ??
                Object.entries(watchdogConfig.stickyModes).find(([name]) =>
                  equalsIgnoreCase(name, config.name),
                )?.[1] ??
                0;
              const liveMode: 0 | 1 | 2 =
                storedModeRaw === 2 ? 2 : storedModeRaw === 1 ? 1 : 0;
              const liveEnabled = liveMode !== 0;
              const triggerApps = Object.entries(watchdogConfig.triggerMap)
                .filter(([, mappedConfig]) => equalsIgnoreCase(mappedConfig, config.name))
                .map(([appName]) => groups.find((group) => equalsIgnoreCase(group.appName, appName))?.appName ?? appName);
              const triggerSummary =
                triggerApps.length > 0
                  ? `${triggerApps[0]}${triggerApps.length > 1 ? ` +${triggerApps.length - 1}` : ""}`
                  : null;
              const hasTrigger = triggerApps.length > 0;

              return (
                <div
                  key={`${sectionId}-${config.name}`}
                  className="rounded-xl border border-zinc-700 bg-zinc-900/55 px-3 py-3"
                >
                  <div className="flex flex-wrap items-center gap-2">
                    <div>
                      <p className="text-sm font-semibold text-zinc-100">{config.name}</p>
                      <div className="mt-0.5 flex flex-wrap items-center gap-2">
                        <p className="text-xs text-zinc-400">
                          {Object.keys(config.configMap).length} targets • Updated{" "}
                          {formatUpdatedAt(config.updatedAt)}
                        </p>
                        {triggerSummary ? (
                          <span className="inline-flex items-center rounded-full border border-zinc-600 bg-zinc-900/80 px-2 py-0.5 text-[11px] text-zinc-300">
                            Trigger: {triggerSummary}
                          </span>
                        ) : null}
                      </div>
                    </div>

                    <div className="ml-auto flex flex-wrap items-center gap-2">
                      <label className="inline-flex h-9 items-center gap-2 rounded-lg border border-zinc-600 bg-zinc-900/70 px-3 text-sm text-zinc-200">
                        <span>Live</span>
                        {renderToggleSwitch(
                          liveEnabled,
                          (next) => {
                            void onSetSticky(config.name, next ? 1 : 0);
                          },
                          `Toggle live mode for ${config.name}`,
                        )}
                        {liveEnabled ? (
                          <ModeSelector
                            value={liveMode}
                            hasTrigger={hasTrigger}
                            onChange={(mode) => {
                              void onSetSticky(config.name, mode);
                            }}
                          />
                        ) : null}
                      </label>
                      {liveMode === 2 && !hasTrigger ? (
                        <span className="text-xs text-zinc-500">No trigger set</span>
                      ) : null}

                      <button className="btn-primary" onClick={() => void onApplyConfig(config)}>
                        Apply Config
                      </button>
                      <button
                        className="btn-ghost px-3 py-2 text-sm"
                        onClick={() => void onExportConfig(config.name)}
                      >
                        Export
                      </button>
                      <button
                        className="btn-ghost px-3 py-2 text-sm"
                        onClick={() => void onCreateShortcut(config.name)}
                      >
                        Shortcut
                      </button>
                      <button className="btn-danger" onClick={() => void onDeleteConfig(config.name)}>
                        Delete
                      </button>
                    </div>
                  </div>
                </div>
              );
            })
          )}
        </div>
      ) : null}
    </section>
  );

  return (
    <>
      <Layout
        activeTab={activeTab}
        onTabChange={setActiveTab}
        title="Optimus"
        groupsCount={groups.length}
        totalProcesses={totalProcesses}
        refreshing={refreshing}
        onRefresh={onRefreshRequested}
        isElevated={isElevated}
        needsElevation={needsElevation}
        onRequestElevation={onRequestElevationClick}
        lastSync={lastSync}
        error={error}
      >
        {activeTab === "home" ? (
          <div className="space-y-4">
            {renderSavedConfigsSection(homeSavedOpen, setHomeSavedOpen, "home")}

            <section className="glass-card rounded-2xl p-4">
              <div className="relative">
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.8"
                  className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-zinc-400"
                >
                  <circle cx="11" cy="11" r="7" />
                  <path d="M20 20l-3.5-3.5" />
                </svg>
                <input
                  className="w-full rounded-xl border border-zinc-700 bg-zinc-900/70 py-2 pl-10 pr-3 text-sm text-zinc-100 outline-none transition focus:border-zinc-400"
                  placeholder="Search running apps"
                  value={searchQuery}
                  onChange={(event) => setSearchQuery(event.target.value)}
                />
              </div>
            </section>

            <section className="space-y-3">
              {filteredGroups.length === 0 ? (
                <div className="glass-card rounded-2xl px-4 py-6 text-sm text-zinc-400">No matching processes found.</div>
              ) : (
                filteredGroups.map((group, index) => (
                  <ProcessGroupCard
                    key={group.appName}
                    group={group}
                    index={index}
                    priorities={PRIORITY_OPTIONS}
                    groupPriorityValue={groupPriority[group.appName] ?? defaultGroupPriority(group)}
                    applyingGroup={Boolean(applyingGroup[group.appName])}
                    pidPriority={pidPriority}
                    applyingPid={applyingPid}
                    killingPid={killingPid}
                    onGroupPriorityChange={onGroupPriorityChange}
                    onApplyGroup={onApplyGroup}
                    onEndGroup={onEndGroup}
                    endingGroup={Boolean(endingGroup[group.appName])}
                    onProcessPriorityChange={onProcessPriorityChange}
                    onApplyProcess={onApplyProcess}
                    onKillProcess={onKillProcess}
                  />
                ))
              )}
            </section>
          </div>
        ) : (
          <div className="space-y-4">
            {renderSavedConfigsSection(settingsSavedOpen, setSettingsSavedOpen, "settings")}

            <section className="glass-card rounded-2xl p-4">
              <h2 className="text-lg font-semibold text-zinc-100">Create Config</h2>
              <p className="mt-1 text-xs text-zinc-400">
                Config targets are mapped by application name globally; PIDs are dynamic and not stored.
              </p>

              <div className="mt-4 grid grid-cols-1 gap-4 xl:grid-cols-[1.1fr_2fr]">
                <div className="rounded-xl border border-zinc-700 bg-zinc-900/45 p-3">
                  <label className="mb-1 block text-xs text-zinc-400">Config Name</label>
                  <input
                    className="w-full rounded-xl border border-zinc-700 bg-zinc-900/70 px-3 py-2 text-sm text-zinc-100 outline-none transition focus:border-zinc-400"
                    placeholder="Gaming / Work / Streaming"
                    value={builderName}
                    onChange={(event) => setBuilderName(event.target.value)}
                  />
                  <button className="btn-primary mt-3 w-full" onClick={() => void onSaveConfig()}>
                    Save Config
                  </button>
                </div>

                <div className="rounded-xl border border-zinc-700 bg-zinc-900/45 p-3">
                  <div className="grid grid-cols-1 gap-3 lg:grid-cols-[2fr_1fr_auto] lg:items-end">
                    <div className="min-w-0">
                      <label className="mb-1 block text-xs text-zinc-400">Target App</label>
                      <AppPickerDropdown
                        options={appPickerOptions}
                        value={builderTargetApp}
                        onChange={setBuilderTargetApp}
                        placeholder="Select running app..."
                      />
                    </div>

                    <div className="w-full lg:w-[220px]">
                      <label className="mb-1 block text-xs text-zinc-400">Priority</label>
                      <select
                        className="select h-[42px] w-full"
                        value={builderTargetPriority}
                        onChange={(event) => setBuilderTargetPriority(event.target.value as PriorityClass)}
                      >
                        {PRIORITY_OPTIONS.map((priority) => (
                          <option key={priority.value} value={priority.value}>
                            {priority.label}
                          </option>
                        ))}
                      </select>
                    </div>

                    <button className="btn-primary h-[42px] w-full px-4 lg:justify-self-end" onClick={onAddBuilderTarget}>
                      Add Target
                    </button>
                  </div>
                </div>
              </div>

              <div className="mt-4 space-y-2">
                {Object.keys(builderTargets).length === 0 ? (
                  <div className="rounded-xl border border-zinc-700 bg-zinc-900/50 px-3 py-3 text-sm text-zinc-400">
                    No staged targets yet.
                  </div>
                ) : (
                  Object.entries(builderTargets).map(([appName, priority]) => (
                    <div
                      key={appName}
                      className="flex flex-wrap items-center gap-2 rounded-xl border border-zinc-700 bg-zinc-900/50 px-3 py-2"
                    >
                      <span className="text-sm text-zinc-100">{appName}</span>
                      <span className="ml-auto rounded border border-zinc-600 px-2 py-1 text-xs text-zinc-300">
                        {PRIORITY_OPTIONS.find((item) => item.value === priority)?.label ?? priority}
                      </span>
                      <button
                        className="btn-danger px-2 py-1 text-xs"
                        onClick={() => onRemoveBuilderTarget(appName)}
                      >
                        Remove
                      </button>
                    </div>
                  ))
                )}
              </div>
            </section>

            <section className="glass-card rounded-2xl p-4">
              <h2 className="text-lg font-semibold text-zinc-100">Application Triggers</h2>
              <p className="mt-1 text-xs text-zinc-400">
                Select a running application to trigger automatic config enforcement.
              </p>

              <div className="mt-4 grid grid-cols-1 gap-3 md:grid-cols-[1.4fr_1fr_auto]">
                <div>
                  <label className="mb-1 block text-xs text-zinc-400">Trigger App</label>
                  <AppPickerDropdown
                    options={appPickerOptions}
                    value={watchTriggerApp}
                    onChange={setWatchTriggerApp}
                    placeholder="Select running app..."
                  />
                </div>
                <div>
                  <label className="mb-1 block text-xs text-zinc-400">Config</label>
                  <select
                    className="select w-full"
                    value={watchConfigName}
                    onChange={(event) => setWatchConfigName(event.target.value)}
                  >
                    <option value="">Select config...</option>
                    {configs.map((config) => (
                      <option key={config.name} value={config.name}>
                        {config.name}
                      </option>
                    ))}
                  </select>
                </div>
                <div className="flex items-end">
                  <button className="btn-primary" onClick={() => void onAddWatchMapping()}>
                    Add Mapping
                  </button>
                </div>
              </div>

              <div className="mt-4 space-y-2">
                {sortedMappings.length === 0 ? (
                  <div className="rounded-xl border border-zinc-700 bg-zinc-900/50 px-3 py-3 text-sm text-zinc-400">
                    No mappings configured.
                  </div>
                ) : (
                  sortedMappings.map(([appName, configName]) => {
                    const displayName =
                      groups.find((group) => equalsIgnoreCase(group.appName, appName))?.appName ?? appName;
                    return (
                      <div
                        key={appName}
                        className="flex flex-wrap items-center gap-2 rounded-xl border border-zinc-700 bg-zinc-900/50 px-3 py-2"
                      >
                        <AppIcon
                          appName={displayName}
                          iconBase64={appIconLookup.get(appName.toLowerCase()) ?? null}
                          className="h-7 w-7"
                        />
                        <span className="font-mono text-sm text-zinc-100">{displayName}</span>
                        <span className="text-zinc-500">{"->"}</span>
                        <span className="text-sm text-zinc-300">{configName}</span>
                        <button
                          className="btn-danger ml-auto px-2 py-1 text-xs"
                          onClick={() => void onRemoveWatchMapping(appName)}
                        >
                          Remove
                        </button>
                      </div>
                    );
                  })
                )}
              </div>
            </section>

            <section className="glass-card rounded-2xl p-4">
              <h2 className="text-lg font-semibold text-zinc-100">Core Engine</h2>
              <div className="mt-4 space-y-3">
                <label className="flex items-center justify-between rounded-xl border border-zinc-700 bg-zinc-900/50 px-3 py-2 text-sm">
                  <span className="text-zinc-200">Watchdog Service</span>
                  {renderToggleSwitch(
                    watchdogEnabled,
                    (next) => {
                      void onToggleWatchdog(next);
                    },
                    "Toggle watchdog service",
                  )}
                </label>

                <label className="flex items-center justify-between rounded-xl border border-zinc-700 bg-zinc-900/50 px-3 py-2 text-sm">
                  <span className="text-zinc-200">Start with Windows</span>
                  {renderToggleSwitch(
                    autostartEnabled,
                    (next) => {
                      void onToggleAutostart(next);
                    },
                    "Toggle start with Windows",
                  )}
                </label>

                <label className="flex items-center justify-between rounded-xl border border-zinc-700 bg-zinc-900/50 px-3 py-2 text-sm">
                  <span className="text-zinc-200">Minimize to Tray on Close</span>
                  {renderToggleSwitch(
                    minimizeToTrayEnabled,
                    (next) => {
                      void onToggleMinimizeToTray(next);
                    },
                    "Toggle minimize to tray on close",
                  )}
                </label>
              </div>
            </section>
          </div>
        )}
      </Layout>

      {elevationPending ? (
        <div className="fixed inset-0 z-[170] flex items-center justify-center bg-zinc-950/70 backdrop-blur-sm">
          <div className="glass-card rounded-xl border border-zinc-700/80 px-5 py-4 text-center">
            <p className="text-sm font-semibold text-zinc-100">Waiting for Administrator Approval</p>
            <p className="mt-1 text-xs text-zinc-400">Confirm the Windows UAC prompt to continue.</p>
          </div>
        </div>
      ) : null}

      <Toasts items={toasts} onDismiss={dismissToast} />
    </>
  );
}


