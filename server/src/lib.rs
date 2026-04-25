//! Notification plugin: external clients (a phone, a script, an OS daemon)
//! POST `/notification/<password>/<app>/<title>/<content>` to record a
//! notification. Stored notifications come back through `/events`. App
//! display names + icons are read from on-disk lookup files.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, routes, Build, Rocket, Route, State};
use serde::{Deserialize, Serialize};
use tokio::fs::{try_exists, File};
use tokio::io::AsyncReadExt;

use timeline_plugin_sdk::auth::AuthedClient;
use timeline_plugin_sdk::launch::PluginState;
use timeline_plugin_sdk::{
    APIError, APIResult, CompressedEvent, Context, Manifest, Plugin, Style, StoredEvent,
    TimeRange, Timing,
};

#[derive(Debug, Clone, Deserialize)]
pub struct NotificationConfig {
    /// Path to a `package_name:Display Name` file (one mapping per line).
    pub apps_file: PathBuf,
    /// Directory containing per-app icon files keyed by package name.
    pub app_icon_files: PathBuf,
    /// Optional path to a default icon to fall back to when no app-specific
    /// file is found.
    #[serde(default)]
    pub default_icon: Option<PathBuf>,
    /// Password external clients must supply in the URL to record a
    /// notification. Distinct from the main-server cookie password.
    pub notification_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub app: String,
    pub title: String,
    pub content: String,
}

pub struct NotificationPlugin {
    ctx: Context,
    config: NotificationConfig,
    apps_map: Arc<AppsMap>,
}

impl Plugin for NotificationPlugin {
    async fn new(ctx: Context) -> anyhow::Result<Self> {
        let config: NotificationConfig = ctx
            .extra
            .clone()
            .try_into()
            .map_err(|e| anyhow::anyhow!("plugin config: {}", e))?;
        let apps_map = AppsMap::load(&config.apps_file).await?;
        Ok(Self {
            ctx,
            config,
            apps_map: Arc::new(apps_map),
        })
    }

    fn manifest(&self) -> Manifest {
        Manifest {
            name: self.ctx.config.name.clone(),
            display_name: self
                .ctx
                .config
                .display_name
                .clone()
                .unwrap_or_else(|| "Notifications".into()),
            style: Style::Acc2,
            icon: None,
            web_entry: Some("timeline_plugin_notification_client.js".into()),
        }
    }

    async fn events(&self, range: TimeRange) -> APIResult<Vec<CompressedEvent>> {
        let stored = self
            .ctx
            .db
            .query_range_typed::<Notification>(&range)
            .await
            .map_err(|e| APIError::DatabaseError(e.to_string()))?;
        let mut out = Vec::with_capacity(stored.len());
        for ev in stored {
            let app_display = self
                .apps_map
                .get(&ev.data.app)
                .cloned()
                .unwrap_or_else(|| ev.data.app.clone());
            out.push(CompressedEvent {
                title: app_display,
                time: ev.time,
                data: serde_json::to_value(&ev.data)?,
            });
        }
        Ok(out)
    }

    fn routes(&self) -> Vec<Route> {
        routes![new_notification, app_icon]
    }

    fn rocket_attach(&self, rocket: Rocket<Build>) -> Rocket<Build> {
        rocket.manage(PluginCfgState(self.config.clone()))
    }
}

// ----- routes -----

struct PluginCfgState(NotificationConfig);

#[get("/notification/<password>/<app>/<title>/<content>")]
async fn new_notification(
    _auth: AuthedClient,
    password: &str,
    app: &str,
    title: &str,
    content: &str,
    state: &State<PluginState>,
    cfg: &State<PluginCfgState>,
) -> (Status, Json<APIResult<()>>) {
    if password != cfg.0.notification_password {
        return (
            Status::Unauthorized,
            Json(Err(APIError::AuthenticationError)),
        );
    }
    let now = Utc::now();
    let stored = StoredEvent {
        id: now.timestamp_millis().to_string(),
        title: title.to_string(),
        time: Timing::Instant(now),
        data: match serde_json::to_value(Notification {
            app: app.to_string(),
            title: title.to_string(),
            content: content.to_string(),
        }) {
            Ok(v) => v,
            Err(e) => {
                return (
                    Status::InternalServerError,
                    Json(Err(APIError::SerdeJsonError(e.to_string()))),
                )
            }
        },
    };
    match state.db.upsert(&stored).await {
        Ok(()) => (Status::Ok, Json(Ok(()))),
        Err(e) => {
            state.errors.report(format!("notification insert: {}", e));
            (
                Status::InternalServerError,
                Json(Err(APIError::DatabaseError(e.to_string()))),
            )
        }
    }
}

#[get("/icon/<app>")]
async fn app_icon(
    _auth: AuthedClient,
    app: &str,
    cfg: &State<PluginCfgState>,
) -> Option<NamedFile> {
    let path = cfg.0.app_icon_files.join(app);
    if matches!(try_exists(&path).await, Ok(true)) {
        if let Ok(f) = NamedFile::open(&path).await {
            return Some(f);
        }
    }
    if let Some(default) = &cfg.0.default_icon {
        if let Ok(f) = NamedFile::open(default).await {
            return Some(f);
        }
    }
    None
}

// ----- apps map -----

struct AppsMap {
    apps_map: HashMap<String, String>,
}

impl AppsMap {
    pub async fn load(path: &Path) -> anyhow::Result<AppsMap> {
        let mut f = File::open(path).await?;
        let mut s = String::new();
        f.read_to_string(&mut s).await?;
        let apps_map = s
            .split('\n')
            .filter_map(|line| line.split_once(':').map(|v| (v.0.to_string(), v.1.to_string())))
            .collect();
        Ok(AppsMap { apps_map })
    }

    pub fn get(&self, package: &str) -> Option<&String> {
        self.apps_map.get(package)
    }
}
