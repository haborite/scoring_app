use dioxus::prelude::*;
use crate::models::Config;
use crate::ui::scorerow::ScoreRow;

#[component]
pub fn ScoreRows(
    cur_student_idx: Signal<usize>,
    cur_question_idx: Signal<usize>,
    config: Signal<Config>,
) -> Element {
    rsx! {
        div { class: "card bg-base-100 shadow",
            div { class: "card-body",
                div { class: "card-title", "採点入力" }

                if config.read().students.is_empty() {
                    div { class: "alert", "受験者が未登録です" }
                } else if config.read().questions.is_empty() {
                    div { class: "alert", "問題が未登録です" }
                } else {
                    div { class: "space-y-2",
                        for (qidx, question) in config.read().questions.iter().enumerate() {
                            ScoreRow {
                                key: "row-{cur_student_idx()}-{qidx}",
                                question_id: question.id,
                                cur_student_idx,
                                cur_question_idx,
                                config,
                            }
                        }
                    }
                }
            }
        }
    }
}
