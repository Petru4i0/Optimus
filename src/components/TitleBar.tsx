import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState, type CSSProperties } from "react";

type TitleBarProps = {
  activeTab: "home" | "settings" | "engine";
  onTabChange: (tab: "home" | "settings" | "engine") => void;
  title?: string;
};

export default function TitleBar({ activeTab, onTabChange, title = "Optimus" }: TitleBarProps) {
  const appWindow = getCurrentWindow();
  const [isMaximized, setIsMaximized] = useState(false);
  const noDragStyle = { WebkitAppRegion: "no-drag" } as CSSProperties;

  useEffect(() => {
    let mounted = true;
    appWindow
      .isMaximized()
      .then((value) => {
        if (mounted) {
          setIsMaximized(value);
        }
      })
      .catch(() => {
        if (mounted) {
          setIsMaximized(false);
        }
      });

    return () => {
      mounted = false;
    };
  }, [appWindow]);

  const handleMaximize = () => {
    appWindow
      .toggleMaximize()
      .then(() => appWindow.isMaximized())
      .then((value) => setIsMaximized(value))
      .catch(() => setIsMaximized(false));
  };

  return (
    <div
      className="titlebar-root glass-titlebar fixed left-0 top-0 z-50 h-12 w-full border-b border-zinc-100/10"
      data-tauri-drag-region
    >
      <div className="absolute inset-0" data-tauri-drag-region />

      <div
        className="absolute left-3 top-0 z-[60] flex h-full items-center gap-2"
        data-tauri-drag-region="false"
        style={noDragStyle}
        onMouseDown={(e) => e.stopPropagation()}
      >
        <button
          className={`rounded-md border px-3 py-1 text-xs font-medium transition ${
            activeTab === "home"
              ? "border-zinc-100/35 bg-zinc-100/12 text-zinc-100"
              : "border-zinc-700/80 bg-zinc-900/60 text-zinc-300 hover:border-zinc-500/80 hover:text-zinc-100"
          }`}
          style={noDragStyle}
          onClick={() => onTabChange("home")}
          aria-label="Open Home tab"
        >
          Home
        </button>
        <button
          className={`rounded-md border px-3 py-1 text-xs font-medium transition ${
            activeTab === "settings"
              ? "border-zinc-100/35 bg-zinc-100/12 text-zinc-100"
              : "border-zinc-700/80 bg-zinc-900/60 text-zinc-300 hover:border-zinc-500/80 hover:text-zinc-100"
          }`}
          style={noDragStyle}
          onClick={() => onTabChange("settings")}
          aria-label="Open Settings tab"
        >
          Settings
        </button>
        <button
          className={`rounded-md border px-3 py-1 text-xs font-medium transition ${
            activeTab === "engine"
              ? "border-zinc-100/35 bg-zinc-100/12 text-zinc-100"
              : "border-zinc-700/80 bg-zinc-900/60 text-zinc-300 hover:border-zinc-500/80 hover:text-zinc-100"
          }`}
          style={noDragStyle}
          onClick={() => onTabChange("engine")}
          aria-label="Open Engine tab"
        >
          Engine
        </button>
      </div>

      <div className="pointer-events-none absolute left-1/2 top-1/2 z-[55] -translate-x-1/2 -translate-y-1/2">
        <span className="text-xs font-semibold uppercase tracking-[0.18em] text-zinc-200/95">{title}</span>
      </div>

      <div
        className="titlebar-controls absolute right-2 top-0 z-[60] flex h-full items-center gap-1"
        data-tauri-drag-region="false"
        style={noDragStyle}
        onMouseDown={(e) => e.stopPropagation()}
      >
        <button
          className="titlebar-control-btn pointer-events-auto relative z-[60]"
          style={noDragStyle}
          onClick={() => appWindow.minimize()}
          aria-label="Minimize window"
        >
          <svg viewBox="0 0 16 16" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.5">
            <path d="M3 8.5h10" />
          </svg>
        </button>

        <button
          className="titlebar-control-btn pointer-events-auto relative z-[60]"
          style={noDragStyle}
          onClick={handleMaximize}
          aria-label="Maximize or restore window"
        >
          {isMaximized ? (
            <svg viewBox="0 0 16 16" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.5">
              <path d="M5 3.5h7.5V11" />
              <rect x="3.5" y="5" width="7.5" height="7.5" rx="1" />
            </svg>
          ) : (
            <svg viewBox="0 0 16 16" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.5">
              <rect x="3.5" y="3.5" width="9" height="9" rx="1" />
            </svg>
          )}
        </button>

        <button
          className="titlebar-control-btn titlebar-control-close pointer-events-auto relative z-[60]"
          style={noDragStyle}
          onClick={() => appWindow.close()}
          aria-label="Close window"
        >
          <svg viewBox="0 0 16 16" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.5">
            <path d="M4 4l8 8" />
            <path d="M12 4l-8 8" />
          </svg>
        </button>
      </div>
    </div>
  );
}
