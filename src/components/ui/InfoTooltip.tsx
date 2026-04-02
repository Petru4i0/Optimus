import { useCallback, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { HelpCircle } from "lucide-react";
import { useTranslation } from "../../hooks/useTranslation";
import type { Translation } from "../../locales/types";

type TooltipPlacementY = "top" | "bottom";
type TooltipPlacementX = "left" | "center" | "right";

type TooltipPosition = {
  top: number;
  left: number;
  placementY: TooltipPlacementY;
  placementX: TooltipPlacementX;
};

type InfoTooltipProps = {
  translationKey?: keyof Translation["tooltips"];
  text?: string;
};

const VIEWPORT_GAP = 12;
const TOOLTIP_GAP = 10;

export default function InfoTooltip({ translationKey, text: textProp }: InfoTooltipProps) {
  const t = useTranslation();
  const text = translationKey ? t.tooltips[translationKey] : textProp ?? "";
  const tooltipText = text || "";
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const tooltipRef = useRef<HTMLDivElement | null>(null);
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState<TooltipPosition | null>(null);

  const updatePosition = useCallback(() => {
    const trigger = triggerRef.current;
    const tooltip = tooltipRef.current;
    if (!trigger || !tooltip) {
      return;
    }

    const triggerRect = trigger.getBoundingClientRect();
    const tooltipRect = tooltip.getBoundingClientRect();

    let placementY: TooltipPlacementY = "bottom";
    let placementX: TooltipPlacementX = "center";

    let top = triggerRect.bottom + TOOLTIP_GAP;
    if (top + tooltipRect.height + VIEWPORT_GAP > window.innerHeight) {
      placementY = "top";
      top = Math.max(VIEWPORT_GAP, triggerRect.top - tooltipRect.height - TOOLTIP_GAP);
    }

    let left = triggerRect.left + triggerRect.width / 2 - tooltipRect.width / 2;
    if (left + tooltipRect.width + VIEWPORT_GAP > window.innerWidth) {
      placementX = "right";
      left = triggerRect.right - tooltipRect.width;
    }
    if (left < VIEWPORT_GAP) {
      placementX = "left";
      left = VIEWPORT_GAP;
    }
    if (left + tooltipRect.width + VIEWPORT_GAP > window.innerWidth) {
      left = Math.max(VIEWPORT_GAP, window.innerWidth - tooltipRect.width - VIEWPORT_GAP);
    }

    setPosition({ top, left, placementY, placementX });
  }, []);

  useLayoutEffect(() => {
    if (!open) {
      setPosition(null);
      return;
    }

    updatePosition();

    const handleWindowChange = () => updatePosition();
    window.addEventListener("scroll", handleWindowChange, true);
    window.addEventListener("resize", handleWindowChange);

    return () => {
      window.removeEventListener("scroll", handleWindowChange, true);
      window.removeEventListener("resize", handleWindowChange);
    };
  }, [open, updatePosition, tooltipText]);

  const originClass =
    position?.placementY === "top"
      ? position.placementX === "right"
        ? "origin-bottom-right"
        : position?.placementX === "left"
          ? "origin-bottom-left"
          : "origin-bottom"
      : position?.placementX === "right"
        ? "origin-top-right"
        : position?.placementX === "left"
          ? "origin-top-left"
          : "origin-top";

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        aria-label="Feature explanation"
        className="group inline-flex h-5 w-5 items-center justify-center rounded-full text-zinc-400 transition hover:text-zinc-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-500/50"
        onMouseEnter={() => setOpen(true)}
        onMouseLeave={() => setOpen(false)}
        onFocus={() => setOpen(true)}
        onBlur={() => setOpen(false)}
      >
        <HelpCircle className="h-4 w-4 transition-transform duration-150 group-hover:scale-105" />
      </button>

      {open
        ? createPortal(
            <div
              ref={tooltipRef}
              className={`pointer-events-none fixed z-[9999] max-w-[20rem] rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-xs leading-5 text-zinc-100 shadow-2xl backdrop-blur-md transition-[opacity,transform] duration-150 ${originClass} ${position ? "opacity-100" : "opacity-0"}`}
              style={{
                top: position?.top ?? -9999,
                left: position?.left ?? -9999,
              }}
              role="tooltip"
            >
              <div className="whitespace-normal break-words">{tooltipText}</div>
            </div>,
            document.body,
          )
        : null}
    </>
  );
}
