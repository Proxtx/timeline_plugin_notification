use {
    client_api::{api, plugin::{PluginData, PluginEventData, PluginTrait}, result::EventResult, style::Style}, leptos::{view, IntoView, View}, serde::Deserialize
};

pub struct Plugin {
    
}

impl PluginTrait for Plugin {
    fn get_style(&self) -> Style {
        Style::Acc2
    }

    async fn new(_data: PluginData) -> Self
        where
            Self: Sized {
        Plugin {}
    }

    fn get_component(&self, data: PluginEventData) -> EventResult<Box<dyn FnOnce() -> leptos::View>> {
        let data = data.get_data::<Notification>()?;
        Ok(Box::new(
            move || -> View {
                view! {
                    <div style="display: flex; flex-direction: row; width: 100%; gap: calc(var(--contentSpacing) * 0.5); background-color: var(--accentColor2);align-items: start;">
                        <img
                            style="width: calc(var(--contentSpacing) * 5); aspect-ratio: 1; padding: var(--contentSpacing);"
                            src=move || {
                                api::relative_url(
                                        &format!(
                                            "/api/plugin/timeline_plugin_notification/icon/{}",
                                            data.app,
                                        ),
                                    )
                                    .unwrap()
                                    .to_string()
                            }
                        />

                        <div style="padding-top: calc(var(--contentSpacing) * 0.5); padding-bottom: calc(var(--contentSpacing) * 0.5); color: var(--lightColor); overflow: hidden;">
                            <h3>{move || { data.title.clone() }}</h3>
                            <a>{move || { data.content.clone() }}</a>
                        </div>
                    </div>
                }.into_view()
            }
        ))
    }
}

#[derive(Deserialize)]
struct Notification {
    app: String,
    title: String,
    content: String,
}