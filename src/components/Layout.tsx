import { ReactNode } from "react";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import StatusPill from "./StatusPill";
import TitleBar from "./TitleBar";

type LayoutProps = {
  activeTab: "home" | "settings";
  onTabChange: (tab: "home" | "settings") => void;
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

function cn(...inputs: (string | undefined | false)[]) {
  return twMerge(clsx(inputs));
}

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
  return (
    <div className="flex h-screen w-full flex-col overflow-hidden bg-zinc-950 text-zinc-200">
      <TitleBar activeTab={activeTab} onTabChange={onTabChange} title={title} />
      <div className="h-12 shrink-0" aria-hidden />

      <main className="flex-1 overflow-y-auto px-4 pb-6">
        <div className="mx-auto flex max-w-7xl flex-col gap-6 pt-4 sm:px-2 lg:px-4">
          <header className="glass-card glow-hover rounded-2xl p-5">
            <div className="flex flex-wrap items-center gap-4">
              <div>
                <h1 className="text-2xl font-semibold tracking-tight sm:text-3xl">Optimus</h1>
                <p className="mt-1 text-sm text-zinc-300/90">
                  Windows process priority manager with native elevation and live telemetry.
                </p>
              </div>

              <div className="ml-auto flex flex-wrap items-center gap-3">
                <StatusPill>Groups: {groupsCount}</StatusPill>
                <StatusPill>Processes: {totalProcesses}</StatusPill>
                <button className="btn-primary" onClick={onRefresh} disabled={refreshing}>
                  {refreshing ? "Refreshing..." : "Refresh"}
                </button>
              </div>
            </div>

            <div className="mt-4 flex flex-wrap items-center gap-3 text-sm text-zinc-300/90">
              <StatusPill className={cn(isElevated ? "border-green-300/30 text-green-200" : "border-zinc-200/20")}>
                {isElevated ? "Running as Administrator" : "Running as Standard User"}
              </StatusPill>

              {lastSync ? <StatusPill>Last Sync: {new Date(lastSync).toLocaleTimeString()}</StatusPill> : null}

              {needsElevation && !isElevated ? (
                <button className="btn-warning" onClick={onRequestElevation}>
                  Restart as Administrator
                </button>
              ) : null}
            </div>

            {error ? (
              <p className="mt-4 rounded-lg border border-rose-400/30 bg-rose-500/10 px-3 py-2 text-sm text-rose-200">
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
