#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod cleaner;
mod core;
mod driver_store;
mod elevation;
mod ghost_devices;
mod icon_service;
mod ipc;
mod memory_purge;
mod msi_util;
mod net_tuning;
mod optimization;
mod power;
mod process;
mod settings_repo;
mod telemetry;
mod timer;
mod types;
mod utils;
mod watchdog;

pub(crate) use core::*;
pub(crate) use types::*;

use elevation::{apply_startup_elevated_payload, show_startup_error_dialog};
use memory_purge::{enable_profile_privilege, run_standby_purge_with_telemetry};
use optimization::{spawn_optimization_reconcile_loop, sync_desired_state_from_settings};
use process::{apply_config_headless, parse_apply_config_arg, spawn_process_sampler_loop};
use settings_repo::{flush_settings_write_behind, load_settings, shutdown_settings_write_behind};
use timer::{
    apply_timer_resolution_request, disable_process_power_throttling, ms_to_hundred_ns,
    release_timer_resolution,
};
use watchdog::spawn_watchdog_loop;

fn signal_runtime_shutdown(runtime_state: &RuntimeControlState) {
    let _ = runtime_state.shutdown_tx.send(true);
}

fn init_tracing_subscriber() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_level(true)
        .try_init();
}

fn main() {
    init_tracing_subscriber();

    let context = tauri::generate_context!();
    let app_identifier = context.config().identifier.clone();

    if let Some(config_name) = parse_apply_config_arg() {
        if let Err(err) = apply_config_headless(&config_name, &app_identifier) {
            error!("[headless] {err}");
        }
        std::process::exit(0);
    }

    apply_startup_elevated_payload();

    if let Err(err) = enable_profile_privilege() {
        error!("Failed to enable SeProfileSingleProcessPrivilege: {err}");
    }
    if let Err(err) = disable_process_power_throttling() {
        error!("Failed to disable process power throttling: {err}");
    }

    let runtime_state = RuntimeControlState::default();

    let run_result = tauri::Builder::default()
        .manage(runtime_state.clone())
        .setup({
            let runtime_state = runtime_state.clone();
            move |app| {
                info!("Optimus Core Engine started");
                let settings = match load_settings(&app.handle()) {
                    Ok(settings) => settings,
                    Err(err) => {
                        warn!("Failed to load settings.json: {err}");
                        AppSettings::default()
                    }
                };
                runtime_state
                    .watchdog_enabled
                    .store(settings.watchdog_enabled, Ordering::Relaxed);
                runtime_state
                    .minimize_to_tray_enabled
                    .store(settings.minimize_to_tray_enabled, Ordering::Relaxed);
                let mut memory_purge_config = settings.memory_purge_config;
                if !is_running_as_admin() && memory_purge_config.master_enabled {
                    memory_purge_config.master_enabled = false;
                    warn!("Memory Purge Engine disabled at startup (administrator privileges required)");
                }
                if let Ok(mut config) = runtime_state.memory_purge_config.write() {
                    *config = memory_purge_config;
                } else {
                    error!("Failed to seed memory purge config from settings");
                }
                if let Err(err) =
                    sync_desired_state_from_settings(&runtime_state, settings.optimization_desired)
                {
                    error!("Failed to seed optimization desired state from settings: {err}");
                }
                if settings.turbo_timer_enabled {
                    if let Err(err) =
                        apply_timer_resolution_request(&runtime_state, Some(ms_to_hundred_ns(0.5)))
                    {
                        error!("Failed to apply startup turbo timer setting: {err}");
                    }
                }

                let show_item = MenuItem::with_id(app, TRAY_SHOW_ID, "Open Optimus", true, None::<&str>)?;
                let purge_item =
                    MenuItem::with_id(app, TRAY_PURGE_ID, "Purge Memory Now", true, None::<&str>)?;
                let exit_item = MenuItem::with_id(app, TRAY_EXIT_ID, "Quit", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show_item, &purge_item, &exit_item])?;

                let show_id = show_item.id().clone();
                let purge_id = purge_item.id().clone();
                let exit_id = exit_item.id().clone();
                let runtime_state_for_menu = runtime_state.clone();

                let mut tray_builder = TrayIconBuilder::with_id("optimus-tray")
                    .menu(&menu)
                    .tooltip("Optimus");

                if let Some(icon) = app.default_window_icon().cloned() {
                    tray_builder = tray_builder.icon(icon);
                }

                tray_builder
                    .on_menu_event(move |app, event| {
                        if event.id == show_id {
                            let _ = show_main_window(app);
                        } else if event.id == purge_id {
                            match run_standby_purge_with_telemetry(&runtime_state_for_menu) {
                                Ok(_) => {
                                    info!("Tray action: standby list purge completed");
                                }
                                Err(err) => {
                                    error!("Tray action: standby list purge failed: {err}");
                                }
                            }
                        } else if event.id == exit_id {
                            runtime_state_for_menu
                                .exit_requested
                                .store(true, Ordering::Relaxed);
                            signal_runtime_shutdown(&runtime_state_for_menu);
                            app.exit(0);
                        }
                    })
                    .on_tray_icon_event(|tray, event| {
                        match event {
                            tauri::tray::TrayIconEvent::Click {
                                button,
                                button_state,
                                ..
                            } => {
                                if button == tauri::tray::MouseButton::Left
                                    && button_state == tauri::tray::MouseButtonState::Up
                                {
                                    let _ = show_main_window(tray.app_handle());
                                } else if button == tauri::tray::MouseButton::Right
                                    && button_state == tauri::tray::MouseButtonState::Up
                                {
                                    info!("Tray right-click received");
                                }
                            }
                            tauri::tray::TrayIconEvent::DoubleClick { button, .. } => {
                                if button == tauri::tray::MouseButton::Left {
                                    let _ = show_main_window(tray.app_handle());
                                }
                            }
                            _ => {}
                        }
                    })
                    .build(app)?;

                spawn_process_sampler_loop(runtime_state.clone());
                spawn_watchdog_loop(app.handle().clone(), runtime_state.clone());
                spawn_optimization_reconcile_loop(app.handle().clone(), runtime_state.clone());
                Ok(())
            }
        })
        .on_window_event({
            let runtime_state = runtime_state.clone();
            move |window, event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    if runtime_state.exit_requested.load(Ordering::Relaxed) {
                        return;
                    }

                    if runtime_state.minimize_to_tray_enabled.load(Ordering::Relaxed) {
                        api.prevent_close();
                        let _ = window.hide();
                    } else {
                        runtime_state.exit_requested.store(true, Ordering::Relaxed);
                        signal_runtime_shutdown(&runtime_state);
                    }
                }
            }
        })
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            ipc::process_commands::check_is_admin,
            ipc::process_commands::run_deep_purge,
            ipc::process_commands::process_get_delta,
            ipc::process_commands::icon_get_png,
            ipc::process_commands::process_get_priority,
            ipc::process_commands::process_set_priority,
            ipc::process_commands::process_set_group_priority,
            ipc::process_commands::process_kill,
            ipc::engine_commands::engine_elevation_restart,
            ipc::config_commands::config_load_configs,
            ipc::config_commands::config_save,
            ipc::config_commands::config_delete,
            ipc::config_commands::config_export,
            ipc::config_commands::config_import,
            ipc::config_commands::config_watchdog_load,
            ipc::config_commands::config_watchdog_upsert_mapping,
            ipc::config_commands::config_watchdog_remove_mapping,
            ipc::config_commands::config_set_sticky_mode,
            ipc::engine_commands::engine_get_app_settings,
            ipc::engine_commands::engine_get_runtime_settings,
            ipc::engine_commands::engine_timer_get_status,
            ipc::engine_commands::engine_timer_set,
            ipc::engine_commands::engine_memory_get_stats,
            ipc::engine_commands::engine_memory_get_config,
            ipc::engine_commands::engine_memory_set_config,
            ipc::engine_commands::engine_memory_purge,
            ipc::engine_commands::engine_watchdog_set_enabled,
            ipc::engine_commands::engine_tray_set_minimize,
            ipc::engine_commands::engine_autostart_configure,
            ipc::engine_commands::engine_autostart_toggle,
            ipc::engine_commands::set_run_as_admin,
            ipc::config_commands::config_create_desktop_shortcut,
            ipc::optimization_commands::optimization_telemetry_toggle,
            ipc::optimization_commands::optimization_net_sniper_toggle,
            ipc::optimization_commands::optimization_power_toggle,
            ipc::optimization_commands::optimization_advanced_toggle,
            ipc::optimization_commands::optimization_get_status,
            ipc::process_commands::hardware_msi_list,
            ipc::process_commands::hardware_msi_apply_batch,
            ipc::process_commands::hardware_driver_list,
            ipc::process_commands::hardware_driver_delete,
            ipc::process_commands::hardware_ghost_list,
            ipc::process_commands::hardware_ghost_remove
        ])
        .run(context);

    if let Err(err) = flush_settings_write_behind() {
        error!("Failed to flush pending settings writes: {err}");
    }
    shutdown_settings_write_behind();
    signal_runtime_shutdown(&runtime_state);
    release_timer_resolution(&runtime_state);

    if let Err(err) = run_result {
        let message = format!("Failed to launch Optimus.\n\n{err}");
        error!("{message}");
        show_startup_error_dialog(&message);
    }
}

