import { memo } from "react";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import { useProcessActions } from "./ProcessActionContext";
import { PriorityClass, PriorityOption, ProcessDto } from "../types/process";
import PrioritySelect from "./PrioritySelect";
import StatusPill from "./StatusPill";

type ProcessRowProps = {
  appName: string;
  process: ProcessDto;
  selectedPriority: PriorityClass;
  priorities: PriorityOption[];
  applying: boolean;
  killing: boolean;
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
      return "bg-zinc-800 text-zinc-100 border-zinc-500";
    case "high":
      return "bg-zinc-800 text-zinc-100 border-zinc-500";
    case "aboveNormal":
      return "bg-zinc-800 text-zinc-100 border-zinc-500";
    case "normal":
      return "bg-zinc-800 text-zinc-100 border-zinc-500";
    case "belowNormal":
      return "bg-zinc-700 text-zinc-100 border-zinc-500";
    case "low":
      return "bg-zinc-800 text-zinc-100 border-zinc-500";
    default:
      return "bg-zinc-800 text-zinc-100 border-zinc-500";
  }
}

function ProcessRow({
  appName,
  process,
  selectedPriority,
  priorities,
  applying,
  killing,
}: ProcessRowProps) {
  const actions = useProcessActions();

  return (
    <div className="rounded-xl border border-zinc-500 bg-zinc-950 px-3 py-2">
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
            onChange={(nextValue) => actions.onProcessPriorityChange(process.pid, nextValue)}
          />

          <button
            className="btn-primary"
            disabled={applying}
            onClick={() => void actions.onApplyProcess(appName, process.pid)}
          >
            {applying ? "Applying..." : "Apply"}
          </button>

          <button
            className="btn-danger"
            disabled={killing}
            onClick={() => void actions.onKillProcess(appName, process.pid)}
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
    prev.process.pid === next.process.pid &&
    prev.process.memoryBytes === next.process.memoryBytes &&
    prev.process.priority === next.process.priority &&
    prev.process.priorityRaw === next.process.priorityRaw &&
    prev.process.priorityLabel === next.process.priorityLabel
  );
}

export default memo(ProcessRow, areProcessRowPropsEqual);
