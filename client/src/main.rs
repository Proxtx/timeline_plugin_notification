use leptos::prelude::*;
use serde::Deserialize;

use timeline_plugin_client_sdk::{plugin_entry, PluginContext};

#[derive(Debug, Clone, Deserialize)]
struct Notification {
    app: String,
    title: String,
    content: String,
}

fn main() {
    console_error_panic_hook::set_once();
}

fn render(ctx: PluginContext) -> impl IntoView {
    let Ok(notif) = serde_json::from_value::<Notification>(ctx.event.data.clone()) else {
        return view! { <div>Malformed notification</div> }.into_any();
    };
    let icon_url = format!(
        "{}/icon/{}",
        ctx.api_base.trim_end_matches('/'),
        notif.app
    );
    view! {
        <div class="notif-card">
            <img class="notif-icon" src=icon_url />
            <div class="notif-meta">
                <h3>{notif.title}</h3>
                <a>{notif.content}</a>
            </div>
        </div>
    }
    .into_any()
}

plugin_entry!(render);
