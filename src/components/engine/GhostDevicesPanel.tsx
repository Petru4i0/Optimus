import { useState } from "react";
import { Ghost } from "lucide-react";
import { useAppStore } from "../../store/appStore";
import { GhostDevice } from "../../types/hardware";
import HardwareActionModal from "../HardwareActionModal";
import InfoTooltip from "../ui/InfoTooltip";

type GhostDevicesPanelProps = {
  ghostDevices: GhostDevice[];
  ghostsLoading: boolean;
  ghostRemoving: boolean;
  onRefreshGhosts: (silent?: boolean) => void;
  onRemoveGhost: (instanceId: string, force: boolean) => void;
};

export default function GhostDevicesPanel({
  ghostDevices,
  ghostsLoading,
  ghostRemoving,
  onRefreshGhosts,
  onRemoveGhost,
}: GhostDevicesPanelProps) {
  const scanMode = useAppStore((state) => state.ghostScanMode);
  const setScanMode = useAppStore((state) => state.setGhostScanMode);
  const [pendingGhostDelete, setPendingGhostDelete] = useState<GhostDevice | null>(null);

  const resolveSafetyLevel = (device: GhostDevice): "Critical" | "Junk" | "Caution" => {
    const normalized = (device.safetyLevel ?? "").trim().toLowerCase();
    if (normalized === "critical") {
      return "Critical";
    }
    if (normalized === "junk") {
      return "Junk";
    }
    return "Caution";
  };

  const safetyBadgeClass = (safety: "Critical" | "Junk" | "Caution") => {
    if (safety === "Critical") {
      return "border border-rose-500/30 bg-rose-500/10 text-rose-500";
    }
    if (safety === "Junk") {
      return "border border-emerald-500/30 bg-emerald-500/10 text-emerald-500";
    }
    return "border border-yellow-500/30 bg-yellow-500/20 text-yellow-500";
  };

  const startScan = () => {
    setScanMode("active");
    void onRefreshGhosts(false);
  };

  const resetScan = () => {
    setScanMode("idle");
  };

  return (
    <div className="mt-4 space-y-3">
      {scanMode === "idle" ? (
        <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-6">
          <div className="flex flex-col items-center justify-center py-12 text-center">
            <Ghost className="h-8 w-8 text-zinc-400" />
            <div className="mt-4 flex items-center gap-2">
              <p className="text-sm text-zinc-400">Inactive Devices</p>
              <InfoTooltip translationKey="ghost_devices" />
            </div>
            <p className="mt-2 text-sm text-zinc-400">Inactive device scan is manual</p>
            <p className="mt-1 max-w-xl text-xs text-zinc-400">
              Scan disconnected devices only when needed to keep hardware operations explicit and safe.
            </p>
            <div className="mt-5 flex flex-wrap items-center justify-center gap-4">
              <button className="btn-primary px-3 py-1.5 text-xs" onClick={startScan}>
                Scan Disconnected Devices
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {scanMode === "active" ? (
        <>
          <div className="flex items-center gap-2">
            <div className="flex items-center gap-2">
              <h3 className="text-sm font-semibold text-zinc-100">Inactive Devices</h3>
              <InfoTooltip translationKey="ghost_devices" />
            </div>
            <p className="text-xs text-zinc-400">
              Disconnected devices from pnputil /enum-devices /disconnected.
            </p>
            <button className="btn-ghost ml-auto px-3 py-1.5 text-xs" onClick={() => void onRefreshGhosts(false)}>
              Refresh
            </button>
            <button className="btn-ghost px-3 py-1.5 text-xs" onClick={resetScan}>
              Reset
            </button>
          </div>

          <div className="overflow-x-auto rounded-xl border border-zinc-800">
            <table className="min-w-full text-left text-sm">
              <thead className="bg-zinc-900 text-xs text-zinc-400">
                <tr>
                  <th className="px-3 py-2">Device</th>
                  <th className="px-3 py-2">Instance ID</th>
                  <th className="px-3 py-2">Class</th>
                  <th className="px-3 py-2">Safety</th>
                  <th className="px-3 py-2" />
                </tr>
              </thead>
              <tbody>
                {ghostsLoading ? (
                  <tr>
                    <td className="px-3 py-3 text-zinc-400" colSpan={5}>
                      Loading inactive devices...
                    </td>
                  </tr>
                ) : ghostDevices.length === 0 ? (
                  <tr>
                    <td className="px-3 py-3 text-zinc-400" colSpan={5}>
                      No disconnected devices found.
                    </td>
                  </tr>
                ) : (
                  ghostDevices.map((device) => {
                    const safetyLevel = resolveSafetyLevel(device);
                    const removeDisabled = ghostRemoving || safetyLevel === "Critical";

                    return (
                      <tr key={device.instanceId} className="border-t border-zinc-800 bg-zinc-900">
                        <td className="px-3 py-2 text-zinc-100">{device.deviceDescription || device.instanceId}</td>
                        <td className="px-3 py-2 font-mono text-xs text-zinc-400">{device.instanceId}</td>
                        <td className="px-3 py-2 text-zinc-400">{device.className || "Unknown"}</td>
                        <td className="px-3 py-2">
                          <span
                            className={`inline-flex rounded px-2 py-0.5 text-[11px] font-medium ${safetyBadgeClass(safetyLevel)}`}
                          >
                            {safetyLevel}
                          </span>
                        </td>
                        <td className="px-3 py-2 text-right">
                          <button
                            className="btn-danger px-2 py-1 text-xs disabled:cursor-not-allowed disabled:opacity-50"
                            disabled={removeDisabled}
                            onClick={() => setPendingGhostDelete(device)}
                            title={
                              safetyLevel === "Critical"
                                ? "Critical device is protected from removal."
                                : undefined
                            }
                          >
                            Remove
                          </button>
                        </td>
                      </tr>
                    );
                  })
                )}
              </tbody>
            </table>
          </div>
        </>
      ) : null}

      <HardwareActionModal
        isOpen={pendingGhostDelete !== null}
        title="Remove Inactive Device"
        warningText={`You are about to remove ${
          pendingGhostDelete?.deviceDescription || pendingGhostDelete?.instanceId || "this disconnected device"
        }. Use force only if normal removal is blocked.`}
        onClose={() => setPendingGhostDelete(null)}
        onConfirm={(force) => {
          if (!pendingGhostDelete) {
            return;
          }
          onRemoveGhost(pendingGhostDelete.instanceId, force);
          setPendingGhostDelete(null);
        }}
      />
    </div>
  );
}
