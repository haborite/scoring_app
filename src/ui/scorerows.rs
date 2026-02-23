use dioxus::prelude::*;
use crate::models::Config;
use crate::ui::ScoreRow;

const TWO_COL_THRESHOLD: usize = 12;

#[component]
pub fn ScoreRows(
    cur_student_idx: Signal<usize>,
    cur_question_id: Signal<Option<u32>>,
    config: Signal<Config>,
    focus_idx: Signal<usize>,
    search_open: Signal<bool>,
) -> Element {
    let questions = config.read().questions.clone();
    let qlen = questions.len();
    let two_col = qlen >= TWO_COL_THRESHOLD;
    let mid = (qlen + 1) / 2;

    rsx! {
        div { class: "card bg-base-100 shadow",
            div { class: "card-body",
                // global hotkeys: F opens search
                onkeydown: move |e| {
                    match e.code() {
                        Code::KeyF | Code::NumpadDecimal => {
                            e.prevent_default();
                            search_open.set(true);
                        }
                        Code::KeyJ | Code::NumpadDivide => {
                            e.prevent_default();
                            mv_prev_student(cur_student_idx);
                        }
                        Code::KeyL | Code::NumpadMultiply => {
                            e.prevent_default();
                            mv_next_student(cur_student_idx, config().students.len());
                        }
                        Code::Escape => {
                            e.prevent_default();
                            search_open.set(false);
                        }
                        _ => {}
                    }
                },

                div { class: "navbar",
                    button {
                        class: "btn max-w-xs",
                        onclick: move |_| { mv_prev_student(cur_student_idx) },
                        "←"
                    }

                    div { class: "text-lg font-bold text-center flex-1 h-min",
                        "{cur_student_label(config, cur_student_idx)}"
                    }

                    button {
                        class: "btn max-w-xs",
                        onclick: move |_| {
                            mv_next_student(cur_student_idx, config().students.len())
                        },
                        "→"
                    }
                }

                if config.read().students.is_empty() {
                    div { class: "alert", "受験者が未登録です" }
                } else if config.read().questions.is_empty() {
                    div { class: "alert", "問題が未登録です" }
                } else {
                    div { class: if two_col { "grid grid-cols-2 gap-x-4 gap-y-1" } else { "space-y-1" },
                        if two_col {
                            for i in 0..qlen {
                                {
                                    let row = i / 2;
                                    let col = i % 2;
                                    let original_idx = if col == 0 { row } else { row + mid };
                                    let question = &questions[original_idx];
                                        rsx! {
                                        ScoreRow {
                                            key: "row-{original_idx}",
                                            question_id: question.id,
                                            cur_question_id,
                                            cur_student_idx,
                                            qidx: original_idx,
                                            config,
                                            is_focused: focus_idx() == original_idx,
                                            move_to_next: move |_| {
                                                let last = qlen.saturating_sub(1);
                                                focus_idx.set(std::cmp::min(original_idx + 1, last));
                                            },
                                            move_to_prev: move |_| {
                                                focus_idx.set(original_idx.saturating_sub(1));
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            for (qidx, question) in questions.iter().enumerate() {
                                ScoreRow {
                                    key: "row-{qidx}",
                                    question_id: question.id,
                                    cur_question_id,
                                    cur_student_idx,
                                    qidx,
                                    config,
                                    is_focused: focus_idx() == qidx,
                                    move_to_next: move |_| {
                                        let last = qlen.saturating_sub(1);
                                        focus_idx.set(std::cmp::min(qidx + 1, last));
                                    },
                                    move_to_prev: move |_| {
                                        focus_idx.set(qidx.saturating_sub(1));
                                    }
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

fn mv_prev_student(mut cur_student_idx: Signal<usize>) {
    let current_idx = cur_student_idx();
    let idx = current_idx.saturating_sub(1);
    cur_student_idx.set(idx);
}

fn mv_next_student(mut cur_student_idx: Signal<usize>, student_count: usize) {
    let current_idx = cur_student_idx();
    let max_idx = student_count.saturating_sub(1);
    let idx = std::cmp::min(current_idx + 1, max_idx);
    cur_student_idx.set(idx);
}