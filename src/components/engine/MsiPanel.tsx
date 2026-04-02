import { useEffect, useMemo, useState } from "react";
import { useAppStore } from "../../store/appStore";
import { MsiPriority, PciDevice } from "../../types/hardware";
import InfoTooltip from "../ui/InfoTooltip";

type MsiPanelProps = {
  pciDevices: PciDevice[];
  pciLoading: boolean;
  pciApplying: boolean;
  onRefreshPci: () => void;
  onApplyMsiBatch: (updates: Array<{ deviceId: string; enable: boolean; priority: MsiPriority }>) => void;
};

type MsiDraft = Record<string, { enable: boolean; priority: MsiPriority }>;

const PRIORITY_OPTIONS: Array<{ value: MsiPriority; label: string }> = [
  { value: "undefined", label: "Undefined" },
  { value: "low", label: "Low" },
  { value: "normal", label: "Normal" },
  { value: "high", label: "High" },
];

export default function MsiPanel({
  pciDevices,
  pciLoading,
  pciApplying,
  onRefreshPci,
  onApplyMsiBatch,
}: MsiPanelProps) {
  const [msiDraft, setMsiDraft] = useState<MsiDraft>({});
  const showSupportedOnly = useAppStore((state) => state.msiShowSupportedOnly);
  const setShowSupportedOnly = useAppStore((state) => state.setMsiShowSupportedOnly);

  useEffect(() => {
    const next: MsiDraft = {};
    for (const device of pciDevices) {
      next[device.deviceId] = {
        enable: device.msiEnabled,
        priority: device.priority,
      };
    }
    setMsiDraft(next);
  }, [pciDevices]);

  const pendingMsiChanges = useMemo(() => {
    const updates: Array<{ deviceId: string; enable: boolean; priority: MsiPriority }> = [];
    for (const device of pciDevices) {
      const draft = msiDraft[device.deviceId];
      if (!draft) continue;
      if (draft.enable !== device.msiEnabled || draft.priority !== device.priority) {
        updates.push({ deviceId: device.deviceId, enable: draft.enable, priority: draft.priority });
      }
    }
    return updates;
  }, [msiDraft, pciDevices]);

  const visibleDevices = useMemo(
    () => (showSupportedOnly ? pciDevices.filter((device) => device.msiSupported) : pciDevices),
    [pciDevices, showSupportedOnly],
  );

  return (
    <div className="mt-4">
      <div className="mb-2 flex items-center gap-2">
        <div>
          <div className="flex items-center gap-2">
            <h3 className="text-sm font-semibold text-zinc-100">MSI Utility</h3>
            <InfoTooltip translationKey="msi_utility" />
          </div>
          <p className="text-xs text-zinc-400">PCI MSI mode and interrupt priority controls.</p>
        </div>
        <button className="btn-ghost ml-auto px-3 py-1.5 text-xs" onClick={onRefreshPci}>
          Refresh
        </button>
        <label className="inline-flex items-center gap-2 rounded-md border border-zinc-800 bg-zinc-900 px-2 py-1 text-xs text-zinc-400">
          <input
            type="checkbox"
            className="h-4 w-4 accent-zinc-200"
            checked={showSupportedOnly}
            onChange={(event) => setShowSupportedOnly(event.target.checked)}
          />
          Supported only
        </label>
        <button
          className="btn-primary px-3 py-1.5 text-xs disabled:cursor-not-allowed disabled:opacity-50"
          disabled={pciApplying || pendingMsiChanges.length === 0}
          onClick={() => onApplyMsiBatch(pendingMsiChanges)}
        >
          {pciApplying ? "Applying..." : `Apply (${pendingMsiChanges.length})`}
        </button>
      </div>

      <div className="overflow-x-auto rounded-xl border border-zinc-800">
        <table className="min-w-full text-left text-sm">
          <thead className="bg-zinc-900 text-xs text-zinc-400">
            <tr>
              <th className="px-3 py-2">Device</th>
              <th className="px-3 py-2">Supported</th>
              <th className="px-3 py-2">MSI</th>
              <th className="px-3 py-2">
                <span className="inline-flex items-center gap-2">
                  <span>Priority</span>
                  <InfoTooltip translationKey="msi_priority" />
                </span>
              </th>
            </tr>
          </thead>
          <tbody>
            {pciLoading ? (
              <tr>
                <td className="px-3 py-3 text-zinc-400" colSpan={4}>
                  Loading PCI devices...
                </td>
              </tr>
            ) : visibleDevices.length === 0 ? (
              <tr>
                <td className="px-3 py-3 text-zinc-400" colSpan={4}>
                  {showSupportedOnly ? "No MSI-supported PCI devices found." : "No PCI devices found."}
                </td>
              </tr>
            ) : (
              visibleDevices.map((device) => {
                const draft = msiDraft[device.deviceId] ?? {
                  enable: device.msiEnabled,
                  priority: device.priority,
                };
                return (
                  <tr key={device.deviceId} className="border-t border-zinc-800 bg-zinc-900">
                    <td className="px-3 py-2 text-zinc-100">{device.displayName}</td>
                    <td className="px-3 py-2 text-zinc-400">
                      {!device.readable ? "Unverifiable" : device.msiSupported ? "MSI" : "Legacy"}
                    </td>
                    <td className="px-3 py-2">
                      <input
                        type="checkbox"
                        className="h-4 w-4 accent-zinc-200"
                        disabled={!device.readable || !device.msiSupported}
                        checked={draft.enable}
                        onChange={(event) =>
                          setMsiDraft((prev) => ({
                            ...prev,
                            [device.deviceId]: {
                              ...draft,
                              enable: event.target.checked,
                            },
                          }))
                        }
                      />
                    </td>
                    <td className="px-3 py-2">
                      <select
                        className="select min-w-32"
                        disabled={!device.readable}
                        value={draft.priority}
                        onChange={(event) =>
                          setMsiDraft((prev) => ({
                            ...prev,
                            [device.deviceId]: {
                              ...draft,
                              priority: event.target.value as MsiPriority,
                            },
                          }))
                        }
                      >
                        {PRIORITY_OPTIONS.map((option) => (
                          <option key={option.value} value={option.value}>
                            {option.label}
                          </option>
                        ))}
                      </select>
                    </td>
                  </tr>
                );
              })
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
