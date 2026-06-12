// Battery Toolkit - Frontend Logic
// Uses Tauri v2 API via window.__TAURI__ (withGlobalTauri enabled)

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const i18nApi = window.__i18n || {};
const __ = i18nApi.t || ((key) => key);
const {
  applyLocaleToDom = () => {},
  getLocaleMode = () => "system",
  setLocaleMode = () => false,
  LOCALE_CHANGE_EVENT = "battery-toolkit:locale-changed",
} = i18nApi;

const THEME_STORAGE_KEY = "battery-toolkit-theme";
const THEME_OPTIONS = new Set(["light", "dark", "system"]);

let state = {
  enabled: false,
  powerDisabled: false,
  connected: false,
  isPlugged: false,
  adapterConnected: false,
  realtimeAdapterConnected: false,
  chargerTelemetryConnected: false,
  isCharging: false,
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
let lastSnapshotPollAt = 0;
let lastRealtimePollAt = 0;
let scrollContainers = [];
let windowVisible = true;
let themeMode = "system";
let activeMenu = null;
let lastFocusedElement = null;
let mediaThemeQuery = null;
let isApplyingLocale = false;

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
  dom.btnRefreshDashboard = document.getElementById("btn-refresh-dashboard");
  dom.btnThemeMenu = document.getElementById("btn-theme-menu");
  dom.btnMoreMenu = document.getElementById("btn-more-menu");
  dom.themeMenu = document.getElementById("theme-menu");
  dom.moreMenu = document.getElementById("more-menu");
  dom.themeMenuItems = Array.from(document.querySelectorAll("[data-theme-option]"));
  dom.btnOpenServiceLog = document.getElementById("btn-open-service-log");

  dom.serviceLogModal = document.getElementById("service-log-modal");
  dom.btnCloseServiceLog = document.getElementById("btn-close-service-log");
  dom.btnRefreshLogs = document.getElementById("btn-refresh-logs");
  dom.serviceLog = document.getElementById("service-log");

  dom.serviceIndicatorDot = document.getElementById("service-indicator-dot");
  dom.serviceStatusLabel = document.getElementById("service-status-label");
  dom.serviceInstalledValue = document.getElementById("service-installed-value");
  dom.serviceRunningValue = document.getElementById("service-running-value");
  dom.serviceControlValue = document.getElementById("service-control-value");
  dom.serviceErrorText = document.getElementById("service-error-text");
  dom.btnInstallService = document.getElementById("btn-install-service");
  dom.btnStartService = document.getElementById("btn-start-service");
  dom.btnStopService = document.getElementById("btn-stop-service");

  dom.indicatorDot = document.getElementById("indicator-dot");
  dom.connectionText = document.getElementById("connection-text");
  dom.connectionStatusDisplay = document.getElementById("connection-status-display");

  dom.batteryShell = document.getElementById("battery-shell");
  dom.batteryFill = document.getElementById("battery-fill");
  dom.batteryPercent = document.getElementById("battery-percent");
  dom.batteryLevelCard = document.querySelector(".battery-level-card");
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
  dom.actionHint = document.getElementById("action-hint");

  dom.btnChargeFull = document.getElementById("btn-charge-full");
  dom.btnChargeLimit = document.getElementById("btn-charge-limit");
  dom.btnResetChargeMode = document.getElementById("btn-reset-charge-mode");
  dom.btnDisableCharging = document.getElementById("btn-disable-charging");
  dom.btnToggleAdapter = document.getElementById("btn-toggle-adapter");
  dom.adapterBtnText = document.getElementById("adapter-btn-text");

  dom.selectLanguage = document.getElementById("select-language");
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

window.addEventListener("DOMContentLoaded", async () => {
  cacheDom();
  initializeTheme();
  initializeLocaleControls();
  bindEvents();
  updateSettingsUI();
  await initialize();
});

async function initialize() {
  try {
    await Promise.all([loadDashboardSnapshot(), refreshServiceLogs()]);
    startPolling();
    setupEventListener();
  } catch (err) {
    console.error("Initialization error:", err);
    showError("Failed to initialize: " + formatError(err));
  } finally {
    hideLoading();
  }
}

function readStoredThemeMode() {
  const saved = localStorage.getItem(THEME_STORAGE_KEY);
  return THEME_OPTIONS.has(saved) ? saved : "system";
}

function resolveActiveTheme(mode = themeMode) {
  if (mode === "light" || mode === "dark") {
    return mode;
  }
  return mediaThemeQuery?.matches ? "dark" : "light";
}

function initializeTheme() {
  mediaThemeQuery = window.matchMedia?.("(prefers-color-scheme: dark)") || null;
  themeMode = readStoredThemeMode();
  applyTheme();

  if (mediaThemeQuery?.addEventListener) {
    mediaThemeQuery.addEventListener("change", onSystemThemeChange);
  } else if (mediaThemeQuery?.addListener) {
    mediaThemeQuery.addListener(onSystemThemeChange);
  }
}

function onSystemThemeChange() {
  if (themeMode === "system") {
    applyTheme();
  }
}

function applyTheme() {
  const activeTheme = resolveActiveTheme();
  document.documentElement.dataset.theme = activeTheme;
  document.documentElement.dataset.themeMode = themeMode;
  updateThemeMenuState();
}

function setThemeMode(nextMode, showToastMessage = false) {
  const normalized = THEME_OPTIONS.has(nextMode) ? nextMode : "system";
  themeMode = normalized;
  localStorage.setItem(THEME_STORAGE_KEY, normalized);
  applyTheme();
  if (showToastMessage) {
    showSuccess(__("msg.theme_updated"));
  }
}

function initializeLocaleControls() {
  if (dom.selectLanguage) {
    dom.selectLanguage.value = getLocaleMode();
  }
}

async function refreshRealtime() {
  try {
    const rt = await invoke("get_battery_realtime");
    if (!rt) return;
    updateRealtimeMetrics(rt);
  } catch (err) {
    console.warn("Could not load realtime data:", err);
  }
}

function updateBatteryHealth(health) {
  if (!health) return;
  dom.healthCycles.textContent = health.cycleCount;
  dom.healthPercent.textContent = health.healthPercent + "%";
  dom.capDesign.textContent = health.designCapacity + " mAh";
  dom.capMax.textContent = health.maxCapacity + " mAh";
  dom.capCurrent.textContent = health.currentCapacity + " mAh";
}

function updateRealtimeMetrics(rt) {
  if (!rt) return;
  state.realtimeAdapterConnected = !!rt.externalConnected;
  if (rt.externalConnected) {
    state.adapterConnected = true;
  }
  dom.rtTemp.textContent = rt.temperature.toFixed(1);
  dom.rtPower.textContent = rt.power.toFixed(2);
  dom.rtVoltage.textContent = (rt.voltage / 1000).toFixed(3) + " V";
  dom.rtCurrent.textContent = rt.amperage + " mA";
}

function updateChargerInfo(charger) {
  state.adapterConnected = !!charger?.connected;
  state.chargerTelemetryConnected = !!(
    charger &&
    (
      charger.connected ||
      charger.wattage > 0 ||
      charger.chargingVoltage > 0 ||
      charger.chargingCurrent > 0 ||
      (charger.name && charger.name !== "—" && charger.name !== "--")
    )
  );
  if (!charger) {
    return;
  }
  dom.chargerName.textContent = charger.name;
  dom.chargerWattage.textContent = charger.connected ? charger.wattage + "W" : "—";
  dom.chargerVoltage.textContent = charger.connected ? charger.chargingVoltage + " mV" : "—";
  dom.chargerCurrent.textContent = charger.connected ? charger.chargingCurrent + " mA" : "—";
}

function updateSystemInfo(info) {
  if (!info) return;
  dom.deviceModel.textContent = info.modelName;
  dom.deviceChip.textContent = info.chip;
  dom.deviceMemory.textContent = info.memoryGb + " GB";
  dom.deviceActivated.textContent = info.activationDate || "--";
}

async function loadDashboardSnapshot() {
  try {
    const snapshot = await invoke("get_dashboard_snapshot");
    if (!snapshot) return;

    if (snapshot.batteryState) {
      Object.assign(state, snapshot.batteryState);
    }
    if (snapshot.serviceStatus) {
      Object.assign(serviceState, snapshot.serviceStatus);
      if (!state.controlAvailable) {
        state.controlAvailable = snapshot.serviceStatus.controlAvailable;
      }
    }
    if (snapshot.settings) {
      state.minCharge = snapshot.settings.minCharge;
      state.maxCharge = snapshot.settings.maxCharge;
      state.adapterSleep = snapshot.settings.adapterSleep;
      state.magsafeSync = snapshot.settings.magsafeSync;
      updateSettingsUI();
    }

    updateBatteryHealth(snapshot.batteryHealth);
    updateRealtimeMetrics(snapshot.batteryRealtime);
    updateChargerInfo(snapshot.chargerInfo);
    updateSystemInfo(snapshot.systemInfo);
    lastSnapshotPollAt = Date.now();
    lastRealtimePollAt = lastSnapshotPollAt;
    scheduleUiUpdate();
  } catch (err) {
    console.error("Failed to load dashboard snapshot:", err);
    state.controlAvailable = false;
    state.lastError = formatError(err);
    serviceState.controlAvailable = false;
    serviceState.lastError = formatError(err);
    scheduleUiUpdate();
    if (serviceState.running) {
      showError("Could not load dashboard snapshot: " + formatError(err));
    }
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
  await loadDashboardSnapshot();
}

async function applySettings() {
  try {
    const minCharge = parseInt(dom.minChargeSlider.value, 10);
    const maxCharge = parseInt(dom.maxChargeSlider.value, 10);

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
      minCharge,
      maxCharge,
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

function bindEvents() {
  dom.minChargeSlider.addEventListener("input", onMinChargeInput);
  dom.maxChargeSlider.addEventListener("input", onMaxChargeInput);
  dom.minChargeSlider.addEventListener("change", onSettingsChange);
  dom.maxChargeSlider.addEventListener("change", onSettingsChange);

  dom.btnChargeFull.addEventListener("click", onChargeToFull);
  dom.btnChargeLimit.addEventListener("click", onChargeToLimit);
  dom.btnResetChargeMode.addEventListener("click", onResetChargeMode);
  dom.btnDisableCharging.addEventListener("click", onDisableCharging);
  dom.btnToggleAdapter.addEventListener("click", onToggleAdapter);
  dom.btnInstallService.addEventListener("click", onInstallService);
  dom.btnStartService.addEventListener("click", onStartService);
  dom.btnStopService.addEventListener("click", onStopService);
  dom.btnRefreshLogs.addEventListener("click", onRefreshLogs);
  dom.btnRefreshDashboard.addEventListener("click", onRefreshDashboard);
  dom.btnThemeMenu.addEventListener("click", () => toggleMenu("theme"));
  dom.btnMoreMenu.addEventListener("click", () => toggleMenu("more"));
  dom.btnOpenServiceLog.addEventListener("click", openServiceLogModal);
  dom.btnCloseServiceLog.addEventListener("click", closeServiceLogModal);
  dom.serviceLogModal.addEventListener("click", onModalBackdropClick);

  for (const item of dom.themeMenuItems) {
    item.addEventListener("click", () => {
      setThemeMode(item.dataset.themeOption, true);
      closeMenus();
    });
  }

  dom.selectLanguage.addEventListener("change", onLanguageChange);
  dom.toggleAdapterSleep.addEventListener("change", onSettingsChange);
  dom.toggleMagSafeSync.addEventListener("change", onSettingsChange);

  dom.errorClose.addEventListener("click", () => {
    dom.errorToast.classList.remove("visible");
  });

  document.addEventListener("click", onDocumentClick);
  document.addEventListener("keydown", onDocumentKeydown);
  window.addEventListener(LOCALE_CHANGE_EVENT, onLocaleChanged);

  for (const container of scrollContainers) {
    container.addEventListener("scroll", onUserScroll, { passive: true });
  }
}

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
  if (settingsDebounceTimer) {
    clearTimeout(settingsDebounceTimer);
  }
  settingsDebounceTimer = setTimeout(() => {
    applySettings();
  }, 500);
}

function onLanguageChange() {
  const changed = setLocaleMode(dom.selectLanguage.value);
  if (changed || !isApplyingLocale) {
    showSuccess(__("msg.language_updated"));
  }
}

function onLocaleChanged() {
  isApplyingLocale = true;
  applyLocaleToDom();
  document.documentElement.lang = getLocaleMode() === "zh" ? "zh" : "en";
  dom.selectLanguage.value = getLocaleMode();
  updateThemeMenuState();
  updateSettingsUI();
  scheduleUiUpdate();
  isApplyingLocale = false;
}

function onDocumentClick(event) {
  if (activeMenu === "theme") {
    const inTheme = dom.themeMenu.contains(event.target) || dom.btnThemeMenu.contains(event.target);
    if (!inTheme) {
      closeMenus();
    }
  }

  if (activeMenu === "more") {
    const inMore = dom.moreMenu.contains(event.target) || dom.btnMoreMenu.contains(event.target);
    if (!inMore) {
      closeMenus();
    }
  }
}

function onDocumentKeydown(event) {
  if (event.key === "Escape") {
    if (!dom.serviceLogModal.hidden) {
      closeServiceLogModal();
      return;
    }
    if (activeMenu) {
      closeMenus();
    }
  }
}

function toggleMenu(menuName) {
  if (activeMenu === menuName) {
    closeMenus();
    return;
  }

  closeMenus();
  activeMenu = menuName;

  const menu = menuName === "theme" ? dom.themeMenu : dom.moreMenu;
  const button = menuName === "theme" ? dom.btnThemeMenu : dom.btnMoreMenu;
  menu.hidden = false;
  button.setAttribute("aria-expanded", "true");
}

function closeMenus() {
  activeMenu = null;
  dom.themeMenu.hidden = true;
  dom.moreMenu.hidden = true;
  dom.btnThemeMenu.setAttribute("aria-expanded", "false");
  dom.btnMoreMenu.setAttribute("aria-expanded", "false");
}

function updateThemeMenuState() {
  for (const item of dom.themeMenuItems) {
    const selected = item.dataset.themeOption === themeMode;
    item.classList.toggle("is-selected", selected);
    item.setAttribute("aria-checked", selected ? "true" : "false");
  }
}

function openServiceLogModal() {
  closeMenus();
  lastFocusedElement = document.activeElement;
  dom.serviceLogModal.hidden = false;
  document.body.classList.add("modal-open");
  dom.btnCloseServiceLog.focus();
  refreshServiceLogs().catch((err) => {
    console.warn("Could not refresh service logs:", err);
  });
}

function closeServiceLogModal() {
  dom.serviceLogModal.hidden = true;
  document.body.classList.remove("modal-open");
  if (lastFocusedElement && typeof lastFocusedElement.focus === "function") {
    lastFocusedElement.focus();
  }
}

function onModalBackdropClick(event) {
  if (event.target === dom.serviceLogModal) {
    closeServiceLogModal();
  }
}

async function onChargeToFull() {
  if (!canUseChargeActions()) return;
  try {
    setButtonsDisabled(true);
    await invoke("charge_to_full");
    showSuccess(__("msg.charging_full"));
    await loadDashboardSnapshot();
  } catch (err) {
    showError("Failed: " + formatError(err));
  } finally {
    setButtonsDisabled(false);
    updateControlAvailability();
  }
}

async function onChargeToLimit() {
  if (!canUseChargeActions()) return;
  try {
    setButtonsDisabled(true);
    await invoke("charge_to_limit");
    showSuccess(__("msg.charging_limit"));
    await loadDashboardSnapshot();
  } catch (err) {
    showError("Failed: " + formatError(err));
  } finally {
    setButtonsDisabled(false);
    updateControlAvailability();
  }
}

async function onResetChargeMode() {
  if (!canUseChargeActions()) return;
  try {
    setButtonsDisabled(true);
    await invoke("reset_charge_mode");
    showSuccess(__("msg.mode_reset"));
    await loadDashboardSnapshot();
  } catch (err) {
    showError("Failed: " + formatError(err));
  } finally {
    setButtonsDisabled(false);
    updateControlAvailability();
  }
}

async function onDisableCharging() {
  if (!canUseChargeActions()) return;
  try {
    setButtonsDisabled(true);
    await invoke("disable_charging_cmd");
    showSuccess(__("msg.charging_disabled"));
    await loadDashboardSnapshot();
  } catch (err) {
    showError("Failed: " + formatError(err));
  } finally {
    setButtonsDisabled(false);
    updateControlAvailability();
  }
}

async function onToggleAdapter() {
  if (!canUseChargeActions()) return;
  try {
    setButtonsDisabled(true);
    if (state.powerDisabled) {
      await invoke("enable_adapter_cmd");
      showSuccess(__("msg.adapter_enabled"));
    } else {
      await invoke("disable_adapter_cmd");
      showSuccess(__("msg.adapter_disabled"));
    }
    await loadDashboardSnapshot();
  } catch (err) {
    showError("Failed: " + formatError(err));
  } finally {
    setButtonsDisabled(false);
    updateControlAvailability();
  }
}

async function onInstallService() {
  try {
    setServiceButtonsDisabled(true);
    await invoke("install_service");
    await loadDashboardSnapshot();
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
    await loadDashboardSnapshot();
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
    await loadDashboardSnapshot();
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
    await refreshServiceLogs();
  } finally {
    setServiceButtonsDisabled(false);
  }
}

async function onRefreshDashboard() {
  try {
    dom.btnRefreshDashboard.disabled = true;
    dom.btnRefreshDashboard.classList.add("is-refreshing");
    await Promise.all([loadDashboardSnapshot(), refreshServiceLogs()]);
    showSuccess(__("msg.refresh_complete"));
  } catch (err) {
    showError("Failed: " + formatError(err));
  } finally {
    dom.btnRefreshDashboard.classList.remove("is-refreshing");
    dom.btnRefreshDashboard.disabled = false;
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
      typeof event.payload === "boolean" ? event.payload : !!event.payload?.visible;

    windowVisible = visible;
    if (!visible) {
      stopPolling();
      return;
    }

    await Promise.all([loadDashboardSnapshot(), refreshServiceLogs()]);
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

function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

function startPolling() {
  stopPolling();
  pollTimer = setInterval(async () => {
    if (!windowVisible || settingsDirty || isUserScrolling) {
      return;
    }

    const now = Date.now();
    if (now - lastSnapshotPollAt >= 30000) {
      lastSnapshotPollAt = now;
      lastRealtimePollAt = now;
      await loadDashboardSnapshot();
      return;
    }
    if (now - lastRealtimePollAt >= 15000) {
      lastRealtimePollAt = now;
      await refreshRealtime();
    }
  }, 5000);
}

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
  const adapterConnected = hasConnectedAdapter();
  const isActivelyCharging =
    state.isCharging &&
    adapterConnected &&
    !state.powerDisabled &&
    !state.chargingDisabled;
  const isStandby =
    adapterConnected &&
    !isActivelyCharging &&
    !state.powerDisabled &&
    !state.chargingDisabled;

  dom.batteryShell.style.setProperty("--fill-pct", pct + "%");
  dom.batteryPercent.textContent = pct + "%";
  dom.batteryFill.classList.toggle("charging", isActivelyCharging);
  dom.batteryShell.classList.toggle("charging", isActivelyCharging);
  dom.batteryFill.classList.toggle("standby", isStandby);
  dom.batteryShell.classList.toggle("standby", isStandby);
  dom.batteryShell.classList.toggle("disabled", !!state.powerDisabled || !!state.chargingDisabled);

  if (dom.batteryLevelCard) {
    dom.batteryLevelCard.classList.toggle("charging", isActivelyCharging);
    dom.batteryLevelCard.classList.toggle("standby", isStandby);
    dom.batteryLevelCard.classList.toggle("disabled", !!state.powerDisabled || !!state.chargingDisabled);
  }

  dom.batteryFill.classList.remove("charge-low", "charge-mid", "charge-good", "charge-full");
  dom.batteryLevelCard?.classList.remove("charge-low", "charge-mid", "charge-good", "charge-full");

  if (pct <= 20) {
    dom.batteryFill.classList.add("charge-low");
    dom.batteryLevelCard?.classList.add("charge-low");
  } else if (pct <= 50) {
    dom.batteryFill.classList.add("charge-mid");
    dom.batteryLevelCard?.classList.add("charge-mid");
  } else if (pct < 95) {
    dom.batteryFill.classList.add("charge-good");
    dom.batteryLevelCard?.classList.add("charge-good");
  } else {
    dom.batteryFill.classList.add("charge-full");
    dom.batteryLevelCard?.classList.add("charge-full");
  }

  dom.chargePercentDisplay.textContent = pct + "%";

  if (state.chargingDisabled) {
    dom.chargingStateDisplay.textContent = __("status.disabled");
    dom.chargingStateDisplay.style.color = "var(--accent-red)";
  } else if (state.powerDisabled) {
    dom.chargingStateDisplay.textContent = __("status.adapter_off");
    dom.chargingStateDisplay.style.color = "var(--accent-orange)";
  } else if (state.isCharging) {
    dom.chargingStateDisplay.textContent = __("status.charging");
    dom.chargingStateDisplay.style.color = "var(--accent-green)";
  } else if (adapterConnected) {
    dom.chargingStateDisplay.textContent = __("status.connected");
    dom.chargingStateDisplay.style.color = "var(--accent-green)";
  } else {
    dom.chargingStateDisplay.textContent = __("status.on_battery");
    dom.chargingStateDisplay.style.color = "var(--text-secondary)";
  }

  dom.adapterStateDisplay.textContent = state.powerDisabled
    ? __("status.disabled")
    : adapterConnected
      ? __("status.connected")
      : __("status.not_connected");
  dom.adapterStateDisplay.style.color = state.powerDisabled
    ? "var(--accent-red)"
    : adapterConnected
      ? "var(--accent-green)"
      : "var(--text-secondary)";
}

function updateStatusBadges() {
  if (state.enabled) {
    dom.badgeEnabled.textContent = __("badge.enabled");
    dom.badgeEnabled.className = "badge badge-enabled";
  } else {
    dom.badgeEnabled.textContent = __("badge.disabled");
    dom.badgeEnabled.className = "badge badge-disabled";
  }

  const mode = state.mode || "Standard";
  const modeKey = "badge." + mode.toLowerCase();
  dom.badgeMode.textContent = __(modeKey);
  dom.badgeMode.className = mode === "ToFull" ? "badge badge-mode-warn" : "badge badge-mode";
}

function updateConnectionIndicator() {
  dom.indicatorDot.classList.remove("connected", "charging");
  let connectionLabel = __("status.disconnected");
  const adapterConnected = hasConnectedAdapter();

  if (adapterConnected && !state.powerDisabled) {
    if (state.isCharging && !state.chargingDisabled) {
      dom.indicatorDot.classList.add("charging");
      connectionLabel = __("status.charging");
    } else {
      dom.indicatorDot.classList.add("connected");
      connectionLabel = __("status.connected");
    }
  } else if (adapterConnected) {
    dom.indicatorDot.classList.add("connected");
    connectionLabel = __("status.connected");
  }

  dom.connectionText.textContent = connectionLabel;
  dom.connectionStatusDisplay.textContent = connectionLabel;
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

function hasConnectedAdapter() {
  return (
    !!state.isPlugged ||
    !!state.adapterConnected ||
    !!state.realtimeAdapterConnected ||
    !!state.chargerTelemetryConnected ||
    !!state.isCharging
  );
}

function canUseChargeActions() {
  return hasConnectedAdapter() && state.controlAvailable && serviceState.controlAvailable;
}

function updateControlAvailability() {
  const available = state.controlAvailable && serviceState.controlAvailable;
  const actionsAvailable = available && hasConnectedAdapter();

  dom.rootNotice.style.display = available ? "none" : "flex";
  setButtonsDisabled(!actionsAvailable);
  dom.minChargeSlider.disabled = !available;
  dom.maxChargeSlider.disabled = !available;
  dom.toggleAdapterSleep.disabled = !available;
  dom.toggleMagSafeSync.disabled = !available;

  if (!available) {
    dom.actionHint.textContent = "";
    dom.actionHint.className = "slider-validation action-hint";
  } else if (!hasConnectedAdapter()) {
    dom.actionHint.textContent = __("validation.adapter_required");
    dom.actionHint.className = "slider-validation action-hint";
  } else {
    dom.actionHint.textContent = "";
    dom.actionHint.className = "slider-validation action-hint";
  }
}

function updateServiceCard() {
  const rawError = serviceState.lastError || state.lastError || "";
  const shouldShowError = serviceState.running || !isExpectedServiceMissingError(rawError);

  dom.serviceInstalledValue.textContent = serviceState.installed ? __("service.yes") : __("service.no");
  dom.serviceRunningValue.textContent = serviceState.running ? __("service.yes") : __("service.no");
  dom.serviceControlValue.textContent = serviceState.controlAvailable
    ? __("service.available")
    : __("service.unavailable");
  dom.serviceErrorText.textContent = shouldShowError ? rawError : "";
  dom.serviceErrorText.className = shouldShowError && rawError
    ? "slider-validation error service-summary"
    : "slider-validation service-summary";

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
  dom.minChargeSlider.value = state.minCharge;
  dom.maxChargeSlider.value = state.maxCharge;
  dom.minChargeValue.textContent = state.minCharge + "%";
  dom.maxChargeValue.textContent = state.maxCharge + "%";
  dom.selectLanguage.value = getLocaleMode();

  updateSliderFill(dom.minChargeSlider, dom.minChargeFill);
  updateSliderFill(dom.maxChargeSlider, dom.maxChargeFill);

  dom.toggleAdapterSleep.checked = state.adapterSleep;
  dom.toggleMagSafeSync.checked = state.magsafeSync;

  validateSliders();
}

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

function setButtonsDisabled(disabled) {
  dom.btnChargeFull.disabled = disabled;
  dom.btnChargeLimit.disabled = disabled;
  dom.btnResetChargeMode.disabled = disabled;
  dom.btnDisableCharging.disabled = disabled;
  dom.btnToggleAdapter.disabled = disabled;
}

function setServiceButtonsDisabled(disabled) {
  dom.btnInstallService.disabled = disabled;
  dom.btnStartService.disabled = disabled || serviceState.running;
  dom.btnStopService.disabled = disabled || !serviceState.running;
  dom.btnRefreshLogs.disabled = disabled;
}

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
