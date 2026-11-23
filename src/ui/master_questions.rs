use dioxus::prelude::*;
use crate::models::{bump_data_gen, Config, Part, Question, DATA_GEN};
use crate::settings::SETTINGS;

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
                            class: "btn btn-sm",
                            title: "問題だけを JSON ファイルに保存します",
                            onclick: move |_| Config::save_part(config, Part::Questions, msg),
                            "保存"
                        }
                        button {
                            class: "btn btn-sm",
                            title: "保存ファイルから問題だけを読み込みます（全体の保存ファイルからも読み込めます）",
                            onclick: move |_| Config::load_part(config, Part::Questions, msg),
                            "読み込み"
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
                                    full_score: SETTINGS.peek().default_full_score,
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
                        "保存 / 読み込み: 問題一覧だけを JSON ファイル（"
                        code { "questions" }
                        " だけを含む、全体の保存ファイルと同じ形式）でやり取りします。読み込むと問題一覧を置き換えます（点数は問題 id で対応付けられます）。"
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
                                    th { class: "w-32 text-right", "full_score" }
                                    th { class: "w-32", "weight" }
                                    th { "comment" }
                                    th { class: "w-24", "" }
                                }
                            }
                            tbody {
                                for (idx, _q) in config.read().questions.iter().cloned().enumerate() {
                                    QuestionRow { key: "{DATA_GEN()}-{idx}", idx, config, msg }
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
                    // Not controlled while typing (rows are re-created when the list is replaced)
                    initial_value: "{q.name}",
                    oninput: move |ev| {
                        let v = ev.value();
                        if let Some(q) = config.write().questions.get_mut(idx) { q.name = v; }
                    }
                }
            }
            td {
                input {
                    class: "input input-bordered input-sm w-28 text-right tabular-nums",
                    r#type: "number",
                    min: 0,
                    initial_value: "{q.full_score}",
                    oninput: move |ev| {
                        // An empty or partial entry keeps the last full score (no flicker of regrade marks)
                        let Ok(v) = ev.value().trim().parse::<u32>() else { return };
                        if let Some(q) = config.write().questions.get_mut(idx) { q.full_score = v; }
                    }
                }
            }
            td {
                input {
                    class: "input input-bordered input-sm w-28",
                    r#type: "number",
                    step: "0.01",
                    initial_value: "{q.weight}",
                    oninput: move |ev| {
                        let v = ev.value().parse::<f32>().unwrap_or(0.0);
                        if let Some(q) = config.write().questions.get_mut(idx) { q.weight = v; }
                    }
                }
            }
            td {
                input {
                    class: "input input-bordered input-sm w-full",
                    initial_value: "{q.comment}",
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
                            drop(config_write);
                            // Rows below move up: re-create them with their new values
                            bump_data_gen();
                        }
                    },
                    "Del"
                }
            }
        }
    }
}
