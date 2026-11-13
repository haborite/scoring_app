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
    let cfg = config.read();
    let qlen = cfg.questions.len();
    let student_count = cfg.students.len();
    let idx = cur_student_idx();
    let student = cfg.students.get(idx).cloned();

    // Two columns are filled top to bottom: 0, mid, 1, mid+1, ...
    let two_col = qlen >= TWO_COL_THRESHOLD;
    let mid = qlen.div_ceil(2);
    let order: Vec<usize> = if two_col {
        (0..qlen).map(|i| if i % 2 == 0 { i / 2 } else { i / 2 + mid }).collect()
    } else {
        (0..qlen).collect()
    };
    let rows: Vec<(usize, u32)> = order.into_iter().map(|q| (q, cfg.questions[q].id)).collect();

    let progress = student.as_ref().map(|s| {
        let score_map = cfg.score_map();
        let missing = cfg.questions.iter()
            .filter(|q| score_map.get(&(s.id.as_str(), q.id)).copied().flatten().is_none())
            .count();
        match cfg.final_score(&score_map, &s.id) {
            Some(f) => (format!("最終得点 {f}"), "badge badge-success"),
            None => (format!("未入力 {missing} 問"), "badge badge-ghost"),
        }
    });
    drop(cfg);

    rsx! {
        div { class: "card bg-base-100 shadow",
            div { class: "card-body",
                div { class: "flex items-center gap-2",
                    button {
                        class: "btn btn-square",
                        title: "前の受験者",
                        disabled: idx == 0,
                        onclick: move |_| cur_student_idx.set(idx.saturating_sub(1)),
                        "←"
                    }

                    div { class: "flex-1 text-center",
                        match &student {
                            Some(s) => rsx! {
                                div { class: "text-lg font-bold",
                                    span { class: "font-mono", "{s.id}" }
                                    span { class: "ml-3", "{s.name}" }
                                }
                                div { class: "flex justify-center items-center gap-2 text-sm opacity-80",
                                    span { "{idx + 1} / {student_count}" }
                                    if let Some((text, cls)) = &progress {
                                        span { class: "{cls}", "{text}" }
                                    }
                                }
                            },
                            None => rsx! { div { class: "text-lg font-bold", "No student" } },
                        }
                    }

                    button {
                        class: "btn btn-square",
                        title: "次の受験者",
                        disabled: idx + 1 >= student_count,
                        onclick: move |_| cur_student_idx.set((idx + 1).min(student_count.saturating_sub(1))),
                        "→"
                    }
                }

                if student_count == 0 {
                    div { class: "alert", "受験者が未登録です" }
                } else if qlen == 0 {
                    div { class: "alert", "問題が未登録です" }
                } else {
                    div { class: if two_col { "grid grid-cols-2 gap-x-4 gap-y-1" } else { "space-y-1" },
                        for (qidx, question_id) in rows {
                            ScoreRow {
                                key: "row-{qidx}",
                                question_id,
                                qidx,
                                qlen,
                                cur_question_id,
                                cur_student_idx,
                                focus_idx,
                                search_open,
                                config,
                            }
                        }
                    }
                }
            }
        }
    }
}
