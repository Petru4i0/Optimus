import { ReactNode } from "react";
import AppIcon from "../components/AppIcon";
import AppPickerDropdown, { AppPickerOption } from "../components/AppPickerDropdown";
import PrioritySelect from "../components/PrioritySelect";
import InfoTooltip from "../components/ui/InfoTooltip";
import { useAppStore } from "../store/appStore";
import { TriggerMapping } from "../types/config";
import { PriorityClass, PriorityOption } from "../types/process";

export type SettingsViewProps = {
  savedConfigsSection: ReactNode;
  builderName: string;
  setBuilderName: (value: string) => void;
  builderTargetApp: string;
  setBuilderTargetApp: (value: string) => void;
  builderTargetPriority: PriorityClass;
  setBuilderTargetPriority: (value: PriorityClass) => void;
  builderTargets: Record<string, PriorityClass>;
  onAddBuilderTarget: () => void;
  onRemoveBuilderTarget: (appName: string) => void;
  onSaveConfig: () => Promise<void>;
  appPickerOptions: AppPickerOption[];
  priorityOptions: PriorityOption[];
  watchTriggerApp: string;
  setWatchTriggerApp: (value: string) => void;
  watchConfigName: string;
  setWatchConfigName: (value: string) => void;
  configNames: string[];
  sortedMappings: Array<[string, TriggerMapping]>;
  equalsIgnoreCase: (a: string, b: string) => boolean;
  groupAppNames: string[];
  onAddWatchMapping: () => Promise<void>;
  onRemoveWatchMapping: (appName: string) => Promise<void>;
  watchdogEnabled: boolean;
  autostartEnabled: boolean;
  alwaysRunAsAdmin: boolean;
  minimizeToTrayEnabled: boolean;
  watchdogPending: boolean;
  autostartPending: boolean;
  alwaysRunAsAdminPending: boolean;
  minimizeToTrayPending: boolean;
  onToggleWatchdog: (enabled: boolean) => Promise<void>;
  onToggleAutostart: (enabled: boolean) => Promise<void>;
  onToggleAlwaysRunAsAdmin: (enabled: boolean) => Promise<void>;
  onToggleMinimizeToTray: (enabled: boolean) => Promise<void>;
};

function renderToggleSwitch(
  checked: boolean,
  onToggle: (next: boolean) => void,
  ariaLabel: string,
  disabled = false,
) {
  return (
    <label
      className={`relative inline-flex h-6 w-11 items-center ${disabled ? "cursor-not-allowed opacity-60" : "cursor-pointer"}`}
    >
      <input
        type="checkbox"
        className="peer sr-only"
        checked={checked}
        disabled={disabled}
        onChange={(event) => onToggle(event.target.checked)}
        aria-label={ariaLabel}
      />
      <span className="absolute inset-0 rounded-full border border-zinc-800 bg-zinc-800 transition-colors peer-checked:border-zinc-500 peer-checked:bg-zinc-700" />
      <span className="absolute left-[2px] top-[2px] h-5 w-5 rounded-full bg-zinc-300 transition-transform peer-checked:translate-x-5 peer-checked:bg-zinc-200" />
    </label>
  );
}

function PillBadge({ children, tone = "neutral" }: { children: ReactNode; tone?: "neutral" | "red" }) {
  return (
    <span
      className={`rounded px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.18em] ${
        tone === "red"
          ? "bg-rose-500/10 text-rose-500"
          : "bg-zinc-700 text-zinc-400"
      }`}
    >
      {children}
    </span>
  );
}

function EngineRow({
  title,
  subtitle,
  badge,
  children,
  inset = false,
}: {
  title: ReactNode;
  subtitle?: ReactNode;
  badge?: ReactNode;
  children: ReactNode;
  inset?: boolean;
}) {
  return (
    <div className={`flex items-center justify-between gap-4 px-3 py-3 ${inset ? "pl-6" : ""}`}>
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-sm text-zinc-100">{title}</span>
          {badge}
        </div>
        {subtitle ? <p className="mt-1 text-xs text-zinc-400">{subtitle}</p> : null}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

export default function SettingsView({
  savedConfigsSection,
  builderName,
  setBuilderName,
  builderTargetApp,
  setBuilderTargetApp,
  builderTargetPriority,
  setBuilderTargetPriority,
  builderTargets,
  onAddBuilderTarget,
  onRemoveBuilderTarget,
  onSaveConfig,
  appPickerOptions,
  priorityOptions,
  watchTriggerApp,
  setWatchTriggerApp,
  watchConfigName,
  setWatchConfigName,
  configNames,
  sortedMappings,
  equalsIgnoreCase,
  groupAppNames,
  onAddWatchMapping,
  onRemoveWatchMapping,
  watchdogEnabled,
  autostartEnabled,
  alwaysRunAsAdmin,
  minimizeToTrayEnabled,
  watchdogPending,
  autostartPending,
  alwaysRunAsAdminPending,
  minimizeToTrayPending,
  onToggleWatchdog,
  onToggleAutostart,
  onToggleAlwaysRunAsAdmin,
  onToggleMinimizeToTray,
}: SettingsViewProps) {
  const locale = useAppStore((state) => state.locale);
  const setLocale = useAppStore((state) => state.setLocale);
  const alwaysRunAsAdminLabel =
    locale === "ru" ? "Всегда запускать от Администратора" : "Always Run as Administrator";
  const alwaysRunAsAdminDescription =
    locale === "ru"
      ? "Заставляет Windows запрашивать права при каждом запуске."
      : "Forces Windows to prompt for UAC elevation every time the app is launched.";

  return (
    <div className="grid grid-cols-1 gap-6 xl:grid-cols-3">
      <div className="space-y-4 xl:col-span-2">
        {savedConfigsSection}

        <section className="glass-card rounded-2xl p-4">
          <div>
            <h2 className="text-lg font-semibold text-zinc-100">Create Config</h2>
            <p className="mt-1 text-xs text-zinc-400">
              Build reusable priority maps by application name. Stored targets stay PID-agnostic.
            </p>
          </div>

          <div className="mt-4 space-y-4">
            <div>
              <label className="mb-1 block text-xs text-zinc-400">Config Name</label>
              <input
                className="w-full rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-2 text-sm text-zinc-100 outline-none transition focus:border-zinc-500"
                placeholder="Gaming / Work / Streaming"
                value={builderName}
                onChange={(event) => setBuilderName(event.target.value)}
              />
            </div>

            <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-3">
              <div className="grid grid-cols-1 gap-3 lg:grid-cols-[minmax(0,2fr)_220px_auto] lg:items-end">
                <div className="min-w-0">
                  <label className="mb-1 block text-xs text-zinc-400">Target App</label>
                  <AppPickerDropdown
                    options={appPickerOptions}
                    value={builderTargetApp}
                    onChange={setBuilderTargetApp}
                    placeholder="Select running app..."
                  />
                </div>

                <div className="w-full lg:w-[220px]">
                  <label className="mb-1 block text-xs text-zinc-400">Priority</label>
                  <PrioritySelect
                    className="h-[42px] w-full"
                    options={priorityOptions}
                    value={builderTargetPriority}
                    onChange={setBuilderTargetPriority}
                  />
                </div>

                <button className="btn-primary h-[42px] w-full px-4 lg:w-auto" onClick={onAddBuilderTarget}>
                  Add Target
                </button>
              </div>
            </div>

            <div className="rounded-xl border border-zinc-800 bg-zinc-950/50 p-3">
              <div className="mb-2 flex items-center justify-between gap-2">
                <span className="text-xs font-semibold uppercase tracking-[0.16em] text-zinc-400">Staging Buffer</span>
                <span className="text-xs text-zinc-400">{Object.keys(builderTargets).length} target{Object.keys(builderTargets).length === 1 ? "" : "s"}</span>
              </div>
              {Object.keys(builderTargets).length === 0 ? (
                <div className="flex min-h-[128px] items-center justify-center rounded-lg border border-zinc-800 bg-zinc-950/40 px-3 py-4 text-center text-sm text-zinc-400">
                  No staged targets yet.
                </div>
              ) : (
                <div className="space-y-2 rounded-lg border border-zinc-800 bg-zinc-950/40 p-2">
                  {Object.entries(builderTargets).map(([appName, priority]) => (
                    <div
                      key={appName}
                      className="flex flex-wrap items-center gap-2 rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2"
                    >
                      <span className="text-sm text-zinc-100">{appName}</span>
                      <span className="ml-auto rounded border border-zinc-800 px-2 py-1 text-xs text-zinc-400">
                        {priorityOptions.find((item) => item.value === priority)?.label ?? priority}
                      </span>
                      <button className="btn-danger px-2 py-1 text-xs" onClick={() => onRemoveBuilderTarget(appName)}>
                        Remove
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>

            <div className="pt-1">
              <button
                className="w-full rounded-xl border border-zinc-500 bg-zinc-100 px-4 py-2.5 text-sm font-semibold text-zinc-950 transition hover:border-zinc-500 hover:bg-zinc-200"
                onClick={() => void onSaveConfig()}
              >
                Save Config
              </button>
            </div>
          </div>
        </section>

        <section className="glass-card rounded-2xl p-4">
          <div className="flex flex-wrap items-end justify-between gap-3">
            <div>
              <h2 className="text-lg font-semibold text-zinc-100">Application Triggers</h2>
              <p className="mt-1 text-xs text-zinc-400">
                Bind running applications to profiles for automatic enforcement.
              </p>
            </div>
          </div>

          <div className="mt-4 grid grid-cols-1 gap-3 md:grid-cols-[1.5fr_1fr_auto] md:items-end">
            <div className="min-w-0">
              <label className="mb-1 block text-xs text-zinc-400">Trigger App</label>
              <AppPickerDropdown
                options={appPickerOptions}
                value={watchTriggerApp}
                onChange={setWatchTriggerApp}
                placeholder="Select running app..."
              />
            </div>
            <div>
              <label className="mb-1 block text-xs text-zinc-400">Config</label>
              <select
                className="select w-full"
                value={watchConfigName}
                onChange={(event) => setWatchConfigName(event.target.value)}
              >
                <option value="">Select config...</option>
                {configNames.map((configName) => (
                  <option key={configName} value={configName}>
                    {configName}
                  </option>
                ))}
              </select>
            </div>
            <button className="btn-primary w-full md:w-auto" onClick={() => void onAddWatchMapping()}>
              Add Mapping
            </button>
          </div>

          <div className="mt-4 space-y-2">
            {sortedMappings.length === 0 ? (
              <div className="rounded-xl border border-dashed border-zinc-800 bg-zinc-900 px-3 py-3 text-sm text-zinc-400">
                No mappings configured.
              </div>
            ) : (
              sortedMappings.map(([appName, mapping]) => {
                const configName = mapping.configName;
                const displayName =
                  groupAppNames.find((groupName) => equalsIgnoreCase(groupName, appName)) ?? appName;
                return (
                  <div
                    key={appName}
                    className="flex flex-wrap items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-2"
                  >
                    {mapping.icon ? (
                      <img src={mapping.icon} alt="" className="mr-1 h-4 w-4 object-contain" />
                    ) : (
                      <AppIcon appName={displayName} className="h-4 w-4" />
                    )}
                    <span className="font-mono text-sm text-zinc-100">{displayName}</span>
                    <span className="text-zinc-400">{"->"}</span>
                    <span className="text-sm text-zinc-400">{configName}</span>
                    <button
                      className="btn-danger ml-auto px-2 py-1 text-xs"
                      onClick={() => void onRemoveWatchMapping(appName)}
                    >
                      Remove
                    </button>
                  </div>
                );
              })
            )}
          </div>
        </section>
      </div>

      <div className="xl:col-span-1">
        <section className="glass-card rounded-2xl p-4 xl:sticky xl:top-6">
          <div className="flex items-center gap-3">
            <div>
              <div className="flex items-center gap-2">
                <h2 className="text-lg font-semibold text-zinc-100">Core Engine</h2>
                {watchdogEnabled ? <span className="relative flex h-2.5 w-2.5"><span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-500/30" /><span className="relative inline-flex h-2.5 w-2.5 rounded-full bg-emerald-500" /></span> : null}
              </div>
              <p className="mt-1 text-xs text-zinc-400">Runtime behavior, elevation, and shell integration.</p>
            </div>
          </div>

          <div className="mt-4 overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900 divide-y divide-zinc-800">
            <EngineRow
              title="Interface Language"
              subtitle="Client-side locale for descriptions and helper text."
            >
              <div className="inline-flex rounded-xl border border-zinc-800 bg-zinc-900 p-1">
                {(["en", "ru"] as const).map((value) => {
                  const active = locale === value;
                  return (
                    <button
                      key={value}
                      type="button"
                      className={`rounded-lg px-3 py-1.5 text-xs font-semibold tracking-wide transition ${
                        active
                          ? "bg-zinc-200 text-zinc-950 shadow-[0_0_0_1px_rgba(255,255,255,0.08)]"
                          : "text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100"
                      }`}
                      onClick={() => setLocale(value)}
                    >
                      {value.toUpperCase()}
                    </button>
                  );
                })}
              </div>
            </EngineRow>

            <EngineRow
              title={
                <span className="flex items-center gap-2">
                  <span>Background Service</span>
                  <PillBadge>CORE</PillBadge>
                  <InfoTooltip translationKey="watchdog" />
                </span>
              }
              subtitle="Lightweight reconcile loop that keeps protected settings from drifting."
            >
              {renderToggleSwitch(
                watchdogEnabled,
                (next) => {
                  void onToggleWatchdog(next);
                },
                "Toggle background service",
                watchdogPending,
              )}
            </EngineRow>

            <EngineRow
              title={
                <span className="flex items-center gap-2">
                  <span>Start with Windows (Elevated)</span>
                  <PillBadge tone="red">ELEVATED</PillBadge>
                </span>
              }
              subtitle="Automatically launch Optimus with Administrator privileges on user sign-in via Task Scheduler (Bypasses UAC)."
            >
              {renderToggleSwitch(
                autostartEnabled,
                (next) => {
                  void onToggleAutostart(next);
                },
                "Toggle start with Windows elevated",
                autostartPending,
              )}
            </EngineRow>

            <EngineRow
              title={alwaysRunAsAdminLabel}
              subtitle={alwaysRunAsAdminDescription}
            >
              {renderToggleSwitch(
                alwaysRunAsAdmin,
                (next) => {
                  void onToggleAlwaysRunAsAdmin(next);
                },
                "Toggle always run as administrator",
                alwaysRunAsAdminPending,
              )}
            </EngineRow>

            <EngineRow
              title="Minimize to Tray on Close"
              subtitle="Keep the app resident instead of closing the process window."
            >
              {renderToggleSwitch(
                minimizeToTrayEnabled,
                (next) => {
                  void onToggleMinimizeToTray(next);
                },
                "Toggle minimize to tray on close",
                minimizeToTrayPending,
              )}
            </EngineRow>
          </div>
        </section>
      </div>
    </div>
  );
}
