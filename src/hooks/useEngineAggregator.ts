import { invoke } from "@tauri-apps/api/core";
import { useCallback, useState } from "react";
import { AppSettings, ElevationStatus, RuntimeSettings, ToastKind } from "../types/config";
import { parseIpcError } from "../types/ipc";
import { useAppStore } from "../store/appStore";
import { useHardwareControl } from "./useHardwareControl";
import { useMemoryPurgeEngine } from "./useMemoryPurgeEngine";
import { useOptimizationEngine } from "./useOptimizationEngine";
import { useTimerEngine } from "./useTimerEngine";

type PushToast = (kind: ToastKind, message: string) => void;

export function useEngineAggregator(pushToast: PushToast) {
  const alwaysRunAsAdmin = useAppStore((state) => state.alwaysRunAsAdmin);
  const setAlwaysRunAsAdmin = useAppStore((state) => state.setAlwaysRunAsAdmin);
  const [watchdogEnabled, setWatchdogEnabled] = useState(true);
  const [autostartEnabled, setAutostartEnabled] = useState(false);
  const [minimizeToTrayEnabled, setMinimizeToTrayEnabled] = useState(true);
  const [alwaysRunAsAdminPending, setAlwaysRunAsAdminPending] = useState(false);
  const [watchdogPending, setWatchdogPending] = useState(false);
  const [autostartPending, setAutostartPending] = useState(false);
  const [minimizeToTrayPending, setMinimizeToTrayPending] = useState(false);
  const [elevationPending, setElevationPending] = useState(false);

  const timer = useTimerEngine(pushToast);
  const memory = useMemoryPurgeEngine(pushToast);
  const optimization = useOptimizationEngine(pushToast);
  const hardware = useHardwareControl(pushToast);

  const loadRuntimeSettings = useCallback(async () => {
    const runtime = await invoke<RuntimeSettings>("engine_get_runtime_settings");
    setAutostartEnabled(runtime.autostartEnabled);
  }, []);

  const loadAppSettings = useCallback(
    async (silent = true) => {
      try {
        const settings = await invoke<AppSettings>("engine_get_app_settings");
        timer.setTimerEnabled(settings.turboTimerEnabled);
        setWatchdogEnabled(settings.watchdogEnabled);
        setMinimizeToTrayEnabled(settings.minimizeToTrayEnabled);
        memory.setMemoryPurgeConfig((prev) => ({
          ...prev,
          ...settings.memoryPurgeConfig,
        }));
      } catch (invokeError) {
        if (!silent) {
          pushToast("error", parseIpcError(invokeError).message);
        }
      }
    },
    [memory, pushToast, timer],
  );

  const onRequestElevation = useCallback(async () => {
    if (import.meta.env.DEV) {
      setElevationPending(false);
      pushToast("warning", "Dev Mode: Restart your terminal as Administrator instead.");
      return;
    }

    setElevationPending(true);
    try {
      const status = await invoke<ElevationStatus>("engine_elevation_restart");
      if (status.status === "elevation_pending") {
        pushToast("info", status.message);
      } else {
        setElevationPending(false);
      }
    } catch (invokeError) {
      setElevationPending(false);
      pushToast("error", parseIpcError(invokeError).message);
    }
  }, [pushToast]);

  const onRequestElevationClick = useCallback(() => {
    void onRequestElevation();
  }, [onRequestElevation]);

  const onToggleWatchdog = useCallback(
    async (enabled: boolean) => {
      if (watchdogPending) {
        return;
      }
      setWatchdogPending(true);
      setWatchdogEnabled(enabled);
      try {
        await invoke("engine_watchdog_set_enabled", { state: enabled });
        await loadAppSettings(true);
      } catch (invokeError) {
        await loadAppSettings(true);
        pushToast("error", parseIpcError(invokeError).message);
      } finally {
        setWatchdogPending(false);
      }
    },
    [loadAppSettings, pushToast, watchdogPending],
  );

  const onToggleAutostart = useCallback(
    async (enabled: boolean) => {
      if (autostartPending) {
        return;
      }
      setAutostartPending(true);
      setAutostartEnabled(enabled);
      try {
        await invoke("engine_autostart_configure", {
          enabled,
          asAdmin: true,
        });
        await loadRuntimeSettings();
      } catch (invokeError) {
        await loadRuntimeSettings();
        pushToast("error", parseIpcError(invokeError).message);
      } finally {
        setAutostartPending(false);
      }
    },
    [autostartPending, loadRuntimeSettings, pushToast],
  );

  const enableElevatedAutostartForOnboarding = useCallback(async () => {
    if (autostartPending) {
      return false;
    }
    setAutostartPending(true);
    try {
      await invoke("engine_autostart_configure", {
        enabled: true,
        asAdmin: true,
      });
      await loadRuntimeSettings();
      return true;
    } catch (invokeError) {
      await loadRuntimeSettings();
      pushToast("error", parseIpcError(invokeError).message);
      return false;
    } finally {
      setAutostartPending(false);
    }
  }, [autostartPending, loadRuntimeSettings, pushToast]);

  const onToggleMinimizeToTray = useCallback(
    async (enabled: boolean) => {
      if (minimizeToTrayPending) {
        return;
      }
      setMinimizeToTrayPending(true);
      setMinimizeToTrayEnabled(enabled);
      try {
        await invoke("engine_tray_set_minimize", { enabled });
        await loadAppSettings(true);
      } catch (invokeError) {
        await loadAppSettings(true);
        pushToast("error", parseIpcError(invokeError).message);
      } finally {
        setMinimizeToTrayPending(false);
      }
    },
    [loadAppSettings, minimizeToTrayPending, pushToast],
  );

  const onToggleAlwaysRunAsAdmin = useCallback(
    async (enabled: boolean) => {
      if (alwaysRunAsAdminPending) {
        return;
      }

      const previous = alwaysRunAsAdmin;
      setAlwaysRunAsAdminPending(true);
      setAlwaysRunAsAdmin(enabled);
      try {
        await invoke("set_run_as_admin", { enable: enabled });
      } catch (invokeError) {
        setAlwaysRunAsAdmin(previous);
        pushToast("error", parseIpcError(invokeError).message);
      } finally {
        setAlwaysRunAsAdminPending(false);
      }
    },
    [alwaysRunAsAdmin, alwaysRunAsAdminPending, pushToast, setAlwaysRunAsAdmin],
  );

  return {
    watchdogEnabled,
    autostartEnabled,
    minimizeToTrayEnabled,
    alwaysRunAsAdmin,
    watchdogPending,
    autostartPending,
    minimizeToTrayPending,
    alwaysRunAsAdminPending,
    elevationPending,
    timerEnabled: timer.timerEnabled,
    timerCurrentMs: timer.timerCurrentMs,
    timerBusy: timer.timerBusy,
    deepPurgeBusy: timer.deepPurgeBusy,
    deepPurgeConfig: timer.deepPurgeConfig,
    totalDeepPurgeCount: timer.totalDeepPurgeCount,
    totalDeepPurgeBytes: timer.totalDeepPurgeBytes,
    memoryStats: memory.memoryStats,
    memoryPurgeConfig: memory.memoryPurgeConfig,
    totalPurges: memory.totalPurges,
    totalRamClearedMb: memory.totalRamClearedMb,
    memoryConfigBusy: memory.memoryConfigBusy,
    memoryPurgeBusy: memory.memoryPurgeBusy,
    optimizationStatus: optimization.optimizationStatus,
    optimizationLoading: optimization.optimizationLoading,
    telemetryBusy: optimization.telemetryBusy,
    netSniperBusy: optimization.netSniperBusy,
    powerBusy: optimization.powerBusy,
    advancedBusy: optimization.advancedBusy,
    pciDevices: hardware.pciDevices,
    drivers: hardware.drivers,
    ghostDevices: hardware.ghostDevices,
    pciLoading: hardware.pciLoading,
    driversLoading: hardware.driversLoading,
    ghostsLoading: hardware.ghostsLoading,
    pciApplying: hardware.pciApplying,
    driverDeleting: hardware.driverDeleting,
    ghostRemoving: hardware.ghostRemoving,
    loadRuntimeSettings,
    loadAppSettings,
    loadOptimizationStatus: optimization.loadOptimizationStatus,
    loadTurboTimerStatus: timer.loadTurboTimerStatus,
    refreshMemoryStats: memory.refreshMemoryStats,
    syncMemoryTelemetry: memory.syncMemoryTelemetry,
    loadPciDevices: hardware.loadPciDevices,
    loadInstalledDrivers: hardware.loadInstalledDrivers,
    loadGhostDevices: hardware.loadGhostDevices,
    onRequestElevationClick,
    onToggleWatchdog,
    onToggleAutostart,
    enableElevatedAutostartForOnboarding,
    onToggleMinimizeToTray,
    onToggleAlwaysRunAsAdmin,
    onToggleTimerResolution: timer.onToggleTimerResolution,
    onRunDeepPurge: timer.onRunDeepPurge,
    setDeepPurgeConfig: timer.setDeepPurgeConfig,
    onMemoryMasterToggle: memory.onMemoryMasterToggle,
    onStandbyTriggerToggle: memory.onStandbyTriggerToggle,
    onStandbyLimitChange: memory.onStandbyLimitChange,
    onStandbyLimitBlur: memory.onStandbyLimitBlur,
    onFreeMemoryTriggerToggle: memory.onFreeMemoryTriggerToggle,
    onFreeMemoryLimitChange: memory.onFreeMemoryLimitChange,
    onFreeMemoryLimitBlur: memory.onFreeMemoryLimitBlur,
    onPurgeNow: memory.onPurgeNow,
    onToggleTelemetry: optimization.onToggleTelemetry,
    onToggleNetSniper: optimization.onToggleNetSniper,
    onTogglePowerMode: optimization.onTogglePowerMode,
    onToggleAdvanced: optimization.onToggleAdvanced,
    applyMsiBatch: hardware.applyMsiBatch,
    removeDriver: hardware.removeDriver,
    removeGhost: hardware.removeGhost,
  };
}
