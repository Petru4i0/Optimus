import {
  type KeyboardEvent,
  useMemo,
  useRef,
  useState,
} from "react";
import PopoverMenu from "./PopoverMenu";
import { type PriorityClass, type PriorityOption } from "../types/process";

type PrioritySelectProps = {
  options: PriorityOption[];
  value: PriorityClass;
  onChange: (value: PriorityClass) => void;
  className?: string;
};

export default function PrioritySelect({
  options,
  value,
  onChange,
  className,
}: PrioritySelectProps) {
  const triggerRef = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);

  const selected = useMemo(
    () => options.find((option) => option.value === value) ?? null,
    [options, value],
  );

  const onTriggerKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (event.key === "Enter" || event.key === " " || event.key === "ArrowDown") {
      event.preventDefault();
      setOpen(true);
    }
    if (event.key === "Escape") {
      event.preventDefault();
      setOpen(false);
    }
  };

  return (
    <div className={`priority-select ${className ?? ""}`}>
      <button
        ref={triggerRef}
        className="priority-select-trigger"
        type="button"
        onClick={() => setOpen((prev) => !prev)}
        onKeyDown={onTriggerKeyDown}
        aria-haspopup="listbox"
        aria-expanded={open}
      >
        <span className="truncate">{selected?.label ?? "Select priority"}</span>
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.9"
          className={`h-4 w-4 text-zinc-400 transition-transform ${open ? "rotate-180" : ""}`}
          aria-hidden="true"
        >
          <path d="M6 9l6 6 6-6" />
        </svg>
      </button>

      <PopoverMenu
        open={open}
        anchorRef={triggerRef}
        onClose={() => setOpen(false)}
        className="priority-select-menu"
        role="listbox"
        width="anchor"
        maxHeight={280}
        minHeight={120}
        openUpThreshold={170}
        zIndex={9999}
      >
        {options.map((option) => {
          const isSelected = option.value === value;
          return (
            <button
              key={option.value}
              type="button"
              className={`priority-select-item ${isSelected ? "priority-select-item-selected" : ""}`}
              onClick={() => {
                onChange(option.value);
                setOpen(false);
              }}
              onKeyDown={(event) => {
                if (event.key === "Escape") {
                  event.preventDefault();
                  setOpen(false);
                }
              }}
            >
              {option.label}
            </button>
          );
        })}
      </PopoverMenu>
    </div>
  );
}
