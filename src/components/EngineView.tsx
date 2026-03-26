type EngineViewProps = {
  timerEnabled: boolean;
  timerCurrentMs: number | null;
  timerBusy: boolean;
  onTimerToggle: (enabled: boolean) => void;
  masterEnabled: boolean;
  standbyListMb: number;
  freeMemoryMb: number;
  totalMemoryMb: number;
  enableStandbyTrigger: boolean;
  standbyLimitMb: number;
  enableFreeMemoryTrigger: boolean;
  freeMemoryLimitMb: number;
  totalPurges: number;
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

export default function EngineView({
  timerEnabled,
  timerCurrentMs,
  timerBusy,
  onTimerToggle,
  masterEnabled,
  standbyListMb,
  freeMemoryMb,
  totalMemoryMb,
  enableStandbyTrigger,
  standbyLimitMb,
  enableFreeMemoryTrigger,
  freeMemoryLimitMb,
  totalPurges,
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
}: EngineViewProps) {
  const controlsBusy = configBusy || purgeBusy;

  return (
    <div className="max-w-xl space-y-4">
      <section className="glass-card rounded-2xl p-5">
        <div className="flex flex-wrap items-center gap-3">
          <div>
            <h2 className="text-lg font-semibold text-zinc-100">Turbo Timer (Instant 0.5ms)</h2>
            <p className="mt-1 text-xs text-zinc-400">
              One-click latency forcing with fixed minimum timer target.
            </p>
          </div>

          <div className="ml-auto flex items-center gap-2 rounded-xl border border-zinc-700 bg-zinc-900/55 px-3 py-2">
            <span className="text-sm text-zinc-200">{timerEnabled ? "Enabled" : "Disabled"}</span>
            <label className="relative inline-flex h-6 w-11 cursor-pointer items-center">
              <input
                type="checkbox"
                className="peer sr-only"
                checked={timerEnabled}
                disabled={timerBusy}
                onChange={(event) => onTimerToggle(event.target.checked)}
                aria-label="Toggle Turbo Timer"
              />
              <span className="absolute inset-0 rounded-full border border-zinc-600 bg-zinc-800/80 transition-colors peer-checked:border-zinc-300 peer-checked:bg-zinc-200/20 peer-disabled:cursor-not-allowed peer-disabled:opacity-50" />
              <span className="absolute left-[2px] top-[2px] h-5 w-5 rounded-full bg-zinc-300 transition-transform peer-checked:translate-x-5 peer-checked:bg-white peer-disabled:opacity-50" />
            </label>
          </div>
        </div>

        <div className="mt-4 rounded-xl border border-zinc-700 bg-zinc-900/55 p-4">
          <p className="text-xs uppercase tracking-wide text-zinc-500">Live Latency Status</p>
          <p className="mt-2 text-2xl font-semibold text-zinc-100">
            {timerCurrentMs === null ? "--" : timerCurrentMs.toFixed(3)} ms
          </p>
          <p className="mt-2 text-xs text-zinc-400">Minimum Support: 0.500 ms</p>
          <p className="mt-1 text-xs text-zinc-500">Target: 0.500 ms</p>
        </div>
      </section>

      <section className="glass-card rounded-2xl p-5">
        <div className="flex flex-wrap items-center gap-3">
          <div>
            <h2 className="text-lg font-semibold text-zinc-100">Memory Purge Engine</h2>
            <p className="mt-1 text-xs text-zinc-400">
              Standby list cleaner with independent trigger conditions.
            </p>
          </div>

          <div className="ml-auto flex items-center gap-2 rounded-xl border border-zinc-700 bg-zinc-900/55 px-3 py-2">
            <span className="text-sm text-zinc-200">{masterEnabled ? "Enabled" : "Disabled"}</span>
            <label className="relative inline-flex h-6 w-11 cursor-pointer items-center">
              <input
                type="checkbox"
                className="peer sr-only"
                checked={masterEnabled}
                disabled={controlsBusy}
                onChange={(event) => onMasterToggle(event.target.checked)}
                aria-label="Toggle Memory Purge Engine"
              />
              <span className="absolute inset-0 rounded-full border border-zinc-600 bg-zinc-800/80 transition-colors peer-checked:border-zinc-300 peer-checked:bg-zinc-200/20 peer-disabled:cursor-not-allowed peer-disabled:opacity-50" />
              <span className="absolute left-[2px] top-[2px] h-5 w-5 rounded-full bg-zinc-300 transition-transform peer-checked:translate-x-5 peer-checked:bg-white peer-disabled:opacity-50" />
            </label>
          </div>
        </div>

        <div className="mt-4 grid gap-3 sm:grid-cols-3">
          <div className="rounded-xl border border-zinc-700 bg-zinc-900/55 px-3 py-3">
            <p className="text-xs text-zinc-500">Total Memory</p>
            <p className="mt-1 text-xl font-semibold text-zinc-100">{totalMemoryMb.toLocaleString()} MB</p>
          </div>
          <div className="rounded-xl border border-zinc-700 bg-zinc-900/55 px-3 py-3">
            <p className="text-xs text-zinc-500">Standby List</p>
            <p className="mt-1 text-xl font-semibold text-zinc-100">{standbyListMb.toLocaleString()} MB</p>
          </div>
          <div className="rounded-xl border border-zinc-700 bg-zinc-900/55 px-3 py-3">
            <p className="text-xs text-zinc-500">Free Memory</p>
            <p className="mt-1 text-xl font-semibold text-zinc-100">{freeMemoryMb.toLocaleString()} MB</p>
          </div>
        </div>

        <div className="mt-4 space-y-3">
          <label className="flex flex-wrap items-center gap-2 rounded-xl border border-zinc-700 bg-zinc-900/45 px-3 py-2 text-sm text-zinc-200">
            <input
              type="checkbox"
              className={`h-4 w-4 cursor-pointer rounded border-zinc-700 bg-zinc-900 accent-zinc-200 text-zinc-200 focus:ring-1 focus:ring-zinc-500/40 ${
                controlsBusy || masterEnabled ? "pointer-events-none opacity-50" : ""
              }`}
              checked={enableStandbyTrigger}
              onChange={(event) => onStandbyTriggerToggle(event.target.checked)}
              tabIndex={controlsBusy || masterEnabled ? -1 : 0}
            />
            <span>Purge if Standby List {">"}</span>
            <input
              type="number"
              min={1}
              step={1}
              value={standbyLimitMb}
              disabled={controlsBusy || masterEnabled}
              onChange={(event) => onStandbyLimitChange(Number(event.target.value))}
              onBlur={onStandbyLimitBlur}
              className="w-28 rounded-lg border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm text-zinc-100 outline-none focus:border-zinc-400 focus:ring-2 focus:ring-zinc-500/40 disabled:cursor-not-allowed disabled:opacity-50"
            />
            <span>MB</span>
          </label>

          <label className="flex flex-wrap items-center gap-2 rounded-xl border border-zinc-700 bg-zinc-900/45 px-3 py-2 text-sm text-zinc-200">
            <input
              type="checkbox"
              className={`h-4 w-4 cursor-pointer rounded border-zinc-700 bg-zinc-900 accent-zinc-200 text-zinc-200 focus:ring-1 focus:ring-zinc-500/40 ${
                controlsBusy || masterEnabled ? "pointer-events-none opacity-50" : ""
              }`}
              checked={enableFreeMemoryTrigger}
              onChange={(event) => onFreeMemoryTriggerToggle(event.target.checked)}
              tabIndex={controlsBusy || masterEnabled ? -1 : 0}
            />
            <span>Purge if Free Memory {"<"}</span>
            <input
              type="number"
              min={1}
              step={1}
              value={freeMemoryLimitMb}
              disabled={controlsBusy || masterEnabled}
              onChange={(event) => onFreeMemoryLimitChange(Number(event.target.value))}
              onBlur={onFreeMemoryLimitBlur}
              className="w-28 rounded-lg border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm text-zinc-100 outline-none focus:border-zinc-400 focus:ring-2 focus:ring-zinc-500/40 disabled:cursor-not-allowed disabled:opacity-50"
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
            {purgeBusy ? "Purging..." : "Purge Now"}
          </button>
          <p className="text-sm text-zinc-300">Total Purges: {totalPurges.toLocaleString()}</p>
        </div>
      </section>
    </div>
  );
}
