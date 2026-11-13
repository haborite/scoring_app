use dioxus::prelude::*;
use crate::Page;
use crate::models::{Config, Student};
use crate::ui::TopBar;

#[component]
pub fn MasterStudentsPage(
    on_nav: EventHandler<Page>,
    config: Signal<Config>,
) -> Element {

    let mut msg = use_signal(String::new);

    rsx! {
        div { class: "p-2 space-y-2",

            TopBar { config, on_nav }

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
                                    // 通常の Config 形式として読むが students だけ使う
                                    match Config::pick_file().await {
                                        Ok(Some((_, cfg))) => {
                                            msg.set(format!("{} 人をインポートしました", cfg.students.len()));
                                            config.write().students = cfg.students;
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
                                let mut ss = config.read().students.clone();
                                let next_id = ss.last()
                                    .map(|s| next_student_id(&s.id))
                                    .unwrap_or_else(|| "S1".to_string());
                                ss.push(Student {
                                    id: next_id,
                                    name: String::new(),
                                });
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
                                    StudentRow { key: "{idx}", idx, config, msg }
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
    msg: Signal<String>,
) -> Element {
    // 表示用にその場で読む（クローンを作らない）
    let s = config.read().students.get(idx).cloned();

    // 既に消えていたら何も出さない（削除直後など）
    let Some(s) = s else {
        return rsx! {};
    };
    let old_id = s.id.clone();

    rsx! {
        tr {
            td {
                input {
                    id: "student-id-{idx}",
                    class: "input input-bordered input-sm w-full",
                    value: "{s.id}",
                    // Applied on commit (not per keystroke) so scores can be carried over safely
                    onchange: move |ev| {
                        let result = config.write().rename_student(idx, &ev.value());
                        match result {
                            Ok(()) => msg.set(String::new()),
                            Err(e) => {
                                msg.set(e);
                                let old = serde_json::to_string(&old_id).unwrap_or_default();
                                let _ = document::eval(&format!(
                                    r#"document.getElementById("student-id-{idx}").value = {old};"#
                                ));
                            }
                        }
                    }
                }
            }
            td {
                input {
                    class: "input input-bordered input-sm w-full",
                    value: "{s.name}",
                    oninput: move |ev| {
                        let v = ev.value();
                        if let Some(s) = config.write().students.get_mut(idx) { s.name = v; }
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

// "A009" -> "A010", "S1" -> "S2" (keeps the prefix and zero padding)
fn next_student_id(id: &str) -> String {
    let prefix: String = id.chars().take_while(|c| !c.is_ascii_digit()).collect();
    let digits: String = id.chars().skip(prefix.chars().count()).take_while(|c| c.is_ascii_digit()).collect();
    let next = digits.parse::<u64>().unwrap_or(0).saturating_add(1);
    format!("{prefix}{next:0width$}", width = digits.len())
}

#[cfg(test)]
mod tests {
    use super::next_student_id;

    #[test]
    fn keeps_prefix_and_padding() {
        assert_eq!(next_student_id("A001"), "A002");
        assert_eq!(next_student_id("A009"), "A010");
        assert_eq!(next_student_id("S1"), "S2");
        assert_eq!(next_student_id("A99"), "A100");
        assert_eq!(next_student_id("ABC"), "ABC1");
    }
}
