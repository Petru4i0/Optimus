import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import {
  AppSettings,
  ElevationStatus,
  MemoryPurgeConfig,
  MemoryStats,
  RuntimeSettings,
  TimerResolutionStatus,
  ToastKind,
} from "../types/process";

const TURBO_TIMER_TARGET_MS = 0.5;

type PushToast = (kind: ToastKind, message: string) => void;

export function useEngineManager(pushToast: PushToast, isElevated: boolean) {
  const [watchdogEnabled, setWatchdogEnabled] = useState(true);
  const [autostartEnabled, setAutostartEnabled] = useState(false);
  const [startAsAdminEnabled, setStartAsAdminEnabled] = useState(true);
  const [minimizeToTrayEnabled, setMinimizeToTrayEnabled] = useState(true);
  const [timerEnabled, setTimerEnabled] = useState(false);
  const [timerCurrentMs, setTimerCurrentMs] = useState<number | null>(null);
  const [timerBusy, setTimerBusy] = useState(false);
  const [memoryStats, setMemoryStats] = useState<MemoryStats>({
    standbyListMb: 0,
    freeMemoryMb: 0,
    totalMemoryMb: 0,
  });
  const [memoryPurgeConfig, setMemoryPurgeConfig] = useState<MemoryPurgeConfig>({
    masterEnabled: false,
    enableStandbyTrigger: false,
    standbyLimitMb: 1024,
    enableFreeMemoryTrigger: false,
    freeMemoryLimitMb: 1024,
    totalPurges: 0,
  });
  const [memoryConfigBusy, setMemoryConfigBusy] = useState(false);
  const [memoryPurgeBusy, setMemoryPurgeBusy] = useState(false);
  const [elevationPending, setElevationPending] = useState(false);

  const syncMemoryPurgeCount = useCallback(async () => {
    try {
      const config = await invoke<MemoryPurgeConfig>("get_memory_purge_config");
      setMemoryPurgeConfig((prev) => ({ ...prev, totalPurges: config.totalPurges }));
    } catch {
      // Optional telemetry sync; keep UI stable if this fails.
    }
  }, []);

  const loadRuntimeSettings = useCallback(async () => {
    const runtime = await invoke<RuntimeSettings>("get_runtime_settings");
    const mode =
      runtime.autostartMode ??
      (runtime.autostartEnabled
        ? runtime.startAsAdminEnabled
          ? "elevated"
          : "user"
        : "off");
    setAutostartEnabled(mode !== "off");
    setStartAsAdminEnabled(mode === "elevated");
  }, []);

  const loadAppSettings = useCallback(
    async (silent = true) => {
      try {
        const settings = await invoke<AppSettings>("get_app_settings");
        setTimerEnabled(settings.turboTimerEnabled);
        setWatchdogEnabled(settings.watchdogEnabled);
        setMinimizeToTrayEnabled(settings.minimizeToTrayEnabled);
        setMemoryPurgeConfig((prev) => ({
          ...prev,
          ...settings.memoryPurgeConfig,
        }));
      } catch (invokeError) {
        if (!silent) {
          const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
          pushToast("error", message);
        }
      }
    },
    [pushToast],
  );

  const refreshMemoryStats = useCallback(
    async (silent = true) => {
      try {
        const stats = await invoke<MemoryStats>("get_memory_stats");
        setMemoryStats(stats);
        await syncMemoryPurgeCount();
      } catch (invokeError) {
        if (!silent) {
          const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
          pushToast("error", message);
        }
      }
    },
    [pushToast, syncMemoryPurgeCount],
  );

  useEffect(() => {
    const onVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        void refreshMemoryStats(true);
      }
    };

    document.addEventListener("visibilitychange", onVisibilityChange);
    return () => {
      document.removeEventListener("visibilitychange", onVisibilityChange);
    };
  }, [refreshMemoryStats]);

  const syncTimerStatus = useCallback((status: TimerResolutionStatus) => {
    setTimerEnabled(status.enabled);
    setTimerCurrentMs(status.currentMs);
  }, []);

  const loadTurboTimerStatus = useCallback(
    async (silent = true) => {
      try {
        const status = await invoke<TimerResolutionStatus>("get_current_timer_res");
        syncTimerStatus(status);
      } catch (invokeError) {
        if (!silent) {
          const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
          pushToast("error", message);
        }
      }
    },
    [pushToast, syncTimerStatus],
  );

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
        await invoke("configure_autostart", {
          enabled,
          asAdmin: startAsAdminEnabled,
        });
      } catch (invokeError) {
        setAutostartEnabled((prev) => !prev);
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      }
    },
    [pushToast, startAsAdminEnabled],
  );

  const onToggleStartAsAdmin = useCallback(
    async (enabled: boolean) => {
      setStartAsAdminEnabled(enabled);
      if (!autostartEnabled) {
        return;
      }

      try {
        await invoke("configure_autostart", {
          enabled: true,
          asAdmin: enabled,
        });
      } catch (invokeError) {
        setStartAsAdminEnabled((prev) => !prev);
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      }
    },
    [autostartEnabled, pushToast],
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

  const onToggleTimerResolution = useCallback(
    async (enabled: boolean) => {
      setTimerBusy(true);
      try {
        const status = await invoke<TimerResolutionStatus>("set_timer_res", {
          value: enabled ? TURBO_TIMER_TARGET_MS : 0,
        });
        setTimerEnabled(status.enabled);
        setTimerCurrentMs(status.currentMs);
        pushToast(
          "success",
          enabled
            ? `Latency optimizer enabled at ${status.currentMs.toFixed(3)} ms`
            : "Latency optimizer disabled",
        );
      } catch (invokeError) {
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      } finally {
        setTimerBusy(false);
      }
    },
    [pushToast],
  );

  const persistMemoryPurgeConfig = useCallback(
    async (
      next: Pick<
        MemoryPurgeConfig,
        | "masterEnabled"
        | "enableStandbyTrigger"
        | "standbyLimitMb"
        | "enableFreeMemoryTrigger"
        | "freeMemoryLimitMb"
      >,
    ) => {
      setMemoryConfigBusy(true);
      try {
        const updated = await invoke<MemoryPurgeConfig>("set_memory_purge_config", {
          masterEnabled: next.masterEnabled,
          enableStandbyTrigger: next.enableStandbyTrigger,
          standbyLimitMb: next.standbyLimitMb,
          enableFreeMemoryTrigger: next.enableFreeMemoryTrigger,
          freeMemoryLimitMb: next.freeMemoryLimitMb,
        });
        setMemoryPurgeConfig(updated);
      } catch (invokeError) {
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      } finally {
        setMemoryConfigBusy(false);
      }
    },
    [pushToast],
  );

  const onMemoryMasterToggle = useCallback(
    (enabled: boolean) => {
      if (enabled && !isElevated) {
        pushToast("warning", "Administrator privileges required. Please restart Optimus as Admin.");
        return;
      }
      if (
        enabled &&
        !memoryPurgeConfig.enableStandbyTrigger &&
        !memoryPurgeConfig.enableFreeMemoryTrigger
      ) {
        pushToast("info", "Select at least one trigger condition before enabling.");
        return;
      }
      const next = {
        ...memoryPurgeConfig,
        masterEnabled: enabled,
      };
      setMemoryPurgeConfig(next);
      void persistMemoryPurgeConfig(next);
    },
    [isElevated, memoryPurgeConfig, persistMemoryPurgeConfig, pushToast],
  );

  const onStandbyTriggerToggle = useCallback(
    (enabled: boolean) => {
      if (memoryPurgeConfig.masterEnabled || memoryConfigBusy) {
        return;
      }
      const next = {
        ...memoryPurgeConfig,
        enableStandbyTrigger: enabled,
      };
      setMemoryPurgeConfig(next);
      void persistMemoryPurgeConfig(next);
    },
    [memoryConfigBusy, memoryPurgeConfig, persistMemoryPurgeConfig],
  );

  const onStandbyLimitChange = useCallback(
    (value: number) => {
      if (memoryPurgeConfig.masterEnabled || memoryConfigBusy) {
        return;
      }
      if (!Number.isFinite(value) || value <= 0) {
        const next = {
          ...memoryPurgeConfig,
          enableStandbyTrigger: false,
          standbyLimitMb: 1024,
        };
        setMemoryPurgeConfig(next);
        void persistMemoryPurgeConfig(next);
        return;
      }
      const normalized = Math.round(value);
      const next = {
        ...memoryPurgeConfig,
        standbyLimitMb: normalized,
      };
      setMemoryPurgeConfig(next);
    },
    [memoryConfigBusy, memoryPurgeConfig, persistMemoryPurgeConfig],
  );

  const onStandbyLimitBlur = useCallback(() => {
    void persistMemoryPurgeConfig(memoryPurgeConfig);
  }, [memoryPurgeConfig, persistMemoryPurgeConfig]);

  const onFreeMemoryLimitBlur = useCallback(() => {
    void persistMemoryPurgeConfig(memoryPurgeConfig);
  }, [memoryPurgeConfig, persistMemoryPurgeConfig]);

  const onFreeMemoryLimitChange = useCallback(
    (value: number) => {
      if (memoryPurgeConfig.masterEnabled || memoryConfigBusy) {
        return;
      }
      if (!Number.isFinite(value) || value <= 0) {
        const next = {
          ...memoryPurgeConfig,
          enableFreeMemoryTrigger: false,
          freeMemoryLimitMb: 1024,
        };
        setMemoryPurgeConfig(next);
        void persistMemoryPurgeConfig(next);
        return;
      }
      const normalized = Math.round(value);
      const next = {
        ...memoryPurgeConfig,
        freeMemoryLimitMb: normalized,
      };
      setMemoryPurgeConfig(next);
    },
    [memoryConfigBusy, memoryPurgeConfig, persistMemoryPurgeConfig],
  );

  const onFreeMemoryTriggerToggle = useCallback(
    (enabled: boolean) => {
      if (memoryPurgeConfig.masterEnabled || memoryConfigBusy) {
        return;
      }
      const next = {
        ...memoryPurgeConfig,
        enableFreeMemoryTrigger: enabled,
      };
      setMemoryPurgeConfig(next);
      void persistMemoryPurgeConfig(next);
    },
    [memoryConfigBusy, memoryPurgeConfig, persistMemoryPurgeConfig],
  );

  const onPurgeNow = useCallback(async () => {
    setMemoryPurgeBusy(true);
    try {
      const updated = await invoke<MemoryPurgeConfig>("run_purge");
      setMemoryPurgeConfig(updated);
      await refreshMemoryStats(true);
      pushToast("success", "Standby list purge completed");
    } catch (invokeError) {
      const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
      pushToast("error", message);
    } finally {
      setMemoryPurgeBusy(false);
    }
  }, [pushToast, refreshMemoryStats]);

  return {
    watchdogEnabled,
    autostartEnabled,
    startAsAdminEnabled,
    minimizeToTrayEnabled,
    timerEnabled,
    timerCurrentMs,
    timerBusy,
    memoryStats,
    memoryPurgeConfig,
    memoryConfigBusy,
    memoryPurgeBusy,
    elevationPending,
    loadRuntimeSettings,
    loadAppSettings,
    loadTurboTimerStatus,
    refreshMemoryStats,
    onRequestElevationClick,
    onToggleWatchdog,
    onToggleAutostart,
    onToggleStartAsAdmin,
    onToggleMinimizeToTray,
    onToggleTimerResolution,
    onMemoryMasterToggle,
    onStandbyTriggerToggle,
    onStandbyLimitChange,
    onStandbyLimitBlur,
    onFreeMemoryTriggerToggle,
    onFreeMemoryLimitChange,
    onFreeMemoryLimitBlur,
    onPurgeNow,
  };
}
