use crate::battery::{HelperState, ServiceStatus};
use crate::helper;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SERVICE_LABEL: &str = "com.smallshow.battery-toolkit-helper";

#[derive(Serialize)]
struct Request {
    id: String,
    command: String,
    payload: Value,
}

#[derive(Deserialize)]
struct Response {
    ok: bool,
    data: Value,
    error: Option<String>,
}

fn next_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_nanos();
    format!("req-{}", nanos)
}

fn quote_shell(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn quote_applescript(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn system_installed_bin() -> PathBuf {
    helper::system_helper_bin_path()
}

fn user_installed_bin() -> PathBuf {
    helper::helper_bin_path()
}

fn launch_agent_path() -> PathBuf {
    helper::launch_agent_path()
}

fn launch_daemon_path() -> PathBuf {
    helper::launch_daemon_path()
}

fn sibling_helper_bin() -> Option<PathBuf> {
    let current = std::env::current_exe().ok()?;
    let candidate = current.with_file_name("battery-helper");
    candidate.exists().then_some(candidate)
}

fn helper_source_bin() -> Result<PathBuf, String> {
    sibling_helper_bin().ok_or_else(|| {
        "Could not locate battery-helper binary. Build the helper target before starting the service."
            .to_string()
    })
}

fn launchctl_available() -> bool {
    Command::new("launchctl")
        .arg("help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn wait_until_running() -> Result<ServiceStatus, String> {
    for _ in 0..16 {
        std::thread::sleep(Duration::from_millis(250));
        let status = get_service_status()?;
        if status.running {
            return Ok(status);
        }
    }
    get_service_status()
}

fn user_plist_contents(program: &Path) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{program}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>WorkingDirectory</key>
  <string>{workdir}</string>
  <key>StandardOutPath</key>
  <string>{stdout_log}</string>
  <key>StandardErrorPath</key>
  <string>{stderr_log}</string>
  <key>EnvironmentVariables</key>
  <dict>
    <key>PATH</key>
    <string>/usr/bin:/bin:/usr/sbin:/sbin</string>
  </dict>
</dict>
</plist>
"#,
        label = SERVICE_LABEL,
        program = program.display(),
        workdir = helper::helper_root().display(),
        stdout_log = helper::helper_stdout_log_path().display(),
        stderr_log = helper::helper_stderr_log_path().display(),
    )
}

fn system_plist_contents(program: &Path) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{program}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>WorkingDirectory</key>
  <string>{workdir}</string>
  <key>StandardOutPath</key>
  <string>{stdout_log}</string>
  <key>StandardErrorPath</key>
  <string>{stderr_log}</string>
  <key>EnvironmentVariables</key>
  <dict>
    <key>PATH</key>
    <string>/usr/bin:/bin:/usr/sbin:/sbin</string>
  </dict>
</dict>
</plist>
"#,
        label = SERVICE_LABEL,
        program = program.display(),
        workdir = helper::system_helper_root().display(),
        stdout_log = helper::helper_stdout_log_path().display(),
        stderr_log = helper::helper_stderr_log_path().display(),
    )
}

fn write_launch_agent(executable: &Path) -> Result<(), String> {
    let plist = launch_agent_path();
    if let Some(parent) = plist.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&plist, user_plist_contents(executable)).map_err(|e| e.to_string())
}

fn launchctl_bootstrap_user() -> Result<(), String> {
    if !launch_agent_path().exists() {
        return Err("LaunchAgent plist does not exist".to_string());
    }

    let _ = Command::new("launchctl")
        .args([
            "bootout",
            &format!("gui/{}/{}", uid_string(), SERVICE_LABEL),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let status = Command::new("launchctl")
        .args([
            "bootstrap",
            &format!("gui/{}", uid_string()),
            launch_agent_path().to_string_lossy().as_ref(),
        ])
        .status()
        .map_err(|e| e.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err("launchctl bootstrap failed".to_string())
    }
}

fn launchctl_bootout_user() -> Result<(), String> {
    if !launchctl_available() {
        return Ok(());
    }
    let status = Command::new("launchctl")
        .args([
            "bootout",
            &format!("gui/{}/{}", uid_string(), SERVICE_LABEL),
        ])
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("launchctl bootout failed".to_string())
    }
}

fn uid_string() -> String {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "0".to_string())
}

fn temp_artifact(name: &str, contents: &str) -> Result<PathBuf, String> {
    let path = std::env::temp_dir().join(format!("battery-toolkit-{}-{}", next_id(), name));
    fs::write(&path, contents).map_err(|e| e.to_string())?;
    Ok(path)
}

fn run_privileged_script(body: &str) -> Result<(), String> {
    let script = temp_artifact("root-service.sh", &format!("#!/bin/sh\nset -e\n{}\n", body))?;
    let chmod_status = Command::new("chmod")
        .args(["755", script.to_string_lossy().as_ref()])
        .status()
        .map_err(|e| e.to_string())?;
    if !chmod_status.success() {
        let _ = fs::remove_file(&script);
        return Err("Failed to make helper install script executable".to_string());
    }

    let shell_command = format!("/bin/sh {}", quote_shell(script.to_string_lossy().as_ref()));
    let status = Command::new("osascript")
        .args([
            "-e",
            &format!(
                "do shell script \"{}\" with administrator privileges",
                quote_applescript(&shell_command)
            ),
        ])
        .status()
        .map_err(|e| e.to_string())?;

    let _ = fs::remove_file(&script);

    if status.success() {
        Ok(())
    } else {
        Err("Administrator authorization failed".to_string())
    }
}

fn install_root_service() -> Result<(), String> {
    let source = helper_source_bin()?;
    let target = system_installed_bin();
    let plist_path = temp_artifact("launchd.plist", &system_plist_contents(&target))?;

    let body = format!(
        "mkdir -p {support_dir}\nmkdir -p {bin_dir}\ninstall -m 755 {source} {target}\ninstall -d -m 755 /Library/LaunchDaemons\ninstall -m 644 {plist_src} {plist_dst}\nlaunchctl bootout system/{label} >/dev/null 2>&1 || true\nrm -f {socket} {pid}\nlaunchctl bootstrap system {plist_dst}\n",
        support_dir = quote_shell(helper::system_helper_root().to_string_lossy().as_ref()),
        bin_dir = quote_shell(
            target
                .parent()
                .unwrap_or_else(|| Path::new("/Library/Application Support/BatteryToolkit/bin"))
                .to_string_lossy()
                .as_ref(),
        ),
        source = quote_shell(source.to_string_lossy().as_ref()),
        target = quote_shell(target.to_string_lossy().as_ref()),
        plist_src = quote_shell(plist_path.to_string_lossy().as_ref()),
        plist_dst = quote_shell(launch_daemon_path().to_string_lossy().as_ref()),
        label = SERVICE_LABEL,
        socket = quote_shell(helper::socket_path().to_string_lossy().as_ref()),
        pid = quote_shell(helper::helper_pid_path().to_string_lossy().as_ref()),
    );

    let result = run_privileged_script(&body);
    let _ = fs::remove_file(plist_path);
    result
}

fn restart_root_service() -> Result<(), String> {
    if !launch_daemon_path().exists() {
        return Err("Root helper is not installed".to_string());
    }

    let body = format!(
        "launchctl bootout system/{label} >/dev/null 2>&1 || true\nrm -f {socket} {pid}\nlaunchctl bootstrap system {plist}\n",
        label = SERVICE_LABEL,
        socket = quote_shell(helper::socket_path().to_string_lossy().as_ref()),
        pid = quote_shell(helper::helper_pid_path().to_string_lossy().as_ref()),
        plist = quote_shell(launch_daemon_path().to_string_lossy().as_ref()),
    );
    run_privileged_script(&body)
}

fn stop_root_service() -> Result<(), String> {
    if !launch_daemon_path().exists() {
        return Ok(());
    }

    let body = format!(
        "launchctl bootout system/{label} >/dev/null 2>&1 || true\nrm -f {socket} {pid}\n",
        label = SERVICE_LABEL,
        socket = quote_shell(helper::socket_path().to_string_lossy().as_ref()),
        pid = quote_shell(helper::helper_pid_path().to_string_lossy().as_ref()),
    );
    run_privileged_script(&body)
}

fn install_user_service() -> Result<(), String> {
    let source = helper_source_bin()?;
    let target = user_installed_bin();
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::copy(source, &target).map_err(|e| e.to_string())?;
    write_launch_agent(&target)
}

fn spawn_fallback() -> Result<(), String> {
    let executable = if user_installed_bin().exists() {
        user_installed_bin()
    } else {
        helper_source_bin()?
    };

    let log_path = helper::helper_log_path();
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let stdout = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| e.to_string())?;
    let stderr = stdout.try_clone().map_err(|e| e.to_string())?;

    let mut cmd = Command::new(executable);
    cmd.stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .stdin(Stdio::null());
    cmd.process_group(0);
    cmd.spawn().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn install_service() -> Result<(), String> {
    let _ = stop_service();
    install_root_service()
}

pub fn start_service() -> Result<ServiceStatus, String> {
    if ping().is_ok() {
        return get_service_status();
    }

    if launch_daemon_path().exists() {
        restart_root_service()?;
        return wait_until_running();
    }

    if install_user_service().is_ok() && launchctl_available() && launchctl_bootstrap_user().is_ok()
    {
        let status = wait_until_running()?;
        if status.running {
            return Ok(status);
        }
    }

    spawn_fallback()?;
    wait_until_running()
}

pub fn stop_service() -> Result<(), String> {
    let mut errors = Vec::new();

    if launch_daemon_path().exists() {
        if let Err(error) = stop_root_service() {
            errors.push(error);
        }
    }

    if launch_agent_path().exists() {
        if let Err(error) = launchctl_bootout_user() {
            errors.push(error);
        }
    }

    if let Some(pid) = helper::helper_pid() {
        let status = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            errors.push("Failed to terminate helper service".to_string());
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

pub fn get_service_status() -> Result<ServiceStatus, String> {
    let running = ping().is_ok();
    let state = helper_state().ok();
    Ok(ServiceStatus {
        installed: launch_daemon_path().exists()
            || user_installed_bin().exists()
            || sibling_helper_bin().is_some()
            || launch_agent_path().exists(),
        running,
        control_available: state.as_ref().is_some_and(|s| s.control_available),
        last_error: state.and_then(|s| s.last_error),
    })
}

pub fn helper_state() -> Result<HelperState, String> {
    request("get_state", Value::Null)
}

pub fn get_settings<T: DeserializeOwned>() -> Result<T, String> {
    request("get_settings", Value::Null)
}

pub fn send_command<T: DeserializeOwned>(command: &str, payload: Value) -> Result<T, String> {
    request(command, payload)
}

pub fn ping() -> Result<(), String> {
    let _: Value = request("ping", Value::Null)?;
    Ok(())
}

pub fn get_logs(lines: usize) -> Result<String, String> {
    match helper::helper_logs(lines) {
        Ok(logs) => Ok(logs),
        Err(error) if error.contains("No such file or directory") => Ok("--".to_string()),
        Err(error) => Err(error),
    }
}

fn request<T: DeserializeOwned>(command: &str, payload: Value) -> Result<T, String> {
    let socket_path = helper::socket_path();
    let mut stream = UnixStream::connect(socket_path).map_err(|e| e.to_string())?;
    let req = Request {
        id: next_id(),
        command: command.to_string(),
        payload,
    };
    let bytes = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
    stream.write_all(&bytes).map_err(|e| e.to_string())?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|e| e.to_string())?;

    let mut raw = String::new();
    stream.read_to_string(&mut raw).map_err(|e| e.to_string())?;
    let response: Response = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    if !response.ok {
        return Err(response
            .error
            .unwrap_or_else(|| "Unknown helper service error".to_string()));
    }
    serde_json::from_value(response.data).map_err(|e| e.to_string())
}

pub fn settings_payload(
    min_charge: u8,
    max_charge: u8,
    adapter_sleep: bool,
    magsafe_sync: bool,
) -> Value {
    json!({
        "minCharge": min_charge,
        "maxCharge": max_charge,
        "adapterSleep": adapter_sleep,
        "magsafeSync": magsafe_sync
    })
}
