use dioxus::prelude::*;
use crate::models::{Config, DATA_GEN};
use crate::ui::{GradingState, ScoreRow};

// From this many questions the inputs go into two columns (one when the pane is too narrow)
const TWO_COL_THRESHOLD: usize = 12;

#[component]
pub fn ScoreRows(
    config: Signal<Config>,
    state: GradingState,
) -> Element {
    let mut cur_student_idx = state.cur_student_idx;
    let gen = DATA_GEN();
    let cfg = config.read();
    let qlen = cfg.questions.len();
    let student_count = cfg.students.len();
    let idx = cur_student_idx();
    let student = cfg.students.get(idx).cloned();
    let question_ids: Vec<u32> = cfg.questions.iter().map(|q| q.id).collect();

    // (text, badge class) shown under the student's name
    let badges: Vec<(String, &str)> = match &student {
        Some(s) => {
            let score_map = cfg.score_map();
            let regrade = cfg.regrade_map();
            let missing = cfg.questions.iter()
                .filter(|q| score_map.get(&(s.id.as_str(), q.id)).copied().flatten().is_none())
                .count();
            let flagged = cfg.questions.iter()
                .filter(|q| regrade.contains_key(&(s.id.as_str(), q.id)))
                .count();
            let mut b = Vec::new();
            if flagged > 0 {
                b.push((format!("⚠ 要再採点 {flagged} 問（集計対象外）"), "badge badge-warning"));
            }
            if missing > 0 {
                b.push((format!("未入力 {missing} 問"), "badge badge-ghost"));
            } else if flagged == 0 {
                if let Some(f) = cfg.final_score(&score_map, &s.id) {
                    b.push((format!("最終得点 {f}"), "badge badge-success"));
                }
            }
            b
        }
        None => Vec::new(),
    };
    drop(cfg);

    rsx! {
        div { class: "card bg-base-100 shadow h-full",
            div { class: "card-body p-4 gap-2 min-h-0",
                div { class: "flex items-center gap-2",
                    button {
                        class: "btn btn-square btn-sm",
                        title: "前の受験者",
                        disabled: idx == 0,
                        onclick: move |_| cur_student_idx.set(idx.saturating_sub(1)),
                        "←"
                    }

                    div { class: "flex-1 text-center min-w-0",
                        match &student {
                            Some(s) => rsx! {
                                div { class: "text-lg font-bold truncate",
                                    if !badges.is_empty() && badges[0].1.contains("warning") {
                                        span { class: "text-warning mr-2", title: "要再採点", "⚠" }
                                    }
                                    span { class: "font-mono", "{s.id}" }
                                    span { class: "ml-3", "{s.name}" }
                                }
                                div { class: "flex flex-wrap justify-center items-center gap-2 text-sm",
                                    span { class: "opacity-80 tabular-nums", "{idx + 1} / {student_count}" }
                                    for (text, cls) in badges.iter() {
                                        span { class: "{cls}", "{text}" }
                                    }
                                }
                            },
                            None => rsx! { div { class: "text-lg font-bold", "No student" } },
                        }
                    }

                    button {
                        class: "btn btn-square btn-sm",
                        title: "次の受験者",
                        disabled: idx + 1 >= student_count,
                        onclick: move |_| cur_student_idx.set((idx + 1).min(student_count.saturating_sub(1))),
                        "→"
                    }
                }

                div { class: "flex-1 min-h-0 overflow-y-auto",
                    match &student {
                        None => rsx! { div { class: "alert", "受験者が未登録です" } },
                        Some(_) if qlen == 0 => rsx! { div { class: "alert", "問題が未登録です" } },
                        Some(s) => rsx! {
                            // Columns fill top to bottom, so DOM (and Tab) order is the question order
                            div {
                                style: if qlen >= TWO_COL_THRESHOLD { "columns: 2 24rem; column-gap: 1rem;" } else { "" },
                                for (qidx, question_id) in question_ids.into_iter().enumerate() {
                                    // Re-created per student, so a typed value never leaks to the next one
                                    ScoreRow {
                                        key: "{gen}-{s.id}-{question_id}",
                                        student_id: s.id.clone(),
                                        question_id,
                                        qidx,
                                        state,
                                        config,
                                    }
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}
