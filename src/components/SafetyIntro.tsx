import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "../hooks/useTranslation";
import { useAppStore } from "../store/appStore";

const INITIAL_SECONDS = 5;

export default function SafetyIntro() {
  const t = useTranslation();
  const locale = useAppStore((state) => state.locale);
  const setLocale = useAppStore((state) => state.setLocale);
  const setHasSeenSafetyIntro = useAppStore((state) => state.setHasSeenSafetyIntro);
  const [secondsLeft, setSecondsLeft] = useState(INITIAL_SECONDS);

  useEffect(() => {
    if (secondsLeft <= 0) {
      return;
    }
    const id = window.setInterval(() => {
      setSecondsLeft((prev) => Math.max(0, prev - 1));
    }, 1000);
    return () => window.clearInterval(id);
  }, [secondsLeft]);

  const buttonLabel = useMemo(() => {
    if (secondsLeft > 0) {
      return t.welcome.buttonWait.replace("{s}", String(secondsLeft));
    }
    return t.welcome.buttonReady;
  }, [secondsLeft, t.welcome.buttonReady, t.welcome.buttonWait]);

  return (
    <div className="min-h-screen bg-zinc-950 px-6 py-8">
      <div className="mx-auto flex max-w-4xl justify-end">
        <div className="inline-flex rounded-xl border border-zinc-800 bg-zinc-900 p-1">
          {(["en", "ru"] as const).map((value) => {
            const active = locale === value;
            return (
              <button
                key={value}
                type="button"
                className={`rounded-lg px-3 py-1.5 text-xs font-semibold tracking-wide transition ${
                  active
                    ? "bg-zinc-200 text-zinc-950"
                    : "text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100"
                }`}
                onClick={() => setLocale(value)}
              >
                {value.toUpperCase()}
              </button>
            );
          })}
        </div>
      </div>

      <div className="mx-auto flex min-h-[calc(100vh-8rem)] max-w-4xl items-center justify-center">
        <div className="w-full rounded-3xl border border-zinc-800 bg-zinc-900 p-8 text-center shadow-2xl">
          <h1 className="text-3xl font-semibold leading-tight text-zinc-100">{t.welcome.title}</h1>
          <p className="mx-auto mt-5 max-w-3xl text-base leading-8 text-zinc-300">{t.welcome.text}</p>

          <div className="mt-10">
            <button
              type="button"
              disabled={secondsLeft > 0}
              onClick={() => setHasSeenSafetyIntro(true)}
              className="w-full rounded-xl bg-zinc-100 px-5 py-3 text-base font-semibold text-zinc-950 transition hover:bg-white disabled:cursor-not-allowed disabled:bg-zinc-500 disabled:text-zinc-300"
            >
              {buttonLabel}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

