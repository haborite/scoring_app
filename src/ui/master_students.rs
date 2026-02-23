use dioxus::prelude::*;
use crate::Page;
use crate::models::{Config, Student};
use crate::ui::SaveBar;

#[component]
pub fn MasterStudentsPage(
    on_nav: EventHandler<Page>,
    config: Signal<Config>,
) -> Element {

    rsx! {
        div { class: "p-2 space-y-2",

            SaveBar { config, on_nav }

            // import panel
            div { class: "card bg-base-100 shadow",
                div { class: "card-body p-2 space-y-3",

                    div { class: "flex flex-wrap items-center gap-2",
                        input {
                            id: "students_import",
                            r#type: "file",
                            accept: "application/json,.json",
                            class: "file-input file-input-bordered file-input-sm w-full max-w-md",
                        }
                        button {
                            class: "btn btn-sm btn-primary",
                            onclick: move |_| {
                                spawn(async move {
                                    let txt = match read_students_json_text().await {
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
                                    config.write().students = cfg.students;
                                });
                            },
                            "Import JSON"
                        }

                        div { class: "flex-1" }

                        button {
                            class: "btn btn-sm", 
                            onclick: move |_| {
                                let mut ss = config.read().students.clone();
                                let last_student = ss.last();
                                if let Some(student) = last_student {
                                    let prefix: String = student.id.chars().take_while(|c| !c.is_ascii_digit()).collect();
                                    let num_part: String = student.id.chars().skip_while(|c| !c.is_ascii_digit()).collect();
                                    let next_num = num_part.parse::<u32>().unwrap_or(0).saturating_add(1);
                                    let next_id = format!("{}{}", prefix, next_num);
                                    ss.push(Student {
                                        id: next_id,
                                        name: String::new(),
                                    });
                                } else {
                                    ss.push(Student {
                                        id: "S1".to_string(),
                                        name: String::new(),
                                    });
                                }
                                config.write().students = ss;
                            }, 
                            "Add row" 
                        }
                        button {
                            class: "btn btn-sm btn-ghost",
                            onclick: move |_| {
                                config.write().students = Vec::new();
                            },
                            "Clear"
                        }
                    }

                    p { class: "text-sm opacity-70",
                        "JSON format is the same as the normal config file. Only "
                        code { "students" }
                        " will be imported; "
                        code { "questions/scores" }
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
                                    th { class: "w-64", "id" }
                                    th { "name" }
                                    th { class: "w-24", "" }
                                }
                            }
                            tbody {
                                for (idx, _s) in config.read().students.iter().cloned().enumerate() {
                                    StudentRow { key: "{idx}", idx, config }
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
pub fn StudentRow(
    idx: usize,
    config: Signal<Config>,
) -> Element {
    // 表示用にその場で読む（クローンを作らない）
    let s = config.read().students.get(idx).cloned();

    // 既に消えていたら何も出さない（削除直後など）
    let Some(s) = s else {
        return rsx! {};
    };

    rsx! {
        tr {
            td {
                input {
                    class: "input input-bordered input-sm w-full",
                    value: "{s.id}",
                    oninput: move |ev| {
                        let v = ev.value();
                        config.write().students.get_mut(idx).unwrap().id = v;
                    }
                }
            }
            td {
                input {
                    class: "input input-bordered input-sm w-full",
                    value: "{s.name}",
                    oninput: move |ev| {
                        let v = ev.value();
                        config.write().students.get_mut(idx).unwrap().name = v;
                    }
                }
            }
            td { class: "text-right",
                button {
                    class: "btn btn-sm btn-ghost",
                    onclick: move |_| {
                        let mut config_write = config.write();
                        if idx < config_write.students.len() {
                            config_write.students.remove(idx);
                        }
                    },
                    "Del"
                }
            }
        }
    }
}

async fn read_students_json_text() -> Result<Option<String>, String> {
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