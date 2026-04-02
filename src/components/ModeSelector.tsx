import { useRef, useState } from "react";
import PopoverMenu from "./PopoverMenu";

type ModeSelectorProps = {
  value: 1 | 2;
  hasTrigger: boolean;
  onChange: (mode: 1 | 2) => void;
};

export default function ModeSelector({ value, hasTrigger, onChange }: ModeSelectorProps) {
  const triggerRef = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);

  const optionClass = (active: boolean, disabled: boolean) =>
    [
      "w-full rounded-lg border px-3 py-2 text-left transition",
      active
        ? "border-zinc-500 bg-zinc-800 text-zinc-100"
        : "border-zinc-800 bg-zinc-900 text-zinc-400 hover:border-zinc-500 hover:text-zinc-100",
      disabled ? "cursor-not-allowed opacity-45 hover:border-zinc-800 hover:text-zinc-400" : "",
    ].join(" ");

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        className="inline-flex h-6 min-w-[30px] items-center justify-center rounded-md border border-zinc-500 bg-zinc-800 px-1.5 text-xs font-semibold text-zinc-100 transition hover:border-zinc-500"
        onClick={() => setOpen((prev) => !prev)}
        aria-label="Select live mode"
        aria-expanded={open}
      >
        {value}
      </button>

      <PopoverMenu
        open={open}
        anchorRef={triggerRef}
        onClose={() => setOpen(false)}
        className="popover-fade-in rounded-xl border border-zinc-800 bg-zinc-900 p-2 shadow-xl"
        width={280}
        zIndex={130}
        openUpThreshold={160}
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
            <div className="rounded-md border border-zinc-800 bg-zinc-900 px-2 py-1 text-xs text-zinc-400">
              Set a trigger to use Smart mode.
            </div>
          ) : null}
        </div>
      </PopoverMenu>
    </>
  );
}
