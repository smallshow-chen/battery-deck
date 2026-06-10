use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChargeMode {
    Standard,
    ToLimit,
    ToFull,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub min_charge: u8,
    pub max_charge: u8,
    pub adapter_sleep: bool,
    pub magsafe_sync: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            min_charge: 40,
            max_charge: 80,
            adapter_sleep: false,
            magsafe_sync: true,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeState {
    pub mode: Option<ChargeMode>,
    pub charging_disabled: Option<bool>,
    pub power_disabled: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperState {
    pub mode: ChargeMode,
    pub charging_disabled: bool,
    pub power_disabled: bool,
    pub supported: bool,
    pub control_available: bool,
    pub settings: Settings,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryState {
    pub enabled: bool,
    pub power_disabled: bool,
    pub connected: bool,
    pub is_plugged: bool,
    pub charging_disabled: bool,
    pub is_charging: bool,
    pub charge_percent: u8,
    pub mode: ChargeMode,
    pub min_charge: u8,
    pub max_charge: u8,
    pub adapter_sleep: bool,
    pub magsafe_sync: bool,
    pub supported: bool,
    pub control_available: bool,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    pub installed: bool,
    pub running: bool,
    pub control_available: bool,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryRealtime {
    pub temperature: f64,
    pub voltage: u32,
    pub amperage: i32,
    pub power: f64,
    pub is_charging: bool,
    pub external_connected: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargerInfo {
    pub name: String,
    pub wattage: u32,
    pub connected: bool,
    pub charging_voltage: u32,
    pub charging_current: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryHealth {
    pub cycle_count: u32,
    pub health_percent: f64,
    pub design_capacity: u32,
    pub max_capacity: u32,
    pub current_capacity: u32,
}

#[cfg(target_os = "macos")]
fn parse_hardware_value<'a>(lines: impl Iterator<Item = &'a str>, key: &str) -> Option<String> {
    let prefix = format!("{}:", key);
    for line in lines {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            let value = rest.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn parse_memory_gb(raw: &str) -> Option<u32> {
    let upper = raw.to_uppercase();
    let pos = upper.find("GB")?;
    let num_part = raw[..pos].trim();
    num_part
        .rfind(' ')
        .map(|i| num_part[i + 1..].trim())
        .unwrap_or(num_part)
        .parse::<u32>()
        .ok()
}

#[cfg(target_os = "macos")]
fn read_activation_date() -> Option<String> {
    let output = std::process::Command::new("stat")
        .args(["-f", "%Sm", "-t", "%Y-%m-%d", "/var/db/.AppleSetupDone"])
        .output()
        .ok()?;
    let date = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if date.is_empty() { None } else { Some(date) }
}

#[cfg(target_os = "macos")]
pub fn get_system_info() -> Option<SystemInfo> {
    let output = std::process::Command::new("system_profiler")
        .arg("SPHardwareDataType")
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    let model_name = parse_hardware_value(lines.iter().copied(), "Model Name")
        .unwrap_or_else(|| "Mac".to_string());
    let chip = parse_hardware_value(lines.iter().copied(), "Chip")
        .or_else(|| parse_hardware_value(lines.iter().copied(), "Processor Name"))
        .unwrap_or_else(|| "Apple Silicon".to_string());
    let memory_raw = parse_hardware_value(lines.iter().copied(), "Memory")
        .unwrap_or_else(|| "0 GB".to_string());
    let memory_gb = parse_memory_gb(&memory_raw).unwrap_or(0);

    let activation_date = read_activation_date();

    Some(SystemInfo {
        model_name,
        chip,
        memory_gb,
        activation_date,
    })
}

#[cfg(not(target_os = "macos"))]
pub fn get_system_info() -> Option<SystemInfo> {
    None
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub model_name: String,
    pub chip: String,
    pub memory_gb: u32,
    pub activation_date: Option<String>,
}

pub fn validate_settings(settings: &Settings) -> Result<(), String> {
    if settings.min_charge < 20 {
        return Err("min_charge must be at least 20".to_string());
    }
    if settings.max_charge < 50 {
        return Err("max_charge must be at least 50".to_string());
    }
    if settings.max_charge > 100 || settings.min_charge > 100 {
        return Err("Charge values must be 0-100".to_string());
    }
    if settings.min_charge >= settings.max_charge {
        return Err("min_charge must be less than max_charge".to_string());
    }
    Ok(())
}

pub fn get_battery_info() -> Option<(u8, bool)> {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("pmset")
            .args(["-g", "batt"])
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with('-') {
                continue;
            }

            let percent = trimmed
                .split_whitespace()
                .find(|s| s.contains('%'))
                .and_then(|s| {
                    let num_str = s.trim_end_matches(|c: char| !c.is_ascii_digit());
                    num_str.parse::<u8>().ok()
                })?;

            let is_charging = trimmed.contains("charging")
                && !trimmed.contains("discharging")
                && !trimmed.contains("not charging");

            return Some((percent, is_charging));
        }
        None
    }

    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

pub fn get_battery_percent() -> u8 {
    get_battery_info().map(|(p, _)| p).unwrap_or(0)
}

#[cfg(target_os = "macos")]
pub fn get_is_plugged() -> bool {
    ioreg_battery_props()
        .map(|s| parse_ioreg_bool(&s, "ExternalConnected"))
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
pub fn get_is_plugged() -> bool {
    false
}

#[cfg(target_os = "macos")]
fn parse_ioreg_value<T: std::str::FromStr>(stdout: &str, key: &str) -> Option<T> {
    use regex::Regex;
    let escaped = regex::escape(key);
    let pattern = format!("\"{}\"\\s*=\\s*\"?([-]?\\d+\\.?\\d*)\"?", escaped);
    let re = Regex::new(&pattern).ok()?;
    let cap = re.captures(stdout)?;
    cap.get(1)?.as_str().parse::<T>().ok()
}

#[cfg(target_os = "macos")]
fn parse_ioreg_bool(stdout: &str, key: &str) -> bool {
    use regex::Regex;
    let escaped = regex::escape(key);
    let pattern = format!("\"{}\"\\s*=\\s*(Yes|No)", escaped);
    Regex::new(&pattern)
        .ok()
        .and_then(|re| re.captures(stdout))
        .map(|cap| cap.get(1).is_some_and(|m| m.as_str() == "Yes"))
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn ioreg_battery_props() -> Option<String> {
    let output = std::process::Command::new("ioreg")
        .args(["-r", "-c", "AppleSmartBattery"])
        .output()
        .ok()?;
    String::from_utf8(output.stdout).ok()
}

#[cfg(target_os = "macos")]
pub fn get_battery_realtime() -> Option<BatteryRealtime> {
    let stdout = ioreg_battery_props()?;

    let voltage = parse_ioreg_value::<u32>(&stdout, "Voltage").unwrap_or(0);
    let amperage = parse_ioreg_value::<i32>(&stdout, "InstantAmperage")
        .filter(|&v| v != 0)
        .or_else(|| parse_ioreg_value::<i32>(&stdout, "Amperage"))
        .unwrap_or(0);
    let temp_raw = parse_ioreg_value::<f64>(&stdout, "Temperature").unwrap_or(0.0);
    let is_charging = parse_ioreg_bool(&stdout, "IsCharging");
    let external_connected = parse_ioreg_bool(&stdout, "ExternalConnected");

    let power = ((voltage as f64 * amperage as f64) / 1_000_000.0).abs();

    Some(BatteryRealtime {
        temperature: (temp_raw / 10.0).round() / 10.0,
        voltage,
        amperage,
        power: (power * 100.0).round() / 100.0,
        is_charging,
        external_connected,
    })
}

#[cfg(not(target_os = "macos"))]
pub fn get_battery_realtime() -> Option<BatteryRealtime> {
    None
}

#[cfg(target_os = "macos")]
pub fn get_charger_info() -> Option<ChargerInfo> {
    let ioreg_out = std::process::Command::new("ioreg")
        .args(["-l", "-w0"])
        .output()
        .ok()?;
    let ioreg_str = String::from_utf8_lossy(&ioreg_out.stdout);

    let external = parse_ioreg_bool(&ioreg_str, "ExternalConnected");
    if !external {
        return Some(ChargerInfo {
            name: "—".to_string(),
            wattage: 0,
            connected: false,
            charging_voltage: 0,
            charging_current: 0,
        });
    }

    let charging_voltage = parse_ioreg_value::<u32>(&ioreg_str, "ChargingVoltage").unwrap_or(0);
    let charging_current = parse_ioreg_value::<u32>(&ioreg_str, "ChargingCurrent").unwrap_or(0);

    let (name, wattage) = match std::process::Command::new("system_profiler")
        .args(["SPPowerDataType"])
        .output()
    {
        Ok(out) => {
            let s = String::from_utf8_lossy(&out.stdout);
            let mut n = String::new();
            let mut w: u32 = 0;
            for line in s.lines() {
                let t = line.trim();
                if t.starts_with("Name:") && n.is_empty() {
                    n = t.strip_prefix("Name:").unwrap_or("").trim().to_string();
                }
                if t.starts_with("Wattage (W):") {
                    w = t
                        .strip_prefix("Wattage (W):")
                        .unwrap_or("")
                        .trim()
                        .parse()
                        .unwrap_or(0);
                }
            }
            (n, w)
        }
        Err(_) => (String::new(), 0),
    };

    Some(ChargerInfo {
        name: if name.is_empty() {
            "USB-C Power Adapter".to_string()
        } else {
            name
        },
        wattage,
        connected: true,
        charging_voltage,
        charging_current,
    })
}

#[cfg(not(target_os = "macos"))]
pub fn get_charger_info() -> Option<ChargerInfo> {
    None
}

#[cfg(target_os = "macos")]
pub fn get_battery_health() -> Option<BatteryHealth> {
    let stdout = ioreg_battery_props()?;

    let design = parse_ioreg_value::<u32>(&stdout, "DesignCapacity")?;
    let max = parse_ioreg_value::<f64>(&stdout, "AppleRawMaxCapacity")
        .map(|v| v as u32)
        .unwrap_or(design);
    let cycles = parse_ioreg_value::<u32>(&stdout, "CycleCount").unwrap_or(0);
    let percent = get_battery_percent();
    let current = (percent as f64 / 100.0 * max as f64).round() as u32;

    let health = if design > 0 {
        (max as f64 / design as f64) * 100.0
    } else {
        0.0
    };

    Some(BatteryHealth {
        cycle_count: cycles,
        health_percent: (health * 10.0).round() / 10.0,
        design_capacity: design,
        max_capacity: max,
        current_capacity: current,
    })
}

#[cfg(not(target_os = "macos"))]
pub fn get_battery_health() -> Option<BatteryHealth> {
    None
}
