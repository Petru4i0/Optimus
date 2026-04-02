import { ShieldAlert } from "lucide-react";

type ElevationModalProps = {
  open: boolean;
  onCancel: () => void;
  onRestartAsAdmin: () => void;
};

export default function ElevationModal({ open, onCancel, onRestartAsAdmin }: ElevationModalProps) {
  if (!open) {
    return null;
  }

  return (
    <div className="fixed inset-0 z-[9999] flex items-center justify-center bg-zinc-950 px-4 backdrop-blur-md">
      <div className="w-full max-w-sm rounded-2xl border border-zinc-800 bg-zinc-900 p-5 text-center shadow-2xl">
        <div className="mx-auto w-fit text-zinc-400">
          <ShieldAlert className="h-8 w-8" strokeWidth={1.75} />
        </div>

        <h3 className="mt-3 text-base font-semibold text-zinc-100">Administrator Privileges Required</h3>
        <p className="mt-1.5 text-sm leading-relaxed text-zinc-400">
          This feature modifies core Windows systems.
        </p>

        <div className="mt-5 flex flex-col items-center justify-center gap-2">
          <button
            className="w-full rounded-lg border border-zinc-800 bg-zinc-900 px-4 py-2 text-sm font-medium text-zinc-100 transition hover:border-zinc-500 hover:bg-zinc-800"
            onClick={onCancel}
          >
            Cancel
          </button>
          <button
            className="w-full rounded-lg bg-zinc-200 px-4 py-2 text-sm font-medium text-zinc-950 transition hover:bg-zinc-200"
            onClick={onRestartAsAdmin}
          >
            Restart as Administrator
          </button>
        </div>
      </div>
    </div>
  );
}
