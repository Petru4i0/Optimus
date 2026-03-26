import { useEffect, useRef } from "react";

export type PollTaskId = "processes" | "timer" | "memory";

export type PollTaskConfig = {
  id: PollTaskId;
  intervalMs: number;
  run: () => void | Promise<void>;
  enabled?: boolean;
  critical?: boolean;
  hiddenBehavior?: "throttle" | "pause";
};

type SchedulerState = {
  intervalMs: number;
  run: () => void | Promise<void>;
  enabled: boolean;
  critical: boolean;
  hiddenBehavior: "throttle" | "pause";
  inFlight: boolean;
  nextDueAt: number;
};

type UsePollingSchedulerOptions = {
  tasks: PollTaskConfig[];
  heartbeatMs?: number;
  hiddenThrottleMultiplier?: number;
};

export function usePollingScheduler({
  tasks,
  heartbeatMs = 500,
  hiddenThrottleMultiplier = 3,
}: UsePollingSchedulerOptions) {
  const registryRef = useRef<Map<PollTaskId, SchedulerState>>(new Map());

  useEffect(() => {
    const now = Date.now();
    const nextRegistry = new Map<PollTaskId, SchedulerState>();

    for (const task of tasks) {
      const prev = registryRef.current.get(task.id);
      nextRegistry.set(task.id, {
        intervalMs: task.intervalMs,
        run: task.run,
        enabled: task.enabled ?? true,
        critical: task.critical ?? false,
        hiddenBehavior: task.hiddenBehavior ?? "throttle",
        inFlight: prev?.inFlight ?? false,
        nextDueAt: prev?.nextDueAt ?? now + task.intervalMs,
      });
    }

    registryRef.current = nextRegistry;
  }, [tasks]);

  useEffect(() => {
    let disposed = false;

    const runTask = (task: SchedulerState, intervalMs: number) => {
      task.inFlight = true;
      void Promise.resolve(task.run())
        .catch(() => {
          // Existing refresh handlers own user-facing error reporting.
        })
        .finally(() => {
          task.inFlight = false;
          task.nextDueAt = Date.now() + intervalMs;
        });
    };

    const tick = () => {
      if (disposed) {
        return;
      }

      const hidden = document.visibilityState === "hidden";
      const now = Date.now();

      for (const task of registryRef.current.values()) {
        if (!task.enabled || task.inFlight) {
          continue;
        }

        if (hidden && task.hiddenBehavior === "pause") {
          continue;
        }

        const effectiveInterval =
          hidden && !task.critical && task.hiddenBehavior === "throttle"
            ? Math.max(task.intervalMs, task.intervalMs * hiddenThrottleMultiplier)
            : task.intervalMs;

        if (task.nextDueAt > now) {
          continue;
        }

        runTask(task, effectiveInterval);
      }
    };

    const onVisibilityChange = () => {
      if (document.visibilityState !== "visible") {
        return;
      }

      const now = Date.now();
      for (const task of registryRef.current.values()) {
        if (!task.enabled || task.inFlight) {
          continue;
        }
        task.nextDueAt = now;
      }

      tick();
    };

    const timerId = window.setInterval(tick, heartbeatMs);
    document.addEventListener("visibilitychange", onVisibilityChange);

    return () => {
      disposed = true;
      window.clearInterval(timerId);
      document.removeEventListener("visibilitychange", onVisibilityChange);
    };
  }, [heartbeatMs, hiddenThrottleMultiplier]);
}
