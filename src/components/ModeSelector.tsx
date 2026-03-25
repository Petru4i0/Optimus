import { createPortal } from "react-dom";
import { type CSSProperties, useCallback, useEffect, useRef, useState } from "react";

type ModeSelectorProps = {
  value: 1 | 2;
  hasTrigger: boolean;
  onChange: (mode: 1 | 2) => void;
};

export default function ModeSelector({ value, hasTrigger, onChange }: ModeSelectorProps) {
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [menuStyle, setMenuStyle] = useState<CSSProperties>({});

  const updateMenuPosition = useCallback(() => {
    const trigger = triggerRef.current;
    if (!trigger) {
      return;
    }

    const rect = trigger.getBoundingClientRect();
    const spacing = 6;
    const viewportPadding = 8;
    const menuWidth = 280;
    const belowSpace = window.innerHeight - rect.bottom - viewportPadding;
    const aboveSpace = rect.top - viewportPadding;
    const openUp = belowSpace < 160 && aboveSpace > belowSpace;

    const left = Math.max(
      viewportPadding,
      Math.min(rect.left, window.innerWidth - menuWidth - viewportPadding),
    );
    const top = openUp ? rect.top - spacing : rect.bottom + spacing;

    setMenuStyle({
      position: "fixed",
      left,
      top,
      width: menuWidth,
      zIndex: 130,
      transform: openUp ? "translateY(-100%)" : "none",
    });
  }, []);

  useEffect(() => {
    if (!open) {
      return;
    }

    updateMenuPosition();
    const onResizeOrScroll = () => updateMenuPosition();
    const onPointerDown = (event: MouseEvent) => {
      const target = event.target as Node;
      const inTrigger = triggerRef.current?.contains(target);
      const inMenu = menuRef.current?.contains(target);
      if (!inTrigger && !inMenu) {
        setOpen(false);
      }
    };
    const onEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
      }
    };

    window.addEventListener("resize", onResizeOrScroll);
    window.addEventListener("scroll", onResizeOrScroll, true);
    window.addEventListener("mousedown", onPointerDown);
    window.addEventListener("keydown", onEscape);
    return () => {
      window.removeEventListener("resize", onResizeOrScroll);
      window.removeEventListener("scroll", onResizeOrScroll, true);
      window.removeEventListener("mousedown", onPointerDown);
      window.removeEventListener("keydown", onEscape);
    };
  }, [open, updateMenuPosition]);

  const optionClass = (active: boolean, disabled: boolean) =>
    [
      "w-full rounded-lg border px-3 py-2 text-left transition",
      active
        ? "border-zinc-300 bg-zinc-800 text-zinc-100"
        : "border-zinc-700 bg-zinc-900/70 text-zinc-300 hover:border-zinc-500 hover:text-zinc-100",
      disabled ? "cursor-not-allowed opacity-45 hover:border-zinc-700 hover:text-zinc-300" : "",
    ].join(" ");

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        className="inline-flex h-6 min-w-[30px] items-center justify-center rounded-md border border-zinc-500 bg-zinc-800/85 px-1.5 text-xs font-semibold text-zinc-100 transition hover:border-zinc-300"
        onClick={() => {
          setOpen((prev) => {
            const next = !prev;
            if (next) {
              window.requestAnimationFrame(updateMenuPosition);
            }
            return next;
          });
        }}
        aria-label="Select live mode"
        aria-expanded={open}
      >
        {value}
      </button>

      {open
        ? createPortal(
            <div
              ref={menuRef}
              style={menuStyle}
              className="popover-fade-in rounded-xl border border-zinc-700 bg-zinc-900 p-2 shadow-xl"
            >
              <div className="space-y-2">
                <button
                  type="button"
                  className={optionClass(value === 1, false)}
                  onClick={() => {
                    onChange(1);
                    setOpen(false);
                  }}
                >
                  <div className="text-sm font-semibold">1 Always</div>
                  <div className="text-xs text-zinc-400">Persistent 24/7 enforcement.</div>
                </button>

                <button
                  type="button"
                  className={optionClass(value === 2, !hasTrigger)}
                  onClick={() => {
                    if (!hasTrigger) {
                      return;
                    }
                    onChange(2);
                    setOpen(false);
                  }}
                >
                  <div className="text-sm font-semibold">2 Smart</div>
                  <div className="text-xs text-zinc-400">
                    Only holds while the trigger app is running.
                  </div>
                </button>

                {!hasTrigger ? (
                  <div className="rounded-md border border-zinc-700 bg-zinc-900/60 px-2 py-1 text-xs text-zinc-500">
                    Set a trigger to use Smart mode.
                  </div>
                ) : null}
              </div>
            </div>,
            document.body,
          )
        : null}
    </>
  );
}
