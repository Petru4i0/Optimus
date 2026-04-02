import { create } from "zustand";
import { persist } from "zustand/middleware";

export type AppTab = "home" | "settings" | "engine" | "optimization";
export type AppLocale = "en" | "ru";
export type HardwareTab = "msi" | "drivers" | "ghosts";
export type DriverScanMode = "idle" | "old" | "all";
export type GhostScanMode = "idle" | "active";

type EnginePendingState = {
  watchdog: boolean;
  autostart: boolean;
  minimizeToTray: boolean;
};

export type DeepPurgeConfig = {
  windows: boolean;
  gpu: boolean;
  browsers: boolean;
  apps: boolean;
  dev: boolean;
};

type AppStore = {
  activeTab: AppTab;
  locale: AppLocale;
  hasSeenSafetyIntro: boolean;
  onboardingCompleted: boolean;
  searchQuery: string;
  alwaysRunAsAdmin: boolean;
  activeHardwareTab: HardwareTab;
  msiShowSupportedOnly: boolean;
  driverScanMode: DriverScanMode;
  ghostScanMode: GhostScanMode;
  totalPurges: number;
  totalRamClearedMb: number;
  totalDeepPurgeCount: number;
  totalDeepPurgeBytes: number;
  deepPurgeConfig: DeepPurgeConfig;
  elevationGateOpen: boolean;
  elevationOverlayVisible: boolean;
  enginePending: EnginePendingState;
  setActiveTab: (tab: AppTab) => void;
  setLocale: (locale: AppLocale) => void;
  setHasSeenSafetyIntro: (value: boolean) => void;
  completeOnboarding: () => void;
  setSearchQuery: (value: string) => void;
  setAlwaysRunAsAdmin: (value: boolean) => void;
  setActiveHardwareTab: (tab: HardwareTab) => void;
  setMsiShowSupportedOnly: (value: boolean) => void;
  setDriverScanMode: (mode: DriverScanMode) => void;
  setGhostScanMode: (mode: GhostScanMode) => void;
  openElevationGate: () => void;
  closeElevationGate: () => void;
  setElevationOverlayVisible: (visible: boolean) => void;
  setEnginePending: (partial: Partial<EnginePendingState>) => void;
  incrementPurgeCount: (by: number) => void;
  addClearedRam: (mb: number) => void;
  recordDeepPurgeSuccess: (bytes: number) => void;
  setDeepPurgeConfig: (key: keyof DeepPurgeConfig, value: boolean) => void;
};

const DEFAULT_ENGINE_PENDING: EnginePendingState = {
  watchdog: false,
  autostart: false,
  minimizeToTray: false,
};

const DEFAULT_DEEP_PURGE_CONFIG: DeepPurgeConfig = {
  windows: true,
  gpu: true,
  browsers: true,
  apps: true,
  dev: true,
};

export const useAppStore = create<AppStore>()(
  persist(
    (set) => ({
      activeTab: "home",
      locale: "en",
      hasSeenSafetyIntro: false,
      onboardingCompleted: false,
      searchQuery: "",
      alwaysRunAsAdmin: false,
      activeHardwareTab: "msi",
      msiShowSupportedOnly: true,
      driverScanMode: "idle",
      ghostScanMode: "idle",
      totalPurges: 0,
      totalRamClearedMb: 0,
      totalDeepPurgeCount: 0,
      totalDeepPurgeBytes: 0,
      deepPurgeConfig: DEFAULT_DEEP_PURGE_CONFIG,
      elevationGateOpen: false,
      elevationOverlayVisible: false,
      enginePending: DEFAULT_ENGINE_PENDING,
      setActiveTab: (tab) => set({ activeTab: tab }),
      setLocale: (locale) => set({ locale }),
      setHasSeenSafetyIntro: (value) => set({ hasSeenSafetyIntro: value }),
      completeOnboarding: () => set({ onboardingCompleted: true }),
      setSearchQuery: (value) => set({ searchQuery: value }),
      setAlwaysRunAsAdmin: (value) => set({ alwaysRunAsAdmin: value }),
      setActiveHardwareTab: (tab) => set({ activeHardwareTab: tab }),
      setMsiShowSupportedOnly: (value) => set({ msiShowSupportedOnly: value }),
      setDriverScanMode: (mode) => set({ driverScanMode: mode }),
      setGhostScanMode: (mode) => set({ ghostScanMode: mode }),
      openElevationGate: () => set({ elevationGateOpen: true }),
      closeElevationGate: () => set({ elevationGateOpen: false }),
      setElevationOverlayVisible: (visible) => set({ elevationOverlayVisible: visible }),
      setEnginePending: (partial) =>
        set((state) => ({
          enginePending: {
            ...state.enginePending,
            ...partial,
          },
        })),
      incrementPurgeCount: (by) =>
        set((state) => ({
          totalPurges: state.totalPurges + Math.max(0, Math.floor(by)),
        })),
      addClearedRam: (mb) =>
        set((state) => ({
          totalRamClearedMb: state.totalRamClearedMb + Math.max(0, Math.floor(mb)),
        })),
      recordDeepPurgeSuccess: (bytes) =>
        set((state) => ({
          totalDeepPurgeCount: state.totalDeepPurgeCount + 1,
          totalDeepPurgeBytes:
            state.totalDeepPurgeBytes +
            (typeof bytes === "number" && Number.isFinite(bytes)
              ? Math.max(0, Math.floor(bytes))
              : 0),
        })),
      setDeepPurgeConfig: (key, value) =>
        set((state) => ({
          deepPurgeConfig: {
            ...state.deepPurgeConfig,
            [key]: value,
          },
        })),
    }),
    {
      name: "optimus-app-store",
      partialize: (state) => ({
        activeTab: state.activeTab,
        locale: state.locale,
        hasSeenSafetyIntro: state.hasSeenSafetyIntro,
        onboardingCompleted: state.onboardingCompleted,
        alwaysRunAsAdmin: state.alwaysRunAsAdmin,
        activeHardwareTab: state.activeHardwareTab,
        msiShowSupportedOnly: state.msiShowSupportedOnly,
        driverScanMode: state.driverScanMode,
        ghostScanMode: state.ghostScanMode,
        totalPurges: state.totalPurges,
        totalRamClearedMb: state.totalRamClearedMb,
        totalDeepPurgeCount: state.totalDeepPurgeCount,
        totalDeepPurgeBytes: state.totalDeepPurgeBytes,
        deepPurgeConfig: state.deepPurgeConfig,
      }),
    },
  ),
);
