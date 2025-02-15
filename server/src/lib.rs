use {
    rocket::{
        fs::NamedFile, futures, get, http::Status, routes, serde::json::Json, State
    }, serde::{Deserialize, Serialize}, server_api::{
        config::Config, db::{Database, Event}, external::{futures::StreamExt, tokio::{fs::{try_exists, File}, io::AsyncReadExt}, toml, types::{api::{APIError, APIResult, CompressedEvent}, available_plugins::AvailablePlugins, external::{chrono::Utc, serde_json}, timing::{TimeRange, Timing}}}, plugin::{PluginData, PluginTrait}
    }, std::{collections::HashMap, path::{Path, PathBuf}, sync::Arc}
};

#[derive(Deserialize, Clone)]
pub struct ConfigData {
    pub apps_file: PathBuf,
    pub app_icon_files: PathBuf
}

pub struct Plugin {
    config: ConfigData,
    plugin_data: PluginData,
    apps_map: Arc<AppsMap>
}

impl PluginTrait for Plugin {
    async fn new(data: crate::PluginData) -> Self
    where
        Self: Sized,
    {
        let config: ConfigData = toml::Value::try_into(
            data.config
                .clone()
                .expect("Failed to init notification plugin! No config was provided!"),
        )
        .unwrap_or_else(|e| {
            panic!(
                "Unable to init notification plugin! Provided config does not fit the requirements: {}",
                e
            )
        });

        let apps_map = match AppsMap::new(&config.apps_file).await {
            Ok(v) => v,
            Err(e) => {
                panic!("Unable to init app names lookup table: {}", e);
            }
        };

        Plugin { plugin_data: data, config, apps_map: Arc::new(apps_map) }
    }

    fn get_type() -> AvailablePlugins
    where
        Self: Sized,
    {
        AvailablePlugins::timeline_plugin_notification
    }

    fn get_routes() -> Vec<rocket::Route>
    where
        Self: Sized,
    {
        routes![new_notification, app_icon]
    }

    fn get_compressed_events(
        &self,
        query_range: &TimeRange,
    ) -> std::pin::Pin<
        Box<
            dyn futures::Future<Output = APIResult<Vec<CompressedEvent>>>
                + Send,
        >,
    > {
        let filter = Database::generate_range_filter(query_range);
        let plg_filter = Database::generate_find_plugin_filter(AvailablePlugins::timeline_plugin_notification);
        let filter = Database::combine_documents(filter, plg_filter);
        let apps_map = self.apps_map.clone();
        let database = self.plugin_data.database.clone();
        Box::pin(async move {
            let mut cursor = database
                .get_events::<Notification>()
                .find(filter, None)
                .await?;
            let mut result = Vec::new();
            while let Some(v) = cursor.next().await {
                let t = v?;
                let app = match apps_map.get_app_name(&t.event.app) {
                    Some(v) => v.to_string(),
                    None => t.event.app.clone()
                };

                result.push(CompressedEvent {
                    title: app,
                    time: t.timing,
                    data: serde_json::to_value(t.event).unwrap(),
                })
            }

            Ok(result)
        })
    }

    fn rocket_build_access(&self, rocket: rocket::Rocket<rocket::Build>) -> rocket::Rocket<rocket::Build> {
        rocket.manage(self.config.clone())
    }
}

#[derive(Deserialize, Serialize)]
struct Notification {
    app: String,
    title: String,
    content: String,
}

#[get("/notification/<password>/<app>/<title>/<content>")]
async fn new_notification(
    password: &str,
    app: &str,
    title: &str,
    content: &str,
    config: &State<Config>,
    database: &State<Arc<Database>>,
) -> (Status, Json<APIResult<()>>) {
    if password != config.password {
        return (Status::Unauthorized, Json(Err(APIError::AuthenticationError)));
    }

    match database
        .register_single_event(&Event {
            timing: Timing::Instant(Utc::now()),
            id: Utc::now().timestamp_millis().to_string(),
            plugin: AvailablePlugins::timeline_plugin_notification,
            event: Notification {
                app: app.to_string(),
                title: title.to_string(),
                content: content.to_string()
            },
        })
        .await
    {
        Ok(_) => (Status::Ok, Json(Ok(()))),
        Err(e) => {
            server_api::error::error(database.inner().clone(), &e, Some(<Plugin as PluginTrait>::get_type()), &config.error_report_url);
            (Status::InternalServerError, Json(Err(e.into())))
        },
    }
}

#[get("/icon/<app>")]
pub async fn app_icon(app: &str, config: &State<ConfigData>) -> Option<NamedFile> {
    let mut path = config.app_icon_files.clone();
    path.push(app);
    match try_exists(&path).await {
        Ok(true) => NamedFile::open(path).await.ok(),
        Err(_) | Ok(false) => {
            let mut path = PathBuf::from("../plugins/timeline_plugin_notification/icons/");
            path.push(format!("{}.ico", app.to_lowercase()));
            match try_exists(&path).await {
                Ok(true) => NamedFile::open(path).await.ok(),
                Err(_) | Ok(false) => NamedFile::open(PathBuf::from("../plugins/timeline_plugin_notification/icon.svg")).await.ok()
            }
        }
    }
}

struct AppsMap {
    apps_map: HashMap<String, String>
}

impl AppsMap {
    pub async fn new (path: &Path) -> Result<AppsMap, String> {
        let apps_map = match File::open(path).await {
            Ok(mut v) => {  
                let mut str = String::new();
                if let Err(e) = v.read_to_string(&mut str).await {
                    return Err(format!("Error reading apps file: {}", e));
                }

                str.split('\n').filter_map(|line| {
                    line.split_once(':').map(|v| (v.0.to_string(), v.1.to_string()))
                }).collect()
            },
            Err(e) => {
                return Err(format!("Error reading apps file: {}", e));
            }
        };

        Ok(AppsMap { apps_map })
    }

    pub fn get_app_name (&self, package: &str) -> Option<&String> {
        self.apps_map.get(package)
    }
}