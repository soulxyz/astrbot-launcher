use tauri_plugin_dialog::{DialogExt as _, MessageDialogButtons};
use tauri_plugin_updater::UpdaterExt as _;

pub fn spawn_check(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = check_and_install_update(app).await {
            log::warn!("Update check failed: {e}");
        }
    });
}

// TODO: Better user experience around updates, e.g. non-blocking notification, background download, etc.
async fn check_and_install_update(app: tauri::AppHandle) -> tauri_plugin_updater::Result<()> {
    let Some(update) = app.updater()?.check().await? else {
        return Ok(());
    };

    let version = update.version.to_string();
    let title = "发现新版本".to_string();
    let message = format!("检测到新版本（{version}），是否立即安装？");

    let ask_handle = app.clone();
    let yes = tauri::async_runtime::spawn_blocking(move || {
        ask_handle
            .dialog()
            .message(message)
            .title(title)
            .buttons(MessageDialogButtons::OkCancelCustom(
                "安装".to_string(),
                "稍后".to_string(),
            ))
            .blocking_show()
    })
    .await
    .unwrap_or(false);

    if !yes {
        return Ok(());
    }

    update
        .download_and_install(|_chunk_length, _content_length| {}, || {})
        .await?;

    app.restart();
}
