import { invoke } from "@tauri-apps/api/core";
import { useCallback, useMemo, useRef, useState } from "react";
import {
  ApplyResultDto,
  PriorityClass,
  ProcessGroupDto,
  ProcessListResponse,
  ProcessPrioritySnapshot,
  ToastKind,
} from "../types/process";

export function defaultGroupPriority(group: ProcessGroupDto): PriorityClass {
  return group.processes[0]?.priority ?? "normal";
}

type PushToast = (kind: ToastKind, message: string) => void;

export function useProcessManager(pushToast: PushToast) {
  const ICON_KEY_TTL_MS = 10 * 60 * 1000;
  const ICON_KEY_MAX = 1200;

  const [groups, setGroups] = useState<ProcessGroupDto[]>([]);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [needsElevation, setNeedsElevation] = useState(false);
  const [isElevated, setIsElevated] = useState(false);
  const [lastSync, setLastSync] = useState<number | null>(null);

  const [groupPriority, setGroupPriority] = useState<Record<string, PriorityClass>>({});
  const [pidPriority, setPidPriority] = useState<Record<number, PriorityClass>>({});

  const [applyingGroup, setApplyingGroup] = useState<Record<string, boolean>>({});
  const [endingGroup, setEndingGroup] = useState<Record<string, boolean>>({});
  const [applyingPid, setApplyingPid] = useState<Record<number, boolean>>({});
  const [killingPid, setKillingPid] = useState<Record<number, boolean>>({});

  const iconCacheRef = useRef<Map<string, string>>(new Map());
  const iconSeenAtRef = useRef<Map<string, number>>(new Map());
  const refreshInFlightRef = useRef(false);
  const refreshQueuedRef = useRef(false);
  const refreshQueuedInteractiveRef = useRef(false);

  const totalProcesses = useMemo(
    () => groups.reduce((sum, group) => sum + group.total, 0),
    [groups],
  );

  const refreshProcesses = useCallback(
    async (silent = false) => {
      if (refreshInFlightRef.current) {
        refreshQueuedRef.current = true;
        refreshQueuedInteractiveRef.current = refreshQueuedInteractiveRef.current || !silent;
        return;
      }
      refreshInFlightRef.current = true;

      if (!silent) {
        setRefreshing(true);
      }

      try {
        const now = Date.now();
        for (const [key, seenAt] of iconSeenAtRef.current) {
          if (now - seenAt > ICON_KEY_TTL_MS) {
            iconSeenAtRef.current.delete(key);
            iconCacheRef.current.delete(key);
          }
        }

        if (iconSeenAtRef.current.size > ICON_KEY_MAX) {
          const oldest = Array.from(iconSeenAtRef.current.entries())
            .sort((a, b) => a[1] - b[1])
            .slice(0, iconSeenAtRef.current.size - ICON_KEY_MAX);
          for (const [key] of oldest) {
            iconSeenAtRef.current.delete(key);
            iconCacheRef.current.delete(key);
          }
        }

        const response = await invoke<ProcessListResponse>("get_process_list_delta", {
          knownIconKeys: Array.from(iconSeenAtRef.current.keys()),
        });
        const nextGroups = response.groups.map((group) => {
          iconSeenAtRef.current.set(group.iconKey, now);

          if (group.iconBase64) {
            iconCacheRef.current.set(group.iconKey, group.iconBase64);
            return group;
          }

          const cached = iconCacheRef.current.get(group.iconKey);
          if (!cached) {
            return group;
          }

          return {
            ...group,
            iconBase64: cached,
          };
        });

        setGroups(nextGroups);
        setNeedsElevation(response.needsElevation);
        setIsElevated(response.isElevated);
        setLastSync(Date.now());
        setError(null);

        setGroupPriority((prev) => {
          const next: Record<string, PriorityClass> = {};
          for (const group of nextGroups) {
            next[group.appName] = prev[group.appName] ?? defaultGroupPriority(group);
          }
          return next;
        });

        setPidPriority((prev) => {
          const next: Record<number, PriorityClass> = {};
          for (const group of nextGroups) {
            for (const process of group.processes) {
              next[process.pid] = prev[process.pid] ?? process.priority ?? "normal";
            }
          }
          return next;
        });
      } catch (invokeError) {
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        setError(message);
        if (!silent) {
          pushToast("error", message);
        }
      } finally {
        if (!silent) {
          setRefreshing(false);
        }
        refreshInFlightRef.current = false;

        if (refreshQueuedRef.current) {
          const nextSilent = !refreshQueuedInteractiveRef.current;
          refreshQueuedRef.current = false;
          refreshQueuedInteractiveRef.current = false;
          void refreshProcesses(nextSilent);
        }
      }
    },
    [pushToast],
  );

  const onRefreshRequested = useCallback(() => {
    void refreshProcesses();
  }, [refreshProcesses]);

  const onGroupPriorityChange = useCallback((appName: string, value: PriorityClass) => {
    setGroupPriority((prev) => ({ ...prev, [appName]: value }));
  }, []);

  const onProcessPriorityChange = useCallback((pid: number, value: PriorityClass) => {
    setPidPriority((prev) => ({ ...prev, [pid]: value }));
  }, []);

  const patchPidPriority = useCallback((snapshot: ProcessPrioritySnapshot) => {
    setGroups((prev) =>
      prev.map((group) => ({
        ...group,
        processes: group.processes.map((process) => {
          if (process.pid !== snapshot.pid) {
            return process;
          }
          return {
            ...process,
            priority: snapshot.priority,
            priorityRaw: snapshot.priorityRaw,
            priorityLabel: snapshot.priorityLabel,
          };
        }),
      })),
    );

    if (snapshot.priority) {
      setPidPriority((prev) => ({ ...prev, [snapshot.pid]: snapshot.priority as PriorityClass }));
    }
  }, []);

  const onApplyProcess = useCallback(
    async (_appName: string, pid: number) => {
      const selected = pidPriority[pid] ?? "normal";
      setApplyingPid((prev) => ({ ...prev, [pid]: true }));

      try {
        const result = await invoke<ApplyResultDto>("set_process_priority", {
          pid,
          priority: selected,
        });

        if (!result.success) {
          pushToast("error", result.message);
          return;
        }

        pushToast("success", `PID ${pid}: ${result.message}`);

        const snapshot = await invoke<ProcessPrioritySnapshot>("get_process_priority", { pid });
        patchPidPriority(snapshot);
      } catch (invokeError) {
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      } finally {
        setApplyingPid((prev) => {
          const { [pid]: _removed, ...rest } = prev;
          return rest;
        });
      }
    },
    [patchPidPriority, pidPriority, pushToast],
  );

  const onApplyGroup = useCallback(
    async (group: ProcessGroupDto) => {
      const selected = groupPriority[group.appName] ?? defaultGroupPriority(group);
      setApplyingGroup((prev) => ({ ...prev, [group.appName]: true }));

      try {
        const results = await invoke<ApplyResultDto[]>("set_group_priority", {
          pids: group.processes.map((process) => process.pid),
          priority: selected,
        });

        const successCount = results.filter((result) => result.success).length;
        const failed = results.length - successCount;

        if (failed > 0) {
          pushToast("info", `Group ${group.appName}: applied ${successCount}, failed ${failed}`);
        } else {
          pushToast("success", `Group ${group.appName}: applied ${successCount}`);
        }

        await refreshProcesses(true);
      } catch (invokeError) {
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      } finally {
        setApplyingGroup((prev) => {
          const { [group.appName]: _removed, ...rest } = prev;
          return rest;
        });
      }
    },
    [groupPriority, pushToast, refreshProcesses],
  );

  const onKillProcess = useCallback(
    async (_appName: string, pid: number) => {
      setKillingPid((prev) => ({ ...prev, [pid]: true }));

      try {
        const result = await invoke<ApplyResultDto>("kill_process", { pid });
        if (!result.success) {
          pushToast("error", result.message);
          return;
        }

        pushToast("success", `PID ${pid}: ${result.message}`);
        await refreshProcesses(true);
      } catch (invokeError) {
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      } finally {
        setKillingPid((prev) => {
          const { [pid]: _removed, ...rest } = prev;
          return rest;
        });
      }
    },
    [pushToast, refreshProcesses],
  );

  const onEndGroup = useCallback(
    async (group: ProcessGroupDto) => {
      setEndingGroup((prev) => ({ ...prev, [group.appName]: true }));

      try {
        const tasks = group.processes.map((process) =>
          invoke<ApplyResultDto>("kill_process", { pid: process.pid }),
        );
        const settled = await Promise.allSettled(tasks);

        let successCount = 0;
        let failedCount = 0;

        for (const item of settled) {
          if (item.status === "fulfilled" && item.value.success) {
            successCount += 1;
          } else {
            failedCount += 1;
          }
        }

        if (failedCount > 0) {
          pushToast("info", `${group.appName}: terminated ${successCount}, failed ${failedCount}`);
        } else {
          pushToast("success", `${group.appName}: terminated ${successCount}`);
        }

        await refreshProcesses(true);
      } catch (invokeError) {
        const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
        pushToast("error", message);
      } finally {
        setEndingGroup((prev) => {
          const { [group.appName]: _removed, ...rest } = prev;
          return rest;
        });
      }
    },
    [pushToast, refreshProcesses],
  );

  return {
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
  };
}
