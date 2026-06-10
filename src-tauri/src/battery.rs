use serde::{Deserialize, Serialize};
#[cfg(target_os = "macos")]
use std::process::Command;
#[cfg(target_os = "macos")]
use std::sync::Mutex;
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};

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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub model_name: String,
    pub chip: String,
    pub memory_gb: u32,
    pub activation_date: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshot {
    pub battery_state: BatteryState,
    pub service_status: ServiceStatus,
    pub battery_health: Option<BatteryHealth>,
    pub battery_realtime: Option<BatteryRealtime>,
    pub charger_info: Option<ChargerInfo>,
    pub system_info: Option<SystemInfo>,
    pub settings: Settings,
}

#[cfg(target_os = "macos")]
const BATTERY_DATA_TTL: Duration = Duration::from_secs(2);
#[cfg(target_os = "macos")]
const CHARGER_PROFILE_TTL: Duration = Duration::from_secs(60);

#[cfg(target_os = "macos")]
#[derive(Clone)]
struct CachedChargerProfile {
    name: String,
    wattage: u32,
    external_connected: bool,
    fetched_at: Instant,
}

#[cfg(target_os = "macos")]
struct BatteryCacheInner {
    battery_ioreg_at: Option<Instant>,
    battery_ioreg_data: Option<String>,
    power_ioreg_at: Option<Instant>,
    power_ioreg_data: Option<String>,
    pmset_at: Option<Instant>,
    pmset_data: Option<String>,
    system_info: Option<SystemInfo>,
    charger_profile: Option<CachedChargerProfile>,
}

#[cfg(target_os = "macos")]
pub struct BatteryCache {
    inner: Mutex<BatteryCacheInner>,
}

#[cfg(not(target_os = "macos"))]
pub struct BatteryCache;

impl BatteryCache {
    pub fn new() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self {
                inner: Mutex::new(BatteryCacheInner {
                    battery_ioreg_at: None,
                    battery_ioreg_data: None,
                    power_ioreg_at: None,
                    power_ioreg_data: None,
                    pmset_at: None,
                    pmset_data: None,
                    system_info: None,
                    charger_profile: None,
                }),
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            Self
        }
    }
}

impl Default for BatteryCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "macos")]
fn parse_hardware_value<'a>(lines: impl Iterator<Item = &'a str>, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
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
    let output = Command::new("stat")
        .args(["-f", "%Sm", "-t", "%Y-%m-%d", "/var/db/.AppleSetupDone"])
        .output()
        .ok()?;
    let date = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if date.is_empty() {
        None
    } else {
        Some(date)
    }
}

#[cfg(target_os = "macos")]
fn parse_pmset_battery(stdout: &str) -> Option<(u8, bool)> {
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

#[cfg(target_os = "macos")]
fn parse_ioreg_number<T: std::str::FromStr>(stdout: &str, key: &str) -> Option<T> {
    let needle = format!("\"{key}\" = ");
    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&needle) {
            return rest.trim().trim_matches('"').parse::<T>().ok();
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn parse_ioreg_bool(stdout: &str, key: &str) -> bool {
    let needle = format!("\"{key}\" = ");
    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&needle) {
            return rest.trim() == "Yes";
        }
    }
    false
}

#[cfg(target_os = "macos")]
fn parse_charger_profile(stdout: &str) -> (String, u32) {
    let mut name = String::new();
    let mut wattage = 0;

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Name:") && name.is_empty() {
            name = trimmed
                .strip_prefix("Name:")
                .unwrap_or("")
                .trim()
                .to_string();
        }
        if trimmed.starts_with("Wattage (W):") {
            wattage = trimmed
                .strip_prefix("Wattage (W):")
                .unwrap_or("")
                .trim()
                .parse()
                .unwrap_or(0);
        }
    }

    (name, wattage)
}

#[cfg(target_os = "macos")]
fn probe_system_info() -> Option<SystemInfo> {
    let output = Command::new("system_profiler")
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
    let memory_raw =
        parse_hardware_value(lines.iter().copied(), "Memory").unwrap_or_else(|| "0 GB".to_string());
    let memory_gb = parse_memory_gb(&memory_raw).unwrap_or(0);

    Some(SystemInfo {
        model_name,
        chip,
        memory_gb,
        activation_date: read_activation_date(),
    })
}

#[cfg(target_os = "macos")]
fn probe_battery_ioreg() -> Option<String> {
    let output = Command::new("ioreg")
        .args(["-r", "-c", "AppleSmartBattery"])
        .output()
        .ok()?;
    String::from_utf8(output.stdout).ok()
}

#[cfg(target_os = "macos")]
fn probe_power_ioreg() -> Option<String> {
    let output = Command::new("ioreg").args(["-l", "-w0"]).output().ok()?;
    String::from_utf8(output.stdout).ok()
}

#[cfg(target_os = "macos")]
fn probe_pmset() -> Option<String> {
    let output = Command::new("pmset").args(["-g", "batt"]).output().ok()?;
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(target_os = "macos")]
fn probe_charger_profile() -> (String, u32) {
    match Command::new("system_profiler")
        .args(["SPPowerDataType"])
        .output()
    {
        Ok(output) => parse_charger_profile(&String::from_utf8_lossy(&output.stdout)),
        Err(_) => (String::new(), 0),
    }
}

#[cfg(target_os = "macos")]
fn cached_battery_ioreg(cache: &BatteryCache) -> Option<String> {
    let mut inner = cache.inner.lock().ok()?;
    if let (Some(at), Some(data)) = (inner.battery_ioreg_at, inner.battery_ioreg_data.as_ref()) {
        if at.elapsed() < BATTERY_DATA_TTL {
            return Some(data.clone());
        }
    }

    let data = probe_battery_ioreg()?;
    inner.battery_ioreg_at = Some(Instant::now());
    inner.battery_ioreg_data = Some(data.clone());
    Some(data)
}

#[cfg(target_os = "macos")]
fn cached_power_ioreg(cache: &BatteryCache) -> Option<String> {
    let mut inner = cache.inner.lock().ok()?;
    if let (Some(at), Some(data)) = (inner.power_ioreg_at, inner.power_ioreg_data.as_ref()) {
        if at.elapsed() < BATTERY_DATA_TTL {
            return Some(data.clone());
        }
    }

    let data = probe_power_ioreg()?;
    inner.power_ioreg_at = Some(Instant::now());
    inner.power_ioreg_data = Some(data.clone());
    Some(data)
}

#[cfg(target_os = "macos")]
fn cached_pmset(cache: &BatteryCache) -> Option<String> {
    let mut inner = cache.inner.lock().ok()?;
    if let (Some(at), Some(data)) = (inner.pmset_at, inner.pmset_data.as_ref()) {
        if at.elapsed() < BATTERY_DATA_TTL {
            return Some(data.clone());
        }
    }

    let data = probe_pmset()?;
    inner.pmset_at = Some(Instant::now());
    inner.pmset_data = Some(data.clone());
    Some(data)
}

#[cfg(target_os = "macos")]
fn build_realtime(stdout: &str) -> BatteryRealtime {
    let voltage = parse_ioreg_number::<u32>(stdout, "Voltage").unwrap_or(0);
    let amperage = parse_ioreg_number::<i32>(stdout, "InstantAmperage")
        .filter(|&v| v != 0)
        .or_else(|| parse_ioreg_number::<i32>(stdout, "Amperage"))
        .unwrap_or(0);
    let temp_raw = parse_ioreg_number::<f64>(stdout, "Temperature").unwrap_or(0.0);
    let is_charging = parse_ioreg_bool(stdout, "IsCharging");
    let external_connected = parse_ioreg_bool(stdout, "ExternalConnected");
    let power = ((voltage as f64 * amperage as f64) / 1_000_000.0).abs();

    BatteryRealtime {
        temperature: (temp_raw / 10.0).round() / 10.0,
        voltage,
        amperage,
        power: (power * 100.0).round() / 100.0,
        is_charging,
        external_connected,
    }
}

#[cfg(target_os = "macos")]
fn build_health(stdout: &str, percent: u8) -> Option<BatteryHealth> {
    let design = parse_ioreg_number::<u32>(stdout, "DesignCapacity")?;
    let max = parse_ioreg_number::<f64>(stdout, "AppleRawMaxCapacity")
        .map(|v| v as u32)
        .unwrap_or(design);
    let cycles = parse_ioreg_number::<u32>(stdout, "CycleCount").unwrap_or(0);
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

#[cfg(target_os = "macos")]
fn build_charger_info(ioreg_stdout: &str, name: String, wattage: u32) -> ChargerInfo {
    let charging_voltage = parse_ioreg_number::<u32>(ioreg_stdout, "ChargingVoltage").unwrap_or(0);
    let charging_current = parse_ioreg_number::<u32>(ioreg_stdout, "ChargingCurrent").unwrap_or(0);
    let external_connected = parse_ioreg_bool(ioreg_stdout, "ExternalConnected");

    if !external_connected {
        return ChargerInfo {
            name: "—".to_string(),
            wattage: 0,
            connected: false,
            charging_voltage,
            charging_current,
        };
    }

    ChargerInfo {
        name: if name.is_empty() {
            "USB-C Power Adapter".to_string()
        } else {
            name
        },
        wattage,
        connected: true,
        charging_voltage,
        charging_current,
    }
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
        parse_pmset_battery(&probe_pmset()?)
    }

    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

pub fn get_battery_percent() -> u8 {
    get_battery_info().map(|(percent, _)| percent).unwrap_or(0)
}

#[cfg(target_os = "macos")]
pub fn get_is_plugged() -> bool {
    probe_battery_ioreg()
        .map(|stdout| parse_ioreg_bool(&stdout, "ExternalConnected"))
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
pub fn get_is_plugged() -> bool {
    false
}

#[cfg(target_os = "macos")]
pub fn get_battery_info_cached(cache: &BatteryCache) -> Option<(u8, bool)> {
    parse_pmset_battery(&cached_pmset(cache)?)
}

#[cfg(not(target_os = "macos"))]
pub fn get_battery_info_cached(_cache: &BatteryCache) -> Option<(u8, bool)> {
    None
}

#[cfg(target_os = "macos")]
pub fn get_battery_percent_cached(cache: &BatteryCache) -> u8 {
    get_battery_info_cached(cache)
        .map(|(percent, _)| percent)
        .unwrap_or(0)
}

#[cfg(not(target_os = "macos"))]
pub fn get_battery_percent_cached(_cache: &BatteryCache) -> u8 {
    0
}

#[cfg(target_os = "macos")]
pub fn get_is_plugged_cached(cache: &BatteryCache) -> bool {
    cached_battery_ioreg(cache)
        .map(|stdout| parse_ioreg_bool(&stdout, "ExternalConnected"))
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
pub fn get_is_plugged_cached(_cache: &BatteryCache) -> bool {
    false
}

#[cfg(target_os = "macos")]
pub fn get_battery_realtime_cached(cache: &BatteryCache) -> Option<BatteryRealtime> {
    Some(build_realtime(&cached_battery_ioreg(cache)?))
}

#[cfg(not(target_os = "macos"))]
pub fn get_battery_realtime_cached(_cache: &BatteryCache) -> Option<BatteryRealtime> {
    None
}

#[cfg(target_os = "macos")]
pub fn get_battery_health_cached(cache: &BatteryCache) -> Option<BatteryHealth> {
    let percent = get_battery_percent_cached(cache);
    build_health(&cached_battery_ioreg(cache)?, percent)
}

#[cfg(not(target_os = "macos"))]
pub fn get_battery_health_cached(_cache: &BatteryCache) -> Option<BatteryHealth> {
    None
}

#[cfg(target_os = "macos")]
pub fn get_charger_info_cached(cache: &BatteryCache) -> Option<ChargerInfo> {
    let power_ioreg = cached_power_ioreg(cache)?;
    let external_connected = parse_ioreg_bool(&power_ioreg, "ExternalConnected");

    let cached_profile = cache
        .inner
        .lock()
        .ok()
        .and_then(|inner| inner.charger_profile.clone());

    let refresh_profile = match &cached_profile {
        Some(profile) => {
            profile.external_connected != external_connected
                || profile.fetched_at.elapsed() >= CHARGER_PROFILE_TTL
        }
        None => external_connected,
    };

    let (name, wattage) = if refresh_profile {
        let (name, wattage) = probe_charger_profile();
        if let Ok(mut inner) = cache.inner.lock() {
            inner.charger_profile = Some(CachedChargerProfile {
                name: name.clone(),
                wattage,
                external_connected,
                fetched_at: Instant::now(),
            });
        }
        (name, wattage)
    } else {
        cached_profile
            .map(|profile| (profile.name, profile.wattage))
            .unwrap_or_else(|| (String::new(), 0))
    };

    Some(build_charger_info(&power_ioreg, name, wattage))
}

#[cfg(not(target_os = "macos"))]
pub fn get_charger_info_cached(_cache: &BatteryCache) -> Option<ChargerInfo> {
    None
}

#[cfg(target_os = "macos")]
pub fn get_system_info_cached(cache: &BatteryCache) -> Option<SystemInfo> {
    {
        let inner = cache.inner.lock().ok()?;
        if let Some(system_info) = inner.system_info.as_ref() {
            return Some(system_info.clone());
        }
    }

    let system_info = probe_system_info()?;
    if let Ok(mut inner) = cache.inner.lock() {
        inner.system_info = Some(system_info.clone());
    }
    Some(system_info)
}

#[cfg(not(target_os = "macos"))]
pub fn get_system_info_cached(_cache: &BatteryCache) -> Option<SystemInfo> {
    None
}
