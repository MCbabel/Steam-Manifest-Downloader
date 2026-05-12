use std::path::PathBuf;
use tauri::{command, AppHandle, Manager};

use crate::services::{
    settings::{self as settings_service, TelemetryConsent},
    telemetry::{self as telemetry_service, Event},
    AppState,
};

fn app_data_dir(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[command]
pub async fn get_telemetry_status(app: AppHandle) -> Result<serde_json::Value, String> {
    let dir = app_data_dir(&app);
    let settings = settings_service::load_settings(&dir).await;
    let consent = match settings.telemetry_consent {
        TelemetryConsent::Pending => {
            if cfg!(debug_assertions) {
                "declined"
            } else {
                "pending"
            }
        }
        TelemetryConsent::Accepted => "accepted",
        TelemetryConsent::Declined => "declined",
    };
    Ok(serde_json::json!({
        "consent": consent,
        "installation_id": settings.installation_id,
    }))
}

#[command]
pub async fn set_telemetry_consent(app: AppHandle, accept: bool) -> Result<(), String> {
    let dir = app_data_dir(&app);
    let mut settings = settings_service::load_settings(&dir).await;
    settings.telemetry_consent = if accept {
        TelemetryConsent::Accepted
    } else {
        TelemetryConsent::Declined
    };
    if accept && settings.installation_id.is_empty() {
        settings.installation_id = uuid::Uuid::new_v4().to_string();
    }
    settings_service::save_settings(&dir, &settings).await?;

    if accept {
        if let Some(state) = app.try_state::<AppState>() {
            if let Some(telemetry) = state.telemetry.clone() {
                telemetry
                    .emit(Event::new("consent_accepted"))
                    .await;
            }
        }
    }
    Ok(())
}

#[command]
pub async fn emit_telemetry_event(
    app: AppHandle,
    kind: String,
    props: Option<serde_json::Value>,
) -> Result<(), String> {
    if !telemetry_service::is_safe_kind(&kind) {
        return Err("unknown event kind".to_string());
    }
    if let Some(state) = app.try_state::<AppState>() {
        if let Some(telemetry) = state.telemetry.clone() {
            let mut event = Event::new(kind);
            if let Some(p) = props {
                event = event.with_props(p);
            }
            telemetry.emit(event).await;
        }
    }
    Ok(())
}