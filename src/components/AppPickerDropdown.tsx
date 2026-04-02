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
import AppIcon from "./AppIcon";

export type AppPickerOption = {
  appName: string;
  iconBase64: string | null;
  iconKey?: string | null;
};

type AppPickerDropdownProps = {
  options: AppPickerOption[];
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
};

export default function AppPickerDropdown({
  options,
  value,
  onChange,
  placeholder,
}: AppPickerDropdownProps) {
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [menuStyle, setMenuStyle] = useState<CSSProperties>({});

  const selected = useMemo(
    () => options.find((option) => option.appName.toLowerCase() === value.toLowerCase()) ?? null,
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
    const openUp = belowSpace < 180 && aboveSpace > belowSpace;
    const maxHeight = Math.max(140, Math.min(320, openUp ? aboveSpace : belowSpace));
    const top = openUp
      ? Math.max(viewportPadding, rect.top - maxHeight - spacing)
      : rect.bottom + spacing;

    setMenuStyle({
      position: "fixed",
      left: rect.left,
      top,
      width: rect.width,
      maxHeight,
      zIndex: 120,
    });
  }, []);

  useEffect(() => {
    const handlePointerDown = (event: MouseEvent) => {
      const target = event.target as Node;
      const inRoot = rootRef.current?.contains(target);
      const inMenu = menuRef.current?.contains(target);
      if (!inRoot && !inMenu) {
        setOpen(false);
      }
    };

    window.addEventListener("mousedown", handlePointerDown);
    return () => {
      window.removeEventListener("mousedown", handlePointerDown);
    };
  }, []);

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
    <div ref={rootRef} className="app-picker">
      <button
        ref={triggerRef}
        className="app-picker-trigger"
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
        {selected ? (
          <span className="app-picker-trigger-content">
            <AppIcon
              appName={selected.appName}
              iconBase64={selected.iconBase64}
              iconKey={selected.iconKey}
              className="h-6 w-6"
            />
            <span className="truncate">{selected.appName}</span>
          </span>
        ) : (
          <span className="app-picker-placeholder">{placeholder}</span>
        )}

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
            <div ref={menuRef} className="app-picker-menu" role="listbox" style={menuStyle}>
              {options.length === 0 ? (
                <div className="app-picker-empty">No running applications</div>
              ) : (
                options.map((option) => {
                  const isSelected = selected?.appName.toLowerCase() === option.appName.toLowerCase();
                  return (
                    <button
                      key={option.appName}
                      type="button"
                      className={`app-picker-item ${isSelected ? "app-picker-item-selected" : ""}`}
                      onClick={() => {
                        onChange(option.appName);
                        setOpen(false);
                      }}
                      onKeyDown={(event) => {
                        if (event.key === "Escape") {
                          event.preventDefault();
                          setOpen(false);
                        }
                      }}
                    >
                      <AppIcon
                        appName={option.appName}
                        iconBase64={option.iconBase64}
                        iconKey={option.iconKey}
                        className="h-7 w-7"
                      />
                      <span className="truncate text-left">{option.appName}</span>
                    </button>
                  );
                })
              )}
            </div>,
            document.body,
          )
        : null}
    </div>
  );
}
