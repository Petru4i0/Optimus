import { Shield } from "lucide-react";

type OnboardingViewProps = {
  busy: boolean;
  onEnableAutostart: () => Promise<void>;
  onSkip: () => void;
};

export default function OnboardingView({
  busy,
  onEnableAutostart,
  onSkip,
}: OnboardingViewProps) {
  return (
    <div className="min-h-screen bg-zinc-950 px-6 py-10">
      <div className="flex min-h-[calc(100vh-5rem)] items-center justify-center">
        <div className="w-full max-w-md rounded-xl border border-zinc-800 bg-zinc-900 p-8 text-center">
          <Shield className="mx-auto mb-4 h-12 w-12 text-zinc-400" />
          <h1 className="mb-2 text-xl font-bold text-zinc-100">Optimus Engine Setup</h1>
          <p className="mb-8 text-sm leading-6 text-zinc-400">
            To automatically manage process priorities in the background, Optimus needs to start
            with Windows. This requires a one-time Administrator permission to configure the Task
            Scheduler.
          </p>

          <div className="flex flex-col items-center">
            <button
              type="button"
              className="w-full rounded-xl bg-zinc-100 px-4 py-3 text-sm font-semibold text-zinc-950 transition hover:bg-white disabled:cursor-not-allowed disabled:opacity-60"
              disabled={busy}
              onClick={() => {
                void onEnableAutostart();
              }}
            >
              {busy ? "Configuring..." : "Enable Autostart (Recommended)"}
            </button>
            <button
              type="button"
              className="mt-4 w-full text-sm text-zinc-500 transition hover:text-zinc-300 disabled:cursor-not-allowed disabled:opacity-60"
              disabled={busy}
              onClick={onSkip}
            >
              Skip for now
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
