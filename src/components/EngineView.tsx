import type { DeepPurgeConfig } from "../store/appStore";
import { useAppStore } from "../store/appStore";
import { Driver, GhostDevice, MsiPriority, PciDevice } from "../types/hardware";
import DriverStorePanel from "./engine/DriverStorePanel";
import GhostDevicesPanel from "./engine/GhostDevicesPanel";
import MemoryPurgeCard from "./engine/MemoryPurgeCard";
import MsiPanel from "./engine/MsiPanel";
import TimerCard from "./engine/TimerCard";

export type EngineViewProps = {
  isAdmin: boolean;
  onRequireAdmin: () => void;
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
  pciDevices: PciDevice[];
  drivers: Driver[];
  ghostDevices: GhostDevice[];
  pciLoading: boolean;
  driversLoading: boolean;
  ghostsLoading: boolean;
  pciApplying: boolean;
  driverDeleting: boolean;
  ghostRemoving: boolean;
  onRefreshPci: (silent?: boolean) => void;
  onRefreshDrivers: (silent?: boolean) => void;
  onRefreshGhosts: (silent?: boolean) => void;
  onApplyMsiBatch: (updates: Array<{ deviceId: string; enable: boolean; priority: MsiPriority }>) => void;
  onDeleteDriver: (publishedName: string, force: boolean) => void;
  onRemoveGhost: (instanceId: string, force: boolean) => void;
};

export default function EngineView({
  isAdmin,
  onRequireAdmin,
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
  pciDevices,
  drivers,
  ghostDevices,
  pciLoading,
  driversLoading,
  ghostsLoading,
  pciApplying,
  driverDeleting,
  ghostRemoving,
  onRefreshPci,
  onRefreshDrivers,
  onRefreshGhosts,
  onApplyMsiBatch,
  onDeleteDriver,
  onRemoveGhost,
}: EngineViewProps) {
  const activeHardwareTab = useAppStore((state) => state.activeHardwareTab);
  const setActiveHardwareTab = useAppStore((state) => state.setActiveHardwareTab);

  const handleMemoryMasterToggle = (enabled: boolean) => {
    if (!isAdmin) {
      onRequireAdmin();
      return;
    }
    onMasterToggle(enabled);
  };

  const handlePurgeNow = () => {
    if (!isAdmin) {
      onRequireAdmin();
      return;
    }
    onPurgeNow();
  };

  const handleRunDeepPurge = (config: DeepPurgeConfig) => {
    if (!isAdmin) {
      onRequireAdmin();
      return;
    }
    onRunDeepPurge(config);
  };

  const handleApplyMsiBatch = (
    updates: Array<{ deviceId: string; enable: boolean; priority: MsiPriority }>,
  ) => {
    if (!isAdmin) {
      onRequireAdmin();
      return;
    }
    onApplyMsiBatch(updates);
  };

  const handleDeleteDriver = (publishedName: string, force: boolean) => {
    if (!isAdmin) {
      onRequireAdmin();
      return;
    }
    onDeleteDriver(publishedName, force);
  };

  const handleRemoveGhost = (instanceId: string, force: boolean) => {
    if (!isAdmin) {
      onRequireAdmin();
      return;
    }
    onRemoveGhost(instanceId, force);
  };

  return (
    <div className="space-y-4">
      <div className="grid gap-4 xl:grid-cols-2">
        <TimerCard
          timerEnabled={timerEnabled}
          timerCurrentMs={timerCurrentMs}
          timerBusy={timerBusy}
          deepPurgeBusy={deepPurgeBusy}
          deepPurgeConfig={deepPurgeConfig}
          totalDeepPurgeCount={totalDeepPurgeCount}
          totalDeepPurgeBytes={totalDeepPurgeBytes}
          onTimerToggle={onTimerToggle}
          onRunDeepPurge={handleRunDeepPurge}
          setDeepPurgeConfig={setDeepPurgeConfig}
        />

        <MemoryPurgeCard
          masterEnabled={masterEnabled}
          standbyListMb={standbyListMb}
          freeMemoryMb={freeMemoryMb}
          totalMemoryMb={totalMemoryMb}
          enableStandbyTrigger={enableStandbyTrigger}
          standbyLimitMb={standbyLimitMb}
          enableFreeMemoryTrigger={enableFreeMemoryTrigger}
          freeMemoryLimitMb={freeMemoryLimitMb}
          totalPurges={totalPurges}
          totalRamClearedMb={totalRamClearedMb}
          configBusy={configBusy}
          purgeBusy={purgeBusy}
          onMasterToggle={handleMemoryMasterToggle}
          onStandbyTriggerToggle={onStandbyTriggerToggle}
          onStandbyLimitChange={onStandbyLimitChange}
          onStandbyLimitBlur={onStandbyLimitBlur}
          onFreeMemoryTriggerToggle={onFreeMemoryTriggerToggle}
          onFreeMemoryLimitChange={onFreeMemoryLimitChange}
          onFreeMemoryLimitBlur={onFreeMemoryLimitBlur}
          onPurgeNow={handlePurgeNow}
        />
      </div>

      <section className="glass-card rounded-2xl p-5">
        <div className="flex flex-wrap items-center gap-2">
          <h2 className="text-lg font-semibold text-zinc-100">Hardware</h2>
          <div className="ml-auto flex rounded-lg border border-zinc-800 bg-zinc-900 p-1">
            {[
              { id: "msi" as const, label: "MSI Utility" },
              { id: "drivers" as const, label: "Driver Store" },
              { id: "ghosts" as const, label: "Inactive Devices" },
            ].map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveHardwareTab(tab.id)}
                className={`rounded-md px-3 py-1.5 text-xs font-medium transition ${
                  activeHardwareTab === tab.id
                    ? "bg-zinc-700 text-zinc-100"
                    : "text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100"
                }`}
              >
                {tab.label}
              </button>
            ))}
          </div>
        </div>

        {activeHardwareTab === "msi" ? (
          <MsiPanel
            pciDevices={pciDevices}
            pciLoading={pciLoading}
            pciApplying={pciApplying}
            onRefreshPci={onRefreshPci}
            onApplyMsiBatch={handleApplyMsiBatch}
          />
        ) : null}

        {activeHardwareTab === "drivers" ? (
          <DriverStorePanel
            drivers={drivers}
            driversLoading={driversLoading}
            driverDeleting={driverDeleting}
            onRefreshDrivers={onRefreshDrivers}
            onDeleteDriver={handleDeleteDriver}
          />
        ) : null}

        {activeHardwareTab === "ghosts" ? (
          <GhostDevicesPanel
            ghostDevices={ghostDevices}
            ghostsLoading={ghostsLoading}
            ghostRemoving={ghostRemoving}
            onRefreshGhosts={onRefreshGhosts}
            onRemoveGhost={handleRemoveGhost}
          />
        ) : null}
      </section>
    </div>
  );
}
