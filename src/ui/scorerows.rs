use dioxus::prelude::*;
use crate::models::Config;
use crate::ui::ScoreRow;

#[component]
pub fn ScoreRows(
    cur_student_idx: Signal<usize>,
    cur_question_id: Signal<Option<u32>>,
    config: Signal<Config>,
    focus_idx: Signal<usize>,
    search_open: Signal<bool>,
) -> Element {
    rsx! {
        div { class: "card bg-base-100 shadow",
            div { class: "card-body",
                // global hotkeys: F opens search
                onkeydown: move |e| {
                    if let Key::Character(chr) = e.key() {
                        match chr.as_str() {
                            "f" | "F" => {
                                e.prevent_default();
                                search_open.set(true);
                            }
                            "j" | "J" => {
                                e.prevent_default();
                                mv_prev_student(cur_student_idx);
                            }
                            "l" | "L" => {
                                e.prevent_default();
                                mv_next_student(cur_student_idx, config().students.len());
                            }
                            _ => {}
                        }
                    } else {
                        match e.key() {
                            Key::Escape => {
                                e.prevent_default();
                                search_open.set(false);
                            }
                            _ => {}
                        }
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
                    div { class: "space-y-1",
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

fn mv_prev_student(
    mut cur_student_idx: Signal<usize>
) {
    let current_idx = cur_student_idx();
    let idx = current_idx.saturating_sub(1);
    cur_student_idx.set(idx);
}

fn mv_next_student(
    mut cur_student_idx: Signal<usize>, 
    student_count: usize
) {
    let current_idx = cur_student_idx();
    let max_idx = student_count.saturating_sub(1);
    let idx = std::cmp::min(current_idx + 1, max_idx);
    cur_student_idx.set(idx);
}


