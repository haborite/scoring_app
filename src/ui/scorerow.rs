use dioxus::prelude::*;
use crate::models::Config;
use crate::ui::{GradingState, Section};

#[component]
pub fn ScoreRow(
    student_id: String,
    question_id: u32,
    qidx: usize,
    state: GradingState,
    config: Signal<Config>,
) -> Element {
    let GradingState { mut focus_idx, mut section, .. } = state;

    let cfg = config.read();
    let question = cfg.questions.iter().find(|q| q.id == question_id).cloned();
    let score = cfg.scores
        .iter()
        .find(|sc| sc.question_id == question_id && sc.student_id == student_id)
        .cloned();
    drop(cfg);
    // The input is not controlled: Rust only gives the value it starts with
    let initial = use_hook(|| score.as_ref().and_then(|s| s.score).map(|v| v.to_string()).unwrap_or_default());
    let Some(question) = question else {
        return rsx! {};
    };
    let full = question.full_score;
    // (full score at grading time, may be confirmed as it is)
    let regrade = score.as_ref().filter(|s| s.needs_regrade(full)).map(|s| {
        (s.graded_full_score, s.score.is_some_and(|v| v <= full))
    });

    let sid = student_id.clone();
    rsx! {
        label {
            class: if regrade.is_some() {
                "grid grid-cols-[minmax(4rem,8rem)_5rem_auto_1fr] gap-2 items-center rounded px-1 py-0.5 break-inside-avoid bg-warning/20 focus-within:bg-warning/35"
            } else {
                "grid grid-cols-[minmax(4rem,8rem)_5rem_auto_1fr] gap-2 items-center rounded px-1 py-0.5 break-inside-avoid focus-within:bg-base-200"
            },
            div { class: "font-semibold truncate", title: "{question.name}", "{question.name}" }
            input {
                id: "score-{qidx}",
                // Lets the key listener apply the score-input bindings here
                "data-score-input": "true",
                r#type: "number",
                initial_value: initial,
                min: 0,
                max: full,
                class: "input input-sm validator text-right tabular-nums",

                oninput: move |e| {
                    if student_id.is_empty() {
                        return;
                    }
                    // Out-of-range values are not stored (the field is shown as invalid)
                    let score = e.value().trim().parse::<u32>().ok().filter(|n| *n <= full);
                    config.write().set_score(&student_id, question_id, score);
                },
                onfocus: move |_| {
                    if *section.peek() != Section::Scores {
                        section.set(Section::Scores);
                    }
                    if *focus_idx.peek() != qidx {
                        focus_idx.set(qidx);
                    }
                }
            }
            div { class: "text-sm opacity-60 tabular-nums", "/ {full}" }
            div { class: "flex items-center gap-2 min-w-0",
                if let Some((graded, can_confirm)) = regrade {
                    span { class: "text-xs text-warning-content truncate",
                        match graded {
                            Some(g) if g != full => rsx! { "⚠ 満点 {g}→{full}" },
                            _ => rsx! { "⚠ 満点超過" },
                        }
                    }
                    button {
                        class: "btn btn-xs",
                        disabled: !can_confirm,
                        title: if can_confirm { "点数を変えずに現在の満点で確定する" } else { "満点を超えているため点数を直してください" },
                        onclick: move |e| {
                            e.prevent_default();
                            config.write().confirm_score(&sid, question_id);
                        },
                        "確認済み"
                    }
                }
            }
        }
    }
}
