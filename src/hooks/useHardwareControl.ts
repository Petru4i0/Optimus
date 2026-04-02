import { invoke } from "@tauri-apps/api/core";
import { useCallback, useState } from "react";
import { ToastKind } from "../types/config";
import {
  Driver,
  GhostDevice,
  MsiApplyDto,
  MsiBatchReportDto,
  MsiPriority,
  PciDevice,
} from "../types/hardware";
import { parseIpcError } from "../types/ipc";

type PushToast = (kind: ToastKind, message: string) => void;

export function useHardwareControl(pushToast: PushToast) {
  const [pciDevices, setPciDevices] = useState<PciDevice[]>([]);
  const [drivers, setDrivers] = useState<Driver[]>([]);
  const [ghostDevices, setGhostDevices] = useState<GhostDevice[]>([]);
  const [pciLoading, setPciLoading] = useState(false);
  const [driversLoading, setDriversLoading] = useState(false);
  const [ghostsLoading, setGhostsLoading] = useState(false);
  const [pciApplying, setPciApplying] = useState(false);
  const [driverDeleting, setDriverDeleting] = useState(false);
  const [ghostRemoving, setGhostRemoving] = useState(false);

  const loadPciDevices = useCallback(
    async (silent = true) => {
      if (!silent) {
        setPciLoading(true);
      }
      try {
        const list = await invoke<PciDevice[]>("hardware_msi_list");
        setPciDevices(list);
      } catch (invokeError) {
        if (!silent) {
          pushToast("error", parseIpcError(invokeError).message);
        }
      } finally {
        if (!silent) {
          setPciLoading(false);
        }
      }
    },
    [pushToast],
  );

  const loadInstalledDrivers = useCallback(
    async (silent = true) => {
      if (!silent) {
        setDriversLoading(true);
      }
      try {
        const list = await invoke<Driver[]>("hardware_driver_list");
        setDrivers(list);
      } catch (invokeError) {
        if (!silent) {
          pushToast("error", parseIpcError(invokeError).message);
        }
      } finally {
        if (!silent) {
          setDriversLoading(false);
        }
      }
    },
    [pushToast],
  );

  const loadGhostDevices = useCallback(
    async (silent = true) => {
      if (!silent) {
        setGhostsLoading(true);
      }
      try {
        const list = await invoke<GhostDevice[]>("hardware_ghost_list");
        setGhostDevices(list);
      } catch (invokeError) {
        if (!silent) {
          pushToast("error", parseIpcError(invokeError).message);
        }
      } finally {
        if (!silent) {
          setGhostsLoading(false);
        }
      }
    },
    [pushToast],
  );

  const applyMsiBatch = useCallback(
    async (updates: Array<{ deviceId: string; enable: boolean; priority: MsiPriority }>) => {
      if (updates.length === 0) {
        pushToast("info", "No MSI changes to apply");
        return;
      }

      setPciApplying(true);
      try {
        const payload: MsiApplyDto[] = updates.map((item) => ({
          deviceId: item.deviceId,
          enable: item.enable,
          priority: item.priority,
        }));
        const report = await invoke<MsiBatchReportDto>("hardware_msi_apply_batch", { payload });
        if (report.failed > 0) {
          pushToast(
            "warning",
            `Applied with ${report.failed} failure(s). System reboot required to take effect.`,
          );
        } else {
          pushToast("success", "Applied. System reboot required to take effect.");
        }
      } catch (invokeError) {
        pushToast("error", parseIpcError(invokeError).message);
      } finally {
        setPciApplying(false);
        await loadPciDevices(true);
      }
    },
    [loadPciDevices, pushToast],
  );

  const removeDriver = useCallback(
    async (publishedName: string, force: boolean) => {
      setDriverDeleting(true);
      try {
        await invoke("hardware_driver_delete", { publishedName, force });
        pushToast("success", `${publishedName} deleted`);
        await loadInstalledDrivers(true);
      } catch (invokeError) {
        pushToast("error", parseIpcError(invokeError).message);
      } finally {
        setDriverDeleting(false);
      }
    },
    [loadInstalledDrivers, pushToast],
  );

  const removeGhost = useCallback(
    async (instanceId: string, force: boolean) => {
      setGhostRemoving(true);
      try {
        await invoke("hardware_ghost_remove", { instanceId, force });
        pushToast("success", "Inactive device removed");
        await loadGhostDevices(true);
      } catch (invokeError) {
        pushToast("error", parseIpcError(invokeError).message);
      } finally {
        setGhostRemoving(false);
      }
    },
    [loadGhostDevices, pushToast],
  );

  return {
    pciDevices,
    drivers,
    ghostDevices,
    pciLoading,
    driversLoading,
    ghostsLoading,
    pciApplying,
    driverDeleting,
    ghostRemoving,
    loadPciDevices,
    loadInstalledDrivers,
    loadGhostDevices,
    applyMsiBatch,
    removeDriver,
    removeGhost,
  };
}
