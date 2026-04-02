import { useMemo, useState } from "react";
import { HardDrive } from "lucide-react";
import { useAppStore } from "../../store/appStore";
import { Driver } from "../../types/hardware";
import HardwareActionModal from "../HardwareActionModal";
import InfoTooltip from "../ui/InfoTooltip";

type DriverStorePanelProps = {
  drivers: Driver[];
  driversLoading: boolean;
  driverDeleting: boolean;
  onRefreshDrivers: (silent?: boolean) => void;
  onDeleteDriver: (publishedName: string, force: boolean) => void;
};

export default function DriverStorePanel({
  drivers,
  driversLoading,
  driverDeleting,
  onRefreshDrivers,
  onDeleteDriver,
}: DriverStorePanelProps) {
  const scanMode = useAppStore((state) => state.driverScanMode);
  const setScanMode = useAppStore((state) => state.setDriverScanMode);
  const [pendingDriverDelete, setPendingDriverDelete] = useState<Driver | null>(null);

  const resolveSafetyLevel = (driver: Driver): "Critical" | "Junk" | "Caution" => {
    const normalized = (driver.safetyLevel ?? "").trim().toLowerCase();
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

  const visibleDrivers = useMemo(() => {
    if (scanMode !== "old") {
      return drivers;
    }

    const buckets = new Map<string, Driver[]>();
    for (const driver of drivers) {
      const key =
        (driver.originalName || driver.publishedName || "").trim().toLowerCase() || driver.publishedName;
      const bucket = buckets.get(key);
      if (bucket) {
        bucket.push(driver);
      } else {
        buckets.set(key, [driver]);
      }
    }

    const parseDateValue = (value: string) => {
      const timestamp = Date.parse(value);
      return Number.isNaN(timestamp) ? 0 : timestamp;
    };
    const parseVersionValue = (value: string) =>
      value
        .split(/[^0-9]+/)
        .filter(Boolean)
        .map((part) => Number.parseInt(part, 10) || 0);

    const compareVersion = (a: string, b: string) => {
      const left = parseVersionValue(a);
      const right = parseVersionValue(b);
      const max = Math.max(left.length, right.length);
      for (let index = 0; index < max; index += 1) {
        const l = left[index] ?? 0;
        const r = right[index] ?? 0;
        if (l !== r) {
          return r - l;
        }
      }
      return 0;
    };

    const obsolete: Driver[] = [];
    for (const [, bucket] of buckets) {
      if (bucket.length <= 1) {
        continue;
      }

      const sorted = [...bucket].sort((a, b) => {
        const versionCmp = compareVersion(a.driverVersion, b.driverVersion);
        if (versionCmp !== 0) {
          return versionCmp;
        }
        return parseDateValue(b.driverDate) - parseDateValue(a.driverDate);
      });
      obsolete.push(...sorted.slice(1));
    }

    return obsolete;
  }, [drivers, scanMode]);

  const startScan = async (mode: "old" | "all") => {
    if (mode === "all") {
      const confirmed = window.confirm(
        "Show all installed drivers? This list may be large and includes system-critical entries.",
      );
      if (!confirmed) {
        return;
      }
    }

    setScanMode(mode);
    void onRefreshDrivers(false);
  };

  const resetScan = () => {
    setScanMode("idle");
  };

  return (
    <div className="mt-4">
      {scanMode === "idle" ? (
        <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-6">
          <div className="flex flex-col items-center justify-center py-12 text-center">
            <HardDrive className="h-8 w-8 text-zinc-400" />
            <div className="mt-4 flex items-center gap-2">
              <p className="text-sm text-zinc-400">Driver Store</p>
              <InfoTooltip translationKey="driver_store" />
            </div>
            <p className="mt-2 text-sm text-zinc-400">Driver Store scan is manual</p>
            <p className="mt-1 max-w-xl text-xs text-zinc-400">
              Scan your system for obsolete or duplicate drivers, or open the full driver list with caution.
            </p>
            <div className="mt-5 flex flex-wrap items-center justify-center gap-4">
              <button className="btn-primary px-3 py-1.5 text-xs" onClick={() => void startScan("old")}>
                Scan Obsolete/Duplicate Drivers
              </button>
              <button
                className="rounded-md border border-rose-500/30 bg-rose-500/10 px-3 py-1.5 text-xs text-rose-500 transition hover:bg-rose-500/10"
                onClick={() => void startScan("all")}
              >
                Show All Drivers (Warning)
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {scanMode !== "idle" ? (
        <div className="mb-2 flex items-center gap-2">
          <div className="flex items-center gap-2">
            <h3 className="text-sm font-semibold text-zinc-100">Driver Store</h3>
            <InfoTooltip translationKey="driver_store" />
          </div>
          <p className="text-xs text-zinc-400">
            {scanMode === "old"
              ? "Showing likely obsolete/duplicate drivers."
              : "Showing all installed drivers."}
          </p>
          <button className="btn-ghost ml-auto px-3 py-1.5 text-xs" onClick={() => void onRefreshDrivers(false)}>
            Refresh
          </button>
          <button className="btn-ghost px-3 py-1.5 text-xs" onClick={resetScan}>
            Reset
          </button>
        </div>
      ) : null}

      {scanMode !== "idle" ? (
        <div className="overflow-x-auto rounded-xl border border-zinc-800">
          <table className="min-w-full text-left text-sm">
            <thead className="bg-zinc-900 text-xs text-zinc-400">
              <tr>
                <th className="px-3 py-2">INF</th>
                <th className="px-3 py-2">Provider</th>
                <th className="px-3 py-2">Class</th>
                <th className="px-3 py-2">Version</th>
                <th className="px-3 py-2">Date</th>
                <th className="px-3 py-2">Safety</th>
                <th className="px-3 py-2" />
              </tr>
            </thead>
            <tbody>
              {driversLoading ? (
                <tr>
                  <td className="px-3 py-3 text-zinc-400" colSpan={7}>
                    Loading installed drivers...
                  </td>
                </tr>
              ) : visibleDrivers.length === 0 ? (
                <tr>
                  <td className="px-3 py-3 text-zinc-400" colSpan={7}>
                    {scanMode === "old" ? "No obsolete/duplicate drivers found." : "No drivers found."}
                  </td>
                </tr>
              ) : (
                visibleDrivers.map((driver) => {
                  const safetyLevel = resolveSafetyLevel(driver);
                  const deleteDisabled = driverDeleting || safetyLevel === "Critical";

                  return (
                    <tr key={driver.publishedName} className="border-t border-zinc-800 bg-zinc-900">
                      <td className="px-3 py-2 text-zinc-100">{driver.publishedName}</td>
                      <td className="px-3 py-2 text-zinc-400">{driver.providerName || "-"}</td>
                      <td className="px-3 py-2 text-zinc-400">{driver.className || "-"}</td>
                      <td className="px-3 py-2 text-zinc-400">{driver.driverVersion || "-"}</td>
                      <td className="px-3 py-2 text-zinc-400">{driver.driverDate || "-"}</td>
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
                          disabled={deleteDisabled}
                          onClick={() => setPendingDriverDelete(driver)}
                          title={safetyLevel === "Critical" ? "Critical driver is protected from deletion." : undefined}
                        >
                          Delete
                        </button>
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      ) : null}

      <HardwareActionModal
        isOpen={pendingDriverDelete !== null}
        title="Delete Driver Package"
        warningText={`You are about to remove ${
          pendingDriverDelete?.publishedName ?? "this driver package"
        }. Force delete may impact active hardware if the driver is currently in use.`}
        onClose={() => setPendingDriverDelete(null)}
        onConfirm={(force) => {
          if (!pendingDriverDelete) {
            return;
          }
          onDeleteDriver(pendingDriverDelete.publishedName, force);
          setPendingDriverDelete(null);
        }}
      />
    </div>
  );
}
