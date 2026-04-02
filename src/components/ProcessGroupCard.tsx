import { AnimatePresence, motion } from "framer-motion";
import { memo, useState } from "react";
import { useProcessActions } from "./ProcessActionContext";
import { PriorityClass, PriorityOption, ProcessGroupDto } from "../types/process";
import AppIcon from "./AppIcon";
import PrioritySelect from "./PrioritySelect";
import ProcessRow from "./ProcessRow";

type ProcessGroupCardProps = {
  group: ProcessGroupDto;
  index: number;
  priorities: PriorityOption[];
  groupPriorityValue: PriorityClass;
  applyingGroup: boolean;
  pidPriority: Record<number, PriorityClass>;
  applyingPid: Record<number, boolean>;
  killingPid: Record<number, boolean>;
  endingGroup: boolean;
};

function ProcessGroupCard({
  group,
  index,
  priorities,
  groupPriorityValue,
  applyingGroup,
  pidPriority,
  applyingPid,
  killingPid,
  endingGroup,
}: ProcessGroupCardProps) {
  const [isOpen, setIsOpen] = useState(false);
  const actions = useProcessActions();

  return (
    <motion.section
      className="glass-card glow-hover rounded-2xl p-4"
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: index * 0.03, duration: 0.24 }}
    >
      <div className="flex flex-wrap items-center gap-3">
        <button
          className="inline-flex h-8 w-8 items-center justify-center text-zinc-400 transition hover:text-zinc-100"
          onClick={() => setIsOpen((prev) => !prev)}
          aria-label={isOpen ? "Collapse app group" : "Expand app group"}
        >
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.9"
            className={`h-5 w-5 transition-transform duration-200 ${isOpen ? "rotate-180" : ""}`}
          >
            <path d="M6 9l6 6 6-6" />
          </svg>
        </button>

        <AppIcon
          appName={group.appName}
          iconBase64={group.iconBase64}
          iconKey={group.iconKey}
          className="h-9 w-9"
        />

        <div>
          <h2 className="text-lg font-semibold sm:text-xl">{group.appName}</h2>
          <p className="text-xs text-zinc-400">{group.total} running processes</p>
        </div>

        <div className="ml-auto flex flex-wrap items-center gap-2">
          <PrioritySelect
            className="w-[190px]"
            options={priorities}
            value={groupPriorityValue}
            onChange={(nextValue) => actions.onGroupPriorityChange(group.appName, nextValue)}
          />
          <button className="btn-primary" disabled={applyingGroup} onClick={() => void actions.onApplyGroup(group)}>
            {applyingGroup ? "Applying..." : "Apply to All"}
          </button>
          <button className="btn-danger" disabled={endingGroup} onClick={() => void actions.onEndGroup(group)}>
            {endingGroup ? "Ending..." : "End App"}
          </button>
        </div>
      </div>

      <AnimatePresence initial={false}>
        {isOpen ? (
          <motion.div
            key="children"
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            transition={{ duration: 0.22 }}
            className="overflow-hidden"
          >
            <div className="mt-4 space-y-2 border-t border-zinc-500 pt-4">
              {group.processes.map((process) => (
                <ProcessRow
                  key={process.pid}
                  appName={group.appName}
                  process={process}
                  selectedPriority={pidPriority[process.pid] ?? "normal"}
                  priorities={priorities}
                  applying={Boolean(applyingPid[process.pid])}
                  killing={Boolean(killingPid[process.pid])}
                />
              ))}
            </div>
          </motion.div>
        ) : null}
      </AnimatePresence>
    </motion.section>
  );
}

function areGroupsEqual(prevGroup: ProcessGroupDto, nextGroup: ProcessGroupDto) {
  if (
    prevGroup.appName !== nextGroup.appName ||
    prevGroup.total !== nextGroup.total ||
    prevGroup.iconKey !== nextGroup.iconKey ||
    prevGroup.iconBase64 !== nextGroup.iconBase64 ||
    prevGroup.processes.length !== nextGroup.processes.length
  ) {
    return false;
  }

  for (let i = 0; i < prevGroup.processes.length; i += 1) {
    const prevProcess = prevGroup.processes[i];
    const nextProcess = nextGroup.processes[i];
    if (
      prevProcess.pid !== nextProcess.pid ||
      prevProcess.memoryBytes !== nextProcess.memoryBytes ||
      prevProcess.priority !== nextProcess.priority ||
      prevProcess.priorityRaw !== nextProcess.priorityRaw ||
      prevProcess.priorityLabel !== nextProcess.priorityLabel
    ) {
      return false;
    }
  }

  return true;
}

function areGroupCardPropsEqual(prev: ProcessGroupCardProps, next: ProcessGroupCardProps) {
  if (
    prev.index !== next.index ||
    prev.groupPriorityValue !== next.groupPriorityValue ||
    prev.applyingGroup !== next.applyingGroup ||
    prev.endingGroup !== next.endingGroup ||
    prev.priorities !== next.priorities
  ) {
    return false;
  }

  if (!areGroupsEqual(prev.group, next.group)) {
    return false;
  }

  for (const process of next.group.processes) {
    const pid = process.pid;
    if ((prev.pidPriority[pid] ?? "normal") !== (next.pidPriority[pid] ?? "normal")) {
      return false;
    }
    if (Boolean(prev.applyingPid[pid]) !== Boolean(next.applyingPid[pid])) {
      return false;
    }
    if (Boolean(prev.killingPid[pid]) !== Boolean(next.killingPid[pid])) {
      return false;
    }
  }

  return true;
}

export default memo(ProcessGroupCard, areGroupCardPropsEqual);
