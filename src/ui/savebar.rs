use dioxus::prelude::*;
use std::path::PathBuf;

use crate::Page;
use crate::models::Config;

#[component]
pub fn SaveBar(
    config: Signal<Config>,
    on_nav: EventHandler<Page>,
) -> Element {

    let mut msg = use_signal(String::new);

    // -------------------------
    // Save（上書き保存）
    // -------------------------
    let on_save = {
        move |_| {
            let cfg = config.read().clone();

            let Some(path_str) = &cfg.save_path else {
                msg.set("No save path. Use 'Save as' first.".to_string());
                return;
            };

            let path = PathBuf::from(path_str);

            spawn(async move {
                match cfg.save_to_filepath(&path).await {
                    Ok(()) => msg.set("Saved.".to_string()),
                    Err(e) => msg.set(format!("Save failed: {e}")),
                }
            });
        }
    };

    // -------------------------
    // Save as
    // -------------------------
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

                match cfg.save_to_filepath(&path).await {
                    Ok(()) => {
                        let mut new_cfg = cfg.clone();
                        new_cfg.save_path = Some(path.to_string_lossy().to_string());
                        config.set(new_cfg);
                        msg.set("Saved.".to_string());
                    }
                    Err(e) => {
                        msg.set(format!("Save failed: {e}"));
                    }
                }
            });
        }
    };

    // -------------------------
    // Load
    // -------------------------
    let on_load = {
        move |_| {
            spawn(async move {

                let handle = rfd::AsyncFileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
                    .await;

                let Some(handle) = handle else {
                    return;
                };

                let path = handle.path().to_path_buf();

                let bytes = handle.read().await;

                let txt = match String::from_utf8(bytes) {
                    Ok(s) => s,
                    Err(e) => {
                        msg.set(format!("Invalid UTF-8: {e}"));
                        return;
                    }
                };

                if txt.trim().is_empty() {
                    msg.set("Empty file.".to_string());
                    return;
                }

                match serde_json::from_str::<Config>(&txt) {
                    Ok(mut cfg) => {
                        cfg.save_path = Some(path.to_string_lossy().to_string());
                        config.set(cfg);
                        msg.set("Loaded.".to_string());
                    }
                    Err(e) => {
                        msg.set(format_json_error(&txt, e));
                    }
                }
            });
        }
    };

    rsx! {
        div { class: "navbar bg-base-100 rounded-box shadow mb-2",

            div { class: "navbar-start gap-2",
                button { class: "btn btn-sm", onclick: move |_| on_nav.call(Page::MasterQuestions), "問題設定" }
                button { class: "btn btn-sm", onclick: move |_| on_nav.call(Page::MasterStudents), "受験者設定" }
                button { class: "btn btn-sm", onclick: move |_| on_nav.call(Page::Rating), "成績評価" }
                button { class: "btn btn-sm", onclick: move |_| on_nav.call(Page::Grading), "採点" }
            }

            div { class: "navbar-center gap-2",
                {(!msg().is_empty()).then(|| rsx!{
                    div { class: "alert alert-info mb-2 py-2",
                        span { class: "text-sm", "{msg()}" }
                    }
                })}
            }

            div { class: "navbar-end gap-2",
                button { class: "btn btn-sm btn-primary", onclick: on_save, "Save" }
                button { class: "btn btn-sm", onclick: on_save_as, "Save as" }
                button { class: "btn btn-sm", onclick: on_load, "Load" }
            }
        }
    }
}

fn format_json_error(src: &str, e: serde_json::Error) -> String {
    let (line, col) = (e.line(), e.column());

    let kind = if e.is_syntax() {
        "JSON syntax error"
    } else if e.is_data() {
        "JSON structure/type mismatch"
    } else if e.is_eof() {
        "Unexpected end of file"
    } else {
        "JSON parse error"
    };

    let line_text = src.lines().nth(line.saturating_sub(1)).unwrap_or("");
    let caret_pad = " ".repeat(col.saturating_sub(1));
    let caret = format!("{caret_pad}^");

    let hint = if e.is_data() {
        "Hint: field name misspelled? wrong type? missing field?"
    } else {
        ""
    };

    if line > 0 {
        format!(
            "{kind} at line {line}, column {col}: {e}\n\
             {line_text}\n\
             {caret}\n\
             {hint}"
        )
    } else {
        format!("{kind}: {e}\n{hint}")
    }
}