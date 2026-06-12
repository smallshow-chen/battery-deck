// Battery Toolkit i18n runtime
// Supports persisted locale mode and runtime re-translation.

const TRANSLATIONS = {
  en: {
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
    "section.settings": "Settings",
    "section.diagnostics": "Diagnostics",
    "section.service_log": "Service Log",
    "section.appearance": "Appearance",
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
    "section.battery_status": "Battery Status",
    "label.charge": "Charge",
    "label.state": "State",
    "label.adapter": "Adapter",
    "label.connection": "Connection",
    "label.min_charge": "Min Charge",
    "label.max_charge": "Max Charge",
    "status.disconnected": "Disconnected",
    "status.charging": "Charging",
    "status.connected": "Connected",
    "status.disabled": "Disabled",
    "status.adapter_off": "Adapter Off",
    "status.on_battery": "On Battery",
    "status.available": "Available",
    "status.not_connected": "Not Connected",
    "status.coming_soon": "Coming Soon",
    "section.charge_limits": "Charge Limits",
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
    "btn.refresh_dashboard": "Refresh",
    "btn.open_theme_menu": "Theme options",
    "btn.open_more_menu": "More",
    "btn.close_modal": "Close",
    "btn.view_service_logs": "Service Logs",
    "btn.check_updates": "Check for Updates",
    "btn.coming_soon": "Coming Soon",
    "setting.adapter_sleep": "Disable sleep when adapter disabled",
    "setting.adapter_sleep_desc":
      "Prevents the system from sleeping when the power adapter is disabled",
    "setting.magsafe_sync": "Sync MagSafe LED",
    "setting.magsafe_sync_desc":
      "Synchronize the MagSafe LED indicator with charging state",
    "setting.language": "Language",
    "setting.language_desc":
      "Choose the dashboard language or follow the system preference",
    "setting.theme": "Theme",
    "setting.theme_desc": "Pick a fixed theme or follow the system appearance",
    "setting.option.system": "Follow System",
    "setting.option.chinese": "Chinese",
    "setting.option.english": "English",
    "theme.light": "Light",
    "theme.dark": "Dark",
    "theme.system": "Follow System",
    "menu.author": "Author",
    "menu.github": "GitHub",
    "menu.about": "More",
    "menu.logs_title": "Service Logs",
    "menu.placeholder_repo": "http:github.com/xxx",
    "menu.author_email": "QQ Mail: 2413067063@qq.com",
    "menu.check_updates_desc": "Placeholder only. No update flow yet.",
    "unsupported.message": "This device is not supported by Battery Toolkit.",
    "section.realtime": "Realtime",
    "rt.temperature": "Temperature",
    "rt.power": "Power",
    "rt.voltage": "Voltage",
    "rt.current": "Current",
    "section.charger": "Charger",
    "charger.voltage": "Output Voltage",
    "charger.current": "Output Current",
    "device.model": "Model",
    "device.chip": "Chip",
    "device.memory": "Memory",
    "device.activated": "Activated",
    "section.battery_health": "Battery Health",
    "health.cycle_count": "Cycle Count",
    "health.battery_life": "Battery Life",
    "health.design_capacity": "Design",
    "health.actual_capacity": "Actual",
    "health.current_capacity": "Current",
    "notice.no_root":
      "Battery control is unavailable. Install the root helper service, then start the service to enable charging control.",
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
    "msg.language_updated": "Language updated",
    "msg.theme_updated": "Theme updated",
    "msg.refresh_complete": "Dashboard refreshed",
    "validation.min_exceeds_max":
      "Min charge (%{min}%) exceeds max charge (%{max}%)",
    "validation.min_at_least": "Min charge must be at least 20%",
    "validation.max_at_least": "Max charge must be at least 50%",
    "validation.adapter_required":
      "Connect a power adapter to use charge strategy actions.",
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
    "section.settings": "设置",
    "section.diagnostics": "诊断",
    "section.service_log": "服务日志",
    "section.appearance": "外观",
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
    "label.min_charge": "最低电量",
    "label.max_charge": "最高电量",
    "status.disconnected": "未连接",
    "status.charging": "充电中",
    "status.connected": "已连接",
    "status.disabled": "已禁用",
    "status.adapter_off": "适配器已关闭",
    "status.on_battery": "使用电池",
    "status.available": "可用",
    "status.not_connected": "未接入",
    "status.coming_soon": "即将支持",
    "section.charge_limits": "充电限制",
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
    "btn.refresh_dashboard": "刷新",
    "btn.open_theme_menu": "主题选项",
    "btn.open_more_menu": "更多",
    "btn.close_modal": "关闭",
    "btn.view_service_logs": "服务日志",
    "btn.check_updates": "检查更新",
    "btn.coming_soon": "即将支持",
    "setting.adapter_sleep": "禁用适配器时阻止系统睡眠",
    "setting.adapter_sleep_desc": "当电源适配器被禁用时，阻止系统进入睡眠状态",
    "setting.magsafe_sync": "同步 MagSafe 指示灯",
    "setting.magsafe_sync_desc": "将 MagSafe 指示灯与充电状态同步",
    "setting.language": "语言",
    "setting.language_desc": "选择界面语言，或默认跟随系统偏好",
    "setting.theme": "主题",
    "setting.theme_desc": "可固定浅色、深色，或自动跟随系统外观",
    "setting.option.system": "跟随系统",
    "setting.option.chinese": "中文",
    "setting.option.english": "English",
    "theme.light": "浅色",
    "theme.dark": "深色",
    "theme.system": "跟随系统",
    "menu.author": "作者",
    "menu.github": "GitHub",
    "menu.about": "更多",
    "menu.logs_title": "服务日志",
    "menu.placeholder_repo": "http:github.com/xxx",
    "menu.author_email": "QQ 邮箱：2413067063@qq.com",
    "menu.check_updates_desc": "当前仅做占位，暂未接入更新功能。",
    "unsupported.message": "此设备不支持电池工具箱。",
    "section.realtime": "实时状态",
    "rt.temperature": "温度",
    "rt.power": "功率",
    "rt.voltage": "电压",
    "rt.current": "电流",
    "section.charger": "充电器",
    "charger.voltage": "输出电压",
    "charger.current": "输出电流",
    "device.model": "机型",
    "device.chip": "芯片",
    "device.memory": "内存",
    "device.activated": "激活日期",
    "section.battery_health": "电池健康",
    "health.cycle_count": "循环次数",
    "health.battery_life": "电池寿命",
    "health.design_capacity": "设计容量",
    "health.actual_capacity": "实际容量",
    "health.current_capacity": "当前容量",
    "notice.no_root":
      "充电控制当前不可用。先安装特权 helper 服务，再启动服务后才能控制充电。",
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
    "msg.language_updated": "语言已更新",
    "msg.theme_updated": "主题已更新",
    "msg.refresh_complete": "面板已刷新",
    "validation.min_exceeds_max": "最低电量（%{min}%）不能超过最高电量（%{max}%）",
    "validation.min_at_least": "最低电量不能低于 20%",
    "validation.max_at_least": "最高电量不能低于 50%",
    "validation.adapter_required": "请先接入电源适配器后再使用充电策略操作。",
  },
};

const LOCALE_STORAGE_KEY = "battery-toolkit-locale";
const LOCALE_MODE_OPTIONS = new Set(["system", "zh", "en"]);
const LOCALE_CHANGE_EVENT = "battery-toolkit:locale-changed";

function normalizeLocale(raw) {
  if (!raw) return "en";
  const lower = raw.toLowerCase();
  if (lower.startsWith("zh")) return "zh";
  return "en";
}

function detectSystemLocale() {
  const nav =
    navigator.language || navigator.userLanguage || navigator.languages?.[0];
  return normalizeLocale(nav || "en");
}

function readStoredLocaleMode() {
  const saved = localStorage.getItem(LOCALE_STORAGE_KEY);
  return LOCALE_MODE_OPTIONS.has(saved) ? saved : "system";
}

let localeMode = readStoredLocaleMode();
let activeLocale = resolveLocale(localeMode);

function resolveLocale(mode = localeMode) {
  if (mode === "zh" || mode === "en") {
    return mode;
  }
  return detectSystemLocale();
}

function getLocaleMode() {
  return localeMode;
}

function getLocale() {
  return activeLocale;
}

function getDictionary() {
  return TRANSLATIONS[activeLocale] || TRANSLATIONS.en;
}

function t(key, params) {
  let str = getDictionary()[key] ?? TRANSLATIONS.en[key] ?? key;
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      str = str.replace("%{" + k + "}", v);
    }
  }
  return str;
}

function applyLocaleToDom(root = document) {
  root.querySelectorAll("[data-i18n]").forEach((el) => {
    const key = el.getAttribute("data-i18n");
    if (key) {
      el.textContent = t(key);
    }
  });

  root.querySelectorAll("[data-i18n-title]").forEach((el) => {
    const key = el.getAttribute("data-i18n-title");
    if (key) {
      const value = t(key);
      el.setAttribute("title", value);
      el.setAttribute("aria-label", value);
    }
  });
}

function emitLocaleChange() {
  window.dispatchEvent(
    new CustomEvent(LOCALE_CHANGE_EVENT, {
      detail: {
        locale: activeLocale,
        mode: localeMode,
      },
    })
  );
}

function setLocaleMode(nextMode) {
  const normalized = LOCALE_MODE_OPTIONS.has(nextMode) ? nextMode : "system";
  localeMode = normalized;
  localStorage.setItem(LOCALE_STORAGE_KEY, normalized);

  const nextLocale = resolveLocale(normalized);
  const changed = nextLocale !== activeLocale;
  activeLocale = nextLocale;
  applyLocaleToDom();
  emitLocaleChange();
  return changed;
}

function refreshSystemLocale() {
  if (localeMode !== "system") {
    return false;
  }
  const nextLocale = resolveLocale("system");
  if (nextLocale === activeLocale) {
    return false;
  }
  activeLocale = nextLocale;
  applyLocaleToDom();
  emitLocaleChange();
  return true;
}

window.addEventListener("languagechange", refreshSystemLocale);

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", () => applyLocaleToDom());
} else {
  applyLocaleToDom();
}

window.__i18n = {
  t,
  applyLocaleToDom,
  detectSystemLocale,
  getLocale,
  getLocaleMode,
  LOCALE_CHANGE_EVENT,
  resolveLocale,
  setLocaleMode,
  refreshSystemLocale,
};
