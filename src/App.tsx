import { type ComponentType, lazy, Suspense, useEffect, useMemo } from "react";
import ElevationModal from "./components/ElevationModal";
import type { EngineViewProps } from "./components/EngineView";
import Layout from "./components/Layout";
import type { OptimizationViewProps } from "./components/OptimizationView";
import SafetyIntro from "./components/SafetyIntro";
import Toasts from "./components/Toasts";
import { useConfigManager } from "./hooks/useConfigManager";
import { useConfigQueries } from "./hooks/useConfigQueries";
import { useEngineManager } from "./hooks/useEngineManager";
import { useEngineQueries } from "./hooks/useEngineQueries";
import { useHardwareQueries } from "./hooks/useHardwareQueries";
import { useOptimizationQueries } from "./hooks/useOptimizationQueries";
import { useProcessQueries } from "./hooks/useProcessQueries";
import { useToastManager } from "./hooks/useToastManager";
import { useAppStore } from "./store/appStore";
import { PriorityOption } from "./types/process";
import HomeView from "./views/HomeView";
import OnboardingView from "./views/OnboardingView";
import SavedConfigsSection from "./views/SavedConfigsSection";
import type { SettingsViewProps } from "./views/SettingsView";

type ViewModule = { default: ComponentType<any> };
function devSmartLazyView(loader: () => Promise<ViewModule>) {
  if (import.meta.env.DEV) {
    void loader();
  }
  return lazy(loader);
}

const SettingsView = devSmartLazyView(
  () => import("./views/SettingsView"),
) as ComponentType<SettingsViewProps>;
const EngineView = devSmartLazyView(
  () => import("./components/EngineView"),
) as ComponentType<EngineViewProps>;
const OptimizationView = devSmartLazyView(
  () => import("./components/OptimizationView"),
) as ComponentType<OptimizationViewProps>;

const PRIORITY_OPTIONS: PriorityOption[] = [
  { label: "Realtime", value: "realtime" },
  { label: "High", value: "high" },
  { label: "Above Normal", value: "aboveNormal" },
  { label: "Normal", value: "normal" },
  { label: "Below Normal", value: "belowNormal" },
  { label: "Low", value: "low" },
];

const LazyFallback = () => (
  <div className="glass-card rounded-2xl px-4 py-6 text-sm text-zinc-400">Loading view...</div>
);

export default function App() {
  const activeTab = useAppStore((state) => state.activeTab);
  const setActiveTab = useAppStore((state) => state.setActiveTab);
  const hasSeenSafetyIntro = useAppStore((state) => state.hasSeenSafetyIntro);
  const onboardingCompleted = useAppStore((state) => state.onboardingCompleted);
  const completeOnboarding = useAppStore((state) => state.completeOnboarding);
  const searchQuery = useAppStore((state) => state.searchQuery);
  const setSearchQuery = useAppStore((state) => state.setSearchQuery);
  const elevationGateOpen = useAppStore((state) => state.elevationGateOpen);
  const elevationOverlayVisible = useAppStore((state) => state.elevationOverlayVisible);
  const openElevationGate = useAppStore((state) => state.openElevationGate);
  const closeElevationGate = useAppStore((state) => state.closeElevationGate);
  const setElevationOverlayVisible = useAppStore((state) => state.setElevationOverlayVisible);
  const setEnginePending = useAppStore((state) => state.setEnginePending);

  const { toasts, pushToast, dismissToast } = useToastManager();

  const process = useProcessQueries(pushToast, onboardingCompleted, activeTab === "home");
  const engine = useEngineManager(pushToast);
  const config = useConfigManager(pushToast, process.groups, process.refreshProcesses);

  useEngineQueries({
    enabled: onboardingCompleted,
    activeTab,
    loadAppSettings: engine.loadAppSettings,
    loadRuntimeSettings: engine.loadRuntimeSettings,
    loadTurboTimerStatus: engine.loadTurboTimerStatus,
    refreshMemoryStats: engine.refreshMemoryStats,
  });

  useConfigQueries({
    enabled: onboardingCompleted,
    loadConfigs: config.loadConfigs,
    loadWatchdog: config.loadWatchdog,
  });

  useOptimizationQueries({
    enabled: onboardingCompleted,
    activeTab,
    loadOptimizationStatus: engine.loadOptimizationStatus,
  });

  useHardwareQueries({
    enabled: onboardingCompleted,
    activeTab,
    loadPciDevices: engine.loadPciDevices,
  });

  useEffect(() => {
    setEnginePending({
      watchdog: engine.watchdogPending,
      autostart: engine.autostartPending,
      minimizeToTray: engine.minimizeToTrayPending,
    });
  }, [
    engine.autostartPending,
    engine.minimizeToTrayPending,
    engine.watchdogPending,
    setEnginePending,
  ]);

  useEffect(() => {
    if (!engine.elevationPending) {
      setElevationOverlayVisible(false);
      return;
    }

    setElevationOverlayVisible(true);
    const timeoutId = window.setTimeout(() => {
      setElevationOverlayVisible(false);
      pushToast("warning", "UAC confirmation timed out. Try 'Restart as Administrator' again.");
    }, 15000);

    return () => {
      window.clearTimeout(timeoutId);
    };
  }, [engine.elevationPending, pushToast, setElevationOverlayVisible]);

  const filteredGroups = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    if (!query) {
      return process.groups;
    }
    return process.groups.filter((group) => group.appName.toLowerCase().includes(query));
  }, [process.groups, searchQuery]);

  const appPickerOptions = useMemo(
    () =>
      process.groups
        .map((group) => ({
          appName: group.appName,
          iconBase64: group.iconBase64,
          iconKey: group.iconKey,
        }))
        .sort((a, b) => a.appName.localeCompare(b.appName)),
    [process.groups],
  );

  const homeSavedSection = (
    <SavedConfigsSection
      isOpen={config.homeSavedOpen}
      setOpen={config.setHomeSavedOpen}
      sectionId="home"
      configs={config.configs}
      watchdogConfig={config.watchdogConfig}
      groups={process.groups}
      equalsIgnoreCase={config.equalsIgnoreCase}
      onImportConfig={config.onImportConfig}
      onSetSticky={config.onSetSticky}
      onApplyConfig={config.onApplyConfig}
      onExportConfig={config.onExportConfig}
      onCreateShortcut={config.onCreateShortcut}
      onDeleteConfig={config.onDeleteConfig}
    />
  );

  const settingsSavedSection = (
    <SavedConfigsSection
      isOpen={config.settingsSavedOpen}
      setOpen={config.setSettingsSavedOpen}
      sectionId="settings"
      configs={config.configs}
      watchdogConfig={config.watchdogConfig}
      groups={process.groups}
      equalsIgnoreCase={config.equalsIgnoreCase}
      onImportConfig={config.onImportConfig}
      onSetSticky={config.onSetSticky}
      onApplyConfig={config.onApplyConfig}
      onExportConfig={config.onExportConfig}
      onCreateShortcut={config.onCreateShortcut}
      onDeleteConfig={config.onDeleteConfig}
    />
  );

  const onRestartFromElevationGate = () => {
    closeElevationGate();
    void engine.onRequestElevationClick();
  };

  const onEnableOnboardingAutostart = async () => {
    const success = await engine.enableElevatedAutostartForOnboarding();
    if (success) {
      completeOnboarding();
    }
  };

  const renderActiveView = () => {
    if (activeTab === "home") {
      return (
        <HomeView
          savedConfigsSection={homeSavedSection}
          searchQuery={searchQuery}
          onSearchChange={setSearchQuery}
          filteredGroups={filteredGroups}
          priorities={PRIORITY_OPTIONS}
          groupPriority={process.groupPriority}
          applyingGroup={process.applyingGroup}
          pidPriority={process.pidPriority}
          applyingPid={process.applyingPid}
          killingPid={process.killingPid}
          endingGroup={process.endingGroup}
          onGroupPriorityChange={process.onGroupPriorityChange}
          onApplyGroup={process.onApplyGroup}
          onEndGroup={process.onEndGroup}
          onProcessPriorityChange={process.onProcessPriorityChange}
          onApplyProcess={process.onApplyProcess}
          onKillProcess={process.onKillProcess}
        />
      );
    }

    if (activeTab === "settings") {
      return (
        <SettingsView
          savedConfigsSection={settingsSavedSection}
          builderName={config.builderName}
          setBuilderName={config.setBuilderName}
          builderTargetApp={config.builderTargetApp}
          setBuilderTargetApp={config.setBuilderTargetApp}
          builderTargetPriority={config.builderTargetPriority}
          setBuilderTargetPriority={config.setBuilderTargetPriority}
          builderTargets={config.builderTargets}
          onAddBuilderTarget={config.onAddBuilderTarget}
          onRemoveBuilderTarget={config.onRemoveBuilderTarget}
          onSaveConfig={config.onSaveConfig}
          appPickerOptions={appPickerOptions}
          priorityOptions={PRIORITY_OPTIONS}
          watchTriggerApp={config.watchTriggerApp}
          setWatchTriggerApp={config.setWatchTriggerApp}
          watchConfigName={config.watchConfigName}
          setWatchConfigName={config.setWatchConfigName}
          configNames={config.configs.map((item) => item.name)}
          sortedMappings={config.sortedMappings}
          equalsIgnoreCase={config.equalsIgnoreCase}
          groupAppNames={process.groups.map((group) => group.appName)}
          onAddWatchMapping={config.onAddWatchMapping}
          onRemoveWatchMapping={config.onRemoveWatchMapping}
          watchdogEnabled={engine.watchdogEnabled}
          autostartEnabled={engine.autostartEnabled}
          alwaysRunAsAdmin={engine.alwaysRunAsAdmin}
          minimizeToTrayEnabled={engine.minimizeToTrayEnabled}
          watchdogPending={engine.watchdogPending}
          autostartPending={engine.autostartPending}
          alwaysRunAsAdminPending={engine.alwaysRunAsAdminPending}
          minimizeToTrayPending={engine.minimizeToTrayPending}
          onToggleWatchdog={engine.onToggleWatchdog}
          onToggleAutostart={engine.onToggleAutostart}
          onToggleAlwaysRunAsAdmin={engine.onToggleAlwaysRunAsAdmin}
          onToggleMinimizeToTray={engine.onToggleMinimizeToTray}
        />
      );
    }

    if (activeTab === "engine") {
      return (
        <EngineView
          isAdmin={process.isElevated}
          onRequireAdmin={openElevationGate}
          timerEnabled={engine.timerEnabled}
          timerCurrentMs={engine.timerCurrentMs}
          timerBusy={engine.timerBusy}
          deepPurgeBusy={engine.deepPurgeBusy}
          deepPurgeConfig={engine.deepPurgeConfig}
          totalDeepPurgeCount={engine.totalDeepPurgeCount}
          totalDeepPurgeBytes={engine.totalDeepPurgeBytes}
          onTimerToggle={(enabled) => {
            void engine.onToggleTimerResolution(enabled);
          }}
          onRunDeepPurge={(config) => {
            void engine.onRunDeepPurge(config);
          }}
          setDeepPurgeConfig={engine.setDeepPurgeConfig}
          masterEnabled={engine.memoryPurgeConfig.masterEnabled}
          standbyListMb={engine.memoryStats.standbyListMb}
          freeMemoryMb={engine.memoryStats.freeMemoryMb}
          totalMemoryMb={engine.memoryStats.totalMemoryMb}
          enableStandbyTrigger={engine.memoryPurgeConfig.enableStandbyTrigger}
          standbyLimitMb={engine.memoryPurgeConfig.standbyLimitMb}
          enableFreeMemoryTrigger={engine.memoryPurgeConfig.enableFreeMemoryTrigger}
          freeMemoryLimitMb={engine.memoryPurgeConfig.freeMemoryLimitMb}
          totalPurges={engine.totalPurges}
          totalRamClearedMb={engine.totalRamClearedMb}
          configBusy={engine.memoryConfigBusy}
          purgeBusy={engine.memoryPurgeBusy}
          onMasterToggle={engine.onMemoryMasterToggle}
          onStandbyTriggerToggle={engine.onStandbyTriggerToggle}
          onStandbyLimitChange={engine.onStandbyLimitChange}
          onStandbyLimitBlur={engine.onStandbyLimitBlur}
          onFreeMemoryTriggerToggle={engine.onFreeMemoryTriggerToggle}
          onFreeMemoryLimitChange={engine.onFreeMemoryLimitChange}
          onFreeMemoryLimitBlur={engine.onFreeMemoryLimitBlur}
          onPurgeNow={() => {
            void engine.onPurgeNow();
          }}
          pciDevices={engine.pciDevices}
          drivers={engine.drivers}
          ghostDevices={engine.ghostDevices}
          pciLoading={engine.pciLoading}
          driversLoading={engine.driversLoading}
          ghostsLoading={engine.ghostsLoading}
          pciApplying={engine.pciApplying}
          driverDeleting={engine.driverDeleting}
          ghostRemoving={engine.ghostRemoving}
          onRefreshPci={() => {
            void engine.loadPciDevices(false);
          }}
          onRefreshDrivers={() => {
            void engine.loadInstalledDrivers(false);
          }}
          onRefreshGhosts={() => {
            void engine.loadGhostDevices(false);
          }}
          onApplyMsiBatch={(updates) => {
            void engine.applyMsiBatch(updates);
          }}
          onDeleteDriver={(publishedName, force) => {
            void engine.removeDriver(publishedName, force);
          }}
          onRemoveGhost={(instanceId, force) => {
            void engine.removeGhost(instanceId, force);
          }}
        />
      );
    }

    return (
      <OptimizationView
        isAdmin={process.isElevated}
        onRequireAdmin={openElevationGate}
        status={engine.optimizationStatus}
        loading={engine.optimizationLoading}
        telemetryBusy={engine.telemetryBusy}
        netSniperBusy={engine.netSniperBusy}
        powerBusy={engine.powerBusy}
        advancedBusy={engine.advancedBusy}
        onToggleTelemetry={(subFeature, enabled) => {
          void engine.onToggleTelemetry(subFeature, enabled);
        }}
        onToggleNetSniper={(subFeature, enabled) => {
          void engine.onToggleNetSniper(subFeature, enabled);
        }}
        onTogglePowerMode={(subFeature, enabled) => {
          void engine.onTogglePowerMode(subFeature, enabled);
        }}
        onToggleAdvanced={(subFeature, enabled) => {
          void engine.onToggleAdvanced(subFeature, enabled);
        }}
      />
    );
  };

  return (
    <>
      {!hasSeenSafetyIntro ? (
        <SafetyIntro />
      ) : !onboardingCompleted ? (
        <OnboardingView
          busy={engine.autostartPending}
          onEnableAutostart={onEnableOnboardingAutostart}
          onSkip={completeOnboarding}
        />
      ) : (
        <>
          <Layout
            activeTab={activeTab}
            onTabChange={setActiveTab}
            title="Optimus"
            groupsCount={process.groups.length}
            totalProcesses={process.totalProcesses}
            refreshing={process.refreshing}
            onRefresh={process.onRefreshRequested}
            isElevated={process.isElevated}
            needsElevation={process.needsElevation}
            onRequestElevation={engine.onRequestElevationClick}
            lastSync={process.lastSync}
            error={process.error}
          >
            <Suspense fallback={<LazyFallback />}>{renderActiveView()}</Suspense>
          </Layout>

          {engine.elevationPending && elevationOverlayVisible ? (
            <div className="fixed inset-0 z-[170] flex items-center justify-center bg-zinc-950 backdrop-blur-sm">
              <div className="glass-card rounded-xl border border-zinc-800 px-5 py-4 text-center">
                <p className="text-sm font-semibold text-zinc-100">Waiting for Administrator Approval</p>
                <p className="mt-1 text-xs text-zinc-400">Confirm the Windows UAC prompt to continue.</p>
              </div>
            </div>
          ) : null}

          <ElevationModal
            open={elevationGateOpen}
            onCancel={closeElevationGate}
            onRestartAsAdmin={onRestartFromElevationGate}
          />
        </>
      )}

      <Toasts items={toasts} onDismiss={dismissToast} />
    </>
  );
}
