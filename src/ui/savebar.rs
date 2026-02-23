use dioxus::prelude::*;
use std::path::PathBuf;

use crate::Page;
use crate::models::Config;

#[component]
pub fn SaveBar(
    config: Signal<Config>,
    on_nav: EventHandler<Page>,
) -> Element {

    let mut save_path = use_signal(|| Option::<PathBuf>::None);
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
                    return;
                };
                let path = handle.path().to_path_buf();

                if cfg.save_to_filepath(&path).await.is_ok() {
                    save_path.set(Some(path));
                }
            });
        }
    };

    rsx! {
        div { class: "navbar bg-base-100 rounded-box shadow mb-2",
            div { class: "navbar-start gap-2",
                button { class: "btn btn-sm", onclick: move |_| on_nav.call(Page::MasterQuestions), "問題設定" }
                button { class: "btn btn-sm", onclick: move |_| on_nav.call(Page::MasterStudents), "受験者設定" }
                button { class: "btn btn-sm", onclick: move |_| on_nav.call(Page::Grading), "採点" }
            }
            div { class: "navbar-end flex justify-end gap-2",
                button { class: "btn btn-sm", onclick: on_save_as, "Save as" }
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
                    "Load"
                }
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