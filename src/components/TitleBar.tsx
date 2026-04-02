import { type CSSProperties } from "react";
import TopNav, { AppTab } from "./TopNav";
import WindowChrome from "./WindowChrome";

type TitleBarProps = {
  activeTab: AppTab;
  onTabChange: (tab: AppTab) => void;
  title?: string;
};

export default function TitleBar({ activeTab, onTabChange, title = "Optimus" }: TitleBarProps) {
  const noDragStyle = { WebkitAppRegion: "no-drag" } as CSSProperties;

  return (
    <div
      className="titlebar-root glass-titlebar fixed left-0 top-0 z-50 h-12 w-full border-b border-zinc-500"
      data-tauri-drag-region
    >
      <div className="relative z-10 flex h-full w-full items-center justify-between px-3 pointer-events-none">
        <TopNav activeTab={activeTab} onTabChange={onTabChange} noDragStyle={noDragStyle} />

        <div className="pointer-events-none absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2">
          <span className="text-xs font-semibold uppercase tracking-[0.18em] text-zinc-100">
            {title}
          </span>
        </div>

        <WindowChrome noDragStyle={noDragStyle} />
      </div>
    </div>
  );
}
