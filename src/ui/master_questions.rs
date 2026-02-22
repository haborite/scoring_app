use dioxus::prelude::*;
use crate::models::{Config, Question};

#[component]
pub fn MasterQuestionsPage(
    on_back: EventHandler<()>,
    config: Signal<Config>,
) -> Element {

    rsx! {
        div { class: "p-4 space-y-4",

            div { class: "flex items-center gap-2",
                button {
                    class: "btn btn-sm",
                    onclick: move |_| on_back.call(()),
                    "Back"
                }
                h2 { class: "text-lg font-bold", "Import / Edit Questions" }
            }

            // import panel
            div { class: "card bg-base-100 shadow",
                div { class: "card-body p-4 space-y-3",

                    div { class: "flex flex-wrap items-center gap-2",
                        input {
                            id: "questions_import",
                            r#type: "file",
                            accept: "application/json,.json",
                            class: "file-input file-input-bordered file-input-sm w-full max-w-md",
                        }
                        button {
                            class: "btn btn-sm btn-primary",
                            onclick: move |_| {
                                spawn(async move {
                                    let txt = match read_questions_json_text().await {
                                        Ok(Some(s)) => s,
                                        Ok(None) => return, // canceled / no file
                                        Err(_) => return,
                                    };
                                    if txt.trim().is_empty() {
                                        return;
                                    }

                                    // 通常の Config 形式として読むが questions だけ使う
                                    let parsed: Result<Config, _> = serde_json::from_str(&txt);
                                    let Ok(cfg) = parsed else {
                                        return;
                                    };

                                    // Signal<Vec<Question>> を差し替え
                                    config.write().questions = cfg.questions;
                                });
                            },
                            "Import JSON"
                        }

                        div { class: "flex-1" }

                        button { 
                            class: "btn btn-sm", 
                            onclick: move |_| {
                                // 追加：max(id)+1
                                let mut qs = config.read().questions.clone();
                                let next_id = qs.iter().map(|q| q.id).max().unwrap_or(0).saturating_add(1);

                                qs.push(Question {
                                    id: next_id,
                                    name: String::new(),
                                    full_score: 0,
                                    weight: 1.0,
                                    comment: String::new(),
                                });

                                config.write().questions = qs;
                            }, 
                            "Add row" 
                        }
                        button { 
                            class: "btn btn-sm btn-ghost", 
                            onclick: move |_| {
                                config.write().questions = Vec::new();
                            }, 
                            "Clear" 
                        }
                    }

                    p { class: "text-sm opacity-70",
                        "JSON format is the same as the normal config file. Only "
                        code { "questions" }
                        " will be imported; "
                        code { "students/scores" }
                        " are ignored."
                    }
                }
            }

            // table
            div { class: "card bg-base-100 shadow",
                div { class: "card-body p-2",
                    div { class: "overflow-x-auto",
                        table { class: "table table-zebra w-full",
                            thead {
                                tr {
                                    th { class: "w-24", "id" }
                                    th { class: "w-64", "name" }
                                    th { class: "w-32", "full_score" }
                                    th { class: "w-32", "weight" }
                                    th { "comment" }
                                    th { class: "w-24", "" }
                                }
                            }
                            tbody {
                                for (idx, _q) in config.read().questions.iter().cloned().enumerate() {
                                    QuestionRow { key: "{idx}", idx, config }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn QuestionRow(
    idx: usize,
    config: Signal<Config>,
) -> Element {
    let q = config.read().questions.get(idx).cloned();
    let Some(q) = q else {
        return rsx! {};
    };

    rsx! {
        tr {
            td {
                input {
                    class: "input input-bordered input-sm w-20",
                    r#type: "number",
                    value: "{q.id}",
                    oninput: move |ev| {
                        let v = ev.value().parse::<u32>().unwrap_or(0);
                        config.write().questions.get_mut(idx).unwrap().id = v;
                    }
                }
            }
            td {
                input {
                    class: "input input-bordered input-sm w-full",
                    value: "{q.name}",
                    oninput: move |ev| {
                        let v = ev.value();
                        config.write().questions.get_mut(idx).unwrap().name = v;
                    }
                }
            }
            td {
                input {
                    class: "input input-bordered input-sm w-28",
                    r#type: "number",
                    value: "{q.full_score}",
                    oninput: move |ev| {
                        let v = ev.value().parse::<u32>().unwrap_or(0);
                        config.write().questions.get_mut(idx).unwrap().full_score = v;
                    }
                }
            }
            td {
                input {
                    class: "input input-bordered input-sm w-28",
                    r#type: "number",
                    step: "0.01",
                    value: "{q.weight}",
                    oninput: move |ev| {
                        let v = ev.value().parse::<f32>().unwrap_or(0.0);
                        config.write().questions.get_mut(idx).unwrap().weight = v;
                    }
                }
            }
            td {
                input {
                    class: "input input-bordered input-sm w-full",
                    value: "{q.comment}",
                    oninput: move |ev| {
                        let v = ev.value();
                        config.write().questions.get_mut(idx).unwrap().comment = v;
                    }
                }
            }
            td { class: "text-right",
                button {
                    class: "btn btn-sm btn-ghost",
                    onclick: move |_| {
                        let mut config_write = config.write();
                        if idx < config_write.questions.len() {
                            config_write.questions.remove(idx);
                        }
                    },
                    "Del"
                }
            }
        }
    }
}

async fn read_questions_json_text() -> Result<Option<String>, String> {
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