use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

const EVENT_UPDATE_STATUS_CHANGED: &str = "update-status-changed";

#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub configured: bool,
    pub checking: bool,
    pub update_available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub notes: Option<String>,
    pub published_at: Option<String>,
    pub download_in_progress: bool,
    pub downloaded_bytes: u64,
    pub content_length: Option<u64>,
    pub ready_to_install: bool,
    pub startup_badge_visible: bool,
    pub error: Option<String>,
}

pub struct UpdateState {
    inner: Mutex<UpdateStatus>,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(UpdateStatus::default()),
        }
    }
}

fn config_is_ready(app: &AppHandle) -> bool {
    let updater = app.config().plugins.0.get("updater");
    let Some(value) = updater else {
        return false;
    };
    let pubkey = value
        .get("pubkey")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .unwrap_or("");
    let has_endpoints = value
        .get("endpoints")
        .and_then(|v| v.as_array())
        .map(|items| !items.is_empty())
        .unwrap_or(false);
    !pubkey.is_empty() && has_endpoints
}

pub fn initialize(app: &AppHandle) {
    let state = app.state::<UpdateState>();
    let mut status = state.inner.lock().unwrap();
    status.current_version = app.package_info().version.to_string();
    status.configured = config_is_ready(app);
}

fn set_status(app: &AppHandle, update: impl FnOnce(&mut UpdateStatus)) -> Result<UpdateStatus, String> {
    let snapshot = {
        let state = app.state::<UpdateState>();
        let mut status = state.inner.lock().map_err(|e| e.to_string())?;
        update(&mut status);
        status.clone()
    };
    let _ = app.emit(EVENT_UPDATE_STATUS_CHANGED, snapshot.clone());
    Ok(snapshot)
}

pub fn get_status(app: &AppHandle) -> Result<UpdateStatus, String> {
    app.state::<UpdateState>()
        .inner
        .lock()
        .map(|status| status.clone())
        .map_err(|e| e.to_string())
}

pub async fn check(app: AppHandle, startup: bool) -> Result<UpdateStatus, String> {
    let configured = config_is_ready(&app);
    set_status(&app, |status| {
        status.current_version = app.package_info().version.to_string();
        status.configured = configured;
        status.error = None;
        status.checking = configured;
        status.download_in_progress = false;
        status.downloaded_bytes = 0;
        status.content_length = None;
        status.ready_to_install = false;
        if !configured {
            status.update_available = false;
            status.latest_version = None;
            status.notes = None;
            status.published_at = None;
            status.startup_badge_visible = false;
        }
    })?;

    if !configured {
        return set_status(&app, |status| {
            status.checking = false;
            status.error = Some("Updater is not configured yet.".to_string());
        });
    }

    let updater = app.updater().map_err(|e| e.to_string())?;
    match updater.check().await.map_err(|e| e.to_string())? {
        Some(update) => {
            let latest_version = update.version.clone();
            let notes = update.body.clone();
            let published_at = update.date.map(|date| date.to_string());
            set_status(&app, move |status| {
                status.checking = false;
                status.update_available = true;
                status.latest_version = Some(latest_version);
                status.notes = notes;
                status.published_at = published_at;
                status.startup_badge_visible = startup || status.startup_badge_visible;
                status.error = None;
            })
        }
        None => set_status(&app, |status| {
            status.checking = false;
            status.update_available = false;
            status.latest_version = None;
            status.notes = None;
            status.published_at = None;
            status.startup_badge_visible = false;
            status.error = None;
        }),
    }
}

pub async fn download_and_install(app: AppHandle) -> Result<UpdateStatus, String> {
    let configured = config_is_ready(&app);
    if !configured {
        return set_status(&app, |status| {
            status.error = Some("Updater is not configured yet.".to_string());
        });
    }

    set_status(&app, |status| {
        status.error = None;
        status.download_in_progress = true;
        status.downloaded_bytes = 0;
        status.content_length = None;
        status.ready_to_install = false;
    })?;

    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No update is currently available.".to_string())?;

    let app_handle = app.clone();
    update
        .download_and_install(
            move |chunk_length, content_length| {
                let _ = set_status(&app_handle, |status| {
                    status.downloaded_bytes += chunk_length as u64;
                    status.content_length = content_length;
                    status.download_in_progress = true;
                });
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;

    set_status(&app, |status| {
        status.download_in_progress = false;
        status.ready_to_install = true;
        status.startup_badge_visible = false;
        status.error = None;
    })
}

pub fn clear_badge(app: &AppHandle) -> Result<UpdateStatus, String> {
    set_status(app, |status| {
        status.startup_badge_visible = false;
    })
}
