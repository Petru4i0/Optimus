import { type Dispatch, type SetStateAction } from "react";
import InfoTooltip from "../components/ui/InfoTooltip";
import ModeSelector from "../components/ModeSelector";
import { Config, WatchdogConfig } from "../types/config";
import { ProcessGroupDto } from "../types/process";

function formatUpdatedAt(timestamp: number) {
  const date = new Date(timestamp * 1000);
  if (Number.isNaN(date.getTime())) {
    return "Unknown";
  }
  return date.toLocaleString();
}

type SavedConfigsSectionProps = {
  isOpen: boolean;
  setOpen: Dispatch<SetStateAction<boolean>>;
  sectionId: "home" | "settings";
  configs: Config[];
  watchdogConfig: WatchdogConfig;
  groups: ProcessGroupDto[];
  equalsIgnoreCase: (a: string, b: string) => boolean;
  onImportConfig: () => Promise<void>;
  onSetSticky: (configName: string, mode: 0 | 1 | 2) => Promise<void>;
  onApplyConfig: (config: Config) => Promise<void>;
  onExportConfig: (name: string) => Promise<void>;
  onCreateShortcut: (name: string) => Promise<void>;
  onDeleteConfig: (name: string) => Promise<void>;
};

function renderToggleSwitch(
  checked: boolean,
  onToggle: (next: boolean) => void,
  ariaLabel: string,
) {
  return (
    <label className="relative inline-flex h-6 w-11 cursor-pointer items-center">
      <input
        type="checkbox"
        className="peer sr-only"
        checked={checked}
        onChange={(event) => onToggle(event.target.checked)}
        aria-label={ariaLabel}
      />
      <span className="absolute inset-0 rounded-full border border-zinc-800 bg-zinc-800 transition-colors peer-checked:border-zinc-500 peer-checked:bg-zinc-700" />
      <span className="absolute left-[2px] top-[2px] h-5 w-5 rounded-full bg-zinc-300 transition-transform peer-checked:translate-x-5 peer-checked:bg-zinc-200" />
    </label>
  );
}

export default function SavedConfigsSection({
  isOpen,
  setOpen,
  sectionId,
  configs,
  watchdogConfig,
  groups,
  equalsIgnoreCase,
  onImportConfig,
  onSetSticky,
  onApplyConfig,
  onExportConfig,
  onCreateShortcut,
  onDeleteConfig,
}: SavedConfigsSectionProps) {
  return (
    <section className="glass-card rounded-2xl p-4">
      <div className="flex flex-wrap items-center gap-3">
        <button
          className="inline-flex h-8 w-8 items-center justify-center rounded-md border border-zinc-800 bg-zinc-900 text-zinc-100 transition hover:border-zinc-500 hover:text-zinc-100"
          onClick={() => setOpen((prev) => !prev)}
          aria-label={isOpen ? "Collapse saved configs" : "Expand saved configs"}
        >
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.9"
            className={`h-5 w-5 transition-transform duration-200 ${isOpen ? "rotate-180" : ""}`}
          >
            <path d="M6 9l6 6 6-6" />
          </svg>
        </button>

        <div>
          <h2 className="text-lg font-semibold text-zinc-100">Saved Configs</h2>
          <p className="text-xs text-zinc-400">Manage reusable app-priority configurations.</p>
        </div>

        <div className="ml-auto flex flex-wrap items-center gap-2">
          <button className="btn-primary" onClick={() => void onImportConfig()}>
            Import Config
          </button>
        </div>
      </div>

      {isOpen ? (
        <div className="mt-4 space-y-2 border-t border-zinc-800 pt-4">
          {configs.length === 0 ? (
            <div className="rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-3 text-sm text-zinc-400">
              No saved configs yet.
            </div>
          ) : (
            configs.map((config) => {
              const storedModeRaw =
                watchdogConfig.stickyModes[config.name.toLowerCase()] ??
                Object.entries(watchdogConfig.stickyModes).find(([name]) =>
                  equalsIgnoreCase(name, config.name),
                )?.[1] ??
                0;
              const liveMode: 0 | 1 | 2 =
                storedModeRaw === 2 ? 2 : storedModeRaw === 1 ? 1 : 0;
              const liveEnabled = liveMode !== 0;
              const triggerApps = Object.entries(watchdogConfig.triggerMap)
                .filter(([, mappedConfig]) => equalsIgnoreCase(mappedConfig.configName, config.name))
                .map(
                  ([appName]) =>
                    groups.find((group) => equalsIgnoreCase(group.appName, appName))?.appName ?? appName,
                );
              const triggerSummary =
                triggerApps.length > 0
                  ? `${triggerApps[0]}${triggerApps.length > 1 ? ` +${triggerApps.length - 1}` : ""}`
                  : null;
              const hasTrigger = triggerApps.length > 0;

              return (
                <div
                  key={`${sectionId}-${config.name}`}
                  className="rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-3"
                >
                  <div className="flex flex-wrap items-center gap-2">
                    <div>
                      <p className="text-sm font-semibold text-zinc-100">{config.name}</p>
                      <div className="mt-0.5 flex flex-wrap items-center gap-2">
                        <p className="text-xs text-zinc-400">
                          {Object.keys(config.configMap).length} targets • Updated {formatUpdatedAt(config.updatedAt)}
                        </p>
                        {triggerSummary ? (
                          <span className="inline-flex items-center rounded-full border border-zinc-800 bg-zinc-900 px-2 py-0.5 text-[11px] text-zinc-400">
                            Trigger: {triggerSummary}
                          </span>
                        ) : null}
                      </div>
                    </div>

                    <div className="ml-auto flex flex-wrap items-center gap-2">
                      <label className="inline-flex h-9 items-center gap-2 rounded-lg border border-zinc-800 bg-zinc-900 px-3 text-sm text-zinc-100">
                        <span>Live</span>
                        {renderToggleSwitch(
                          liveEnabled,
                          (next) => {
                            void onSetSticky(config.name, next ? 1 : 0);
                          },
                          `Toggle live mode for ${config.name}`,
                        )}
                        {liveEnabled ? (
                          <div className="flex items-center gap-2">
                            <ModeSelector
                              value={liveMode}
                              hasTrigger={hasTrigger}
                              onChange={(mode) => {
                                void onSetSticky(config.name, mode);
                              }}
                            />
                            <InfoTooltip translationKey="live_mode" />
                          </div>
                        ) : null}
                      </label>
                      {liveMode === 2 && !hasTrigger ? (
                        <span className="text-xs text-zinc-400">No trigger set</span>
                      ) : null}

                      <button className="btn-primary" onClick={() => void onApplyConfig(config)}>
                        Apply Config
                      </button>
                      <button
                        className="btn-ghost px-3 py-2 text-sm"
                        onClick={() => void onExportConfig(config.name)}
                      >
                        Export
                      </button>
                      <button
                        className="btn-ghost px-3 py-2 text-sm"
                        onClick={() => void onCreateShortcut(config.name)}
                      >
                        Shortcut
                      </button>
                      <button className="btn-danger" onClick={() => void onDeleteConfig(config.name)}>
                        Delete
                      </button>
                    </div>
                  </div>
                </div>
              );
            })
          )}
        </div>
      ) : null}
    </section>
  );
}
