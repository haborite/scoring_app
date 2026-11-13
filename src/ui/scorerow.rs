use dioxus::prelude::*;
use crate::models::{Config, Score};

#[component]
pub fn ScoreRow(
    question_id: u32,
    qidx: usize,
    qlen: usize,
    cur_question_id: Signal<Option<u32>>,
    cur_student_idx: Signal<usize>,
    focus_idx: Signal<usize>,
    search_open: Signal<bool>,
    config: Signal<Config>,
) -> Element {

    // Text of an out-of-range entry, kept so the field is not cleared while it is not stored
    let mut invalid_text = use_signal(|| None::<(String, String)>);

    let cfg = config.read();
    let student_id = cfg.students.get(cur_student_idx()).map(|s| s.id.clone()).unwrap_or_default();
    let Some(question) = cfg.questions.iter().find(|q| q.id == question_id).cloned() else {
        return rsx! {};
    };
    let student_count = cfg.students.len();
    let stored = cfg.scores
        .iter()
        .find(|sc| sc.question_id == question_id && sc.student_id == student_id)
        .and_then(|sc| sc.score)
        .map(|scv| scv.to_string())
        // Always pass a string: None would leave the previous student's value in the DOM
        .unwrap_or_default();
    drop(cfg);
    let value = match &*invalid_text.read() {
        Some((sid, text)) if *sid == student_id => text.clone(),
        _ => stored,
    };
    let full = question.full_score;

    rsx! {
        label {
            class: "grid grid-cols-[1fr_auto_auto] md:grid-cols-[8rem_6rem_auto_1fr] gap-2 items-center rounded px-1 focus-within:bg-base-200",
            div { class: "font-semibold truncate", title: "{question.name}", "{question.name}" }
            input {
                id: "score-{qidx}",
                r#type: "number",
                value,
                min: 0,
                max: full,
                class: "input validator",

                oninput: move |e| {
                    if student_id.is_empty() {
                        return;
                    }
                    // Out-of-range values are not stored (the field is shown as invalid)
                    let text = e.value();
                    let score = text.trim().parse::<u32>().ok().filter(|n| *n <= full);
                    let invalid = score.is_none() && !text.trim().is_empty();
                    invalid_text.set(invalid.then(|| (student_id.clone(), text)));
                    let mut cfg = config.write();
                    match cfg.scores.iter_mut()
                        .find(|sc| sc.question_id == question_id && sc.student_id == student_id)
                    {
                        Some(sc) => sc.score = score,
                        None => cfg.scores.push(Score {
                            student_id: student_id.clone(),
                            question_id,
                            score,
                        }),
                    }
                },

                onkeydown: move |e| {
                    let next = matches!(e.key(), Key::Enter | Key::ArrowDown) || e.code() == Code::NumpadAdd;
                    let prev = e.key() == Key::ArrowUp || e.code() == Code::NumpadSubtract;
                    if next || prev {
                        e.prevent_default();
                        let idx = if next { (qidx + 1).min(qlen.saturating_sub(1)) } else { qidx.saturating_sub(1) };
                        focus_idx.set(idx);
                        return;
                    }
                    match e.code() {
                        // Numpad-only grading: these keys never produce a valid score anyway
                        Code::NumpadDivide => {
                            e.prevent_default();
                            let idx = cur_student_idx.peek().saturating_sub(1);
                            cur_student_idx.set(idx);
                        }
                        Code::NumpadMultiply => {
                            e.prevent_default();
                            let idx = (*cur_student_idx.peek() + 1).min(student_count.saturating_sub(1));
                            cur_student_idx.set(idx);
                        }
                        Code::NumpadDecimal => {
                            e.prevent_default();
                            search_open.set(true);
                        }
                        _ => {}
                    }
                },
                onfocus: move |_| {
                    cur_question_id.set(Some(question_id));
                    if *focus_idx.peek() != qidx {
                        focus_idx.set(qidx);
                    }
                }
            }
            div { class: "text-sm opacity-60", "/ {full}" }
            p { class: "validator-hint hidden m-0", "0〜{full} の整数" }
        }
    }
}
