# Migration: MongoDB → new SQLite

## Old layout (MongoDB)

```json
{ "plugin": "timeline_plugin_notification",
  "id": "<utc_millis>",
  "timing": [<utc_nanos>],
  "event": { "app": "com.example.app", "title": "Hi", "content": "…" }
}
```

App display-name lookup file (`apps_file`) and the directory of icons
(`app_icon_files`) are unchanged from the old config — paths just move
out of the main server `[plugin_config.timeline_plugin_notification]`
into this plugin's `[config]`.

The notification-creation password used to be `Config.password` (the
main server's cookie password). It now lives in the plugin's own
`[config].notification_password` — pick whatever you want; if you keep
the value identical to the old main-server password, existing external
clients keep working.

## New layout

Per-plugin data dir (default `./data/plugins/timeline_plugin_notification/`):

### `events.db` (SQLite)

| column | value |
|---|---|
| `id` | unchanged from the old `id` (utc_millis as a string) |
| `start_ts`, `end_ts` | both = utc_millis |
| `title` | the notification's `title` field |
| `data` | JSON `{ "app": "…", "title": "…", "content": "…" }` |

(The `apps_file` is consulted at query time to translate `app` → display
name for the timeline UI; not stored in the row.)

### `apps_file`, `app_icon_files`

No transformation. Keep the existing paths and point the plugin's
`[config].apps_file` / `[config].app_icon_files` at them.

## Per-row conversion

For every `{plugin: "timeline_plugin_notification"}` row:

1. `id = old_id`.
2. `start_ts = end_ts = timing[0] / 1_000_000`.
3. `title = event.title`.
4. `data = json!({ app: event.app, title: event.title, content: event.content })`.

## Notes

- Idempotent on the `id` primary key.
- Validate: `SELECT COUNT(*) FROM events` should equal the count of old
  `plugin = "timeline_plugin_notification"` rows.
- `notification_password` is now a per-plugin secret; rotate it if the
  old main-server password should no longer authorize new notifications.
