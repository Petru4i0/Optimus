pub(crate) mod backup_manager;
pub(crate) mod mmcss;

use crate::core::{error, warn};
use crate::core::{is_running_as_admin, AtomicBool, OnceLock, Ordering};
use crate::net_tuning::{
    apply_advanced_net_tweaks, apply_tcp_tweaks, capture_active_dns_snapshots,
    capture_active_tcp_registry_snapshots, capture_registry_throttling_snapshot,
    restore_cloudflare_dns_from_snapshot, restore_interrupt_moderation_from_snapshot,
    restore_registry_throttling_from_snapshot, restore_tcp_tweaks_from_snapshot, set_cloudflare_dns,
    apply_interrupt_moderation, capture_interrupt_moderation_snapshots, check_interrupt_moderation_status,
    InterruptModerationStatus,
};
use crate::power::{
    capture_power_snapshot_state, disable_core_parking, enable_ultimate_performance,
    restore_power_from_snapshot,
};
use crate::power::timers::{
    apply_hpet_dynamic_tick, capture_timer_bcd_snapshot, check_hpet_dynamic_tick_status,
    restore_bcd_value, restore_timer_defaults, TimerOptimizationStatus,
};
use crate::settings_repo::save_runtime_settings;
use crate::telemetry::{
    apply_telemetry_registry_policies, apply_telemetry_services_hard_kill, block_telemetry_hosts,
    capture_telemetry_hosts_snapshot, capture_telemetry_policy_snapshot,
    capture_telemetry_services_snapshot, capture_telemetry_tasks_snapshot, deep_purge_tasks,
    restore_telemetry_from_snapshot,
};
use crate::types::{
    AdvancedStatusDto, AppError, NetSniperStatusDto, OptimizationDesiredState,
    OptimizationStatusDto, PowerStatusDto, RuntimeControlState, TelemetryStatusDto,
};
use tauri::AppHandle;

const RECONCILE_INTERVAL_SECS: u64 = 60;

fn non_admin_reconcile_warned() -> &'static AtomicBool {
    static WARNED: OnceLock<AtomicBool> = OnceLock::new();
    WARNED.get_or_init(|| AtomicBool::new(false))
}

fn desired_any_enabled(desired: &OptimizationDesiredState) -> bool {
    desired.telemetry.any_enabled()
        || desired.net_sniper.any_enabled()
        || desired.power_mode.any_enabled()
        || desired.advanced.any_enabled()
}

fn telemetry_category_verified(status: &TelemetryStatusDto) -> bool {
    status.services_readable
        && status.registry_policies_readable
        && status.scheduled_tasks_readable
        && status.hosts_readable
        && status.services_disabled
        && status.registry_policies_disabled
        && status.scheduled_tasks_disabled
        && status.hosts_blocked
}

fn net_category_verified(status: &NetSniperStatusDto) -> bool {
    status.tcp_tweaks_readable
        && status.registry_throttling_readable
        && status.cloudflare_dns_readable
        && status.tcp_tweaks_applied
        && status.registry_throttling_applied
        && status.cloudflare_dns_applied
}

fn power_category_verified(status: &PowerStatusDto) -> bool {
    status.ultimate_plan_readable
        && status.core_parking_readable
        && status.ultimate_plan_active
        && status.core_parking_disabled
}

fn advanced_category_verified(status: &AdvancedStatusDto) -> bool {
    status.hpet_dynamic_tick_readable
        && status.interrupt_moderation_readable
        && status.mmcss_readable
        && status.hpet_dynamic_tick_applied
        && status.interrupt_moderation_applied
        && status.mmcss_applied
}

fn fallback_timer_status() -> TimerOptimizationStatus {
    TimerOptimizationStatus {
        applied: false,
        readable: false,
    }
}

fn fallback_interrupt_status() -> InterruptModerationStatus {
    InterruptModerationStatus {
        adapters_total: 0,
        adapters_tuned: 0,
        readable: false,
        applied: false,
    }
}

fn fallback_mmcss_status() -> mmcss::MmcssStatus {
    mmcss::MmcssStatus {
        applied: false,
        readable: false,
    }
}

fn build_advanced_status(
    hpet_status: Result<TimerOptimizationStatus, AppError>,
    interrupt_status: Result<InterruptModerationStatus, AppError>,
    mmcss_status: Result<mmcss::MmcssStatus, AppError>,
) -> AdvancedStatusDto {
    let hpet_status = match hpet_status {
        Ok(status) => status,
        Err(err) => {
            error!("failed to collect advanced sub-feature status 'hpet_dynamic_tick': {err}");
            fallback_timer_status()
        }
    };
    let interrupt_status = match interrupt_status {
        Ok(status) => status,
        Err(err) => {
            error!("failed to collect advanced sub-feature status 'interrupt_moderation': {err}");
            fallback_interrupt_status()
        }
    };
    let mmcss_status = match mmcss_status {
        Ok(status) => status,
        Err(err) => {
            error!("failed to collect advanced sub-feature status 'mmcss': {err}");
            fallback_mmcss_status()
        }
    };

    let mut status = AdvancedStatusDto {
        verified: false,
        hpet_dynamic_tick_applied: hpet_status.applied,
        interrupt_moderation_applied: interrupt_status.applied,
        mmcss_applied: mmcss_status.applied,
        hpet_dynamic_tick_readable: hpet_status.readable,
        interrupt_moderation_readable: interrupt_status.readable,
        mmcss_readable: mmcss_status.readable,
        interrupt_moderation_adapters_total: interrupt_status.adapters_total,
        interrupt_moderation_adapters_tuned: interrupt_status.adapters_tuned,
    };
    status.verified = advanced_category_verified(&status);
    status
}

fn collect_advanced_status() -> Result<AdvancedStatusDto, AppError> {
    Ok(build_advanced_status(
        check_hpet_dynamic_tick_status(),
        check_interrupt_moderation_status(),
        mmcss::check_mmcss_status(),
    ))
}

fn fallback_telemetry_status() -> TelemetryStatusDto {
    TelemetryStatusDto {
        verified: false,
        services_disabled: false,
        registry_policies_disabled: false,
        scheduled_tasks_disabled: false,
        hosts_blocked: false,
        services_readable: false,
        registry_policies_readable: false,
        scheduled_tasks_readable: false,
        hosts_readable: false,
    }
}

fn fallback_net_sniper_status() -> NetSniperStatusDto {
    NetSniperStatusDto {
        verified: false,
        tcp_tweaks_applied: false,
        registry_throttling_applied: false,
        cloudflare_dns_applied: false,
        tcp_tweaks_readable: false,
        registry_throttling_readable: false,
        cloudflare_dns_readable: false,
        interfaces_total: 0,
        interfaces_tuned: 0,
        dns_interfaces_total: 0,
        dns_interfaces_tuned: 0,
    }
}

fn fallback_power_status() -> PowerStatusDto {
    PowerStatusDto {
        verified: false,
        ultimate_plan_active: false,
        core_parking_disabled: false,
        ultimate_plan_readable: false,
        core_parking_readable: false,
    }
}

fn fallback_advanced_status() -> AdvancedStatusDto {
    AdvancedStatusDto {
        verified: false,
        hpet_dynamic_tick_applied: false,
        interrupt_moderation_applied: false,
        mmcss_applied: false,
        hpet_dynamic_tick_readable: false,
        interrupt_moderation_readable: false,
        mmcss_readable: false,
        interrupt_moderation_adapters_total: 0,
        interrupt_moderation_adapters_tuned: 0,
    }
}

pub(crate) fn collect_status(
    _runtime: &RuntimeControlState,
) -> Result<OptimizationStatusDto, AppError> {
    let mut telemetry = match crate::telemetry::check_telemetry_status() {
        Ok(status) => status,
        Err(err) => {
            error!("failed to collect telemetry optimization status: {err}");
            fallback_telemetry_status()
        }
    };
    let mut net_sniper = match crate::net_tuning::check_net_sniper_status() {
        Ok(status) => status,
        Err(err) => {
            error!("failed to collect internet optimization status: {err}");
            fallback_net_sniper_status()
        }
    };
    let mut power_mode = match crate::power::check_power_status() {
        Ok(status) => status,
        Err(err) => {
            error!("failed to collect power optimization status: {err}");
            fallback_power_status()
        }
    };
    let mut advanced = match collect_advanced_status() {
        Ok(status) => status,
        Err(err) => {
            error!("failed to collect advanced optimization status: {err}");
            fallback_advanced_status()
        }
    };

    telemetry.verified = telemetry_category_verified(&telemetry);
    net_sniper.verified = net_category_verified(&net_sniper);
    power_mode.verified = power_category_verified(&power_mode);
    advanced.verified = advanced_category_verified(&advanced);

    Ok(OptimizationStatusDto {
        telemetry,
        net_sniper,
        power_mode,
        advanced,
    })
}

fn persist_desired_state(
    app: &AppHandle,
    runtime: &RuntimeControlState,
    desired: OptimizationDesiredState,
) -> Result<(), AppError> {
    {
        let mut guard = runtime
            .optimization_desired
            .write()
            .map_err(|_| AppError::Message("optimization desired state lock poisoned".to_owned()))?;
        *guard = desired;
    }
    save_runtime_settings(app, runtime)
}

fn with_desired_state<F>(
    app: &AppHandle,
    runtime: &RuntimeControlState,
    mutator: F,
) -> Result<OptimizationDesiredState, AppError>
where
    F: FnOnce(&mut OptimizationDesiredState),
{
    let mut desired = *runtime
        .optimization_desired
        .read()
        .map_err(|_| AppError::Message("optimization desired state lock poisoned".to_owned()))?;
    mutator(&mut desired);
    persist_desired_state(app, runtime, desired)?;
    Ok(desired)
}

fn maybe_delete_snapshot_if_category_clean(
    app: &AppHandle,
    desired: OptimizationDesiredState,
) -> Result<(), AppError> {
    if !desired.net_sniper.any_enabled() {
        backup_manager::delete_internet_snapshot(app)?;
    }
    if !desired.telemetry.any_enabled() {
        backup_manager::delete_telemetry_snapshot(app)?;
    }
    if !desired.power_mode.any_enabled() {
        backup_manager::delete_power_snapshot(app)?;
    }
    if !desired.advanced.any_enabled() {
        backup_manager::delete_advanced_snapshot(app)?;
    }
    Ok(())
}

fn apply_telemetry_feature(app: &AppHandle, feature: &str) -> Result<(), AppError> {
    match feature {
        "services" => {
            capture_telemetry_services_snapshot(app)?;
            apply_telemetry_services_hard_kill()
        }
        "registry_policies" => {
            capture_telemetry_policy_snapshot(app)?;
            apply_telemetry_registry_policies()
        }
        "scheduled_tasks" => {
            capture_telemetry_tasks_snapshot(app)?;
            deep_purge_tasks()
        }
        "hosts_block" => {
            capture_telemetry_hosts_snapshot(app)?;
            block_telemetry_hosts()
        }
        other => Err(AppError::Message(format!(
            "Unknown telemetry feature '{other}'"
        ))),
    }
}

fn rollback_telemetry_feature(app: &AppHandle, feature: &str) -> Result<(), AppError> {
    restore_telemetry_from_snapshot(app, Some(feature))
}

fn apply_net_feature(app: &AppHandle, feature: &str) -> Result<(), AppError> {
    match feature {
        "tcp_tweaks" => {
            capture_active_tcp_registry_snapshots(app)?;
            apply_tcp_tweaks()
        }
        "registry_throttling" => {
            capture_registry_throttling_snapshot(app)?;
            apply_advanced_net_tweaks()
        }
        "cloudflare_dns" => {
            capture_active_dns_snapshots(app)?;
            set_cloudflare_dns()
        }
        other => Err(AppError::Message(format!(
            "Unknown internet feature '{other}'"
        ))),
    }
}

fn rollback_net_feature(app: &AppHandle, feature: &str) -> Result<(), AppError> {
    match feature {
        "tcp_tweaks" => restore_tcp_tweaks_from_snapshot(app),
        "registry_throttling" => restore_registry_throttling_from_snapshot(app),
        "cloudflare_dns" => restore_cloudflare_dns_from_snapshot(app),
        other => Err(AppError::Message(format!(
            "Unknown internet feature '{other}'"
        ))),
    }
}

fn apply_power_feature(app: &AppHandle, feature: &str) -> Result<(), AppError> {
    capture_power_snapshot_state(app, feature)?;
    match feature {
        "ultimate_plan" => enable_ultimate_performance(),
        "core_parking" => disable_core_parking(),
        other => Err(AppError::Message(format!("Unknown power feature '{other}'"))),
    }
}

fn rollback_power_feature(app: &AppHandle, feature: &str) -> Result<(), AppError> {
    restore_power_from_snapshot(app, Some(feature))
}

fn apply_advanced_feature(app: &AppHandle, feature: &str) -> Result<(), AppError> {
    match feature {
        "hpet_dynamic_tick" => {
            capture_timer_bcd_snapshot(app)?;
            apply_hpet_dynamic_tick()
        }
        "interrupt_moderation" => {
            capture_interrupt_moderation_snapshots(app)?;
            apply_interrupt_moderation()
        }
        "mmcss" => {
            mmcss::capture_mmcss_snapshot(app)?;
            mmcss::apply_mmcss()
        }
        other => Err(AppError::Message(format!(
            "Unknown advanced feature '{other}'"
        ))),
    }
}

fn rollback_advanced_feature(app: &AppHandle, feature: &str) -> Result<(), AppError> {
    match feature {
        "hpet_dynamic_tick" => {
            let Some(snapshot) = backup_manager::load_advanced_snapshot(app)? else {
                warn!("advanced snapshot missing; restoring timer overrides to defaults");
                return restore_timer_defaults();
            };
            for entry in snapshot.bcd_entries.iter().filter(|entry| {
                entry.name.eq_ignore_ascii_case("useplatformclock")
                    || entry.name.eq_ignore_ascii_case("disabledynamictick")
            }) {
                restore_bcd_value(entry)?;
            }
            Ok(())
        }
        "interrupt_moderation" => restore_interrupt_moderation_from_snapshot(app),
        "mmcss" => mmcss::restore_mmcss_from_snapshot(app),
        other => Err(AppError::Message(format!(
            "Unknown advanced feature '{other}'"
        ))),
    }
}

pub(crate) fn toggle_telemetry(
    app: &AppHandle,
    runtime: &RuntimeControlState,
    sub_feature: &str,
    enabled: bool,
) -> Result<OptimizationStatusDto, AppError> {
    let features: &[&str] = match sub_feature {
        "all" => &["services", "registry_policies", "scheduled_tasks", "hosts_block"],
        "services" => &["services"],
        "registry_policies" | "registry_policy" => &["registry_policies"],
        "scheduled_tasks" | "tasks" => &["scheduled_tasks"],
        "hosts_block" | "hosts" => &["hosts_block"],
        other => {
            return Err(AppError::Message(format!(
                "Unknown telemetry sub-feature '{other}'. Expected: all, services, registry_policies, scheduled_tasks, hosts_block"
            )))
        }
    };

    for feature in features {
        if enabled {
            apply_telemetry_feature(app, feature)?;
        } else {
            rollback_telemetry_feature(app, feature)?;
        }
    }

    let desired = with_desired_state(app, runtime, |state| {
        for feature in features {
            match *feature {
                "services" => state.telemetry.services = enabled,
                "registry_policies" => state.telemetry.registry_policies = enabled,
                "scheduled_tasks" => state.telemetry.scheduled_tasks = enabled,
                "hosts_block" => state.telemetry.hosts_block = enabled,
                _ => {}
            }
        }
    })?;
    maybe_delete_snapshot_if_category_clean(app, desired)?;
    collect_status(runtime)
}

pub(crate) fn toggle_net_sniper(
    app: &AppHandle,
    runtime: &RuntimeControlState,
    sub_feature: &str,
    enabled: bool,
) -> Result<OptimizationStatusDto, AppError> {
    let features: &[&str] = match sub_feature {
        "all" => &["tcp_tweaks", "registry_throttling", "cloudflare_dns"],
        "tcp_tweaks" | "tcp" => &["tcp_tweaks"],
        "registry_throttling" | "throttling" => &["registry_throttling"],
        "cloudflare_dns" | "dns" => &["cloudflare_dns"],
        other => {
            return Err(AppError::Message(format!(
                "Unknown internet sub-feature '{other}'. Expected: all, tcp_tweaks, registry_throttling, cloudflare_dns"
            )))
        }
    };

    for feature in features {
        if enabled {
            apply_net_feature(app, feature)?;
        } else {
            rollback_net_feature(app, feature)?;
        }
    }

    let desired = with_desired_state(app, runtime, |state| {
        for feature in features {
            match *feature {
                "tcp_tweaks" => state.net_sniper.tcp_tweaks = enabled,
                "registry_throttling" => state.net_sniper.registry_throttling = enabled,
                "cloudflare_dns" => state.net_sniper.cloudflare_dns = enabled,
                _ => {}
            }
        }
    })?;
    maybe_delete_snapshot_if_category_clean(app, desired)?;
    collect_status(runtime)
}

pub(crate) fn toggle_power(
    app: &AppHandle,
    runtime: &RuntimeControlState,
    sub_feature: &str,
    enabled: bool,
) -> Result<OptimizationStatusDto, AppError> {
    let features: &[&str] = match sub_feature {
        "all" => &["ultimate_plan", "core_parking"],
        "ultimate_plan" | "ultimate" => &["ultimate_plan"],
        "core_parking" => &["core_parking"],
        other => {
            return Err(AppError::Message(format!(
                "Unknown power sub-feature '{other}'. Expected: all, ultimate_plan, core_parking"
            )))
        }
    };

    for feature in features {
        if enabled {
            apply_power_feature(app, feature)?;
        } else {
            rollback_power_feature(app, feature)?;
        }
    }

    let desired = with_desired_state(app, runtime, |state| {
        for feature in features {
            match *feature {
                "ultimate_plan" => state.power_mode.ultimate_plan = enabled,
                "core_parking" => state.power_mode.core_parking = enabled,
                _ => {}
            }
        }
    })?;
    maybe_delete_snapshot_if_category_clean(app, desired)?;
    collect_status(runtime)
}

pub(crate) fn toggle_advanced(
    app: &AppHandle,
    runtime: &RuntimeControlState,
    sub_feature: &str,
    enabled: bool,
) -> Result<OptimizationStatusDto, AppError> {
    let features: &[&str] = match sub_feature {
        "all" => &["hpet_dynamic_tick", "interrupt_moderation", "mmcss"],
        "hpet_dynamic_tick" | "timers" => &["hpet_dynamic_tick"],
        "interrupt_moderation" | "irq" => &["interrupt_moderation"],
        "mmcss" => &["mmcss"],
        other => {
            return Err(AppError::Message(format!(
                "Unknown advanced sub-feature '{other}'. Expected: all, hpet_dynamic_tick, interrupt_moderation, mmcss"
            )))
        }
    };

    for feature in features {
        if enabled {
            apply_advanced_feature(app, feature)?;
        } else {
            rollback_advanced_feature(app, feature)?;
        }
    }

    let desired = with_desired_state(app, runtime, |state| {
        for feature in features {
            match *feature {
                "hpet_dynamic_tick" => state.advanced.hpet_dynamic_tick = enabled,
                "interrupt_moderation" => state.advanced.interrupt_moderation = enabled,
                "mmcss" => state.advanced.mmcss = enabled,
                _ => {}
            }
        }
    })?;
    maybe_delete_snapshot_if_category_clean(app, desired)?;
    collect_status(runtime)
}

pub(crate) fn sync_desired_state_from_settings(
    runtime: &RuntimeControlState,
    desired: OptimizationDesiredState,
) -> Result<(), AppError> {
    let mut guard = runtime
        .optimization_desired
        .write()
        .map_err(|_| AppError::Message("optimization desired state lock poisoned".to_owned()))?;
    *guard = desired;
    Ok(())
}

pub(crate) fn reconcile_desired_state(
    app: &AppHandle,
    runtime: &RuntimeControlState,
) -> Result<(), AppError> {
    let desired = *runtime
        .optimization_desired
        .read()
        .map_err(|_| AppError::Message("optimization desired state lock poisoned".to_owned()))?;

    if !is_running_as_admin() {
        if let Err(err) = collect_status(runtime) {
            error!("optimization status collection failed in non-admin reconcile path: {err}");
        }
        if desired_any_enabled(&desired)
            && !non_admin_reconcile_warned().swap(true, Ordering::Relaxed)
        {
            warn!(
                "optimization reconcile write pass skipped: process is not elevated; desired state enforcement paused"
            );
        }
        return Ok(());
    }

    let status = collect_status(runtime)?;

    if desired.telemetry.services
        && (!status.telemetry.services_readable || !status.telemetry.services_disabled)
    {
        apply_telemetry_feature(app, "services")?;
    }
    if desired.telemetry.registry_policies
        && (!status.telemetry.registry_policies_readable
            || !status.telemetry.registry_policies_disabled)
    {
        apply_telemetry_feature(app, "registry_policies")?;
    }
    if desired.telemetry.scheduled_tasks
        && (!status.telemetry.scheduled_tasks_readable
            || !status.telemetry.scheduled_tasks_disabled)
    {
        apply_telemetry_feature(app, "scheduled_tasks")?;
    }
    if desired.telemetry.hosts_block
        && (!status.telemetry.hosts_readable || !status.telemetry.hosts_blocked)
    {
        apply_telemetry_feature(app, "hosts_block")?;
    }

    if desired.net_sniper.tcp_tweaks
        && (!status.net_sniper.tcp_tweaks_readable || !status.net_sniper.tcp_tweaks_applied)
    {
        apply_net_feature(app, "tcp_tweaks")?;
    }
    if desired.net_sniper.registry_throttling
        && (!status.net_sniper.registry_throttling_readable
            || !status.net_sniper.registry_throttling_applied)
    {
        apply_net_feature(app, "registry_throttling")?;
    }
    if desired.net_sniper.cloudflare_dns
        && (!status.net_sniper.cloudflare_dns_readable
            || !status.net_sniper.cloudflare_dns_applied)
    {
        apply_net_feature(app, "cloudflare_dns")?;
    }

    if desired.power_mode.ultimate_plan
        && (!status.power_mode.ultimate_plan_readable || !status.power_mode.ultimate_plan_active)
    {
        apply_power_feature(app, "ultimate_plan")?;
    }
    if desired.power_mode.core_parking
        && (!status.power_mode.core_parking_readable || !status.power_mode.core_parking_disabled)
    {
        apply_power_feature(app, "core_parking")?;
    }
    if desired.advanced.hpet_dynamic_tick
        && (!status.advanced.hpet_dynamic_tick_readable
            || !status.advanced.hpet_dynamic_tick_applied)
    {
        apply_advanced_feature(app, "hpet_dynamic_tick")?;
    }
    if desired.advanced.interrupt_moderation
        && (!status.advanced.interrupt_moderation_readable
            || !status.advanced.interrupt_moderation_applied)
    {
        apply_advanced_feature(app, "interrupt_moderation")?;
    }
    if desired.advanced.mmcss
        && (!status.advanced.mmcss_readable || !status.advanced.mmcss_applied)
    {
        apply_advanced_feature(app, "mmcss")?;
    }

    Ok(())
}

pub(crate) fn spawn_optimization_reconcile_loop(
    app: AppHandle,
    runtime: RuntimeControlState,
) {
    let mut shutdown_rx = runtime.shutdown_tx.subscribe();
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(
            RECONCILE_INTERVAL_SECS,
        ));
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let app_for_tick = app.clone();
                    let runtime_for_tick = runtime.clone();
                    let mut worker = tokio::task::spawn_blocking(move || reconcile_desired_state(&app_for_tick, &runtime_for_tick));
                    tokio::select! {
                        worker_result = &mut worker => {
                            match worker_result {
                                Ok(Ok(())) => {}
                                Ok(Err(err)) => warn!("optimization reconcile tick failed: {err}"),
                                Err(err) => warn!("optimization reconcile worker join failed: {err}"),
                            }
                        }
                        _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
                            worker.abort();
                            warn!("optimization reconcile tick timed out after 30 seconds");
                        }
                    }
                }
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        break;
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use crate::optimization::{
        advanced_category_verified, build_advanced_status, net_category_verified,
        power_category_verified, telemetry_category_verified,
    };
    use crate::power::timers::TimerOptimizationStatus;
    use crate::types::{
        AdvancedStatusDto, AppError, NetSniperStatusDto, PowerStatusDto, TelemetryStatusDto,
    };

    #[test]
    fn telemetry_category_verified_requires_all_sub_items() {
        let mut status = TelemetryStatusDto {
            verified: false,
            services_disabled: true,
            registry_policies_disabled: true,
            scheduled_tasks_disabled: true,
            hosts_blocked: true,
            services_readable: true,
            registry_policies_readable: true,
            scheduled_tasks_readable: true,
            hosts_readable: true,
        };
        assert!(telemetry_category_verified(&status));

        status.hosts_blocked = false;
        assert!(!telemetry_category_verified(&status));

        status.hosts_blocked = true;
        status.hosts_readable = false;
        assert!(!telemetry_category_verified(&status));
    }

    #[test]
    fn net_category_verified_requires_all_sub_items() {
        let mut status = NetSniperStatusDto {
            verified: false,
            tcp_tweaks_applied: true,
            registry_throttling_applied: true,
            cloudflare_dns_applied: true,
            tcp_tweaks_readable: true,
            registry_throttling_readable: true,
            cloudflare_dns_readable: true,
            interfaces_total: 1,
            interfaces_tuned: 1,
            dns_interfaces_total: 1,
            dns_interfaces_tuned: 1,
        };
        assert!(net_category_verified(&status));

        status.cloudflare_dns_applied = false;
        assert!(!net_category_verified(&status));
    }

    #[test]
    fn power_category_verified_requires_all_sub_items() {
        let mut status = PowerStatusDto {
            verified: false,
            ultimate_plan_active: true,
            core_parking_disabled: true,
            ultimate_plan_readable: true,
            core_parking_readable: true,
        };
        assert!(power_category_verified(&status));

        status.core_parking_readable = false;
        assert!(!power_category_verified(&status));
    }

    #[test]
    fn advanced_status_isolated_fallback_preserves_other_sub_items() {
        let status = build_advanced_status(
            Ok(TimerOptimizationStatus {
                applied: true,
                readable: true,
            }),
            Err(AppError::Message("os error 5".into())),
            Ok(crate::optimization::mmcss::MmcssStatus {
                applied: true,
                readable: true,
            }),
        );

        assert!(status.hpet_dynamic_tick_applied);
        assert!(status.hpet_dynamic_tick_readable);
        assert!(!status.interrupt_moderation_applied);
        assert!(!status.interrupt_moderation_readable);
        assert!(status.mmcss_applied);
        assert!(status.mmcss_readable);
        assert!(!status.verified);
    }

    #[test]
    fn advanced_category_verified_requires_all_sub_items() {
        let mut status = AdvancedStatusDto {
            verified: false,
            hpet_dynamic_tick_applied: true,
            interrupt_moderation_applied: true,
            mmcss_applied: true,
            hpet_dynamic_tick_readable: true,
            interrupt_moderation_readable: true,
            mmcss_readable: true,
            interrupt_moderation_adapters_total: 1,
            interrupt_moderation_adapters_tuned: 1,
        };
        assert!(advanced_category_verified(&status));

        status.interrupt_moderation_readable = false;
        assert!(!advanced_category_verified(&status));
    }
}
