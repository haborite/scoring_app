use dioxus::prelude::*;
use crate::models::Config;
use crate::ui::ScoreRow;

#[component]
pub fn ScoreRows(
    cur_student_idx: Signal<usize>,
    cur_question_id: Signal<Option<u32>>,
    config: Signal<Config>,
    focus_idx: Signal<usize>,
) -> Element {
    rsx! {
        div { class: "card bg-base-100 shadow",
            div { class: "card-body",
                div { class: "card-title", "{cur_student_label(config, cur_student_idx)}" }

                if config.read().students.is_empty() {
                    div { class: "alert", "受験者が未登録です" }
                } else if config.read().questions.is_empty() {
                    div { class: "alert", "問題が未登録です" }
                } else {
                    div { class: "space-y-2",
                        for (qidx, question) in config.read().questions.iter().enumerate() {
                            ScoreRow {
                                key: "row-{qidx}",
                                question_id: question.id,
                                cur_question_id,
                                cur_student_idx,
                                qidx,
                                config,
                                is_focused: focus_idx() == qidx,
                                move_to_next: move |_| {
                                    let idx = std::cmp::min(qidx + 1, config.read().questions.len());
                                    focus_idx.set(idx);
                                },
                                move_to_prev: move |_| {
                                    // let idx = if qidx <= 0 {0} else {qidx - 1};
                                    let idx = qidx.saturating_sub(1);
                                    focus_idx.set(idx);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn cur_student_label(
    config: Signal<Config>,
    cur_student_idx: Signal<usize>,
) -> String {
    config().students
        .get(cur_student_idx())
        .map(|s| format!("{} {}", s.id, s.name))
        .unwrap_or_else(|| "No student".to_string())
}

