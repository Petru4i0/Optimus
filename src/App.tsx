import { info } from "@tauri-apps/plugin-log";
import { type Dispatch, type SetStateAction, useEffect, useMemo, useState } from "react";
import AppIcon from "./components/AppIcon";
import AppPickerDropdown, { type AppPickerOption } from "./components/AppPickerDropdown";
import EngineView from "./components/EngineView";
import Layout from "./components/Layout";
import ModeSelector from "./components/ModeSelector";
import PrioritySelect from "./components/PrioritySelect";
import ProcessGroupCard from "./components/ProcessGroupCard";
import Toasts from "./components/Toasts";
import { useConfigManager } from "./hooks/useConfigManager";
import { useEngineManager } from "./hooks/useEngineManager";
import { usePollingScheduler } from "./hooks/usePollingScheduler";
import { defaultGroupPriority, useProcessManager } from "./hooks/useProcessManager";
import { useToastManager } from "./hooks/useToastManager";
import { PriorityOption } from "./types/process";

const PRIORITY_OPTIONS: PriorityOption[] = [
  { label: "Realtime", value: "realtime" },
  { label: "High", value: "high" },
  { label: "Above Normal", value: "aboveNormal" },
  { label: "Normal", value: "normal" },
  { label: "Below Normal", value: "belowNormal" },
  { label: "Low", value: "low" },
];

function formatUpdatedAt(timestamp: number) {
  const date = new Date(timestamp * 1000);
  if (Number.isNaN(date.getTime())) {
    return "Unknown";
  }
  return date.toLocaleString();
}

export default function App() {
  const [activeTab, setActiveTab] = useState<"home" | "settings" | "engine">("home");
  const [searchQuery, setSearchQuery] = useState("");

  const { toasts, pushToast, dismissToast } = useToastManager();

  const {
    groups,
    refreshing,
    error,
    needsElevation,
    isElevated,
    lastSync,
    totalProcesses,
    groupPriority,
    pidPriority,
    applyingGroup,
    endingGroup,
    applyingPid,
    killingPid,
    refreshProcesses,
    onRefreshRequested,
    onGroupPriorityChange,
    onProcessPriorityChange,
    onApplyProcess,
    onApplyGroup,
    onKillProcess,
    onEndGroup,
  } = useProcessManager(pushToast);

  const {
    watchdogEnabled,
    autostartEnabled,
    startAsAdminEnabled,
    minimizeToTrayEnabled,
    timerEnabled,
    timerCurrentMs,
    timerBusy,
    memoryStats,
    memoryPurgeConfig,
    memoryConfigBusy,
    memoryPurgeBusy,
    elevationPending,
    loadRuntimeSettings,
    loadAppSettings,
    loadTurboTimerStatus,
    refreshMemoryStats,
    onRequestElevationClick,
    onToggleWatchdog,
    onToggleAutostart,
    onToggleStartAsAdmin,
    onToggleMinimizeToTray,
    onToggleTimerResolution,
    onMemoryMasterToggle,
    onStandbyTriggerToggle,
    onStandbyLimitChange,
    onStandbyLimitBlur,
    onFreeMemoryTriggerToggle,
    onFreeMemoryLimitChange,
    onFreeMemoryLimitBlur,
    onPurgeNow,
  } = useEngineManager(pushToast, isElevated);

  const {
    configs,
    homeSavedOpen,
    settingsSavedOpen,
    setHomeSavedOpen,
    setSettingsSavedOpen,
    builderName,
    setBuilderName,
    builderTargetApp,
    setBuilderTargetApp,
    builderTargetPriority,
    setBuilderTargetPriority,
    builderTargets,
    watchTriggerApp,
    setWatchTriggerApp,
    watchConfigName,
    setWatchConfigName,
    watchdogConfig,
    loadConfigs,
    loadWatchdog,
    onAddBuilderTarget,
    onRemoveBuilderTarget,
    onSaveConfig,
    onApplyConfig,
    onDeleteConfig,
    onExportConfig,
    onImportConfig,
    onCreateShortcut,
    onSetSticky,
    onAddWatchMapping,
    onRemoveWatchMapping,
    sortedMappings,
    appIconLookup,
    equalsIgnoreCase,
  } = useConfigManager(pushToast, groups, refreshProcesses);

  const filteredGroups = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    if (!query) {
      return groups;
    }
    return groups.filter((group) => group.appName.toLowerCase().includes(query));
  }, [groups, searchQuery]);

  const appPickerOptions = useMemo<AppPickerOption[]>(
    () =>
      groups
        .map((group) => ({ appName: group.appName, iconBase64: group.iconBase64 }))
        .sort((a, b) => a.appName.localeCompare(b.appName)),
    [groups],
  );

  const pollTasks = useMemo(
    () => [
      {
        id: "processes" as const,
        intervalMs: 3000,
        run: () => refreshProcesses(true),
        critical: false,
        enabled: true,
        hiddenBehavior: "pause" as const,
      },
      {
        id: "timer" as const,
        intervalMs: 1000,
        run: () => loadTurboTimerStatus(true),
        critical: true,
        enabled: true,
        hiddenBehavior: "throttle" as const,
      },
      {
        id: "memory" as const,
        intervalMs: 2000,
        run: () => refreshMemoryStats(true),
        critical: false,
        enabled: true,
        hiddenBehavior: "throttle" as const,
      },
    ],
    [loadTurboTimerStatus, refreshMemoryStats, refreshProcesses],
  );

  usePollingScheduler({
    tasks: pollTasks,
    heartbeatMs: 500,
    hiddenThrottleMultiplier: 3,
  });

  useEffect(() => {
    void info("Optimus Frontend mounted");
    void refreshProcesses();
    void loadAppSettings(false);
    void loadTurboTimerStatus(false);
    void refreshMemoryStats(false);
    void loadConfigs().catch((e) => {
      pushToast("error", e instanceof Error ? e.message : String(e));
    });
    void loadWatchdog().catch((e) => {
      pushToast("error", e instanceof Error ? e.message : String(e));
    });
    void loadRuntimeSettings().catch((e) => {
      pushToast("error", e instanceof Error ? e.message : String(e));
    });
  }, [
    loadAppSettings,
    loadConfigs,
    loadRuntimeSettings,
    loadWatchdog,
    loadTurboTimerStatus,
    pushToast,
    refreshMemoryStats,
    refreshProcesses,
  ]);

  const renderToggleSwitch = (
    checked: boolean,
    onToggle: (next: boolean) => void,
    ariaLabel: string,
  ) => (
    <label className="relative inline-flex h-6 w-11 cursor-pointer items-center">
      <input
        type="checkbox"
        className="peer sr-only"
        checked={checked}
        onChange={(event) => onToggle(event.target.checked)}
        aria-label={ariaLabel}
      />
      <span className="absolute inset-0 rounded-full border border-zinc-600 bg-zinc-800/80 transition-colors peer-checked:border-zinc-300 peer-checked:bg-zinc-200/20" />
      <span className="absolute left-[2px] top-[2px] h-5 w-5 rounded-full bg-zinc-300 transition-transform peer-checked:translate-x-5 peer-checked:bg-white" />
    </label>
  );

  const renderSavedConfigsSection = (
    isOpen: boolean,
    setOpen: Dispatch<SetStateAction<boolean>>,
    sectionId: "home" | "settings",
  ) => (
    <section className="glass-card rounded-2xl p-4">
      <div className="flex flex-wrap items-center gap-3">
        <button
          className="inline-flex h-8 w-8 items-center justify-center rounded-md border border-zinc-700 bg-zinc-900/60 text-zinc-200 transition hover:border-zinc-400 hover:text-zinc-100"
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
        <div className="mt-4 space-y-2 border-t border-zinc-700/70 pt-4">
          {configs.length === 0 ? (
            <div className="rounded-xl border border-zinc-700 bg-zinc-900/50 px-3 py-3 text-sm text-zinc-400">
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
                .filter(([, mappedConfig]) => equalsIgnoreCase(mappedConfig, config.name))
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
                  className="rounded-xl border border-zinc-700 bg-zinc-900/55 px-3 py-3"
                >
                  <div className="flex flex-wrap items-center gap-2">
                    <div>
                      <p className="text-sm font-semibold text-zinc-100">{config.name}</p>
                      <div className="mt-0.5 flex flex-wrap items-center gap-2">
                        <p className="text-xs text-zinc-400">
                          {Object.keys(config.configMap).length} targets • Updated{" "}
                          {formatUpdatedAt(config.updatedAt)}
                        </p>
                        {triggerSummary ? (
                          <span className="inline-flex items-center rounded-full border border-zinc-600 bg-zinc-900/80 px-2 py-0.5 text-[11px] text-zinc-300">
                            Trigger: {triggerSummary}
                          </span>
                        ) : null}
                      </div>
                    </div>

                    <div className="ml-auto flex flex-wrap items-center gap-2">
                      <label className="inline-flex h-9 items-center gap-2 rounded-lg border border-zinc-600 bg-zinc-900/70 px-3 text-sm text-zinc-200">
                        <span>Live</span>
                        {renderToggleSwitch(
                          liveEnabled,
                          (next) => {
                            void onSetSticky(config.name, next ? 1 : 0);
                          },
                          `Toggle live mode for ${config.name}`,
                        )}
                        {liveEnabled ? (
                          <ModeSelector
                            value={liveMode}
                            hasTrigger={hasTrigger}
                            onChange={(mode) => {
                              void onSetSticky(config.name, mode);
                            }}
                          />
                        ) : null}
                      </label>
                      {liveMode === 2 && !hasTrigger ? (
                        <span className="text-xs text-zinc-500">No trigger set</span>
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

  return (
    <>
      <Layout
        activeTab={activeTab}
        onTabChange={setActiveTab}
        title="Optimus"
        groupsCount={groups.length}
        totalProcesses={totalProcesses}
        refreshing={refreshing}
        onRefresh={onRefreshRequested}
        isElevated={isElevated}
        needsElevation={needsElevation}
        onRequestElevation={onRequestElevationClick}
        lastSync={lastSync}
        error={error}
      >
        {activeTab === "home" ? (
          <div className="space-y-4">
            {renderSavedConfigsSection(homeSavedOpen, setHomeSavedOpen, "home")}

            <section className="glass-card rounded-2xl p-4">
              <div className="relative">
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.8"
                  className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-zinc-400"
                >
                  <circle cx="11" cy="11" r="7" />
                  <path d="M20 20l-3.5-3.5" />
                </svg>
                <input
                  className="w-full rounded-xl border border-zinc-700 bg-zinc-900/70 py-2 pl-10 pr-3 text-sm text-zinc-100 outline-none transition focus:border-zinc-400"
                  placeholder="Search running apps"
                  value={searchQuery}
                  onChange={(event) => setSearchQuery(event.target.value)}
                />
              </div>
            </section>

            <section className="space-y-3">
              {filteredGroups.length === 0 ? (
                <div className="glass-card rounded-2xl px-4 py-6 text-sm text-zinc-400">No matching processes found.</div>
              ) : (
                filteredGroups.map((group, index) => (
                  <ProcessGroupCard
                    key={group.appName}
                    group={group}
                    index={index}
                    priorities={PRIORITY_OPTIONS}
                    groupPriorityValue={groupPriority[group.appName] ?? defaultGroupPriority(group)}
                    applyingGroup={Boolean(applyingGroup[group.appName])}
                    pidPriority={pidPriority}
                    applyingPid={applyingPid}
                    killingPid={killingPid}
                    onGroupPriorityChange={onGroupPriorityChange}
                    onApplyGroup={onApplyGroup}
                    onEndGroup={onEndGroup}
                    endingGroup={Boolean(endingGroup[group.appName])}
                    onProcessPriorityChange={onProcessPriorityChange}
                    onApplyProcess={onApplyProcess}
                    onKillProcess={onKillProcess}
                  />
                ))
              )}
            </section>
          </div>
        ) : activeTab === "settings" ? (
          <div className="space-y-4">
            {renderSavedConfigsSection(settingsSavedOpen, setSettingsSavedOpen, "settings")}

            <section className="glass-card rounded-2xl p-4">
              <h2 className="text-lg font-semibold text-zinc-100">Create Config</h2>
              <p className="mt-1 text-xs text-zinc-400">
                Config targets are mapped by application name globally; PIDs are dynamic and not stored.
              </p>

              <div className="mt-4 grid grid-cols-1 gap-4 xl:grid-cols-[1.1fr_2fr]">
                <div className="rounded-xl border border-zinc-700 bg-zinc-900/45 p-3">
                  <label className="mb-1 block text-xs text-zinc-400">Config Name</label>
                  <input
                    className="w-full rounded-xl border border-zinc-700 bg-zinc-900/70 px-3 py-2 text-sm text-zinc-100 outline-none transition focus:border-zinc-400"
                    placeholder="Gaming / Work / Streaming"
                    value={builderName}
                    onChange={(event) => setBuilderName(event.target.value)}
                  />
                  <button className="btn-primary mt-3 w-full" onClick={() => void onSaveConfig()}>
                    Save Config
                  </button>
                </div>

                <div className="rounded-xl border border-zinc-700 bg-zinc-900/45 p-3">
                  <div className="grid grid-cols-1 gap-3 lg:grid-cols-[2fr_1fr_auto] lg:items-end">
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
                        options={PRIORITY_OPTIONS}
                        value={builderTargetPriority}
                        onChange={setBuilderTargetPriority}
                      />
                    </div>

                    <button className="btn-primary h-[42px] w-full px-4 lg:justify-self-end" onClick={onAddBuilderTarget}>
                      Add Target
                    </button>
                  </div>
                </div>
              </div>

              <div className="mt-4 space-y-2">
                {Object.keys(builderTargets).length === 0 ? (
                  <div className="rounded-xl border border-zinc-700 bg-zinc-900/50 px-3 py-3 text-sm text-zinc-400">
                    No staged targets yet.
                  </div>
                ) : (
                  Object.entries(builderTargets).map(([appName, priority]) => (
                    <div
                      key={appName}
                      className="flex flex-wrap items-center gap-2 rounded-xl border border-zinc-700 bg-zinc-900/50 px-3 py-2"
                    >
                      <span className="text-sm text-zinc-100">{appName}</span>
                      <span className="ml-auto rounded border border-zinc-600 px-2 py-1 text-xs text-zinc-300">
                        {PRIORITY_OPTIONS.find((item) => item.value === priority)?.label ?? priority}
                      </span>
                      <button
                        className="btn-danger px-2 py-1 text-xs"
                        onClick={() => onRemoveBuilderTarget(appName)}
                      >
                        Remove
                      </button>
                    </div>
                  ))
                )}
              </div>
            </section>

            <section className="glass-card rounded-2xl p-4">
              <h2 className="text-lg font-semibold text-zinc-100">Application Triggers</h2>
              <p className="mt-1 text-xs text-zinc-400">
                Select a running application to trigger automatic config enforcement.
              </p>

              <div className="mt-4 grid grid-cols-1 gap-3 md:grid-cols-[1.4fr_1fr_auto]">
                <div>
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
                    {configs.map((config) => (
                      <option key={config.name} value={config.name}>
                        {config.name}
                      </option>
                    ))}
                  </select>
                </div>
                <div className="flex items-end">
                  <button className="btn-primary" onClick={() => void onAddWatchMapping()}>
                    Add Mapping
                  </button>
                </div>
              </div>

              <div className="mt-4 space-y-2">
                {sortedMappings.length === 0 ? (
                  <div className="rounded-xl border border-zinc-700 bg-zinc-900/50 px-3 py-3 text-sm text-zinc-400">
                    No mappings configured.
                  </div>
                ) : (
                  sortedMappings.map(([appName, configName]) => {
                    const displayName =
                      groups.find((group) => equalsIgnoreCase(group.appName, appName))?.appName ?? appName;
                    return (
                      <div
                        key={appName}
                        className="flex flex-wrap items-center gap-2 rounded-xl border border-zinc-700 bg-zinc-900/50 px-3 py-2"
                      >
                        <AppIcon
                          appName={displayName}
                          iconBase64={appIconLookup.get(appName.toLowerCase()) ?? null}
                          className="h-7 w-7"
                        />
                        <span className="font-mono text-sm text-zinc-100">{displayName}</span>
                        <span className="text-zinc-500">{"->"}</span>
                        <span className="text-sm text-zinc-300">{configName}</span>
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

            <section className="glass-card rounded-2xl p-4">
              <h2 className="text-lg font-semibold text-zinc-100">Core Engine</h2>
              <div className="mt-4 space-y-3">
                <label className="flex items-center justify-between rounded-xl border border-zinc-700 bg-zinc-900/50 px-3 py-2 text-sm">
                  <span className="text-zinc-200">Watchdog Service</span>
                  {renderToggleSwitch(
                    watchdogEnabled,
                    (next) => {
                      void onToggleWatchdog(next);
                    },
                    "Toggle watchdog service",
                  )}
                </label>

                <label className="flex items-center justify-between rounded-xl border border-zinc-700 bg-zinc-900/50 px-3 py-2 text-sm">
                  <span className="text-zinc-200">Start with Windows</span>
                  {renderToggleSwitch(
                    autostartEnabled,
                    (next) => {
                      void onToggleAutostart(next);
                    },
                    "Toggle start with Windows",
                  )}
                </label>
                <label className="ml-4 flex items-center justify-between rounded-xl border border-zinc-700 bg-zinc-900/40 px-3 py-2 text-sm">
                  <span className="text-zinc-300">Start as Administrator (Bypass UAC)</span>
                  {renderToggleSwitch(
                    startAsAdminEnabled,
                    (next) => {
                      void onToggleStartAsAdmin(next);
                    },
                    "Toggle start as administrator",
                  )}
                </label>

                <label className="flex items-center justify-between rounded-xl border border-zinc-700 bg-zinc-900/50 px-3 py-2 text-sm">
                  <span className="text-zinc-200">Minimize to Tray on Close</span>
                  {renderToggleSwitch(
                    minimizeToTrayEnabled,
                    (next) => {
                      void onToggleMinimizeToTray(next);
                    },
                    "Toggle minimize to tray on close",
                  )}
                </label>
              </div>
            </section>
          </div>
        ) : (
          <EngineView
            timerEnabled={timerEnabled}
            timerCurrentMs={timerCurrentMs}
            timerBusy={timerBusy}
            onTimerToggle={(enabled) => {
              void onToggleTimerResolution(enabled);
            }}
            masterEnabled={memoryPurgeConfig.masterEnabled}
            standbyListMb={memoryStats.standbyListMb}
            freeMemoryMb={memoryStats.freeMemoryMb}
            totalMemoryMb={memoryStats.totalMemoryMb}
            enableStandbyTrigger={memoryPurgeConfig.enableStandbyTrigger}
            standbyLimitMb={memoryPurgeConfig.standbyLimitMb}
            enableFreeMemoryTrigger={memoryPurgeConfig.enableFreeMemoryTrigger}
            freeMemoryLimitMb={memoryPurgeConfig.freeMemoryLimitMb}
            totalPurges={memoryPurgeConfig.totalPurges}
            configBusy={memoryConfigBusy}
            purgeBusy={memoryPurgeBusy}
            onMasterToggle={onMemoryMasterToggle}
            onStandbyTriggerToggle={onStandbyTriggerToggle}
            onStandbyLimitChange={onStandbyLimitChange}
            onStandbyLimitBlur={onStandbyLimitBlur}
            onFreeMemoryTriggerToggle={onFreeMemoryTriggerToggle}
            onFreeMemoryLimitChange={onFreeMemoryLimitChange}
            onFreeMemoryLimitBlur={onFreeMemoryLimitBlur}
            onPurgeNow={() => {
              void onPurgeNow();
            }}
          />
        )}
      </Layout>

      {elevationPending ? (
        <div className="fixed inset-0 z-[170] flex items-center justify-center bg-zinc-950/70 backdrop-blur-sm">
          <div className="glass-card rounded-xl border border-zinc-700/80 px-5 py-4 text-center">
            <p className="text-sm font-semibold text-zinc-100">Waiting for Administrator Approval</p>
            <p className="mt-1 text-xs text-zinc-400">Confirm the Windows UAC prompt to continue.</p>
          </div>
        </div>
      ) : null}

      <Toasts items={toasts} onDismiss={dismissToast} />
    </>
  );
}


