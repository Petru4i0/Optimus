import { getCurrentWindow } from "@tauri-apps/api/window";
import { CSSProperties, useEffect, useState } from "react";

type WindowChromeProps = {
  noDragStyle: CSSProperties;
};

export default function WindowChrome({ noDragStyle }: WindowChromeProps) {
  const appWindow = getCurrentWindow();
  const [isMaximized, setIsMaximized] = useState(false);

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
      .catch((error) => {
        setIsMaximized(false);
        console.error(error);
      });
  };

  return (
    <div
      className="titlebar-controls titlebar-no-drag pointer-events-auto"
      style={noDragStyle}
      onMouseDown={(e) => e.stopPropagation()}
    >
      <button
        className="titlebar-control-btn"
        style={noDragStyle}
        onClick={() => {
          appWindow.minimize().catch(console.error);
        }}
        aria-label="Minimize window"
      >
        <svg viewBox="0 0 16 16" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.5">
          <path d="M3 8.5h10" />
        </svg>
      </button>

      <button
        className="titlebar-control-btn"
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
        className="titlebar-control-btn titlebar-control-close"
        style={noDragStyle}
        onClick={() => {
          appWindow.close().catch(console.error);
        }}
        aria-label="Close window"
      >
        <svg viewBox="0 0 16 16" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.5">
          <path d="M4 4l8 8" />
          <path d="M12 4l-8 8" />
        </svg>
      </button>
    </div>
  );
}
