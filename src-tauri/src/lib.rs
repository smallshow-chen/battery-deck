mod battery;
pub mod helper;
mod service;
mod smc;

use battery::{BatteryHealth, BatteryRealtime, BatteryState, ChargerInfo, ServiceStatus, Settings};
use tauri::{AppHandle, Emitter};

struct AppState;

fn build_battery_state() -> Result<BatteryState, String> {
    let (percent, is_charging) = battery::get_battery_info().unwrap_or((0, false));
    let is_plugged = battery::get_is_plugged();
    let helper_state = service::helper_state().unwrap_or_else(|error| battery::HelperState {
        mode: battery::ChargeMode::Standard,
        charging_disabled: false,
        power_disabled: false,
        supported: false,
        control_available: false,
        settings: helper::load_settings(),
        last_error: Some(error),
    });

    Ok(BatteryState {
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
        last_error: helper_state.last_error,
    })
}

#[tauri::command]
fn get_battery_state(_: tauri::State<AppState>) -> Result<BatteryState, String> {
    build_battery_state()
}

#[tauri::command]
fn get_settings(_: tauri::State<AppState>) -> Result<Settings, String> {
    service::get_settings()
}

#[tauri::command]
fn set_settings(
    _: tauri::State<AppState>,
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

#[tauri::command]
fn charge_to_full(_: tauri::State<AppState>) -> Result<(), String> {
    let _: battery::HelperState = service::send_command("charge_to_full", serde_json::Value::Null)?;
    Ok(())
}

#[tauri::command]
fn charge_to_limit(_: tauri::State<AppState>) -> Result<(), String> {
    let _: battery::HelperState =
        service::send_command("charge_to_limit", serde_json::Value::Null)?;
    Ok(())
}

#[tauri::command]
fn disable_charging_cmd(_: tauri::State<AppState>) -> Result<(), String> {
    let _: battery::HelperState =
        service::send_command("disable_charging", serde_json::Value::Null)?;
    Ok(())
}

#[tauri::command]
fn disable_adapter_cmd(_: tauri::State<AppState>) -> Result<(), String> {
    let _: battery::HelperState =
        service::send_command("disable_adapter", serde_json::Value::Null)?;
    Ok(())
}

#[tauri::command]
fn enable_adapter_cmd(_: tauri::State<AppState>) -> Result<(), String> {
    let _: battery::HelperState = service::send_command("enable_adapter", serde_json::Value::Null)?;
    Ok(())
}

#[tauri::command]
fn is_supported(_: tauri::State<AppState>) -> Result<bool, String> {
    Ok(build_battery_state()?.supported)
}

#[tauri::command]
fn get_service_status(_: tauri::State<AppState>) -> Result<ServiceStatus, String> {
    service::get_service_status()
}

#[tauri::command]
fn install_service(_: tauri::State<AppState>) -> Result<ServiceStatus, String> {
    service::install_service()?;
    service::get_service_status()
}

#[tauri::command]
fn start_service(_: tauri::State<AppState>) -> Result<ServiceStatus, String> {
    service::start_service()
}

#[tauri::command]
fn stop_service(_: tauri::State<AppState>) -> Result<(), String> {
    service::stop_service()
}

#[tauri::command]
fn get_service_logs(_: tauri::State<AppState>, lines: Option<usize>) -> Result<String, String> {
    service::get_logs(lines.unwrap_or(50))
}

#[tauri::command]
fn get_battery_health() -> Result<Option<BatteryHealth>, String> {
    Ok(battery::get_battery_health())
}

#[tauri::command]
fn get_battery_realtime() -> Result<Option<BatteryRealtime>, String> {
    Ok(battery::get_battery_realtime())
}

#[tauri::command]
fn get_charger_info() -> Result<Option<ChargerInfo>, String> {
    Ok(battery::get_charger_info())
}

#[tauri::command]
fn reset_charge_mode(_: tauri::State<AppState>) -> Result<(), String> {
    let _: battery::HelperState =
        service::send_command("reset_charge_mode", serde_json::Value::Null)?;
    Ok(())
}

fn spawn_battery_poll(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_state: Option<String> = None;

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            if let Ok(state) = build_battery_state() {
                if let Ok(serialized) = serde_json::to_string(&state) {
                    if last_state.as_deref() != Some(serialized.as_str()) {
                        last_state = Some(serialized);
                        let _ = app_handle.emit("battery-state-changed", &state);
                    }
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let app_handle = app.handle().clone();
            spawn_battery_poll(app_handle);
            Ok(())
        })
        .manage(AppState)
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
            reset_charge_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
