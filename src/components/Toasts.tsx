import { ToastMessage } from "../types/process";

type ToastsProps = {
  items: ToastMessage[];
  onDismiss: (id: number) => void;
};

function toneClass(kind: ToastMessage["kind"]) {
  switch (kind) {
    case "success":
      return "border-zinc-200/35 bg-zinc-200/12 text-zinc-100";
    case "error":
      return "border-zinc-300/35 bg-zinc-700/35 text-zinc-100";
    default:
      return "border-zinc-400/30 bg-zinc-800/45 text-zinc-100";
  }
}

export default function Toasts({ items, onDismiss }: ToastsProps) {
  if (items.length === 0) {
    return null;
  }

  return (
    <div className="pointer-events-none fixed bottom-4 right-4 z-[80] flex w-[min(420px,calc(100vw-2rem))] flex-col gap-2">
      {items.map((toast) => (
        <div
          key={toast.id}
          className={`pointer-events-auto glass-card flex items-start gap-3 rounded-xl border px-3 py-2 text-sm ${toneClass(toast.kind)}`}
          role="status"
        >
          <p className="flex-1 leading-5">{toast.message}</p>
          <button
            className="btn-ghost h-7 min-w-[1.75rem] rounded-md px-2 text-xs"
            onClick={() => onDismiss(toast.id)}
            aria-label="Dismiss notification"
          >
            x
          </button>
        </div>
      ))}
    </div>
  );
}
