import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import { useAppStore } from "../store/appStore";
import { ToastKind } from "../types/config";
import { MemoryPurgeConfig, MemoryStats } from "../types/engine";
import { parseIpcError } from "../types/ipc";

type PushToast = (kind: ToastKind, message: string) => void;

export function useMemoryPurgeEngine(pushToast: PushToast) {
  const totalPurges = useAppStore((state) => state.totalPurges);
  const totalRamClearedMb = useAppStore((state) => state.totalRamClearedMb);
  const incrementPurgeCount = useAppStore((state) => state.incrementPurgeCount);
  const addClearedRam = useAppStore((state) => state.addClearedRam);

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
    totalClearedMb: 0,
  });
  const [memoryConfigBusy, setMemoryConfigBusy] = useState(false);
  const [memoryPurgeBusy, setMemoryPurgeBusy] = useState(false);
  const backendTelemetryRef = useRef({
    initialized: false,
    totalPurges: 0,
    totalClearedMb: 0,
  });

  const applyRuntimeTelemetryDelta = useCallback(
    (config: MemoryPurgeConfig) => {
      const runtimePurges = Math.max(0, Math.floor(config.totalPurges));
      const runtimeClearedMb = Math.max(0, Math.floor(config.totalClearedMb));
      const previous = backendTelemetryRef.current;

      if (!previous.initialized) {
        if (runtimePurges > 0) {
          incrementPurgeCount(runtimePurges);
        }
        if (runtimeClearedMb > 0) {
          addClearedRam(runtimeClearedMb);
        }
        backendTelemetryRef.current = {
          initialized: true,
          totalPurges: runtimePurges,
          totalClearedMb: runtimeClearedMb,
        };
        return;
      }

      if (runtimePurges >= previous.totalPurges) {
        const purgeDelta = runtimePurges - previous.totalPurges;
        if (purgeDelta > 0) {
          incrementPurgeCount(purgeDelta);
        }
      }

      if (runtimeClearedMb >= previous.totalClearedMb) {
        const clearedDelta = runtimeClearedMb - previous.totalClearedMb;
        if (clearedDelta > 0) {
          addClearedRam(clearedDelta);
        }
      }

      backendTelemetryRef.current = {
        initialized: true,
        totalPurges: runtimePurges,
        totalClearedMb: runtimeClearedMb,
      };
    },
    [addClearedRam, incrementPurgeCount],
  );

  const syncMemoryTelemetry = useCallback(
    async (silent = true) => {
      try {
        const config = await invoke<MemoryPurgeConfig>("engine_memory_get_config");
        setMemoryPurgeConfig(config);
        applyRuntimeTelemetryDelta(config);
      } catch (invokeError) {
        if (!silent) {
          pushToast("error", parseIpcError(invokeError).message);
        }
      }
    },
    [applyRuntimeTelemetryDelta, pushToast],
  );

  const refreshMemoryStats = useCallback(
    async (silent = true): Promise<MemoryStats | null> => {
      try {
        const stats = await invoke<MemoryStats>("engine_memory_get_stats");
        setMemoryStats(stats);
        return stats;
      } catch (invokeError) {
        if (!silent) {
          pushToast("error", parseIpcError(invokeError).message);
        }
        return null;
      }
    },
    [pushToast],
  );

  useEffect(() => {
    const onVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        void refreshMemoryStats(true);
        void syncMemoryTelemetry(true);
      }
    };

    document.addEventListener("visibilitychange", onVisibilityChange);
    return () => {
      document.removeEventListener("visibilitychange", onVisibilityChange);
    };
  }, [refreshMemoryStats, syncMemoryTelemetry]);

  useEffect(() => {
    void syncMemoryTelemetry(true);

    const intervalId = window.setInterval(() => {
      if (document.visibilityState !== "visible") {
        return;
      }
      void syncMemoryTelemetry(true);
    }, 5_000);

    return () => {
      window.clearInterval(intervalId);
    };
  }, [syncMemoryTelemetry]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;

    void listen<number>("memory_purged_auto", (event) => {
      const freedMb = Math.max(0, Math.floor(Number(event.payload) || 0));
      incrementPurgeCount(1);
      addClearedRam(freedMb);
      setMemoryPurgeConfig((prev) => ({
        ...prev,
        totalPurges: Math.max(0, Math.floor(prev.totalPurges)) + 1,
        totalClearedMb: Math.max(0, Math.floor(prev.totalClearedMb)) + freedMb,
      }));
      backendTelemetryRef.current = {
        initialized: true,
        totalPurges: backendTelemetryRef.current.totalPurges + 1,
        totalClearedMb: backendTelemetryRef.current.totalClearedMb + freedMb,
      };
      void refreshMemoryStats(true);
    })
      .then((unsubscribe) => {
        if (disposed) {
          unsubscribe();
          return;
        }
        unlisten = unsubscribe;
      })
      .catch((error) => {
        console.error("Failed to subscribe to memory_purged_auto:", error);
      });

    return () => {
      disposed = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, [addClearedRam, incrementPurgeCount, refreshMemoryStats]);

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
        const updated = await invoke<MemoryPurgeConfig>("engine_memory_set_config", {
          masterEnabled: next.masterEnabled,
          enableStandbyTrigger: next.enableStandbyTrigger,
          standbyLimitMb: next.standbyLimitMb,
          enableFreeMemoryTrigger: next.enableFreeMemoryTrigger,
          freeMemoryLimitMb: next.freeMemoryLimitMb,
        });
        setMemoryPurgeConfig(updated);
      } catch (invokeError) {
        pushToast("error", parseIpcError(invokeError).message);
      } finally {
        setMemoryConfigBusy(false);
      }
    },
    [pushToast],
  );

  const onMemoryMasterToggle = useCallback(
    (enabled: boolean) => {
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
    [memoryPurgeConfig, persistMemoryPurgeConfig, pushToast],
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
      const latestStats = await refreshMemoryStats(true);
      const freedMb = Math.max(
        0,
        Math.floor(latestStats?.standbyListMb ?? memoryStats.standbyListMb),
      );
      const updated = await invoke<MemoryPurgeConfig>("engine_memory_purge");
      setMemoryPurgeConfig(updated);
      incrementPurgeCount(1);
      addClearedRam(freedMb);
      backendTelemetryRef.current = {
        initialized: true,
        totalPurges: Math.max(0, Math.floor(updated.totalPurges)),
        totalClearedMb: Math.max(0, Math.floor(updated.totalClearedMb)),
      };
      await refreshMemoryStats(true);
      pushToast("success", `Standby list cleanup completed (${freedMb.toLocaleString()} MB released)`);
    } catch (invokeError) {
      pushToast("error", parseIpcError(invokeError).message);
    } finally {
      setMemoryPurgeBusy(false);
    }
  }, [addClearedRam, incrementPurgeCount, memoryStats.standbyListMb, pushToast, refreshMemoryStats]);

  return {
    memoryStats,
    memoryPurgeConfig,
    totalPurges,
    totalRamClearedMb,
    memoryConfigBusy,
    memoryPurgeBusy,
    setMemoryPurgeConfig,
    refreshMemoryStats,
    syncMemoryTelemetry,
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
