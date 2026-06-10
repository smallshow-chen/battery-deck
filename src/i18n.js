// i18n module for Battery Toolkit
// Supports English (default) and Chinese (Simplified/Traditional).
// Detects system locale on init and applies translations to HTML elements
// marked with data-i18n attributes.

const TRANSLATIONS = {
  en: {
    // App chrome
    "loading.text": "Loading battery state...",
    "app.title": "Battery Toolkit",
    "app.subtitle": "Battery control for Apple Silicon Macs",
    "badge.enabled": "Enabled",
    "badge.disabled": "Disabled",
    "badge.standard": "Standard",
    "badge.tofull": "ToFull",
    "badge.tolimit": "ToLimit",
    "section.overview": "Overview",
    "section.strategy": "Charge Strategy",
    "section.health_snapshot": "Health",
    "section.live_metrics": "Live Metrics",
    "section.runtime": "Runtime",
    "section.power_input": "Power Input",
    "section.automation": "Automation",
    "section.service": "Service",
    "service.installed": "Installed",
    "service.running": "Running",
    "service.control": "Control",
    "service.ready": "Ready",
    "service.running_status": "Running",
    "service.stopped": "Stopped",
    "service.available": "Available",
    "service.unavailable": "Unavailable",
    "service.yes": "Yes",
    "service.no": "No",

    // Battery Status card
    "section.battery_status": "Battery Status",
    "label.charge": "Charge",
    "label.state": "State",
    "label.adapter": "Adapter",
    "label.connection": "Connection",
    "status.disconnected": "Disconnected",
    "status.charging": "Charging",
    "status.connected": "Connected",
    "status.disabled": "Disabled",
    "status.adapter_off": "Adapter Off",
    "status.on_battery": "On Battery",
    "status.available": "Available",

    // Charge Limits card
    "section.charge_limits": "Charge Limits",
    "label.min_charge": "Min Charge",
    "label.max_charge": "Max Charge",

    // Actions card
    "section.actions": "Actions",
    "btn.charge_full": "Charge to Full",
    "btn.charge_limit": "Charge to Limit",
    "btn.reset_charge_mode": "Resume Limits",
    "btn.disable_charging": "Disable Charging",
    "btn.disable_adapter": "Disable Adapter",
    "btn.enable_adapter": "Enable Adapter",
    "btn.install_service": "Install Root Service",
    "btn.start_service": "Start Service",
    "btn.stop_service": "Stop Service",
    "btn.refresh_logs": "Refresh Logs",

    // Settings card
    "section.settings": "Settings",
    "section.diagnostics": "Diagnostics",
    "section.service_log": "Service Log",
    "setting.adapter_sleep": "Disable sleep when adapter disabled",
    "setting.adapter_sleep_desc":
      "Prevents the system from sleeping when the power adapter is disabled",
    "setting.magsafe_sync": "Sync MagSafe LED",
    "setting.magsafe_sync_desc":
      "Synchronize the MagSafe LED indicator with charging state",

    // Unsupported banner
    "unsupported.message": "This device is not supported by Battery Toolkit.",

    // Realtime card
    "section.realtime": "Realtime",
    "rt.temperature": "Temperature",
    "rt.power": "Power",
    "rt.voltage": "Voltage",
    "rt.current": "Current",

    // Charger card
    "section.charger": "Charger",
    "charger.voltage": "Output Voltage",
    "charger.current": "Output Current",

    // Battery Health card
    "section.battery_health": "Battery Health",
    "health.cycle_count": "Cycle Count",
    "health.battery_life": "Battery Life",
    "health.design_capacity": "Design",
    "health.actual_capacity": "Actual",
    "health.current_capacity": "Current",

    // Service notice
    "notice.no_root": "Battery control is unavailable. Install the root helper service, then start the service to enable charging control.",

    // Success / toast messages
    "msg.settings_saved": "Settings saved",
    "msg.charging_full": "Charging to full",
    "msg.charging_limit": "Charging to limit",
    "msg.mode_reset": "Returned to standard charge limits",
    "msg.charging_disabled": "Charging disabled",
    "msg.adapter_enabled": "Adapter enabled",
    "msg.adapter_disabled": "Adapter disabled",
    "msg.service_installed": "Service installed",
    "msg.service_started": "Service started",
    "msg.service_stopped": "Service stopped",

    // Validation messages
    "validation.min_exceeds_max":
      "Min charge (%{min}%) exceeds max charge (%{max}%)",
    "validation.min_at_least": "Min charge must be at least 20%",
    "validation.max_at_least": "Max charge must be at least 50%",
  },

  zh: {
    "loading.text": "正在加载电池状态...",
    "app.title": "电池工具箱",
    "app.subtitle": "适用于 Apple Silicon Mac 的电池控制工具",
    "badge.enabled": "已启用",
    "badge.disabled": "已禁用",
    "badge.standard": "标准",
    "badge.tofull": "充满",
    "badge.tolimit": "限充",
    "section.overview": "总览",
    "section.strategy": "充电策略",
    "section.health_snapshot": "健康概览",
    "section.live_metrics": "实时指标",
    "section.runtime": "运行状态",
    "section.power_input": "供电输入",
    "section.automation": "自动化",
    "section.service": "服务",
    "service.installed": "已安装",
    "service.running": "运行中",
    "service.control": "控制",
    "service.ready": "就绪",
    "service.running_status": "运行中",
    "service.stopped": "已停止",
    "service.available": "可用",
    "service.unavailable": "不可用",
    "service.yes": "是",
    "service.no": "否",

    "section.battery_status": "电池状态",
    "label.charge": "电量",
    "label.state": "状态",
    "label.adapter": "适配器",
    "label.connection": "连接",
    "status.disconnected": "未连接",
    "status.charging": "充电中",
    "status.connected": "已连接",
    "status.disabled": "已禁用",
    "status.adapter_off": "适配器已关闭",
    "status.on_battery": "使用电池",
    "status.available": "可用",

    "section.charge_limits": "充电限制",
    "label.min_charge": "最低电量",
    "label.max_charge": "最高电量",

    "section.actions": "操作",
    "btn.charge_full": "充至满电",
    "btn.charge_limit": "充至上限",
    "btn.reset_charge_mode": "恢复限充",
    "btn.disable_charging": "停止充电",
    "btn.disable_adapter": "禁用适配器",
    "btn.enable_adapter": "启用适配器",
    "btn.install_service": "安装特权服务",
    "btn.start_service": "启动服务",
    "btn.stop_service": "停止服务",
    "btn.refresh_logs": "刷新日志",

    "section.settings": "设置",
    "section.diagnostics": "诊断",
    "section.service_log": "服务日志",
    "setting.adapter_sleep": "禁用适配器时阻止系统睡眠",
    "setting.adapter_sleep_desc": "当电源适配器被禁用时，阻止系统进入睡眠状态",
    "setting.magsafe_sync": "同步 MagSafe 指示灯",
    "setting.magsafe_sync_desc": "将 MagSafe 指示灯与充电状态同步",

    "unsupported.message": "此设备不支持电池工具箱。",

    "section.realtime": "实时状态",
    "rt.temperature": "温度",
    "rt.power": "功率",
    "rt.voltage": "电压",
    "rt.current": "电流",

    "section.charger": "充电器",
    "charger.voltage": "输出电压",
    "charger.current": "输出电流",

    "section.battery_health": "电池健康",
    "health.cycle_count": "循环次数",
    "health.battery_life": "电池寿命",
    "health.design_capacity": "设计容量",
    "health.actual_capacity": "实际容量",
    "health.current_capacity": "当前容量",

    "notice.no_root": "充电控制当前不可用。先安装特权 helper 服务，再启动服务后才能控制充电。",

    "msg.settings_saved": "设置已保存",
    "msg.charging_full": "正在充至满电",
    "msg.charging_limit": "正在充至上限",
    "msg.mode_reset": "已恢复标准限充模式",
    "msg.charging_disabled": "充电已禁用",
    "msg.adapter_enabled": "适配器已启用",
    "msg.adapter_disabled": "适配器已禁用",
    "msg.service_installed": "服务已安装",
    "msg.service_started": "服务已启动",
    "msg.service_stopped": "服务已停止",

    "validation.min_exceeds_max": "最低电量（%{min}%）不能超过最高电量（%{max}%）",
    "validation.min_at_least": "最低电量不能低于 20%",
    "validation.max_at_least": "最高电量不能低于 50%",
  },
};

// ── Language Detection ──────────────────────────────────────────────────────

function detectLocale() {
  // Try Tauri v2 system locale first
  if (window.__TAURI__?.locale) {
    try {
      const loc = window.__TAURI__.locale();
      if (loc) return normalizeLocale(loc);
    } catch (_) {}
  }

  // Browser language
  const nav =
    navigator.language || navigator.userLanguage || navigator.languages?.[0];
  return normalizeLocale(nav || "en");
}

function normalizeLocale(raw) {
  if (!raw) return "en";
  const lower = raw.toLowerCase();
  if (lower.startsWith("zh")) return "zh";
  return "en";
}

// ── Public API ──────────────────────────────────────────────────────────────

const LOCALE = detectLocale();
const _dict = TRANSLATIONS[LOCALE] || TRANSLATIONS.en;

/**
 * Look up a translation key. Supports %{name} interpolation.
 *
 *   t("validation.min_exceeds_max", { min: 20, max: 80 })
 *   // zh: "最低电量（20%）不能超过最高电量（80%）"
 */
function t(key, params) {
  let str = _dict[key] ?? key;
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      str = str.replace("%{" + k + "}", v);
    }
  }
  return str;
}

/**
 * Walk the DOM for all elements with a data-i18n attribute and replace
 * their textContent with the translated string for the current locale.
 * Skips elements that have no matching key (leaves the English default).
 */
function applyLocaleToDom() {
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    const key = el.getAttribute("data-i18n");
    if (key && _dict[key] !== undefined) {
      el.textContent = _dict[key];
    }
  });
}

// ── Init ────────────────────────────────────────────────────────────────────

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", applyLocaleToDom);
} else {
  applyLocaleToDom();
}

// Expose to main.js via global scope (no bundler available)
window.__i18n = { t, LOCALE };
