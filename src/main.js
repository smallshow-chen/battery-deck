// Battery Toolkit - Frontend Logic
// Uses Tauri v2 API via window.__TAURI__ (withGlobalTauri enabled)

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// i18n helpers (exposed by i18n.js)
const { t: __, LOCALE } = window.__i18n || { t: (k) => k, LOCALE: "en" };

// ---- State ----
let state = {
  enabled: false,
  powerDisabled: false,
  connected: false,
  chargingDisabled: false,
  chargePercent: 0,
  mode: "Standard",
  minCharge: 75,
  maxCharge: 80,
  adapterSleep: false,
  magsafeSync: false,
  supported: true,
  controlAvailable: false,
  lastError: null,
};

let serviceState = {
  installed: false,
  running: false,
  controlAvailable: false,
  lastError: null,
};

let settingsDirty = false;
let pollTimer = null;
let toastTimer = null;
let settingsDebounceTimer = null;
let scrollResumeTimer = null;
let isUserScrolling = false;
let uiFramePending = false;
let needsUiRefresh = false;
let lastServicePollAt = 0;
let lastBatteryPollAt = 0;
let lastRealtimePollAt = 0;
let scrollContainers = [];
let windowVisible = true;

// ---- DOM References ----
const dom = {};

function cacheDom() {
  dom.loadingOverlay = document.getElementById("loading-overlay");
  dom.appShell = document.getElementById("app");
  dom.dashboardScroll = document.getElementById("dashboard-scroll");
  dom.errorToast = document.getElementById("error-toast");
  dom.errorMessage = document.getElementById("error-message");
  dom.errorClose = document.getElementById("error-close");
  dom.successToast = document.getElementById("success-toast");
  dom.successMessage = document.getElementById("success-message");

  dom.badgeEnabled = document.getElementById("badge-enabled");
  dom.badgeMode = document.getElementById("badge-mode");
  dom.serviceIndicatorDot = document.getElementById("service-indicator-dot");
  dom.serviceStatusLabel = document.getElementById("service-status-label");
  dom.serviceInstalledValue = document.getElementById("service-installed-value");
  dom.serviceRunningValue = document.getElementById("service-running-value");
  dom.serviceControlValue = document.getElementById("service-control-value");
  dom.serviceErrorText = document.getElementById("service-error-text");
  dom.serviceLog = document.getElementById("service-log");
  dom.btnInstallService = document.getElementById("btn-install-service");
  dom.btnStartService = document.getElementById("btn-start-service");
  dom.btnStopService = document.getElementById("btn-stop-service");
  dom.btnRefreshLogs = document.getElementById("btn-refresh-logs");

  dom.indicatorDot = document.getElementById("indicator-dot");
  dom.connectionText = document.getElementById("connection-text");
  dom.connectionStatusDisplay = document.getElementById("connection-status-display");

  dom.batteryFill = document.getElementById("battery-fill");
  dom.batteryPercent = document.getElementById("battery-percent");
  dom.chargePercentDisplay = document.getElementById("charge-percent-display");
  dom.chargingStateDisplay = document.getElementById("charging-state-display");
  dom.adapterStateDisplay = document.getElementById("adapter-state-display");

  dom.minChargeSlider = document.getElementById("min-charge-slider");
  dom.maxChargeSlider = document.getElementById("max-charge-slider");
  dom.minChargeValue = document.getElementById("min-charge-value");
  dom.maxChargeValue = document.getElementById("max-charge-value");
  dom.minChargeFill = document.getElementById("min-charge-fill");
  dom.maxChargeFill = document.getElementById("max-charge-fill");
  dom.sliderValidation = document.getElementById("slider-validation");

  dom.btnChargeFull = document.getElementById("btn-charge-full");
  dom.btnChargeLimit = document.getElementById("btn-charge-limit");
  dom.btnResetChargeMode = document.getElementById("btn-reset-charge-mode");
  dom.btnDisableCharging = document.getElementById("btn-disable-charging");
  dom.btnToggleAdapter = document.getElementById("btn-toggle-adapter");
  dom.adapterBtnText = document.getElementById("adapter-btn-text");

  dom.toggleAdapterSleep = document.getElementById("toggle-adapter-sleep");
  dom.toggleMagSafeSync = document.getElementById("toggle-magsafe-sync");

  dom.unsupportedBanner = document.getElementById("unsupported-banner");
  dom.rootNotice = document.getElementById("root-notice");

  dom.healthCycles = document.getElementById("health-cycles");
  dom.healthPercent = document.getElementById("health-percent");
  dom.capDesign = document.getElementById("cap-design");
  dom.capMax = document.getElementById("cap-max");
  dom.capCurrent = document.getElementById("cap-current");

  dom.rtTemp = document.getElementById("rt-temp");
  dom.rtPower = document.getElementById("rt-power");
  dom.rtVoltage = document.getElementById("rt-voltage");
  dom.rtCurrent = document.getElementById("rt-current");

  dom.chargerName = document.getElementById("charger-name");
  dom.chargerWattage = document.getElementById("charger-wattage");
  dom.chargerVoltage = document.getElementById("charger-voltage");
  dom.chargerCurrent = document.getElementById("charger-current");

  dom.deviceModel = document.getElementById("device-model");
  dom.deviceChip = document.getElementById("device-chip");
  dom.deviceMemory = document.getElementById("device-memory");
  dom.deviceActivated = document.getElementById("device-activated");

  scrollContainers = [dom.dashboardScroll].filter(Boolean);
}

// ---- Initialization ----
window.addEventListener("DOMContentLoaded", async () => {
  cacheDom();
  bindEvents();
  updateSettingsUI();
  await initialize();
});

async function initialize() {
  try {
    await refreshVisibleState();
    startPolling();
    setupEventListener();
  } catch (err) {
    console.error("Initialization error:", err);
    showError("Failed to initialize: " + formatError(err));
  } finally {
    hideLoading();
  }
}

// ---- Tauri Invocations ----
async function refreshBatteryHealth() {
  try {
    const h = await invoke("get_battery_health");
    if (!h) return;
    dom.healthCycles.textContent = h.cycleCount;
    dom.healthPercent.textContent = h.healthPercent + "%";
    dom.capDesign.textContent = h.designCapacity + " mAh";
    dom.capMax.textContent = h.maxCapacity + " mAh";
    dom.capCurrent.textContent = h.currentCapacity + " mAh";
  } catch (err) {
    console.warn("Could not load battery health:", err);
  }
}

async function refreshRealtime() {
  try {
    const rt = await invoke("get_battery_realtime");
    if (!rt) return;
    dom.rtTemp.textContent = rt.temperature.toFixed(1);
    dom.rtPower.textContent = rt.power.toFixed(2);
    dom.rtVoltage.textContent = (rt.voltage / 1000).toFixed(3) + " V";
    dom.rtCurrent.textContent = rt.amperage + " mA";
  } catch (err) {
    console.warn("Could not load realtime data:", err);
  }
}

async function refreshChargerInfo() {
  try {
    const c = await invoke("get_charger_info");
    if (!c) return;
    dom.chargerName.textContent = c.name;
    dom.chargerWattage.textContent = c.connected ? c.wattage + "W" : "—";
    dom.chargerVoltage.textContent = c.connected ? c.chargingVoltage + " mV" : "—";
    dom.chargerCurrent.textContent = c.connected ? c.chargingCurrent + " mA" : "—";
  } catch (err) {
    console.warn("Could not load charger info:", err);
  }
}

async function refreshSystemInfo() {
  try {
    const info = await invoke("get_system_info");
    if (!info) return;
    dom.deviceModel.textContent = info.modelName;
    dom.deviceChip.textContent = info.chip;
    dom.deviceMemory.textContent = info.memoryGb + " GB";
    dom.deviceActivated.textContent = info.activationDate || "--";
  } catch (err) {
    console.warn("Could not load system info:", err);
  }
}

async function refreshBatteryState() {
  try {
    const batteryState = await invoke("get_battery_state");
    Object.assign(state, batteryState);
    scheduleUiUpdate();
  } catch (err) {
    console.error("Failed to get battery state:", err);
    state.controlAvailable = false;
    state.lastError = formatError(err);
    scheduleUiUpdate();
    if (serviceState.running) {
      showError("Could not read battery state: " + formatError(err));
    }
  }
}

async function refreshServiceStatus() {
  try {
    const status = await invoke("get_service_status");
    serviceState.installed = status.installed;
    serviceState.running = status.running;
    serviceState.controlAvailable = status.controlAvailable;
    serviceState.lastError = status.lastError;
    if (!state.controlAvailable) {
      state.controlAvailable = status.controlAvailable;
    }
    scheduleUiUpdate();
  } catch (err) {
    console.error("Failed to get service status:", err);
    serviceState.installed = false;
    serviceState.running = false;
    serviceState.controlAvailable = false;
    serviceState.lastError = formatError(err);
    scheduleUiUpdate();
  }
}

async function refreshServiceLogs() {
  try {
    const logs = await invoke("get_service_logs", { lines: 40 });
    dom.serviceLog.textContent = logs && logs.trim() ? logs : "--";
  } catch (err) {
    dom.serviceLog.textContent = formatError(err);
  }
}

async function refreshVisibleState() {
  await Promise.all([
    refreshServiceStatus(),
    refreshBatteryState(),
    refreshBatteryHealth(),
    refreshRealtime(),
    refreshChargerInfo(),
    refreshSystemInfo(),
    loadSettings(),
    refreshServiceLogs(),
  ]);
}

async function loadSettings() {
  try {
    const settings = await invoke("get_settings");
    state.minCharge = settings.minCharge;
    state.maxCharge = settings.maxCharge;
    state.adapterSleep = settings.adapterSleep;
    state.magsafeSync = settings.magsafeSync;
    updateSettingsUI();
  } catch (err) {
    console.error("Failed to get settings:", err);
    updateSettingsUI();
    if (state.controlAvailable) {
      showError("Could not load settings: " + formatError(err));
    }
  }
}

async function applySettings() {
  try {
    const minCharge = parseInt(dom.minChargeSlider.value, 10);
    const maxCharge = parseInt(dom.maxChargeSlider.value, 10);

    // Validate
    if (minCharge > maxCharge) {
      showValidationError(__("validation.min_exceeds_max", { min: minCharge, max: maxCharge }));
      return;
    }
    if (minCharge < 20) {
      showValidationError(__("validation.min_at_least"));
      return;
    }
    if (maxCharge < 50) {
      showValidationError(__("validation.max_at_least"));
      return;
    }

    await invoke("set_settings", {
      minCharge: minCharge,
      maxCharge: maxCharge,
      adapterSleep: dom.toggleAdapterSleep.checked,
      magsafeSync: dom.toggleMagSafeSync.checked,
    });

    state.minCharge = minCharge;
    state.maxCharge = maxCharge;
    state.adapterSleep = dom.toggleAdapterSleep.checked;
    state.magsafeSync = dom.toggleMagSafeSync.checked;

    clearValidation();
    showSuccess(__("msg.settings_saved"));
    settingsDirty = false;
  } catch (err) {
    console.error("Failed to set settings:", err);
    showError("Could not save settings: " + formatError(err));
  }
}

// ---- Event Binding ----
function bindEvents() {
  // Sliders
  dom.minChargeSlider.addEventListener("input", onMinChargeInput);
  dom.maxChargeSlider.addEventListener("input", onMaxChargeInput);
  dom.minChargeSlider.addEventListener("change", onSettingsChange);
  dom.maxChargeSlider.addEventListener("change", onSettingsChange);

  // Action buttons
  dom.btnChargeFull.addEventListener("click", onChargeToFull);
  dom.btnChargeLimit.addEventListener("click", onChargeToLimit);
  dom.btnResetChargeMode.addEventListener("click", onResetChargeMode);
  dom.btnDisableCharging.addEventListener("click", onDisableCharging);
  dom.btnToggleAdapter.addEventListener("click", onToggleAdapter);
  dom.btnInstallService.addEventListener("click", onInstallService);
  dom.btnStartService.addEventListener("click", onStartService);
  dom.btnStopService.addEventListener("click", onStopService);
  dom.btnRefreshLogs.addEventListener("click", onRefreshLogs);

  // Setting toggles
  dom.toggleAdapterSleep.addEventListener("change", onSettingsChange);
  dom.toggleMagSafeSync.addEventListener("change", onSettingsChange);

  // Toast close
  dom.errorClose.addEventListener("click", () => {
    dom.errorToast.classList.remove("visible");
  });

  for (const container of scrollContainers) {
    container.addEventListener("scroll", onUserScroll, { passive: true });
  }
}

// ---- Event Handlers ----
function onMinChargeInput() {
  const val = parseInt(dom.minChargeSlider.value, 10);
  dom.minChargeValue.textContent = val + "%";
  updateSliderFill(dom.minChargeSlider, dom.minChargeFill);
  validateSliders();
}

function onMaxChargeInput() {
  const val = parseInt(dom.maxChargeSlider.value, 10);
  dom.maxChargeValue.textContent = val + "%";
  updateSliderFill(dom.maxChargeSlider, dom.maxChargeFill);
  validateSliders();
}

function onSettingsChange() {
  settingsDirty = true;
  // Debounce settings save by 500ms
  if (settingsDebounceTimer) {
    clearTimeout(settingsDebounceTimer);
  }
  settingsDebounceTimer = setTimeout(() => {
    applySettings();
  }, 500);
}

async function onChargeToFull() {
  try {
    setButtonsDisabled(true);
    await invoke("charge_to_full");
    showSuccess(__("msg.charging_full"));
    await refreshBatteryState();
  } catch (err) {
    showError("Failed: " + formatError(err));
  } finally {
    setButtonsDisabled(false);
  }
}

async function onChargeToLimit() {
  try {
    setButtonsDisabled(true);
    await invoke("charge_to_limit");
    showSuccess(__("msg.charging_limit"));
    await refreshBatteryState();
  } catch (err) {
    showError("Failed: " + formatError(err));
  } finally {
    setButtonsDisabled(false);
  }
}

async function onResetChargeMode() {
  try {
    setButtonsDisabled(true);
    await invoke("reset_charge_mode");
    showSuccess(__("msg.mode_reset"));
    await refreshBatteryState();
  } catch (err) {
    showError("Failed: " + formatError(err));
  } finally {
    setButtonsDisabled(false);
  }
}

async function onDisableCharging() {
  try {
    setButtonsDisabled(true);
    await invoke("disable_charging_cmd");
    showSuccess(__("msg.charging_disabled"));
    await refreshBatteryState();
  } catch (err) {
    showError("Failed: " + formatError(err));
  } finally {
    setButtonsDisabled(false);
  }
}

async function onToggleAdapter() {
  try {
    setButtonsDisabled(true);
    if (state.powerDisabled) {
      await invoke("enable_adapter_cmd");
      showSuccess(__("msg.adapter_enabled"));
    } else {
      await invoke("disable_adapter_cmd");
      showSuccess(__("msg.adapter_disabled"));
    }
    await refreshBatteryState();
  } catch (err) {
    showError("Failed: " + formatError(err));
  } finally {
    setButtonsDisabled(false);
  }
}

async function onInstallService() {
  try {
    setServiceButtonsDisabled(true);
    await invoke("install_service");
    await refreshServiceStatus();
    await refreshBatteryState();
    await refreshServiceLogs();
    showSuccess(__("msg.service_installed"));
  } catch (err) {
    showError("Failed: " + formatError(err));
  } finally {
    setServiceButtonsDisabled(false);
  }
}

async function onStartService() {
  try {
    setServiceButtonsDisabled(true);
    await invoke("start_service");
    await refreshServiceStatus();
    await refreshBatteryState();
    await refreshServiceLogs();
    showSuccess(__("msg.service_started"));
  } catch (err) {
    showError("Failed: " + formatError(err));
  } finally {
    setServiceButtonsDisabled(false);
  }
}

async function onStopService() {
  try {
    setServiceButtonsDisabled(true);
    await invoke("stop_service");
    await refreshServiceStatus();
    await refreshServiceLogs();
    showSuccess(__("msg.service_stopped"));
  } catch (err) {
    showError("Failed: " + formatError(err));
  } finally {
    setServiceButtonsDisabled(false);
  }
}

async function onRefreshLogs() {
  try {
    setServiceButtonsDisabled(true);
    await Promise.all([refreshServiceStatus(), refreshServiceLogs()]);
  } finally {
    setServiceButtonsDisabled(false);
  }
}

function onUserScroll() {
  isUserScrolling = true;
  if (scrollResumeTimer) {
    clearTimeout(scrollResumeTimer);
  }
  scrollResumeTimer = setTimeout(() => {
    isUserScrolling = false;
    scheduleUiUpdate();
  }, 180);
}

function scheduleUiUpdate() {
  needsUiRefresh = true;
  if (isUserScrolling || uiFramePending) {
    return;
  }
  uiFramePending = true;
  requestAnimationFrame(() => {
    uiFramePending = false;
    if (!needsUiRefresh) {
      return;
    }
    needsUiRefresh = false;
    updateUI();
  });
}

// ---- Tauri Event Listener ----
function setupEventListener() {
  listen("battery-state-changed", (event) => {
    if (event.payload) {
      Object.assign(state, event.payload);
      scheduleUiUpdate();
    }
  }).catch((err) => {
    console.warn("Could not listen for battery-state-changed events:", err);
  });

  listen("app-window-visibility-changed", async (event) => {
    const visible =
      typeof event.payload === "boolean"
        ? event.payload
        : !!event.payload?.visible;

    windowVisible = visible;
    if (!visible) {
      stopPolling();
      return;
    }

    await refreshVisibleState();
    startPolling();
  }).catch((err) => {
    console.warn("Could not listen for app-window-visibility-changed events:", err);
  });

  listen("app-state-refresh-requested", async () => {
    if (!windowVisible) {
      return;
    }
    await refreshVisibleState();
  }).catch((err) => {
    console.warn("Could not listen for app-state-refresh-requested events:", err);
  });

  listen("tray-action-error", async (event) => {
    const message = event.payload?.message || formatError(event.payload);
    showError(message);
    if (windowVisible) {
      await refreshVisibleState();
    }
  }).catch((err) => {
    console.warn("Could not listen for tray-action-error events:", err);
  });
}

// ---- Polling ----
function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

function startPolling() {
  let lastExternal = null;
  stopPolling();
  pollTimer = setInterval(async () => {
    if (!windowVisible || settingsDirty || isUserScrolling) {
      return;
    }

    const now = Date.now();
    const tasks = [];

    if (now - lastServicePollAt >= 15000) {
      lastServicePollAt = now;
      tasks.push(refreshServiceStatus());
    }
    if (now - lastBatteryPollAt >= 8000) {
      lastBatteryPollAt = now;
      tasks.push(refreshBatteryState());
    }
    if (now - lastRealtimePollAt >= 5000) {
      lastRealtimePollAt = now;
      tasks.push(refreshRealtime());
    }

    if (tasks.length > 0) {
      await Promise.all(tasks);
    }

    const currentExternal = state.isPlugged;
    if (currentExternal !== lastExternal) {
      lastExternal = currentExternal;
      await refreshChargerInfo();
    }
  }, 5000);
}

// ---- UI Update ----
function updateUI() {
  updateBatteryDisplay();
  updateStatusBadges();
  updateConnectionIndicator();
  updateAdapterButton();
  updateActionButtons();
  updateUnsupportedBanner();
  updateControlAvailability();
  updateServiceCard();
}

function updateBatteryDisplay() {
  const pct = state.chargePercent ?? 0;

  // Battery fill width
  dom.batteryFill.style.width = pct + "%";
  dom.batteryPercent.textContent = pct + "%";

  // Battery color
  dom.batteryFill.classList.remove(
    "charge-low",
    "charge-mid",
    "charge-good",
    "charge-full"
  );
  if (pct <= 20) {
    dom.batteryFill.classList.add("charge-low");
  } else if (pct <= 50) {
    dom.batteryFill.classList.add("charge-mid");
  } else if (pct < 95) {
    dom.batteryFill.classList.add("charge-good");
  } else {
    dom.batteryFill.classList.add("charge-full");
  }

  // Details
  dom.chargePercentDisplay.textContent = pct + "%";

  // Charging state
  if (state.chargingDisabled) {
    dom.chargingStateDisplay.textContent = __("status.disabled");
    dom.chargingStateDisplay.style.color = "var(--accent-red)";
  } else if (state.powerDisabled) {
    dom.chargingStateDisplay.textContent = __("status.adapter_off");
    dom.chargingStateDisplay.style.color = "var(--accent-orange)";
  } else if (state.isCharging) {
    dom.chargingStateDisplay.textContent = __("status.charging");
    dom.chargingStateDisplay.style.color = "var(--accent-green)";
  } else if (state.isPlugged) {
    dom.chargingStateDisplay.textContent = __("status.connected");
    dom.chargingStateDisplay.style.color = "var(--accent-green)";
  } else {
    dom.chargingStateDisplay.textContent = __("status.on_battery");
    dom.chargingStateDisplay.style.color = "var(--text-secondary)";
  }

  // Adapter state
  dom.adapterStateDisplay.textContent = state.powerDisabled
    ? __("status.disabled")
    : state.isPlugged
      ? __("status.connected")
      : __("status.available");
  dom.adapterStateDisplay.style.color = state.powerDisabled
    ? "var(--accent-red)"
    : state.isPlugged
      ? "var(--accent-green)"
      : "var(--text-secondary)";
}

function updateStatusBadges() {
  // Enabled/Disabled badge
  if (state.enabled) {
    dom.badgeEnabled.textContent = __("badge.enabled");
    dom.badgeEnabled.className = "badge badge-enabled";
  } else {
    dom.badgeEnabled.textContent = __("badge.disabled");
    dom.badgeEnabled.className = "badge badge-disabled";
  }

  // Mode badge
  const mode = state.mode || "Standard";
  const modeKey = "badge." + mode.toLowerCase();
  dom.badgeMode.textContent = __(modeKey);
  if (mode === "ToFull") {
    dom.badgeMode.className = "badge badge-mode-warn";
  } else if (mode === "ToLimit") {
    dom.badgeMode.className = "badge badge-mode";
  } else {
    dom.badgeMode.className = "badge badge-mode";
  }
}

function updateConnectionIndicator() {
  dom.indicatorDot.classList.remove("connected", "charging");
  let connectionLabel = __("status.disconnected");

  if (state.isPlugged && !state.powerDisabled) {
    if (state.isCharging && !state.chargingDisabled) {
      dom.indicatorDot.classList.add("charging");
      connectionLabel = __("status.charging");
    } else {
      dom.indicatorDot.classList.add("connected");
      connectionLabel = __("status.connected");
    }
  } else if (state.isPlugged) {
    dom.indicatorDot.classList.add("connected");
    connectionLabel = __("status.connected");
  }

  dom.connectionText.textContent = connectionLabel;
  if (dom.connectionStatusDisplay) {
    dom.connectionStatusDisplay.textContent = connectionLabel;
  }
}

function updateAdapterButton() {
  if (state.powerDisabled) {
    dom.adapterBtnText.textContent = __("btn.enable_adapter");
    dom.btnToggleAdapter.classList.add("action-active");
  } else {
    dom.adapterBtnText.textContent = __("btn.disable_adapter");
    dom.btnToggleAdapter.classList.remove("action-active");
  }
}

function updateActionButtons() {
  const mode = state.mode || "Standard";
  dom.btnChargeFull.classList.toggle("is-selected", mode === "ToFull");
  dom.btnChargeLimit.classList.toggle("is-selected", mode === "ToLimit");
  dom.btnResetChargeMode.classList.toggle("is-selected", mode === "Standard");
  dom.btnDisableCharging.classList.toggle("is-selected", !!state.chargingDisabled);
  dom.btnToggleAdapter.classList.toggle("is-selected", !!state.powerDisabled);
}

function isExpectedServiceMissingError(error) {
  if (!error) {
    return false;
  }

  const message = String(error).toLowerCase();
  return (
    message.includes("no such file or directory") ||
    message.includes("connection refused") ||
    message.includes("failed to connect") ||
    message.includes("os error 2")
  );
}

function updateUnsupportedBanner() {
  const shouldShow = serviceState.running && !state.supported;
  dom.unsupportedBanner.style.display = shouldShow ? "flex" : "none";
}

function updateControlAvailability() {
  const available = state.controlAvailable && serviceState.controlAvailable;
  dom.rootNotice.style.display = available ? "none" : "flex";
  setButtonsDisabled(!available);
  dom.minChargeSlider.disabled = !available;
  dom.maxChargeSlider.disabled = !available;
  dom.toggleAdapterSleep.disabled = !available;
  dom.toggleMagSafeSync.disabled = !available;
}

function updateServiceCard() {
  const rawError = serviceState.lastError || state.lastError || "";
  const showError = serviceState.running || !isExpectedServiceMissingError(rawError);

  dom.serviceInstalledValue.textContent = serviceState.installed ? __("service.yes") : __("service.no");
  dom.serviceRunningValue.textContent = serviceState.running ? __("service.yes") : __("service.no");
  dom.serviceControlValue.textContent = serviceState.controlAvailable
    ? __("service.available")
    : __("service.unavailable");
  dom.serviceErrorText.textContent = showError ? rawError : "";
  dom.serviceErrorText.className = showError && rawError
    ? "slider-validation error"
    : "slider-validation";

  dom.serviceIndicatorDot.classList.remove("connected", "charging");
  if (serviceState.controlAvailable) {
    dom.serviceIndicatorDot.classList.add("charging");
    dom.serviceStatusLabel.textContent = __("service.ready");
  } else if (serviceState.running) {
    dom.serviceIndicatorDot.classList.add("connected");
    dom.serviceStatusLabel.textContent = __("service.running_status");
  } else {
    dom.serviceStatusLabel.textContent = __("service.stopped");
  }

  dom.btnInstallService.disabled = false;
  dom.btnStartService.disabled = serviceState.running;
  dom.btnStopService.disabled = !serviceState.running;
}

function updateSettingsUI() {
  // Sliders
  dom.minChargeSlider.value = state.minCharge;
  dom.maxChargeSlider.value = state.maxCharge;
  dom.minChargeValue.textContent = state.minCharge + "%";
  dom.maxChargeValue.textContent = state.maxCharge + "%";

  updateSliderFill(dom.minChargeSlider, dom.minChargeFill);
  updateSliderFill(dom.maxChargeSlider, dom.maxChargeFill);

  // Toggles
  dom.toggleAdapterSleep.checked = state.adapterSleep;
  dom.toggleMagSafeSync.checked = state.magsafeSync;

  validateSliders();
}

// ---- Slider Helpers ----
function updateSliderFill(slider, fillEl) {
  const min = parseInt(slider.min, 10);
  const max = parseInt(slider.max, 10);
  const val = parseInt(slider.value, 10);
  const pct = ((val - min) / (max - min)) * 100;
  fillEl.style.width = pct + "%";
}

function validateSliders() {
  const minVal = parseInt(dom.minChargeSlider.value, 10);
  const maxVal = parseInt(dom.maxChargeSlider.value, 10);

  if (minVal > maxVal) {
    showValidationError(__("validation.min_exceeds_max", { min: minVal, max: maxVal }));
  } else if (minVal < 20) {
    showValidationError(__("validation.min_at_least"));
  } else if (maxVal < 50) {
    showValidationError(__("validation.max_at_least"));
  } else {
    clearValidation();
  }
}

// ---- Button Helpers ----
function setButtonsDisabled(disabled) {
  dom.btnChargeFull.disabled = disabled;
  dom.btnChargeLimit.disabled = disabled;
  dom.btnResetChargeMode.disabled = disabled;
  dom.btnDisableCharging.disabled = disabled;
  dom.btnToggleAdapter.disabled = disabled;
}

function setServiceButtonsDisabled(disabled) {
  dom.btnInstallService.disabled = disabled;
  dom.btnStartService.disabled = disabled;
  dom.btnStopService.disabled = disabled;
  dom.btnRefreshLogs.disabled = disabled;
}

function disableControls() {
  setButtonsDisabled(true);
  dom.minChargeSlider.disabled = true;
  dom.maxChargeSlider.disabled = true;
  dom.toggleAdapterSleep.disabled = true;
  dom.toggleMagSafeSync.disabled = true;
}

// ---- Toast / Message Helpers ----
function showError(message) {
  if (toastTimer) clearTimeout(toastTimer);
  dom.successToast.classList.remove("visible");

  dom.errorMessage.textContent = message;
  dom.errorToast.classList.add("visible");

  toastTimer = setTimeout(() => {
    dom.errorToast.classList.remove("visible");
  }, 5000);
}

function showSuccess(message) {
  if (toastTimer) clearTimeout(toastTimer);
  dom.errorToast.classList.remove("visible");

  dom.successMessage.textContent = message;
  dom.successToast.classList.add("visible");

  toastTimer = setTimeout(() => {
    dom.successToast.classList.remove("visible");
  }, 2500);
}

function showValidationError(message) {
  dom.sliderValidation.textContent = message;
  dom.sliderValidation.className = "slider-validation error";
}

function clearValidation() {
  dom.sliderValidation.textContent = "";
  dom.sliderValidation.className = "slider-validation";
}

function hideLoading() {
  dom.loadingOverlay.classList.add("hidden");
  // Remove from DOM after transition
  setTimeout(() => {
    if (dom.loadingOverlay.parentNode) {
      dom.loadingOverlay.style.display = "none";
    }
  }, 300);
}

function formatError(err) {
  if (typeof err === "string") return err;
  if (err && err.message) return err.message;
  if (err && err.toString) return err.toString();
  return "Unknown error";
}
