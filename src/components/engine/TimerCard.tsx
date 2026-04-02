import { ChevronDown, ChevronUp, SlidersHorizontal } from "lucide-react";
import { useState } from "react";
import type { DeepPurgeConfig } from "../../store/appStore";
import InfoTooltip from "../ui/InfoTooltip";
import ToggleSwitch from "./ToggleSwitch";

type TimerCardProps = {
  timerEnabled: boolean;
  timerCurrentMs: number | null;
  timerBusy: boolean;
  deepPurgeBusy: boolean;
  deepPurgeConfig: DeepPurgeConfig;
  totalDeepPurgeCount: number;
  totalDeepPurgeBytes: number;
  onTimerToggle: (enabled: boolean) => void;
  onRunDeepPurge: (config: DeepPurgeConfig) => void;
  setDeepPurgeConfig: (key: keyof DeepPurgeConfig, value: boolean) => void;
};

export default function TimerCard({
  timerEnabled,
  timerCurrentMs,
  timerBusy,
  deepPurgeBusy,
  deepPurgeConfig,
  totalDeepPurgeCount,
  totalDeepPurgeBytes,
  onTimerToggle,
  onRunDeepPurge,
  setDeepPurgeConfig,
}: TimerCardProps) {
  const [configOpen, setConfigOpen] = useState(false);

  const formatFreed = (bytes: number) => {
    const normalized = Math.max(0, Math.floor(bytes));
    const mb = normalized / (1024 * 1024);
    if (mb >= 1024) {
      return `${(mb / 1024).toFixed(2)} GB`;
    }
    return `${mb.toFixed(0)} MB`;
  };

  return (
    <section className="glass-card rounded-2xl p-5">
      <div className="flex flex-wrap items-center gap-3">
        <div>
          <div className="flex items-center gap-2">
            <h2 className="text-lg font-semibold text-zinc-100">Timer Resolution</h2>
            <InfoTooltip translationKey="turbo_timer" />
          </div>
          <p className="mt-1 text-xs text-zinc-400">
            Applies a fixed 0.5 ms timer request for low-latency scheduling.
          </p>
        </div>

        <div className="ml-auto flex items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-2">
          <span className="text-sm text-zinc-100">{timerEnabled ? "Enabled" : "Disabled"}</span>
          <ToggleSwitch
            checked={timerEnabled}
            disabled={timerBusy}
            onChange={onTimerToggle}
            ariaLabel="Toggle Timer Resolution"
          />
        </div>
      </div>

      <div className="mt-4 rounded-xl border border-zinc-800 bg-zinc-900 p-4">
        <p className="text-xs uppercase tracking-wide text-zinc-400">Live Latency Status</p>
        <p className="mt-2 text-2xl font-semibold text-zinc-100">
          {timerCurrentMs === null ? "--" : timerCurrentMs.toFixed(3)} ms
        </p>
        <p className="mt-2 text-xs text-zinc-400">Minimum Support: 0.500 ms</p>
        <p className="mt-1 text-xs text-zinc-400">Target: 0.500 ms</p>
      </div>

      <div className="mt-4 rounded-xl border border-zinc-800 bg-zinc-900 p-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <div className="flex items-center gap-2">
              <p className="text-sm font-semibold text-zinc-100">System Deep Purge</p>
              <InfoTooltip translationKey="purgeTooltip" />
            </div>
            <p className="mt-1 text-xs text-zinc-400">
              Clears temporary files, update cache, and selected system cleanup targets.
            </p>
          </div>
          <div className="ml-auto flex w-full max-w-[220px] flex-col">
            <button
              className="btn-primary w-full px-3 py-1.5 text-xs disabled:cursor-not-allowed disabled:opacity-50"
              onClick={() => onRunDeepPurge(deepPurgeConfig)}
              disabled={deepPurgeBusy}
            >
              Run Deep Purge
            </button>
            <button
              type="button"
              className="mt-2 inline-flex w-full items-center justify-center gap-1.5 rounded-md border border-zinc-800 bg-zinc-900/40 px-3 py-1.5 text-xs text-zinc-400 transition hover:bg-zinc-800/50 hover:text-zinc-200 disabled:cursor-not-allowed disabled:opacity-50"
              onClick={() => setConfigOpen((prev) => !prev)}
              disabled={deepPurgeBusy}
            >
              <SlidersHorizontal className="h-3.5 w-3.5" />
              Configure Targets
              {configOpen ? (
                <ChevronUp className="h-3.5 w-3.5" />
              ) : (
                <ChevronDown className="h-3.5 w-3.5" />
              )}
            </button>
          </div>
        </div>

        {configOpen ? (
          <div className="mt-3 rounded-lg border border-zinc-800 bg-zinc-950/50 p-3">
            <div className="grid gap-2 sm:grid-cols-2">
              {([
                ["windows", "Windows System & Temp"],
                ["gpu", "GPU Caches & Installers"],
                ["browsers", "Browser Media Caches"],
                ["apps", "Game Launchers & Apps"],
                ["dev", "Developer Ecosystem"],
              ] as Array<[keyof DeepPurgeConfig, string]>).map(([key, label]) => (
                <label key={key} className="flex items-center gap-2 text-sm text-zinc-200">
                  <input
                    type="checkbox"
                    checked={deepPurgeConfig[key]}
                    onChange={(event) => setDeepPurgeConfig(key, event.target.checked)}
                    disabled={deepPurgeBusy}
                    className="h-4 w-4 rounded border-zinc-700 bg-zinc-900 accent-zinc-200 focus:ring-zinc-500/40"
                  />
                  <span>{label}</span>
                </label>
              ))}
            </div>
          </div>
        ) : null}

        <div className="mt-3 flex items-center gap-4 text-sm text-zinc-400">
          <p>
            Total Purges: {totalDeepPurgeCount.toLocaleString()}
          </p>
          <p>
            Total Freed: {formatFreed(totalDeepPurgeBytes)}
          </p>
        </div>
      </div>
    </section>
  );
}
