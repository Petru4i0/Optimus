import { useCallback, useEffect, useRef, useState } from "react";
import { ToastKind, ToastMessage } from "../types/config";

export function useToastManager() {
  const [toasts, setToasts] = useState<ToastMessage[]>([]);
  const toastId = useRef(0);
  const toastTimersRef = useRef<Map<number, number>>(new Map());

  const pushToast = useCallback((kind: ToastKind, message: string) => {
    toastId.current += 1;
    const id = toastId.current;
    setToasts((prev) => [...prev, { id, kind, message }]);
    const timeoutId = window.setTimeout(() => {
      setToasts((prev) => prev.filter((item) => item.id !== id));
      toastTimersRef.current.delete(id);
    }, 4200);
    toastTimersRef.current.set(id, timeoutId);
  }, []);

  const dismissToast = useCallback((id: number) => {
    const timeoutId = toastTimersRef.current.get(id);
    if (timeoutId !== undefined) {
      window.clearTimeout(timeoutId);
      toastTimersRef.current.delete(id);
    }
    setToasts((prev) => prev.filter((item) => item.id !== id));
  }, []);

  useEffect(() => {
    return () => {
      for (const timeoutId of toastTimersRef.current.values()) {
        window.clearTimeout(timeoutId);
      }
      toastTimersRef.current.clear();
    };
  }, []);

  return {
    toasts,
    pushToast,
    dismissToast,
  };
}

