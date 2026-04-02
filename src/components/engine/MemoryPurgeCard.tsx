import InfoTooltip from "../ui/InfoTooltip";
import ToggleSwitch from "./ToggleSwitch";

type MemoryPurgeCardProps = {
  masterEnabled: boolean;
  standbyListMb: number;
  freeMemoryMb: number;
  totalMemoryMb: number;
  enableStandbyTrigger: boolean;
  standbyLimitMb: number;
  enableFreeMemoryTrigger: boolean;
  freeMemoryLimitMb: number;
  totalPurges: number;
  totalRamClearedMb: number;
  configBusy: boolean;
  purgeBusy: boolean;
  onMasterToggle: (enabled: boolean) => void;
  onStandbyTriggerToggle: (enabled: boolean) => void;
  onStandbyLimitChange: (value: number) => void;
  onStandbyLimitBlur: () => void;
  onFreeMemoryTriggerToggle: (enabled: boolean) => void;
  onFreeMemoryLimitChange: (value: number) => void;
  onFreeMemoryLimitBlur: () => void;
  onPurgeNow: () => void;
};

export default function MemoryPurgeCard({
  masterEnabled,
  standbyListMb,
  freeMemoryMb,
  totalMemoryMb,
  enableStandbyTrigger,
  standbyLimitMb,
  enableFreeMemoryTrigger,
  freeMemoryLimitMb,
  totalPurges,
  totalRamClearedMb,
  configBusy,
  purgeBusy,
  onMasterToggle,
  onStandbyTriggerToggle,
  onStandbyLimitChange,
  onStandbyLimitBlur,
  onFreeMemoryTriggerToggle,
  onFreeMemoryLimitChange,
  onFreeMemoryLimitBlur,
  onPurgeNow,
}: MemoryPurgeCardProps) {
  const controlsBusy = configBusy || purgeBusy;
  const ramFreedLabel =
    totalRamClearedMb >= 1024
      ? `${(totalRamClearedMb / 1024).toFixed(1)} GB`
      : `${totalRamClearedMb.toLocaleString()} MB`;

  return (
    <section className="glass-card rounded-2xl p-5">
      <div className="flex flex-wrap items-center gap-3">
        <div>
          <div className="flex items-center gap-2">
            <h2 className="text-lg font-semibold text-zinc-100">Standby List</h2>
            <InfoTooltip translationKey="memory_purge" />
          </div>
          <p className="mt-1 text-xs text-zinc-400">Standby list cleanup with independent trigger conditions.</p>
        </div>

        <div className="ml-auto flex items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-2">
          <span className="text-sm text-zinc-100">{masterEnabled ? "Enabled" : "Disabled"}</span>
          <ToggleSwitch
            checked={masterEnabled}
            disabled={controlsBusy}
            onChange={onMasterToggle}
            ariaLabel="Toggle Standby List"
          />
        </div>
      </div>

      <div className="mt-4 grid gap-3 sm:grid-cols-3">
        <div className="rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-3">
          <p className="text-xs text-zinc-400">Total Memory</p>
          <p className="mt-1 text-xl font-semibold text-zinc-100">{totalMemoryMb.toLocaleString()} MB</p>
        </div>
        <div className="rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-3">
          <p className="text-xs text-zinc-400">Standby List</p>
          <p className="mt-1 text-xl font-semibold text-zinc-100">{standbyListMb.toLocaleString()} MB</p>
        </div>
        <div className="rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-3">
          <p className="text-xs text-zinc-400">Free Memory</p>
          <p className="mt-1 text-xl font-semibold text-zinc-100">{freeMemoryMb.toLocaleString()} MB</p>
        </div>
      </div>

      <div className="mt-4 space-y-3">
        <label className="flex flex-wrap items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-2 text-sm text-zinc-100">
          <input
            type="checkbox"
            className={`h-4 w-4 cursor-pointer rounded border-zinc-800 bg-zinc-900 accent-zinc-200 text-zinc-100 focus:ring-1 focus:ring-zinc-500/40 ${
              controlsBusy || masterEnabled ? "pointer-events-none opacity-50" : ""
            }`}
            checked={enableStandbyTrigger}
            onChange={(event) => onStandbyTriggerToggle(event.target.checked)}
            tabIndex={controlsBusy || masterEnabled ? -1 : 0}
          />
          <span>Clean if Standby List {">"}</span>
          <InfoTooltip translationKey="auto_purge_triggers" />
          <input
            type="number"
            min={1}
            step={1}
            value={standbyLimitMb}
            disabled={controlsBusy || masterEnabled}
            onChange={(event) => onStandbyLimitChange(Number(event.target.value))}
            onBlur={onStandbyLimitBlur}
            className="w-28 rounded-lg border border-zinc-800 bg-zinc-900 px-2 py-1 text-sm text-zinc-100 outline-none focus:border-zinc-500 focus:ring-2 focus:ring-zinc-500/40 disabled:cursor-not-allowed disabled:opacity-50"
          />
          <span>MB</span>
        </label>

        <label className="flex flex-wrap items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-2 text-sm text-zinc-100">
          <input
            type="checkbox"
            className={`h-4 w-4 cursor-pointer rounded border-zinc-800 bg-zinc-900 accent-zinc-200 text-zinc-100 focus:ring-1 focus:ring-zinc-500/40 ${
              controlsBusy || masterEnabled ? "pointer-events-none opacity-50" : ""
            }`}
            checked={enableFreeMemoryTrigger}
            onChange={(event) => onFreeMemoryTriggerToggle(event.target.checked)}
            tabIndex={controlsBusy || masterEnabled ? -1 : 0}
          />
          <span>Clean if Free Memory {"<"}</span>
          <InfoTooltip translationKey="auto_purge_triggers" />
          <input
            type="number"
            min={1}
            step={1}
            value={freeMemoryLimitMb}
            disabled={controlsBusy || masterEnabled}
            onChange={(event) => onFreeMemoryLimitChange(Number(event.target.value))}
            onBlur={onFreeMemoryLimitBlur}
            className="w-28 rounded-lg border border-zinc-800 bg-zinc-900 px-2 py-1 text-sm text-zinc-100 outline-none focus:border-zinc-500 focus:ring-2 focus:ring-zinc-500/40 disabled:cursor-not-allowed disabled:opacity-50"
          />
          <span>MB</span>
        </label>
      </div>

      <div className="mt-4 flex flex-wrap items-center gap-3">
        <button
          className="btn-primary disabled:cursor-not-allowed disabled:opacity-50"
          onClick={onPurgeNow}
          disabled={purgeBusy}
        >
          {purgeBusy ? "Cleaning..." : "Run Cleanup"}
        </button>
        <div className="flex flex-wrap items-center gap-3 text-sm text-zinc-400">
          <p>Total Cleanups: {totalPurges.toLocaleString()}</p>
          <span className="h-1 w-1 rounded-full bg-zinc-500" />
          <p>RAM Freed: {ramFreedLabel}</p>
        </div>
      </div>
    </section>
  );
}
