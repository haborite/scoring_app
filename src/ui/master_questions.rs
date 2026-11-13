use dioxus::prelude::*;
use crate::models::{Config, Question};

#[component]
pub fn MasterQuestionsPage(
    config: Signal<Config>,
) -> Element {

    let mut msg = use_signal(String::new);

    rsx! {
        div { class: "space-y-2",

            {(!msg().is_empty()).then(|| rsx! {
                div { class: "alert alert-warning py-2 whitespace-pre-wrap font-mono text-sm",
                    span { class: "flex-1", "{msg}" }
                    button { class: "btn btn-xs btn-ghost", onclick: move |_| msg.set(String::new()), "✕" }
                }
            })}

            // import panel
            div { class: "card bg-base-100 shadow",
                div { class: "card-body p-2 space-y-3",

                    div { class: "flex flex-wrap items-center gap-2",
                        button {
                            class: "btn btn-sm btn-primary",
                            onclick: move |_| {
                                spawn(async move {
                                    // 通常の Config 形式として読むが questions だけ使う
                                    match Config::pick_file().await {
                                        Ok(Some((_, cfg))) => {
                                            msg.set(format!("{} 問をインポートしました", cfg.questions.len()));
                                            config.write().questions = cfg.questions;
                                        }
                                        Ok(None) => {}
                                        Err(e) => msg.set(e),
                                    }
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
                                    name: format!("問{next_id}"),
                                    full_score: 10,
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
                                    QuestionRow { key: "{idx}", idx, config, msg }
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
    msg: Signal<String>,
) -> Element {
    let q = config.read().questions.get(idx).cloned();
    let Some(q) = q else {
        return rsx! {};
    };

    rsx! {
        tr {
            td {
                input {
                    id: "question-id-{idx}",
                    class: "input input-bordered input-sm w-20",
                    r#type: "number",
                    min: 0,
                    value: "{q.id}",
                    // Applied on commit (not per keystroke) so scores can be carried over safely
                    onchange: move |ev| {
                        let result = match ev.value().parse::<u32>() {
                            Ok(v) => config.write().rename_question(idx, v),
                            Err(_) => Err("問題 id には 0 以上の整数を入力してください".to_string()),
                        };
                        match result {
                            Ok(()) => msg.set(String::new()),
                            Err(e) => {
                                msg.set(e);
                                let _ = document::eval(&format!(
                                    r#"document.getElementById("question-id-{idx}").value = "{}";"#,
                                    q.id
                                ));
                            }
                        }
                    }
                }
            }
            td {
                input {
                    class: "input input-bordered input-sm w-full",
                    value: "{q.name}",
                    oninput: move |ev| {
                        let v = ev.value();
                        if let Some(q) = config.write().questions.get_mut(idx) { q.name = v; }
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
                        if let Some(q) = config.write().questions.get_mut(idx) { q.full_score = v; }
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
                        if let Some(q) = config.write().questions.get_mut(idx) { q.weight = v; }
                    }
                }
            }
            td {
                input {
                    class: "input input-bordered input-sm w-full",
                    value: "{q.comment}",
                    oninput: move |ev| {
                        let v = ev.value();
                        if let Some(q) = config.write().questions.get_mut(idx) { q.comment = v; }
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
