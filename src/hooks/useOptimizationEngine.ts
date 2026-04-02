import { invoke } from "@tauri-apps/api/core";
import { useCallback, useState } from "react";
import { OptimizationStatus, ToastKind } from "../types/config";
import {
  AdvancedSubFeature,
  NetSniperSubFeature,
  PowerSubFeature,
  TelemetrySubFeature,
} from "../types/engine";
import { parseIpcError } from "../types/ipc";

type PushToast = (kind: ToastKind, message: string) => void;

export function useOptimizationEngine(pushToast: PushToast) {
  const [optimizationStatus, setOptimizationStatus] = useState<OptimizationStatus | null>(null);
  const [statusFetchFailed, setStatusFetchFailed] = useState(false);
  const [optimizationLoading, setOptimizationLoading] = useState(false);
  const [telemetryBusy, setTelemetryBusy] = useState(false);
  const [netSniperBusy, setNetSniperBusy] = useState(false);
  const [powerBusy, setPowerBusy] = useState(false);
  const [advancedBusy, setAdvancedBusy] = useState(false);

  const loadOptimizationStatus = useCallback(
    async (silent = true) => {
      if (!silent) {
        setOptimizationLoading(true);
      }
      try {
        const status = await invoke<OptimizationStatus>("optimization_get_status");
        setOptimizationStatus(status);
        setStatusFetchFailed(false);
      } catch (invokeError) {
        setStatusFetchFailed(true);
        if (!silent) {
          pushToast("error", parseIpcError(invokeError).message);
        }
      } finally {
        if (!silent) {
          setOptimizationLoading(false);
        }
      }
    },
    [pushToast],
  );

  const onToggleTelemetry = useCallback(
    async (subFeature: TelemetrySubFeature, enabled: boolean) => {
      setTelemetryBusy(true);
      try {
        const status = await invoke<OptimizationStatus>("optimization_telemetry_toggle", {
          subFeature,
          enabled,
        });
        setOptimizationStatus(status);
        pushToast(
          "success",
          enabled ? `Telemetry '${subFeature}' enabled` : `Telemetry '${subFeature}' disabled`,
        );
      } catch (invokeError) {
        pushToast("error", parseIpcError(invokeError).message);
      } finally {
        setTelemetryBusy(false);
      }
    },
    [pushToast],
  );

  const onToggleNetSniper = useCallback(
    async (subFeature: NetSniperSubFeature, enabled: boolean) => {
      setNetSniperBusy(true);
      try {
        const status = await invoke<OptimizationStatus>("optimization_net_sniper_toggle", {
          subFeature,
          enabled,
        });
        setOptimizationStatus(status);
        pushToast(
          "success",
          enabled ? `Internet '${subFeature}' enabled` : `Internet '${subFeature}' disabled`,
        );
      } catch (invokeError) {
        pushToast("error", parseIpcError(invokeError).message);
      } finally {
        setNetSniperBusy(false);
      }
    },
    [pushToast],
  );

  const onTogglePowerMode = useCallback(
    async (subFeature: PowerSubFeature, enabled: boolean) => {
      setPowerBusy(true);
      try {
        const status = await invoke<OptimizationStatus>("optimization_power_toggle", {
          subFeature,
          enabled,
        });
        setOptimizationStatus(status);
        pushToast("success", enabled ? `Power '${subFeature}' enabled` : `Power '${subFeature}' disabled`);
      } catch (invokeError) {
        pushToast("error", parseIpcError(invokeError).message);
      } finally {
        setPowerBusy(false);
      }
    },
    [pushToast],
  );

  const onToggleAdvanced = useCallback(
    async (subFeature: AdvancedSubFeature, enabled: boolean) => {
      setAdvancedBusy(true);
      try {
        const status = await invoke<OptimizationStatus>("optimization_advanced_toggle", {
          subFeature,
          enabled,
        });
        setOptimizationStatus(status);
        pushToast(
          "success",
          enabled ? `Advanced '${subFeature}' enabled` : `Advanced '${subFeature}' disabled`,
        );
      } catch (invokeError) {
        pushToast("error", parseIpcError(invokeError).message);
      } finally {
        setAdvancedBusy(false);
      }
    },
    [pushToast],
  );

  return {
    optimizationStatus,
    statusFetchFailed,
    optimizationLoading,
    telemetryBusy,
    netSniperBusy,
    powerBusy,
    advancedBusy,
    loadOptimizationStatus,
    onToggleTelemetry,
    onToggleNetSniper,
    onTogglePowerMode,
    onToggleAdvanced,
  };
}
