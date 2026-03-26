import { createPortal } from "react-dom";
import {
  type CSSProperties,
  type KeyboardEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { type PriorityClass, type PriorityOption } from "../types/process";

type OutsideClickEntry = {
  contains: (target: Node) => boolean;
  close: () => void;
};

const outsideClickRegistry = new Map<number, OutsideClickEntry>();
let outsideClickListenerAttached = false;
let outsideClickIdCounter = 0;

const handleGlobalOutsideClick = (event: MouseEvent) => {
  const target = event.target as Node | null;
  if (!target) {
    return;
  }

  for (const entry of outsideClickRegistry.values()) {
    if (!entry.contains(target)) {
      entry.close();
    }
  }
};

const ensureOutsideClickListener = () => {
  if (outsideClickListenerAttached) {
    return;
  }
  window.addEventListener("mousedown", handleGlobalOutsideClick);
  outsideClickListenerAttached = true;
};

const cleanupOutsideClickListener = () => {
  if (!outsideClickListenerAttached || outsideClickRegistry.size > 0) {
    return;
  }
  window.removeEventListener("mousedown", handleGlobalOutsideClick);
  outsideClickListenerAttached = false;
};

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
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const outsideClickIdRef = useRef<number | null>(null);
  const [open, setOpen] = useState(false);
  const [menuStyle, setMenuStyle] = useState<CSSProperties>({});

  const selected = useMemo(
    () => options.find((option) => option.value === value) ?? null,
    [options, value],
  );

  const updateMenuPosition = useCallback(() => {
    const trigger = triggerRef.current;
    if (!trigger) {
      return;
    }

    const rect = trigger.getBoundingClientRect();
    const spacing = 6;
    const viewportPadding = 8;
    const belowSpace = window.innerHeight - rect.bottom - viewportPadding;
    const aboveSpace = rect.top - viewportPadding;
    const openUp = belowSpace < 170 && aboveSpace > belowSpace;
    const maxHeight = Math.max(120, Math.min(280, openUp ? aboveSpace : belowSpace));
    const top = openUp
      ? Math.max(viewportPadding, rect.top - maxHeight - spacing)
      : rect.bottom + spacing;

    setMenuStyle({
      position: "fixed",
      left: rect.left,
      top,
      width: rect.width,
      maxHeight,
      zIndex: 9999,
    });
  }, []);

  useEffect(() => {
    if (!open) {
      if (outsideClickIdRef.current !== null) {
        outsideClickRegistry.delete(outsideClickIdRef.current);
        outsideClickIdRef.current = null;
        cleanupOutsideClickListener();
      }
      return;
    }

    outsideClickIdCounter += 1;
    const entryId = outsideClickIdCounter;
    outsideClickIdRef.current = entryId;
    outsideClickRegistry.set(entryId, {
      contains: (target) =>
        Boolean(rootRef.current?.contains(target) || menuRef.current?.contains(target)),
      close: () => setOpen(false),
    });
    ensureOutsideClickListener();

    return () => {
      outsideClickRegistry.delete(entryId);
      if (outsideClickIdRef.current === entryId) {
        outsideClickIdRef.current = null;
      }
      cleanupOutsideClickListener();
    };
  }, [open]);

  useEffect(() => {
    if (!open) {
      return;
    }

    updateMenuPosition();
    const reposition = () => updateMenuPosition();
    window.addEventListener("resize", reposition);
    window.addEventListener("scroll", reposition, true);
    return () => {
      window.removeEventListener("resize", reposition);
      window.removeEventListener("scroll", reposition, true);
    };
  }, [open, updateMenuPosition]);

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
    <div ref={rootRef} className={`priority-select ${className ?? ""}`}>
      <button
        ref={triggerRef}
        className="priority-select-trigger"
        type="button"
        onClick={() => {
          setOpen((prev) => {
            const next = !prev;
            if (!prev && next) {
              window.requestAnimationFrame(updateMenuPosition);
            }
            return next;
          });
        }}
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

      {open
        ? createPortal(
            <div ref={menuRef} className="priority-select-menu" role="listbox" style={menuStyle}>
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
            </div>,
            document.body,
          )
        : null}
    </div>
  );
}
