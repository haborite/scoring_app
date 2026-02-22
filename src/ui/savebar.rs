use dioxus::prelude::*;
use std::path::PathBuf;

use crate::models::Config;

#[component]
pub fn SaveBar(config: Signal<Config>) -> Element {
    // 前回の保存先（未保存なら None）
    let mut save_path = use_signal(|| Option::<PathBuf>::None);

    // Save（初回はダイアログ、2回目以降は同じ場所に上書き）
    let on_save = {
        move |_| {
            let current_path = save_path.read().clone();
            let cfg = config.read().clone();

            spawn(async move {
                let path = if let Some(p) = current_path {
                    p
                } else {
                    let handle = rfd::AsyncFileDialog::new()
                        .add_filter("JSON", &["json"])
                        .set_file_name("config.json")
                        .save_file()
                        .await;

                    let Some(handle) = handle else {
                        return; // canceled
                    };
                    handle.path().to_path_buf()
                };

                if cfg.save_to_filepath(&path).await.is_ok() {
                    save_path.set(Some(path));
                }
                // エラー表示したいならここで toast 用 Signal を更新
            });
        }
    };

    // Save As（毎回ダイアログ）
    let on_save_as = {
        move |_| {
            let cfg = config.read().clone();
            spawn(async move {
                let handle = rfd::AsyncFileDialog::new()
                    .add_filter("JSON", &["json"])
                    .set_file_name("config.json")
                    .save_file()
                    .await;

                let Some(handle) = handle else {
                    return; // canceled
                };
                let path = handle.path().to_path_buf();

                if cfg.save_to_filepath(&path).await.is_ok() {
                    save_path.set(Some(path));
                }
            });
        }
    };

    rsx! {
        div { class: "flex items-center gap-2",
            button { class: "btn btn-sm btn-primary", onclick: on_save, "Save" }
            button { class: "btn btn-sm", onclick: on_save_as, "Save As" }
            button { 
                class: "btn btn-sm", 
                onclick: move |_| {
                    spawn(async move {
                        let txt = match read_json_text().await {
                            Ok(Some(s)) => s,
                            Ok(None) => return,
                            Err(_) => return,
                        };
                        if txt.trim().is_empty() {
                            return;
                        }
                        let parsed: Result<Config, _> = serde_json::from_str(&txt);
                        let Ok(cfg) = parsed else {
                            return;
                        };
                        config.set(cfg);
                    });
                },
                "Load JSON"
            }

            if let Some(p) = save_path.read().as_ref() {
                span { class: "text-sm opacity-70", "Saved to: {p.display()}" }
            } else {
                span { class: "text-sm opacity-70", "Not saved yet" }
            }
        }
    }
}

async fn read_json_text() -> Result<Option<String>, String> {
    let handle = rfd::AsyncFileDialog::new()
        .add_filter("JSON", &["json"])
        .pick_file()
        .await;

    let Some(handle) = handle else {
        return Ok(None);
    };

    let bytes = handle.read().await;
    let s = String::from_utf8(bytes).map_err(|e| format!("Invalid UTF-8: {e}"))?;
    if s.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(s))
    }
}