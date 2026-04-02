import { CSSProperties } from "react";

export type AppTab = "home" | "settings" | "engine" | "optimization";

type TopNavProps = {
  activeTab: AppTab;
  onTabChange: (tab: AppTab) => void;
  noDragStyle: CSSProperties;
};

export default function TopNav({ activeTab, onTabChange, noDragStyle }: TopNavProps) {
  return (
    <div
      className="titlebar-no-drag flex items-center gap-2 pointer-events-auto"
      style={noDragStyle}
      onMouseDown={(e) => e.stopPropagation()}
    >
      <button
        className={`rounded-md border px-3 py-1 text-xs font-medium transition ${
          activeTab === "home"
            ? "border-zinc-500 bg-zinc-800 text-zinc-100"
            : "border-zinc-800 bg-zinc-900 text-zinc-400 hover:border-zinc-500 hover:text-zinc-100"
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
            ? "border-zinc-500 bg-zinc-800 text-zinc-100"
            : "border-zinc-800 bg-zinc-900 text-zinc-400 hover:border-zinc-500 hover:text-zinc-100"
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
            ? "border-zinc-500 bg-zinc-800 text-zinc-100"
            : "border-zinc-800 bg-zinc-900 text-zinc-400 hover:border-zinc-500 hover:text-zinc-100"
        }`}
        style={noDragStyle}
        onClick={() => onTabChange("engine")}
        aria-label="Open Engine tab"
      >
        Engine
      </button>
      <button
        className={`rounded-md border px-3 py-1 text-xs font-medium transition ${
          activeTab === "optimization"
            ? "border-zinc-500 bg-zinc-800 text-zinc-100"
            : "border-zinc-800 bg-zinc-900 text-zinc-400 hover:border-zinc-500 hover:text-zinc-100"
        }`}
        style={noDragStyle}
        onClick={() => onTabChange("optimization")}
        aria-label="Open Optimization tab"
      >
        Optimization
      </button>
    </div>
  );
}
