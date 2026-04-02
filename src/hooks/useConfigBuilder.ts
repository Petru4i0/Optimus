import { useCallback, useState } from "react";
import { ToastKind } from "../types/config";
import { PriorityClass } from "../types/process";

type PushToast = (kind: ToastKind, message: string) => void;

export function useConfigBuilder(pushToast: PushToast) {
  const [builderName, setBuilderName] = useState("");
  const [builderTargetApp, setBuilderTargetApp] = useState("");
  const [builderTargetPriority, setBuilderTargetPriority] = useState<PriorityClass>("normal");
  const [builderTargets, setBuilderTargets] = useState<Record<string, PriorityClass>>({});

  const onAddBuilderTarget = useCallback(() => {
    const appName = builderTargetApp.trim();
    if (!appName) {
      pushToast("error", "Select target app");
      return;
    }

    setBuilderTargets((prev) => ({ ...prev, [appName]: builderTargetPriority }));
    pushToast("success", `${appName} mapped to ${builderTargetPriority}`);
  }, [builderTargetApp, builderTargetPriority, pushToast]);

  const onRemoveBuilderTarget = useCallback((appName: string) => {
    setBuilderTargets((prev) => {
      const { [appName]: _removed, ...rest } = prev;
      return rest;
    });
  }, []);

  const clearBuilder = useCallback(() => {
    setBuilderName("");
    setBuilderTargets({});
  }, []);

  return {
    builderName,
    setBuilderName,
    builderTargetApp,
    setBuilderTargetApp,
    builderTargetPriority,
    setBuilderTargetPriority,
    builderTargets,
    setBuilderTargets,
    onAddBuilderTarget,
    onRemoveBuilderTarget,
    clearBuilder,
  };
}


