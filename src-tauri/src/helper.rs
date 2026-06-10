use crate::battery::{get_battery_percent, validate_settings, ChargeMode, HelperState, RuntimeState, Settings};
use crate::smc::{self, SmcHandle};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
struct SharedState {
    smc: Arc<Mutex<Option<SmcHandle>>>,
    data: Arc<Mutex<HelperState>>,
}

#[derive(Serialize, Deserialize)]
struct Request {
    id: String,
    command: String,
    #[serde(default)]
    payload: Value,
}

#[derive(Serialize, Deserialize)]
struct Response {
    id: String,
    ok: bool,
    #[serde(default)]
    data: Value,
    error: Option<String>,
}

fn home_dir() -> PathBuf {
    env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

fn pick_dir(primary: PathBuf, fallback: PathBuf) -> PathBuf {
    if primary.exists() || fs::create_dir_all(&primary).is_ok() {
        primary
    } else {
        let _ = fs::create_dir_all(&fallback);
        fallback
    }
}

const SOCKET_NAME: &str = "com.smallshow.battery-toolkit-helper.sock";
const PID_NAME: &str = "com.smallshow.battery-toolkit-helper.pid";
const LOG_NAME: &str = "com.smallshow.battery-toolkit-helper.log";
const STDOUT_LOG_NAME: &str = "com.smallshow.battery-toolkit-helper.stdout.log";
const STDERR_LOG_NAME: &str = "com.smallshow.battery-toolkit-helper.stderr.log";
const ROOT_SUPPORT_DIR: &str = "/Library/Application Support/BatteryToolkit";
const ROOT_PLIST: &str = "/Library/LaunchDaemons/com.smallshow.battery-toolkit-helper.plist";
const SHARED_RUNTIME_DIR: &str = "/tmp/battery-toolkit";

fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

fn root_support_dir() -> PathBuf {
    PathBuf::from(ROOT_SUPPORT_DIR)
}

pub fn system_helper_root() -> PathBuf {
    root_support_dir()
}

fn user_support_dir() -> PathBuf {
    pick_dir(
        home_dir().join(".local/share/battery-toolkit"),
        env::temp_dir().join("battery-toolkit"),
    )
}

fn config_dir() -> PathBuf {
    if is_root() {
        root_support_dir()
    } else {
        pick_dir(
            home_dir().join(".config/battery-toolkit"),
            env::temp_dir().join("battery-toolkit-config"),
        )
    }
}

fn shared_runtime_dir() -> PathBuf {
    let path = PathBuf::from(SHARED_RUNTIME_DIR);
    let _ = fs::create_dir_all(&path);
    path
}

pub fn socket_path() -> PathBuf {
    shared_runtime_dir().join(SOCKET_NAME)
}

pub fn helper_root() -> PathBuf {
    if is_root() {
        root_support_dir()
    } else {
        user_support_dir()
    }
}

pub fn helper_bin_path() -> PathBuf {
    helper_root().join("bin/battery-helper")
}

pub fn system_helper_bin_path() -> PathBuf {
    system_helper_root().join("bin/battery-helper")
}

pub fn helper_pid_path() -> PathBuf {
    shared_runtime_dir().join(PID_NAME)
}

pub fn helper_log_path() -> PathBuf {
    shared_runtime_dir().join(LOG_NAME)
}

pub fn helper_stdout_log_path() -> PathBuf {
    shared_runtime_dir().join(STDOUT_LOG_NAME)
}

pub fn helper_stderr_log_path() -> PathBuf {
    shared_runtime_dir().join(STDERR_LOG_NAME)
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn runtime_path() -> PathBuf {
    helper_root().join("state.json")
}

pub fn launch_agent_path() -> PathBuf {
    home_dir().join("Library/LaunchAgents/com.smallshow.battery-toolkit-helper.plist")
}

pub fn launch_daemon_path() -> PathBuf {
    PathBuf::from(ROOT_PLIST)
}

fn ensure_parent(path: &PathBuf) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create {:?}: {}", parent, e))?;
    }
    Ok(())
}

fn read_json<T: DeserializeOwned>(path: &PathBuf) -> Option<T> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn write_json<T: Serialize>(path: &PathBuf, value: &T) -> Result<(), String> {
    ensure_parent(path)?;
    let json = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| format!("Failed to write {:?}: {}", path, e))
}

fn log_line(message: &str) {
    let path = helper_log_path();
    if ensure_parent(&path).is_err() {
        return;
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{}", message);
    }
}

pub fn load_settings() -> Settings {
    read_json(&settings_path()).unwrap_or_default()
}

pub fn save_settings(settings: &Settings) -> Result<(), String> {
    validate_settings(settings)?;
    write_json(&settings_path(), settings)
}

fn load_runtime() -> RuntimeState {
    read_json(&runtime_path()).unwrap_or_default()
}

fn save_runtime(state: &HelperState) -> Result<(), String> {
    let runtime = RuntimeState {
        mode: Some(state.mode),
        charging_disabled: Some(state.charging_disabled),
        power_disabled: Some(state.power_disabled),
    };
    write_json(&runtime_path(), &runtime)
}

fn initial_state() -> HelperState {
    let settings = load_settings();
    let runtime = load_runtime();
    HelperState {
        mode: runtime.mode.unwrap_or(ChargeMode::Standard),
        charging_disabled: runtime.charging_disabled.unwrap_or(false),
        power_disabled: runtime.power_disabled.unwrap_or(false),
        supported: false,
        control_available: false,
        settings,
        last_error: None,
    }
}

fn set_error(shared: &SharedState, message: String) {
    if let Ok(mut state) = shared.data.lock() {
        state.last_error = Some(message.clone());
    }
    log_line(&message);
}

fn clear_error(shared: &SharedState) {
    if let Ok(mut state) = shared.data.lock() {
        state.last_error = None;
    }
}

fn with_smc<F>(shared: &SharedState, action: F) -> Result<(), String>
where
    F: FnOnce(&SmcHandle) -> std::io::Result<()>,
{
    let guard = shared.smc.lock().map_err(|e| e.to_string())?;
    let handle = guard
        .as_ref()
        .ok_or("SMC not available for helper control".to_string())?;
    action(handle).map_err(|e| e.to_string())
}

fn enable_charging(shared: &SharedState) -> Result<(), String> {
    with_smc(shared, smc::enable_charging)?;
    if let Ok(mut state) = shared.data.lock() {
        state.charging_disabled = false;
        save_runtime(&state.clone())?;
    }
    clear_error(shared);
    Ok(())
}

fn disable_charging(shared: &SharedState) -> Result<(), String> {
    with_smc(shared, smc::disable_charging)?;
    if let Ok(mut state) = shared.data.lock() {
        state.charging_disabled = true;
        save_runtime(&state.clone())?;
    }
    clear_error(shared);
    Ok(())
}

fn enable_adapter(shared: &SharedState) -> Result<(), String> {
    with_smc(shared, smc::enable_adapter)?;
    if let Ok(mut state) = shared.data.lock() {
        state.power_disabled = false;
        save_runtime(&state.clone())?;
    }
    clear_error(shared);
    Ok(())
}

fn disable_adapter(shared: &SharedState) -> Result<(), String> {
    with_smc(shared, smc::disable_adapter)?;
    if let Ok(mut state) = shared.data.lock() {
        state.power_disabled = true;
        save_runtime(&state.clone())?;
    }
    clear_error(shared);
    Ok(())
}

fn set_magsafe_system(shared: &SharedState) {
    let _ = with_smc(shared, |handle| smc::set_magsafe_led(handle, 0x00));
}

fn apply_settings(shared: &SharedState, settings: Settings) -> Result<HelperState, String> {
    save_settings(&settings)?;
    let snapshot = {
        let mut state = shared.data.lock().map_err(|e| e.to_string())?;
        state.settings = settings;
        save_runtime(&state.clone())?;
        state.clone()
    };
    if snapshot.settings.magsafe_sync {
        set_magsafe_system(shared);
    }
    clear_error(shared);
    Ok(snapshot)
}

fn current_state(shared: &SharedState) -> Result<HelperState, String> {
    shared.data.lock().map(|s| s.clone()).map_err(|e| e.to_string())
}

fn update_mode(shared: &SharedState, mode: ChargeMode) -> Result<HelperState, String> {
    let snapshot = {
        let mut state = shared.data.lock().map_err(|e| e.to_string())?;
        state.mode = mode;
        state.clone()
    };
    save_runtime(&snapshot)?;
    Ok(snapshot)
}

fn evaluate(shared: &SharedState) -> Result<(), String> {
    let percent = get_battery_percent();
    let snapshot = current_state(shared)?;
    let disable = match snapshot.mode {
        ChargeMode::Standard => {
            if percent >= snapshot.settings.max_charge {
                true
            } else if percent < snapshot.settings.min_charge {
                false
            } else {
                snapshot.charging_disabled
            }
        }
        ChargeMode::ToLimit => percent >= snapshot.settings.max_charge,
        ChargeMode::ToFull => false,
    };

    if disable && !snapshot.charging_disabled {
        disable_charging(shared)?;
    } else if !disable && snapshot.charging_disabled {
        enable_charging(shared)?;
    }

    if let Ok(mut state) = shared.data.lock() {
        if matches!(state.mode, ChargeMode::ToLimit) && percent >= state.settings.max_charge {
            state.mode = ChargeMode::Standard;
        } else if matches!(state.mode, ChargeMode::ToFull) && percent >= 100 {
            state.mode = ChargeMode::Standard;
        }
        save_runtime(&state.clone())?;
    }

    Ok(())
}

fn handle_request(shared: &SharedState, request: Request) -> Response {
    let result = match request.command.as_str() {
        "ping" => Ok(json!({"pong": true})),
        "get_status" | "get_state" => current_state(shared).map(|state| json!(state)),
        "get_settings" => current_state(shared).map(|state| json!(state.settings)),
        "set_settings" => serde_json::from_value::<Settings>(request.payload)
            .map_err(|e| e.to_string())
            .and_then(|settings| apply_settings(shared, settings))
            .map(|state| json!(state)),
        "charge_to_full" => update_mode(shared, ChargeMode::ToFull)
            .and_then(|_| enable_charging(shared))
            .and_then(|_| current_state(shared))
            .map(|state| json!(state)),
        "charge_to_limit" => update_mode(shared, ChargeMode::ToLimit)
            .and_then(|_| enable_charging(shared))
            .and_then(|_| current_state(shared))
            .map(|state| json!(state)),
        "disable_charging" => disable_charging(shared)
            .and_then(|_| current_state(shared))
            .map(|state| json!(state)),
        "disable_adapter" => disable_adapter(shared)
            .and_then(|_| current_state(shared))
            .map(|state| json!(state)),
        "enable_adapter" => enable_adapter(shared)
            .and_then(|_| current_state(shared))
            .map(|state| json!(state)),
        "reset_charge_mode" => update_mode(shared, ChargeMode::Standard)
            .and_then(|_| enable_charging(shared))
            .and_then(|_| current_state(shared))
            .map(|state| json!(state)),
        _ => Err(format!("Unknown helper command: {}", request.command)),
    };

    match result {
        Ok(data) => Response {
            id: request.id,
            ok: true,
            data,
            error: None,
        },
        Err(error) => {
            set_error(shared, error.clone());
            Response {
                id: request.id,
                ok: false,
                data: Value::Null,
                error: Some(error),
            }
        }
    }
}

fn handle_stream(shared: &SharedState, stream: UnixStream) -> Result<(), String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut input = String::new();
    reader.read_to_string(&mut input).map_err(|e| e.to_string())?;
    let request: Request = serde_json::from_str(&input).map_err(|e| e.to_string())?;
    let response = handle_request(shared, request);
    let mut writer = stream;
    let payload = serde_json::to_vec(&response).map_err(|e| e.to_string())?;
    writer.write_all(&payload).map_err(|e| e.to_string())
}

fn polling_interval(shared: &SharedState) -> Duration {
    if crate::battery::get_is_plugged() {
        Duration::from_secs(5)
    } else {
        let state = current_state(shared).ok();
        if state.as_ref().is_some_and(|s| s.power_disabled) {
            Duration::from_secs(5)
        } else {
            Duration::from_secs(15)
        }
    }
}

fn write_pid() -> Result<(), String> {
    let path = helper_pid_path();
    ensure_parent(&path)?;
    fs::write(path, std::process::id().to_string()).map_err(|e| e.to_string())
}

fn remove_socket() {
    let path = socket_path();
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

pub fn helper_logs(lines: usize) -> Result<String, String> {
    let file = File::open(helper_log_path()).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let content: Vec<String> = reader.lines().map_while(Result::ok).collect();
    let start = content.len().saturating_sub(lines);
    Ok(content[start..].join("\n"))
}

pub fn helper_pid() -> Option<u32> {
    fs::read_to_string(helper_pid_path()).ok()?.trim().parse().ok()
}

pub fn run_daemon() -> Result<(), String> {
    remove_socket();
    let socket = socket_path();
    ensure_parent(&socket)?;
    write_pid()?;

    let smc_handle = match SmcHandle::open() {
        Ok(handle) => Some(handle),
        Err(error) => {
            log_line(&format!("SMC open failed: {}", error));
            None
        }
    };
    let supported = smc_handle
        .as_ref()
        .and_then(|handle| smc::probe_supported(handle).ok())
        .is_some();

    let mut state = initial_state();
    state.supported = supported;
    state.control_available = supported && smc_handle.is_some();

    let shared = SharedState {
        smc: Arc::new(Mutex::new(smc_handle)),
        data: Arc::new(Mutex::new(state)),
    };

    if let Ok(snapshot) = current_state(&shared) {
        let _ = save_runtime(&snapshot);
    }

    let poll_shared = shared.clone();
    std::thread::spawn(move || loop {
        if let Err(error) = evaluate(&poll_shared) {
            set_error(&poll_shared, error);
        }
        std::thread::sleep(polling_interval(&poll_shared));
    });

    let listener = UnixListener::bind(&socket).map_err(|e| e.to_string())?;
    let _ = fs::set_permissions(&socket, fs::Permissions::from_mode(0o666));
    log_line("battery-helper started");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = handle_stream(&shared, stream) {
                    set_error(&shared, error);
                }
            }
            Err(error) => set_error(&shared, error.to_string()),
        }
    }

    Ok(())
}
