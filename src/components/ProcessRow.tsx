import { PriorityClass, PriorityOption, ProcessDto } from "../types/process";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import { memo } from "react";
import PrioritySelect from "./PrioritySelect";
import StatusPill from "./StatusPill";

type ProcessRowProps = {
  appName: string;
  process: ProcessDto;
  selectedPriority: PriorityClass;
  priorities: PriorityOption[];
  applying: boolean;
  killing: boolean;
  onPriorityChange: (pid: number, value: PriorityClass) => void;
  onApply: (appName: string, pid: number) => Promise<void>;
  onKill: (appName: string, pid: number) => Promise<void>;
};

function cn(...inputs: (string | undefined | false)[]) {
  return twMerge(clsx(inputs));
}

function formatMiB(bytes: number) {
  return `${(bytes / (1024 * 1024)).toFixed(2)} MiB`;
}

function badgeTone(priority: ProcessDto["priority"]) {
  switch (priority) {
    case "realtime":
      return "bg-zinc-100/18 text-zinc-100 border-zinc-100/35";
    case "high":
      return "bg-zinc-100/14 text-zinc-100 border-zinc-200/30";
    case "aboveNormal":
      return "bg-zinc-100/12 text-zinc-100 border-zinc-300/28";
    case "normal":
      return "bg-zinc-500/20 text-zinc-100 border-zinc-300/30";
    case "belowNormal":
      return "bg-zinc-700/35 text-zinc-200 border-zinc-400/25";
    case "low":
      return "bg-zinc-500/20 text-zinc-200 border-zinc-300/30";
    default:
      return "bg-zinc-800/60 text-zinc-200 border-zinc-500/35";
  }
}

function ProcessRow({
  appName,
  process,
  selectedPriority,
  priorities,
  applying,
  killing,
  onPriorityChange,
  onApply,
  onKill,
}: ProcessRowProps) {
  return (
    <div className="rounded-xl border border-zinc-200/10 bg-zinc-950/35 px-3 py-2">
      <div className="flex flex-wrap items-center gap-2">
        <StatusPill className="min-w-[108px] justify-center">PID {process.pid}</StatusPill>
        <StatusPill className="min-w-[132px] justify-center">{formatMiB(process.memoryBytes)}</StatusPill>
        <StatusPill className={cn("min-w-[130px] justify-center", badgeTone(process.priority))}>
          {process.priorityLabel}
        </StatusPill>

        <div className="ml-auto flex flex-wrap items-center gap-2">
          <PrioritySelect
            className="w-[180px]"
            options={priorities}
            value={selectedPriority}
            onChange={(nextValue) => onPriorityChange(process.pid, nextValue)}
          />

          <button
            className="btn-primary"
            disabled={applying}
            onClick={() => void onApply(appName, process.pid)}
          >
            {applying ? "Applying..." : "Apply"}
          </button>

          <button
            className="btn-danger"
            disabled={killing}
            onClick={() => void onKill(appName, process.pid)}
          >
            {killing ? "Ending..." : "End Task"}
          </button>
        </div>
      </div>
    </div>
  );
}

function areProcessRowPropsEqual(prev: ProcessRowProps, next: ProcessRowProps) {
  return (
    prev.appName === next.appName &&
    prev.selectedPriority === next.selectedPriority &&
    prev.applying === next.applying &&
    prev.killing === next.killing &&
    prev.priorities === next.priorities &&
    prev.onPriorityChange === next.onPriorityChange &&
    prev.onApply === next.onApply &&
    prev.onKill === next.onKill &&
    prev.process.pid === next.process.pid &&
    prev.process.memoryBytes === next.process.memoryBytes &&
    prev.process.priority === next.process.priority &&
    prev.process.priorityRaw === next.process.priorityRaw &&
    prev.process.priorityLabel === next.process.priorityLabel
  );
}

export default memo(ProcessRow, areProcessRowPropsEqual);
