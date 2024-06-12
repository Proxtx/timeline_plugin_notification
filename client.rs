use {
    leptos::{view, View, IntoView},
    serde::Deserialize
};

pub struct Plugin {
    
}

impl crate::plugin_manager::Plugin for Plugin {
    fn get_style(&self) -> crate::plugin_manager::Style {
        crate::plugin_manager::Style::Acc2
    }

    async fn new(_data: crate::plugin_manager::PluginData) -> Self
        where
            Self: Sized {
        Plugin {}
    }

    fn get_component(&self, data: crate::plugin_manager::PluginEventData) -> crate::event_manager::EventResult<Box<dyn FnOnce() -> leptos::View>> {
        let data = data.get_data::<Notification>()?;
        Ok(Box::new(
            move || -> View {
                view! {
                    <div style="display: flex; flex-direction: row; width: 100%; gap: calc(var(--contentSpacing) * 0.5); background-color: var(--accentColor2);align-items: start;">
                        <img
                            style="width: calc(var(--contentSpacing) * 5); aspect-ratio: 1; padding: var(--contentSpacing);"
                            src=move || {
                                crate::api::relative_url(
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