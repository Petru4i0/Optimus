import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { ShieldAlert } from "lucide-react";
import InfoTooltip from "./ui/InfoTooltip";

type HardwareActionModalProps = {
  isOpen: boolean;
  title: string;
  warningText: string;
  onClose: () => void;
  onConfirm: (force: boolean) => void;
};

export default function HardwareActionModal({
  isOpen,
  title,
  warningText,
  onClose,
  onConfirm,
}: HardwareActionModalProps) {
  const [forceDelete, setForceDelete] = useState(false);

  useEffect(() => {
    if (isOpen) {
      setForceDelete(false);
    }
  }, [isOpen]);

  if (!isOpen) {
    return null;
  }

  if (typeof document === "undefined" || !document.body) {
    return null;
  }

  return createPortal(
    <div className="fixed inset-0 z-[9999] flex items-center justify-center bg-zinc-950 px-4 backdrop-blur-md">
      <div className="w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900 p-5 text-center shadow-2xl">
        <div className="mx-auto w-fit text-zinc-400">
          <ShieldAlert className="h-8 w-8" strokeWidth={1.75} />
        </div>

        <h3 className="mt-3 text-base font-semibold text-zinc-100">{title}</h3>
        <p className="mt-1.5 text-sm leading-relaxed text-zinc-400">{warningText}</p>

        <label className="mt-4 flex items-center justify-center gap-2 text-sm text-zinc-400">
          <input
            type="checkbox"
            className="h-4 w-4 rounded border-zinc-800 bg-zinc-900 accent-zinc-200"
            checked={forceDelete}
            onChange={(event) => setForceDelete(event.target.checked)}
          />
          <span>Force Delete (Bypass safety checks)</span>
          <InfoTooltip translationKey="force_delete" />
        </label>

        <div className="mt-5 flex flex-col items-center justify-center gap-2">
          <button
            className="w-full rounded-lg border border-zinc-800 bg-zinc-900 px-4 py-2 text-sm font-medium text-zinc-100 transition hover:border-zinc-500 hover:bg-zinc-800"
            onClick={onClose}
          >
            Cancel
          </button>
          <button
            className={`w-full rounded-lg px-4 py-2 text-sm font-medium transition ${
              forceDelete
                ? "bg-rose-500 text-white hover:bg-rose-500"
                : "bg-zinc-200 text-zinc-950 hover:bg-zinc-200"
            }`}
            onClick={() => onConfirm(forceDelete)}
          >
            Proceed
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
