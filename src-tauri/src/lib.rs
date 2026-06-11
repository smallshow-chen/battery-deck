mod battery;
pub mod helper;
mod service;
mod smc;

use battery::{
    BatteryCache, BatteryHealth, BatteryRealtime, BatteryState, ChargerInfo, DashboardSnapshot,
    ServiceStatus, Settings, SystemInfo,
};
use serde::Serialize;
use serde_json::Value;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State, WebviewWindow, WindowEvent,
};

const APP_DISPLAY_NAME: &str = "MyBatteryManager";
const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_ID: &str = "main-tray";

const EVENT_WINDOW_VISIBILITY_CHANGED: &str = "app-window-visibility-changed";
const EVENT_WINDOW_REFRESH_REQUESTED: &str = "app-state-refresh-requested";
const EVENT_TRAY_ACTION_ERROR: &str = "tray-action-error";

const MENU_SHOW_WINDOW: &str = "tray.show_window";
const MENU_CHARGE_FULL: &str = "tray.charge_full";
const MENU_CHARGE_LIMIT: &str = "tray.charge_limit";
const MENU_RESUME_LIMITS: &str = "tray.resume_limits";
const MENU_DISABLE_CHARGING: &str = "tray.disable_charging";
const MENU_TOGGLE_ADAPTER: &str = "tray.toggle_adapter";
const MENU_INSTALL_SERVICE: &str = "tray.install_service";
const MENU_START_SERVICE: &str = "tray.start_service";
const MENU_STOP_SERVICE: &str = "tray.stop_service";
const MENU_QUIT: &str = "tray.quit";

#[cfg(target_os = "macos")]
fn restore_app_icon(app: &AppHandle) -> Result<(), String> {
    use objc2::{AllocAnyThread, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    let icon_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("icons/icon.png");
    let icon_bytes = std::fs::read(&icon_path).map_err(|e| e.to_string())?;

    app.run_on_main_thread(move || {
        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        let ns_app = NSApplication::sharedApplication(mtm);
        let data = NSData::with_bytes(&icon_bytes);
        if let Some(app_icon) = NSImage::initWithData(NSImage::alloc(), &data) {
            unsafe { ns_app.setApplicationIconImage(Some(&app_icon)) };
        }
    })
    .map_err(|e| e.to_string())
}

#[cfg(not(target_os = "macos"))]
fn restore_app_icon(_: &AppHandle) -> Result<(), String> {
    Ok(())
}

struct AppCache(BatteryCache);

struct AppState {
    window_visible: AtomicBool,
    quitting: AtomicBool,
    tray: Mutex<Option<TrayMenuHandles>>,
    cache: AppCache,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            window_visible: AtomicBool::new(false),
            quitting: AtomicBool::new(false),
            tray: Mutex::new(None),
            cache: AppCache(BatteryCache::new()),
        }
    }
}

#[derive(Clone)]
struct TrayMenuHandles {
    summary_battery: MenuItem<tauri::Wry>,
    summary_mode: MenuItem<tauri::Wry>,
    summary_service: MenuItem<tauri::Wry>,
    show_window: MenuItem<tauri::Wry>,
    charge_full: MenuItem<tauri::Wry>,
    charge_limit: MenuItem<tauri::Wry>,
    resume_limits: MenuItem<tauri::Wry>,
    disable_charging: MenuItem<tauri::Wry>,
    toggle_adapter: MenuItem<tauri::Wry>,
    install_service: MenuItem<tauri::Wry>,
    start_service: MenuItem<tauri::Wry>,
    stop_service: MenuItem<tauri::Wry>,
}

#[derive(Clone, Serialize)]
struct TrayActionErrorPayload {
    message: String,
}

fn fallback_helper_state(error: Option<String>) -> battery::HelperState {
    battery::HelperState {
        mode: battery::ChargeMode::Standard,
        charging_disabled: false,
        power_disabled: false,
        supported: false,
        control_available: false,
        settings: helper::load_settings(),
        last_error: error,
    }
}

fn build_battery_state_with_cache(
    cache: &BatteryCache,
    helper_state: &battery::HelperState,
) -> BatteryState {
    let (percent, is_charging) = battery::get_battery_info_cached(cache).unwrap_or((0, false));
    let is_plugged = battery::get_is_plugged_cached(cache);

    BatteryState {
        enabled: true,
        power_disabled: helper_state.power_disabled,
        connected: !helper_state.power_disabled,
        is_plugged,
        charging_disabled: helper_state.charging_disabled,
        is_charging,
        charge_percent: percent,
        mode: helper_state.mode,
        min_charge: helper_state.settings.min_charge,
        max_charge: helper_state.settings.max_charge,
        adapter_sleep: helper_state.settings.adapter_sleep,
        magsafe_sync: helper_state.settings.magsafe_sync,
        supported: helper_state.supported,
        control_available: helper_state.control_available,
        last_error: helper_state.last_error.clone(),
    }
}

fn build_dashboard_snapshot(app: &AppHandle) -> DashboardSnapshot {
    let cache = &app.state::<AppState>().cache.0;
    let helper_result = service::helper_state();
    let helper_error = helper_result.as_ref().err().cloned();
    let helper_state = helper_result
        .as_ref()
        .ok()
        .cloned()
        .unwrap_or_else(|| fallback_helper_state(helper_error.clone()));

    DashboardSnapshot {
        battery_state: build_battery_state_with_cache(cache, &helper_state),
        service_status: service::derive_service_status(helper_result.as_ref().ok(), helper_error),
        battery_health: battery::get_battery_health_cached(cache),
        battery_realtime: battery::get_battery_realtime_cached(cache),
        charger_info: battery::get_charger_info_cached(cache),
        system_info: battery::get_system_info_cached(cache),
        settings: helper_state.settings,
    }
}

fn build_battery_state(app: &AppHandle) -> Result<BatteryState, String> {
    Ok(build_dashboard_snapshot(app).battery_state)
}

fn main_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "Main window not found".to_string())
}

fn with_tray_handles<R>(app: &AppHandle, f: impl FnOnce(&TrayMenuHandles) -> R) -> Option<R> {
    let handles = app.state::<AppState>().tray.lock().ok()?.clone()?;
    Some(f(&handles))
}

fn emit_window_visibility(app: &AppHandle, visible: bool) {
    let _ = app.emit(EVENT_WINDOW_VISIBILITY_CHANGED, visible);
}

fn request_window_refresh_if_visible(app: &AppHandle) {
    if app
        .state::<AppState>()
        .window_visible
        .load(Ordering::Relaxed)
    {
        let _ = app.emit(EVENT_WINDOW_REFRESH_REQUESTED, true);
    }
}

fn emit_tray_error(app: &AppHandle, message: String) {
    let _ = app.emit(EVENT_TRAY_ACTION_ERROR, TrayActionErrorPayload { message });
}

fn bilingual_label(zh: &str, en: &str) -> String {
    format!("{zh} / {en}")
}

fn set_app_activation_policy(app: &AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        app.set_activation_policy(tauri::ActivationPolicy::Regular)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn show_main_window(app: &AppHandle) -> Result<(), String> {
    set_app_activation_policy(app)?;
    restore_app_icon(app)?;
    #[cfg(target_os = "macos")]
    app.set_dock_visibility(true).map_err(|e| e.to_string())?;

    let window = main_window(app)?;
    if let Some(icon) = app.default_window_icon().cloned() {
        let _ = window.set_icon(icon);
    }
    window.show().map_err(|e| e.to_string())?;
    let _ = window.set_focus();

    let was_visible = app
        .state::<AppState>()
        .window_visible
        .swap(true, Ordering::Relaxed);
    if !was_visible {
        emit_window_visibility(app, true);
    }
    Ok(())
}

fn hide_main_window(app: &AppHandle) -> Result<(), String> {
    let window = main_window(app)?;
    window.hide().map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    app.set_dock_visibility(false).map_err(|e| e.to_string())?;

    let was_visible = app
        .state::<AppState>()
        .window_visible
        .swap(false, Ordering::Relaxed);
    if was_visible {
        emit_window_visibility(app, false);
    }
    Ok(())
}

fn mode_label(mode: battery::ChargeMode) -> String {
    match mode {
        battery::ChargeMode::Standard => bilingual_label("标准", "Standard"),
        battery::ChargeMode::ToLimit => bilingual_label("充至上限", "Charge to Limit"),
        battery::ChargeMode::ToFull => bilingual_label("充至满电", "Charge to Full"),
    }
}

fn charging_label(state: &BatteryState) -> String {
    if state.charging_disabled {
        bilingual_label("已停止", "Stopped")
    } else if state.power_disabled {
        bilingual_label("适配器已关闭", "Adapter Off")
    } else if state.is_charging {
        bilingual_label("充电中", "Charging")
    } else if state.is_plugged {
        bilingual_label("已接通电源", "Plugged In")
    } else {
        bilingual_label("使用电池", "On Battery")
    }
}

fn service_label(state: &BatteryState, service_status: &ServiceStatus) -> String {
    if service_status.control_available {
        bilingual_label("就绪", "Ready")
    } else if service_status.running && !state.supported {
        bilingual_label("设备不支持", "Unsupported Device")
    } else if service_status.running {
        bilingual_label("运行中", "Running")
    } else if service_status.installed {
        bilingual_label("已安装，已停止", "Installed, Stopped")
    } else {
        bilingual_label("未安装", "Not Installed")
    }
}

fn refresh_tray_menu(app: &AppHandle) -> Result<(), String> {
    let snapshot = build_dashboard_snapshot(app);
    let battery_state = snapshot.battery_state;
    let service_status = snapshot.service_status;

    with_tray_handles(app, |tray| {
        let control_ready = battery_state.control_available && service_status.control_available;

        let _ = tray.summary_battery.set_text(format!(
            "电池 Battery: {}% ({})",
            battery_state.charge_percent,
            charging_label(&battery_state)
        ));
        let _ = tray
            .summary_mode
            .set_text(format!("模式 Mode: {}", mode_label(battery_state.mode)));
        let _ = tray.summary_service.set_text(format!(
            "服务 Service: {}",
            service_label(&battery_state, &service_status)
        ));

        let _ = tray.show_window.set_enabled(true);
        let _ = tray.charge_full.set_enabled(control_ready);
        let _ = tray.charge_limit.set_enabled(control_ready);
        let _ = tray.resume_limits.set_enabled(control_ready);
        let _ = tray.disable_charging.set_enabled(control_ready);
        let _ = tray.toggle_adapter.set_enabled(control_ready);
        let _ = tray.install_service.set_enabled(true);
        let _ = tray.start_service.set_enabled(!service_status.running);
        let _ = tray.stop_service.set_enabled(service_status.running);

        let adapter_text = if battery_state.power_disabled {
            bilingual_label("启用适配器", "Enable Adapter")
        } else {
            bilingual_label("禁用适配器", "Disable Adapter")
        };
        let _ = tray.toggle_adapter.set_text(adapter_text);
    });

    Ok(())
}

fn handle_tray_action(app: &AppHandle, action: impl FnOnce() -> Result<(), String>) {
    match action() {
        Ok(()) => {
            let _ = refresh_tray_menu(app);
            request_window_refresh_if_visible(app);
        }
        Err(error) => {
            let _ = refresh_tray_menu(app);
            let _ = show_main_window(app);
            emit_tray_error(app, error);
        }
    }
}

fn create_tray_menu(app: &AppHandle) -> Result<(Menu<tauri::Wry>, TrayMenuHandles), String> {
    let summary_battery =
        MenuItem::new(app, "电池 Battery: --", false, None::<&str>).map_err(|e| e.to_string())?;
    let summary_mode =
        MenuItem::new(app, "模式 Mode: --", false, None::<&str>).map_err(|e| e.to_string())?;
    let summary_service =
        MenuItem::new(app, "服务 Service: --", false, None::<&str>).map_err(|e| e.to_string())?;

    let show_window_label = bilingual_label("显示主窗口", "Show Window");
    let charge_full_label = bilingual_label("充至满电", "Charge to Full");
    let charge_limit_label = bilingual_label("充至上限", "Charge to Limit");
    let resume_limits_label = bilingual_label("恢复限充", "Resume Limits");
    let disable_charging_label = bilingual_label("停止充电", "Stop Charging");
    let disable_adapter_label = bilingual_label("禁用适配器", "Disable Adapter");
    let install_service_label = bilingual_label("安装特权服务", "Install Privileged Service");
    let start_service_label = bilingual_label("启动服务", "Start Service");
    let stop_service_label = bilingual_label("停止服务", "Stop Service");
    let quit_label = bilingual_label("退出应用", "Quit");

    let show_window = MenuItem::with_id(
        app,
        MENU_SHOW_WINDOW,
        &show_window_label,
        true,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;

    let charge_full = MenuItem::with_id(
        app,
        MENU_CHARGE_FULL,
        &charge_full_label,
        true,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;
    let charge_limit = MenuItem::with_id(
        app,
        MENU_CHARGE_LIMIT,
        &charge_limit_label,
        true,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;
    let resume_limits = MenuItem::with_id(
        app,
        MENU_RESUME_LIMITS,
        &resume_limits_label,
        true,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;
    let disable_charging = MenuItem::with_id(
        app,
        MENU_DISABLE_CHARGING,
        &disable_charging_label,
        true,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;
    let toggle_adapter = MenuItem::with_id(
        app,
        MENU_TOGGLE_ADAPTER,
        &disable_adapter_label,
        true,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;

    let install_service = MenuItem::with_id(
        app,
        MENU_INSTALL_SERVICE,
        &install_service_label,
        true,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;
    let start_service = MenuItem::with_id(
        app,
        MENU_START_SERVICE,
        &start_service_label,
        true,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;
    let stop_service = MenuItem::with_id(
        app,
        MENU_STOP_SERVICE,
        &stop_service_label,
        true,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;
    let quit = MenuItem::with_id(app, MENU_QUIT, &quit_label, true, None::<&str>)
        .map_err(|e| e.to_string())?;

    let sep1 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let sep2 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let sep3 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let sep4 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;

    let menu = Menu::with_items(
        app,
        &[
            &summary_battery,
            &summary_mode,
            &summary_service,
            &sep1,
            &show_window,
            &sep2,
            &charge_full,
            &charge_limit,
            &resume_limits,
            &disable_charging,
            &toggle_adapter,
            &sep3,
            &install_service,
            &start_service,
            &stop_service,
            &sep4,
            &quit,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok((
        menu,
        TrayMenuHandles {
            summary_battery,
            summary_mode,
            summary_service,
            show_window,
            charge_full,
            charge_limit,
            resume_limits,
            disable_charging,
            toggle_adapter,
            install_service,
            start_service,
            stop_service,
        },
    ))
}

fn setup_tray(app: &AppHandle) -> Result<(), String> {
    let (menu, handles) = create_tray_menu(app)?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip(APP_DISPLAY_NAME)
        .show_menu_on_left_click(false)
        .icon_as_template(true)
        .on_menu_event(|app, event| match event.id().0.as_str() {
            MENU_SHOW_WINDOW => {
                let _ = show_main_window(app);
            }
            MENU_CHARGE_FULL => handle_tray_action(app, perform_charge_to_full),
            MENU_CHARGE_LIMIT => handle_tray_action(app, perform_charge_to_limit),
            MENU_RESUME_LIMITS => handle_tray_action(app, perform_reset_charge_mode),
            MENU_DISABLE_CHARGING => handle_tray_action(app, perform_disable_charging),
            MENU_TOGGLE_ADAPTER => {
                let app_clone = app.clone();
                handle_tray_action(app, || perform_toggle_adapter(&app_clone))
            }
            MENU_INSTALL_SERVICE => handle_tray_action(app, perform_install_service),
            MENU_START_SERVICE => handle_tray_action(app, perform_start_service),
            MENU_STOP_SERVICE => handle_tray_action(app, perform_stop_service),
            MENU_QUIT => {
                app.state::<AppState>()
                    .quitting
                    .store(true, Ordering::Relaxed);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    let _ = show_main_window(app);
                }
                TrayIconEvent::Click {
                    button: MouseButton::Right,
                    button_state: MouseButtonState::Down,
                    ..
                } => {
                    let _ = refresh_tray_menu(app);
                }
                _ => {}
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder.build(app).map_err(|e| e.to_string())?;
    *app.state::<AppState>().tray.lock().unwrap() = Some(handles);
    refresh_tray_menu(app)?;
    Ok(())
}

fn attach_window_handlers(app: &AppHandle) -> Result<(), String> {
    let app_handle = app.clone();
    let window = main_window(app)?;
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            let state = app_handle.state::<AppState>();
            if !state.quitting.load(Ordering::Relaxed) {
                api.prevent_close();
                let _ = hide_main_window(&app_handle);
            }
        }
    });
    Ok(())
}

fn perform_set_settings(
    min_charge: u8,
    max_charge: u8,
    adapter_sleep: bool,
    magsafe_sync: bool,
) -> Result<(), String> {
    let _: battery::HelperState = service::send_command(
        "set_settings",
        service::settings_payload(min_charge, max_charge, adapter_sleep, magsafe_sync),
    )?;
    Ok(())
}

fn perform_charge_to_full() -> Result<(), String> {
    let _: battery::HelperState = service::send_command("charge_to_full", Value::Null)?;
    Ok(())
}

fn perform_charge_to_limit() -> Result<(), String> {
    let _: battery::HelperState = service::send_command("charge_to_limit", Value::Null)?;
    Ok(())
}

fn perform_disable_charging() -> Result<(), String> {
    let _: battery::HelperState = service::send_command("disable_charging", Value::Null)?;
    Ok(())
}

fn perform_disable_adapter() -> Result<(), String> {
    let _: battery::HelperState = service::send_command("disable_adapter", Value::Null)?;
    Ok(())
}

fn perform_enable_adapter() -> Result<(), String> {
    let _: battery::HelperState = service::send_command("enable_adapter", Value::Null)?;
    Ok(())
}

fn perform_toggle_adapter(app: &AppHandle) -> Result<(), String> {
    let state = build_battery_state(app)?;
    if state.power_disabled {
        perform_enable_adapter()
    } else {
        perform_disable_adapter()
    }
}

fn perform_reset_charge_mode() -> Result<(), String> {
    let _: battery::HelperState = service::send_command("reset_charge_mode", Value::Null)?;
    Ok(())
}

fn perform_install_service() -> Result<(), String> {
    service::install_service()
}

fn perform_start_service() -> Result<(), String> {
    service::start_service().map(|_| ())
}

fn perform_stop_service() -> Result<(), String> {
    service::stop_service()
}

#[tauri::command]
fn get_battery_state(app: AppHandle, _: State<AppState>) -> Result<BatteryState, String> {
    build_battery_state(&app)
}

#[tauri::command]
fn get_settings(_: State<AppState>) -> Result<Settings, String> {
    service::get_settings()
}

#[tauri::command]
fn set_settings(
    app: AppHandle,
    _: State<AppState>,
    min_charge: u8,
    max_charge: u8,
    adapter_sleep: bool,
    magsafe_sync: bool,
) -> Result<(), String> {
    perform_set_settings(min_charge, max_charge, adapter_sleep, magsafe_sync)?;
    let _ = refresh_tray_menu(&app);
    Ok(())
}

#[tauri::command]
fn charge_to_full(app: AppHandle, _: State<AppState>) -> Result<(), String> {
    perform_charge_to_full()?;
    let _ = refresh_tray_menu(&app);
    Ok(())
}

#[tauri::command]
fn charge_to_limit(app: AppHandle, _: State<AppState>) -> Result<(), String> {
    perform_charge_to_limit()?;
    let _ = refresh_tray_menu(&app);
    Ok(())
}

#[tauri::command]
fn disable_charging_cmd(app: AppHandle, _: State<AppState>) -> Result<(), String> {
    perform_disable_charging()?;
    let _ = refresh_tray_menu(&app);
    Ok(())
}

#[tauri::command]
fn disable_adapter_cmd(app: AppHandle, _: State<AppState>) -> Result<(), String> {
    perform_disable_adapter()?;
    let _ = refresh_tray_menu(&app);
    Ok(())
}

#[tauri::command]
fn enable_adapter_cmd(app: AppHandle, _: State<AppState>) -> Result<(), String> {
    perform_enable_adapter()?;
    let _ = refresh_tray_menu(&app);
    Ok(())
}

#[tauri::command]
fn is_supported(app: AppHandle, _: State<AppState>) -> Result<bool, String> {
    Ok(build_battery_state(&app)?.supported)
}

#[tauri::command]
fn get_service_status(_: State<AppState>) -> Result<ServiceStatus, String> {
    service::get_service_status()
}

#[tauri::command]
fn install_service(app: AppHandle, _: State<AppState>) -> Result<ServiceStatus, String> {
    service::install_service()?;
    let _ = refresh_tray_menu(&app);
    service::get_service_status()
}

#[tauri::command]
fn start_service(app: AppHandle, _: State<AppState>) -> Result<ServiceStatus, String> {
    let status = service::start_service()?;
    let _ = refresh_tray_menu(&app);
    Ok(status)
}

#[tauri::command]
fn stop_service(app: AppHandle, _: State<AppState>) -> Result<(), String> {
    service::stop_service()?;
    let _ = refresh_tray_menu(&app);
    Ok(())
}

#[tauri::command]
fn get_service_logs(_: State<AppState>, lines: Option<usize>) -> Result<String, String> {
    service::get_logs(lines.unwrap_or(50))
}

#[tauri::command]
fn get_battery_health(_: State<AppState>, app: AppHandle) -> Result<Option<BatteryHealth>, String> {
    Ok(battery::get_battery_health_cached(
        &app.state::<AppState>().cache.0,
    ))
}

#[tauri::command]
fn get_battery_realtime(
    _: State<AppState>,
    app: AppHandle,
) -> Result<Option<BatteryRealtime>, String> {
    Ok(battery::get_battery_realtime_cached(
        &app.state::<AppState>().cache.0,
    ))
}

#[tauri::command]
fn get_charger_info(_: State<AppState>, app: AppHandle) -> Result<Option<ChargerInfo>, String> {
    Ok(battery::get_charger_info_cached(
        &app.state::<AppState>().cache.0,
    ))
}

#[tauri::command]
fn get_system_info(_: State<AppState>, app: AppHandle) -> Result<Option<SystemInfo>, String> {
    Ok(battery::get_system_info_cached(
        &app.state::<AppState>().cache.0,
    ))
}

#[tauri::command]
fn get_dashboard_snapshot(app: AppHandle, _: State<AppState>) -> Result<DashboardSnapshot, String> {
    Ok(build_dashboard_snapshot(&app))
}

#[tauri::command]
fn reset_charge_mode(app: AppHandle, _: State<AppState>) -> Result<(), String> {
    perform_reset_charge_mode()?;
    let _ = refresh_tray_menu(&app);
    Ok(())
}

fn spawn_battery_poll(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_state: Option<String> = None;

        loop {
            let sleep_secs = if app_handle
                .state::<AppState>()
                .window_visible
                .load(Ordering::Relaxed)
            {
                5
            } else {
                60
            };

            tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;

            if !app_handle
                .state::<AppState>()
                .window_visible
                .load(Ordering::Relaxed)
            {
                continue;
            }

            if let Ok(state) = build_battery_state(&app_handle) {
                if let Ok(serialized) = serde_json::to_string(&state) {
                    if last_state.as_deref() != Some(serialized.as_str()) {
                        last_state = Some(serialized);
                        let _ = app_handle.emit("battery-state-changed", &state);
                        let _ = refresh_tray_menu(&app_handle);
                    }
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .setup(move |app| {
            let app_handle = app.handle().clone();
            setup_tray(&app_handle)?;
            attach_window_handlers(&app_handle)?;
            spawn_battery_poll(app_handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_battery_state,
            get_settings,
            set_settings,
            charge_to_full,
            charge_to_limit,
            disable_charging_cmd,
            disable_adapter_cmd,
            enable_adapter_cmd,
            is_supported,
            get_service_status,
            install_service,
            start_service,
            stop_service,
            get_service_logs,
            get_battery_health,
            get_battery_realtime,
            get_charger_info,
            get_system_info,
            get_dashboard_snapshot,
            reset_charge_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
