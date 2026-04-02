import { invoke } from "@tauri-apps/api/core";
import { useCallback, useState } from "react";
import { DeepPurgeConfig, useAppStore } from "../store/appStore";
import { ToastKind } from "../types/config";
import { TimerResolutionStatus } from "../types/engine";
import { parseIpcError } from "../types/ipc";

const TURBO_TIMER_TARGET_MS = 0.5;

type PushToast = (kind: ToastKind, message: string) => void;

export function useTimerEngine(pushToast: PushToast) {
  const totalDeepPurgeCount = useAppStore((state) => state.totalDeepPurgeCount);
  const totalDeepPurgeBytes = useAppStore((state) => state.totalDeepPurgeBytes);
  const deepPurgeConfig = useAppStore((state) => state.deepPurgeConfig);
  const setDeepPurgeConfig = useAppStore((state) => state.setDeepPurgeConfig);
  const recordDeepPurgeSuccess = useAppStore((state) => state.recordDeepPurgeSuccess);

  const [timerEnabled, setTimerEnabled] = useState(false);
  const [timerCurrentMs, setTimerCurrentMs] = useState<number | null>(null);
  const [timerBusy, setTimerBusy] = useState(false);
  const [deepPurgeBusy, setDeepPurgeBusy] = useState(false);

  const syncTimerStatus = useCallback((status: TimerResolutionStatus) => {
    setTimerEnabled(status.enabled);
    setTimerCurrentMs(status.currentMs);
  }, []);

  const loadTurboTimerStatus = useCallback(
    async (silent = true) => {
      try {
        const status = await invoke<TimerResolutionStatus>("engine_timer_get_status");
        syncTimerStatus(status);
      } catch (invokeError) {
        if (!silent) {
          pushToast("error", parseIpcError(invokeError).message);
        }
      }
    },
    [pushToast, syncTimerStatus],
  );

  const onToggleTimerResolution = useCallback(
    async (enabled: boolean) => {
      setTimerBusy(true);
      try {
        const status = await invoke<TimerResolutionStatus>("engine_timer_set", {
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
        pushToast("error", parseIpcError(invokeError).message);
      } finally {
        setTimerBusy(false);
      }
    },
    [pushToast],
  );

  const onRunDeepPurge = useCallback(async (config: DeepPurgeConfig) => {
    setDeepPurgeBusy(true);
    try {
      const bytesFreed = await invoke<number>("run_deep_purge", { config });
      const freedMb = Math.max(0, Math.floor((Number(bytesFreed) || 0) / (1024 * 1024)));
      recordDeepPurgeSuccess(Number(bytesFreed) || 0);
      pushToast("success", `Deep purge completed (${freedMb.toLocaleString()} MB freed)`);
    } catch (invokeError) {
      pushToast("error", parseIpcError(invokeError).message);
    } finally {
      setDeepPurgeBusy(false);
    }
  }, [pushToast, recordDeepPurgeSuccess]);

  return {
    timerEnabled,
    timerCurrentMs,
    timerBusy,
    deepPurgeBusy,
    deepPurgeConfig,
    totalDeepPurgeCount,
    totalDeepPurgeBytes,
    setDeepPurgeConfig,
    setTimerEnabled,
    loadTurboTimerStatus,
    onToggleTimerResolution,
    onRunDeepPurge,
  };
}
