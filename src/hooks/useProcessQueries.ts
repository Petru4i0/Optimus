import { useQuery, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useMemo, useState } from "react";
import { ToastKind } from "../types/config";
import { parseIpcError } from "../types/ipc";
import {
  ApplyResultDto,
  PriorityClass,
  ProcessDeltaPayload,
  ProcessGroupDto,
  ProcessPrioritySnapshot,
  ProcessRowDto,
} from "../types/process";

type PushToast = (kind: ToastKind, message: string) => void;

type ProcessRowsCache = {
  sequence: number;
  byPid: Record<number, ProcessRowDto>;
  needsElevation: boolean;
  isElevated: boolean;
};

const PROCESS_ROWS_QUERY_KEY = ["process", "rows"] as const;

const EMPTY_ROWS: ProcessRowsCache = {
  sequence: 0,
  byPid: {},
  needsElevation: false,
  isElevated: false,
};

export function defaultGroupPriority(group: ProcessGroupDto): PriorityClass {
  return group.processes[0]?.priority ?? "normal";
}

function mergeDelta(prev: ProcessRowsCache, delta: ProcessDeltaPayload): ProcessRowsCache {
  if (
    delta.sequence <= prev.sequence &&
    prev.sequence - delta.sequence < 1000
  ) {
    return prev;
  }

  const nextByPid = { ...prev.byPid };
  for (const pid of delta.removed) {
    delete nextByPid[pid];
  }
  for (const row of delta.added) {
    nextByPid[row.pid] = row;
  }
  for (const row of delta.updated) {
    nextByPid[row.pid] = row;
  }

  return {
    sequence: delta.sequence,
    byPid: nextByPid,
    needsElevation: delta.needsElevation,
    isElevated: delta.isElevated,
  };
}

export function useProcessQueries(pushToast: PushToast, queryEnabled: boolean, pollEnabled: boolean) {
  const queryClient = useQueryClient();
  const [isDocumentVisible, setIsDocumentVisible] = useState(
    typeof document === "undefined" ? true : document.visibilityState === "visible",
  );

  const [groupPriority, setGroupPriority] = useState<Record<string, PriorityClass>>({});
  const [pidPriority, setPidPriority] = useState<Record<number, PriorityClass>>({});

  const [applyingGroup, setApplyingGroup] = useState<Record<string, boolean>>({});
  const [endingGroup, setEndingGroup] = useState<Record<string, boolean>>({});
  const [applyingPid, setApplyingPid] = useState<Record<number, boolean>>({});
  const [killingPid, setKillingPid] = useState<Record<number, boolean>>({});

  useEffect(() => {
    const handleVisibilityChange = () => {
      setIsDocumentVisible(document.visibilityState === "visible");
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);
    return () => {
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, []);

  const processQuery = useQuery({
    queryKey: PROCESS_ROWS_QUERY_KEY,
    enabled: queryEnabled,
    initialData: EMPTY_ROWS,
    refetchInterval: queryEnabled && pollEnabled && isDocumentVisible ? 1000 : false,
    refetchIntervalInBackground: false,
    queryFn: async () => {
      const delta = await invoke<ProcessDeltaPayload>("process_get_delta");
      const prev = queryClient.getQueryData<ProcessRowsCache>(PROCESS_ROWS_QUERY_KEY) ?? EMPTY_ROWS;
      return mergeDelta(prev, delta);
    },
  });

  const rowsByPid = processQuery.data?.byPid ?? EMPTY_ROWS.byPid;

  const groups = useMemo(() => {
    const grouped = new Map<
      string,
      { appName: string; iconKey: string; processes: ProcessGroupDto["processes"] }
    >();

    for (const row of Object.values(rowsByPid)) {
      let entry = grouped.get(row.appName);
      if (!entry) {
        entry = { appName: row.appName, iconKey: row.iconKey, processes: [] };
        grouped.set(row.appName, entry);
      }
      entry.processes.push({
        pid: row.pid,
        memoryBytes: row.memoryBytes,
        priority: row.priority,
        priorityRaw: row.priorityRaw,
        priorityLabel: row.priorityLabel,
      });
    }

    const result: ProcessGroupDto[] = [];
    for (const group of grouped.values()) {
      group.processes.sort((a, b) => a.pid - b.pid);
      result.push({
        appName: group.appName,
        iconKey: group.iconKey,
        iconBase64: null,
        total: group.processes.length,
        processes: group.processes,
      });
    }

    result.sort((a, b) => a.appName.localeCompare(b.appName));
    return result;
  }, [rowsByPid]);

  useEffect(() => {
    const grouped = new Map<string, ProcessRowDto[]>();
    for (const row of Object.values(rowsByPid)) {
      const list = grouped.get(row.appName);
      if (list) {
        list.push(row);
      } else {
        grouped.set(row.appName, [row]);
      }
    }

    setGroupPriority((prev) => {
      const next: Record<string, PriorityClass> = {};
      for (const [appName, rows] of grouped.entries()) {
        next[appName] = prev[appName] ?? rows[0]?.priority ?? "normal";
      }
      return next;
    });

    setPidPriority((prev) => {
      const next: Record<number, PriorityClass> = {};
      for (const row of Object.values(rowsByPid)) {
        next[row.pid] = prev[row.pid] ?? row.priority ?? "normal";
      }
      return next;
    });
  }, [rowsByPid]);

  const totalProcesses = useMemo(
    () => groups.reduce((sum, group) => sum + group.total, 0),
    [groups],
  );

  const refreshProcesses = useCallback(
    async (silent = false) => {
      const result = await processQuery.refetch();
      if (result.error && !silent) {
        pushToast("error", parseIpcError(result.error).message);
      }
    },
    [processQuery, pushToast],
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

  const patchPidPriority = useCallback(
    (snapshot: ProcessPrioritySnapshot) => {
      queryClient.setQueryData<ProcessRowsCache>(PROCESS_ROWS_QUERY_KEY, (prev) => {
        if (!prev) {
          return prev;
        }
        const current = prev.byPid[snapshot.pid];
        if (!current) {
          return prev;
        }
        return {
          ...prev,
          byPid: {
            ...prev.byPid,
            [snapshot.pid]: {
              ...current,
              priority: snapshot.priority,
              priorityRaw: snapshot.priorityRaw,
              priorityLabel: snapshot.priorityLabel,
            },
          },
        };
      });

      if (snapshot.priority) {
        setPidPriority((prev) => ({ ...prev, [snapshot.pid]: snapshot.priority as PriorityClass }));
      }
    },
    [queryClient],
  );

  const onApplyProcess = useCallback(
    async (_appName: string, pid: number) => {
      const selected = pidPriority[pid] ?? "normal";
      setApplyingPid((prev) => ({ ...prev, [pid]: true }));

      try {
        const result = await invoke<ApplyResultDto>("process_set_priority", {
          pid,
          priority: selected,
        });

        if (!result.success) {
          pushToast("error", result.message);
          return;
        }

        pushToast("success", `PID ${pid}: ${result.message}`);

        const snapshot = await invoke<ProcessPrioritySnapshot>("process_get_priority", { pid });
        patchPidPriority(snapshot);
      } catch (invokeError) {
        pushToast("error", parseIpcError(invokeError).message);
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
        const results = await invoke<ApplyResultDto[]>("process_set_group_priority", {
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
        pushToast("error", parseIpcError(invokeError).message);
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
        const result = await invoke<ApplyResultDto>("process_kill", { pid });
        if (!result.success) {
          pushToast("error", result.message);
          return;
        }

        pushToast("success", `PID ${pid}: ${result.message}`);
        await refreshProcesses(true);
      } catch (invokeError) {
        pushToast("error", parseIpcError(invokeError).message);
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
          invoke<ApplyResultDto>("process_kill", { pid: process.pid }),
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
        pushToast("error", parseIpcError(invokeError).message);
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
    refreshing: processQuery.isFetching,
    error: processQuery.error ? parseIpcError(processQuery.error).message : null,
    needsElevation: processQuery.data?.needsElevation ?? false,
    isElevated: processQuery.data?.isElevated ?? false,
    lastSync: processQuery.dataUpdatedAt || null,
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
