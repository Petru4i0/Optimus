import { createPortal } from "react-dom";
import {
  CSSProperties,
  ReactNode,
  RefObject,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";

type PopoverMenuProps = {
  open: boolean;
  anchorRef: RefObject<HTMLElement | null>;
  onClose: () => void;
  children: ReactNode;
  className?: string;
  width?: number | "anchor";
  zIndex?: number;
  spacing?: number;
  viewportPadding?: number;
  openUpThreshold?: number;
  maxHeight?: number;
  minHeight?: number;
  role?: string;
};

export default function PopoverMenu({
  open,
  anchorRef,
  onClose,
  children,
  className,
  width = "anchor",
  zIndex = 9999,
  spacing = 6,
  viewportPadding = 8,
  openUpThreshold = 170,
  maxHeight,
  minHeight = 120,
  role,
}: PopoverMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);
  const [style, setStyle] = useState<CSSProperties>({});

  const updateMenuPosition = useCallback(() => {
    const anchor = anchorRef.current;
    if (!anchor) {
      return;
    }

    const rect = anchor.getBoundingClientRect();
    const belowSpace = window.innerHeight - rect.bottom - viewportPadding;
    const aboveSpace = rect.top - viewportPadding;
    const openUp = belowSpace < openUpThreshold && aboveSpace > belowSpace;
    const menuWidth = width === "anchor" ? rect.width : width;
    const left = Math.max(
      viewportPadding,
      Math.min(rect.left, window.innerWidth - menuWidth - viewportPadding),
    );

    if (typeof maxHeight === "number") {
      const boundedMaxHeight = Math.max(
        minHeight,
        Math.min(maxHeight, openUp ? aboveSpace : belowSpace),
      );
      const top = openUp
        ? Math.max(viewportPadding, rect.top - boundedMaxHeight - spacing)
        : rect.bottom + spacing;

      setStyle({
        position: "fixed",
        left,
        top,
        width: menuWidth,
        maxHeight: boundedMaxHeight,
        zIndex,
      });
      return;
    }

    const top = openUp ? rect.top - spacing : rect.bottom + spacing;
    setStyle({
      position: "fixed",
      left,
      top,
      width: menuWidth,
      zIndex,
      transform: openUp ? "translateY(-100%)" : "none",
    });
  }, [
    anchorRef,
    maxHeight,
    minHeight,
    openUpThreshold,
    spacing,
    viewportPadding,
    width,
    zIndex,
  ]);

  useEffect(() => {
    if (!open) {
      return;
    }

    updateMenuPosition();
    const onResizeOrScroll = () => updateMenuPosition();
    const onPointerDown = (event: MouseEvent) => {
      const target = event.target as Node;
      const inAnchor = anchorRef.current?.contains(target);
      const inMenu = menuRef.current?.contains(target);
      if (!inAnchor && !inMenu) {
        onClose();
      }
    };
    const onEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };

    window.addEventListener("resize", onResizeOrScroll);
    window.addEventListener("scroll", onResizeOrScroll, true);
    window.addEventListener("mousedown", onPointerDown);
    window.addEventListener("keydown", onEscape);

    return () => {
      window.removeEventListener("resize", onResizeOrScroll);
      window.removeEventListener("scroll", onResizeOrScroll, true);
      window.removeEventListener("mousedown", onPointerDown);
      window.removeEventListener("keydown", onEscape);
    };
  }, [anchorRef, onClose, open, updateMenuPosition]);

  if (!open) {
    return null;
  }

  return createPortal(
    <div ref={menuRef} style={style} className={className} role={role}>
      {children}
    </div>,
    document.body,
  );
}
