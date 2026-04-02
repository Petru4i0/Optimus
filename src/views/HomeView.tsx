import { useVirtualizer } from "@tanstack/react-virtual";
import { ChangeEvent, ReactNode, RefObject, useMemo, useRef } from "react";
import ProcessGroupCard from "../components/ProcessGroupCard";
import { ProcessActionProvider } from "../components/ProcessActionContext";
import { defaultGroupPriority } from "../hooks/useProcessQueries";
import { PriorityClass, PriorityOption, ProcessGroupDto } from "../types/process";

type HomeViewProps = {
  savedConfigsSection: ReactNode;
  searchQuery: string;
  onSearchChange: (value: string) => void;
  filteredGroups: ProcessGroupDto[];
  priorities: PriorityOption[];
  groupPriority: Record<string, PriorityClass>;
  applyingGroup: Record<string, boolean>;
  pidPriority: Record<number, PriorityClass>;
  applyingPid: Record<number, boolean>;
  killingPid: Record<number, boolean>;
  endingGroup: Record<string, boolean>;
  onGroupPriorityChange: (appName: string, value: PriorityClass) => void;
  onApplyGroup: (group: ProcessGroupDto) => Promise<void>;
  onEndGroup: (group: ProcessGroupDto) => Promise<void>;
  onProcessPriorityChange: (pid: number, value: PriorityClass) => void;
  onApplyProcess: (appName: string, pid: number) => Promise<void>;
  onKillProcess: (appName: string, pid: number) => Promise<void>;
};

export default function HomeView({
  savedConfigsSection,
  searchQuery,
  onSearchChange,
  filteredGroups,
  priorities,
  groupPriority,
  applyingGroup,
  pidPriority,
  applyingPid,
  killingPid,
  endingGroup,
  onGroupPriorityChange,
  onApplyGroup,
  onEndGroup,
  onProcessPriorityChange,
  onApplyProcess,
  onKillProcess,
}: HomeViewProps) {
  const scrollRef = useRef<HTMLDivElement | null>(null);

  const processActions = useMemo(
    () => ({
      onGroupPriorityChange,
      onApplyGroup,
      onEndGroup,
      onProcessPriorityChange,
      onApplyProcess,
      onKillProcess,
    }),
    [
      onApplyGroup,
      onApplyProcess,
      onEndGroup,
      onGroupPriorityChange,
      onKillProcess,
      onProcessPriorityChange,
    ],
  );

  return (
    <div className="space-y-4">
      {savedConfigsSection}

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
            className="w-full rounded-xl border border-zinc-800 bg-zinc-900 py-2 pl-10 pr-3 text-sm text-zinc-100 outline-none transition focus:border-zinc-500"
            placeholder="Search running apps"
            value={searchQuery}
            onChange={(event: ChangeEvent<HTMLInputElement>) => onSearchChange(event.target.value)}
          />
        </div>
      </section>

      <ProcessActionProvider value={processActions}>
        <section>
          {filteredGroups.length === 0 ? (
            <div className="glass-card rounded-2xl px-4 py-6 text-sm text-zinc-400">No matching processes found.</div>
          ) : (
            <VirtualizedGroups
              filteredGroups={filteredGroups}
              priorities={priorities}
              groupPriority={groupPriority}
              applyingGroup={applyingGroup}
              pidPriority={pidPriority}
              applyingPid={applyingPid}
              killingPid={killingPid}
              endingGroup={endingGroup}
              scrollRef={scrollRef}
            />
          )}
        </section>
      </ProcessActionProvider>
    </div>
  );
}

type VirtualizedGroupsProps = {
  filteredGroups: ProcessGroupDto[];
  priorities: PriorityOption[];
  groupPriority: Record<string, PriorityClass>;
  applyingGroup: Record<string, boolean>;
  pidPriority: Record<number, PriorityClass>;
  applyingPid: Record<number, boolean>;
  killingPid: Record<number, boolean>;
  endingGroup: Record<string, boolean>;
  scrollRef: RefObject<HTMLDivElement | null>;
};

function VirtualizedGroups({
  filteredGroups,
  priorities,
  groupPriority,
  applyingGroup,
  pidPriority,
  applyingPid,
  killingPid,
  endingGroup,
  scrollRef,
}: VirtualizedGroupsProps) {
  const rowVirtualizer = useVirtualizer({
    count: filteredGroups.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 120,
    overscan: 4,
  });

  return (
    <div
      ref={scrollRef}
      className="max-h-[calc(100vh-320px)] overflow-y-auto pr-1"
      role="region"
      aria-label="Running process groups"
    >
      <div className="relative w-full" style={{ height: `${rowVirtualizer.getTotalSize()}px` }}>
        {rowVirtualizer.getVirtualItems().map((virtualRow) => {
          const group = filteredGroups[virtualRow.index];
          return (
            <div
              key={`${group.appName}-${virtualRow.key}`}
              ref={rowVirtualizer.measureElement}
              data-index={virtualRow.index}
              className="absolute left-0 top-0 w-full pb-3"
              style={{ transform: `translateY(${virtualRow.start}px)` }}
            >
              <ProcessGroupCard
                group={group}
                index={virtualRow.index}
                priorities={priorities}
                groupPriorityValue={groupPriority[group.appName] ?? defaultGroupPriority(group)}
                applyingGroup={Boolean(applyingGroup[group.appName])}
                pidPriority={pidPriority}
                applyingPid={applyingPid}
                killingPid={killingPid}
                endingGroup={Boolean(endingGroup[group.appName])}
              />
            </div>
          );
        })}
      </div>
    </div>
  );
}
