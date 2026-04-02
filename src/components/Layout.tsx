import { ReactNode } from "react";
import TitleBar from "./TitleBar";

type LayoutProps = {
  activeTab: "home" | "settings" | "engine" | "optimization";
  onTabChange: (tab: "home" | "settings" | "engine" | "optimization") => void;
  title?: string;
  groupsCount: number;
  totalProcesses: number;
  refreshing: boolean;
  onRefresh: () => void;
  isElevated: boolean;
  needsElevation: boolean;
  onRequestElevation: () => void;
  lastSync: number | null;
  error: string | null;
  children: ReactNode;
};

export default function Layout({
  activeTab,
  onTabChange,
  title,
  groupsCount,
  totalProcesses,
  refreshing,
  onRefresh,
  isElevated,
  needsElevation,
  onRequestElevation,
  lastSync,
  error,
  children,
}: LayoutProps) {
  const syncLabel = lastSync ? new Date(lastSync).toLocaleTimeString() : "Waiting...";

  return (
    <div className="flex h-screen w-full flex-col overflow-hidden bg-zinc-950 text-zinc-100">
      <TitleBar activeTab={activeTab} onTabChange={onTabChange} title={title} />
      <div className="h-12 shrink-0" aria-hidden />

      <main className="relative z-0 pointer-events-auto flex-1 overflow-y-auto px-4 pb-6">
        <div className="mx-auto flex max-w-7xl flex-col gap-6 pt-4 sm:px-2 lg:px-4">
          <header className="glass-card glow-hover rounded-2xl px-4 py-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="flex min-w-0 flex-wrap items-center gap-2">
                <h1 className="text-xl font-bold tracking-tight text-zinc-100">Optimus</h1>
                <span
                  className={`inline-flex items-center rounded-md border px-2 py-1 text-xs font-medium ${
                    isElevated
                      ? "border-emerald-500/30 bg-emerald-500/10 text-emerald-500"
                      : "border-rose-500/30 bg-rose-500/10 text-rose-500"
                  }`}
                >
                  {isElevated ? "Administrator" : "Standard User"}
                </span>
                {needsElevation && !isElevated ? (
                  <button className="btn-warning inline-flex items-center justify-center py-0 h-6 px-3 text-[11px] font-medium leading-none" onClick={onRequestElevation}>
                    Restart as Administrator
                  </button>
                ) : null}
              </div>

              <div className="flex flex-wrap items-center gap-3 lg:justify-end">
                <div className="flex items-center divide-x divide-zinc-800 rounded-md border border-zinc-800 bg-zinc-900 text-xs text-zinc-400">
                  <div className="px-3 py-1.5 first:pl-3">Groups: {groupsCount}</div>
                  <div className="px-3 py-1.5">Processes: {totalProcesses}</div>
                  <div className="px-3 py-1.5">Sync: {syncLabel}</div>
                </div>
                <button className="btn-primary inline-flex items-center justify-center py-0 h-6 px-3 text-[11px] font-medium leading-none" onClick={onRefresh} disabled={refreshing}>
                  {refreshing ? "Refreshing..." : "Refresh"}
                </button>
              </div>
            </div>

            {error ? (
              <p className="mt-3 rounded-lg border border-rose-500/30 bg-rose-500/10 px-3 py-2 text-sm text-rose-500">
                {error}
              </p>
            ) : null}
          </header>

          {children}
        </div>
      </main>
    </div>
  );
}
