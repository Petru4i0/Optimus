import { useCallback, useMemo, useState } from "react";
import { ToastKind } from "../types/config";
import { ProcessGroupDto } from "../types/process";
import { useConfigBuilder } from "./useConfigBuilder";
import { useConfigCrud } from "./useConfigCrud";

type PushToast = (kind: ToastKind, message: string) => void;
type RefreshProcesses = (silent?: boolean) => Promise<void>;

function equalsIgnoreCase(a: string, b: string) {
  return a.toLowerCase() === b.toLowerCase();
}

function hasConfigName(configs: { name: string }[], name: string) {
  return configs.some((config) => equalsIgnoreCase(config.name, name));
}

export function useConfigManager(
  pushToast: PushToast,
  groups: ProcessGroupDto[],
  refreshProcesses: RefreshProcesses,
) {
  const [homeSavedOpen, setHomeSavedOpen] = useState(false);
  const [settingsSavedOpen, setSettingsSavedOpen] = useState(true);
  const [watchTriggerApp, setWatchTriggerApp] = useState("");
  const [watchConfigName, setWatchConfigName] = useState("");

  const builder = useConfigBuilder(pushToast);
  const crud = useConfigCrud(pushToast, groups, refreshProcesses, equalsIgnoreCase);

  const {
    builderName,
    setBuilderName,
    builderTargetApp,
    setBuilderTargetApp,
    builderTargetPriority,
    setBuilderTargetPriority,
    builderTargets,
    onAddBuilderTarget,
    onRemoveBuilderTarget,
    clearBuilder,
  } = builder;

  const {
    configs,
    watchdogConfig,
    loadConfigs,
    loadWatchdog,
    saveConfig,
    onApplyConfig,
    onDeleteConfig,
    onExportConfig,
    onImportConfig,
    onCreateShortcut,
    onSetSticky,
    onAddWatchMapping: crudAddWatchMapping,
    onRemoveWatchMapping,
  } = crud;

  const onSaveConfig = useCallback(async () => {
    const name = builderName.trim();
    if (!name) {
      pushToast("error", "Enter config name");
      return;
    }

    if (Object.keys(builderTargets).length === 0) {
      pushToast("error", "Add at least one target");
      return;
    }

    try {
      await saveConfig(name, builderTargets);
      pushToast("success", `Config '${name}' saved`);
      clearBuilder();
    } catch (invokeError) {
      const message = invokeError instanceof Error ? invokeError.message : String(invokeError);
      pushToast("error", message);
    }
  }, [builderName, builderTargets, clearBuilder, pushToast, saveConfig]);

  const onAddWatchMapping = useCallback(async () => {
    const appName = watchTriggerApp.trim();
    const configName = watchConfigName.trim();

    if (!appName) {
      pushToast("error", "Select trigger application");
      return;
    }

    if (!configName) {
      pushToast("error", "Select a config");
      return;
    }

    if (!hasConfigName(configs, configName)) {
      pushToast("error", "Selected config no longer exists");
      return;
    }

    const selectedGroup =
      groups.find((group) => equalsIgnoreCase(group.appName, appName)) ?? null;

    await crudAddWatchMapping(appName, configName, selectedGroup?.iconBase64);
    setWatchTriggerApp("");
  }, [configs, crudAddWatchMapping, groups, pushToast, watchConfigName, watchTriggerApp]);

  const sortedMappings = useMemo(
    () =>
      Object.entries(watchdogConfig.triggerMap).sort((a, b) =>
        a[0].toLowerCase().localeCompare(b[0].toLowerCase()),
      ),
    [watchdogConfig.triggerMap],
  );

  return {
    configs,
    homeSavedOpen,
    settingsSavedOpen,
    setHomeSavedOpen,
    setSettingsSavedOpen,
    builderName,
    setBuilderName,
    builderTargetApp,
    setBuilderTargetApp,
    builderTargetPriority,
    setBuilderTargetPriority,
    builderTargets,
    watchTriggerApp,
    setWatchTriggerApp,
    watchConfigName,
    setWatchConfigName,
    watchdogConfig,
    loadConfigs,
    loadWatchdog,
    onAddBuilderTarget,
    onRemoveBuilderTarget,
    onSaveConfig,
    onApplyConfig,
    onDeleteConfig,
    onExportConfig,
    onImportConfig,
    onCreateShortcut,
    onSetSticky,
    onAddWatchMapping,
    onRemoveWatchMapping,
    sortedMappings,
    equalsIgnoreCase,
  };
}


