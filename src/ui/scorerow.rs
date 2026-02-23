use dioxus::prelude::*;
use crate::models::{Config, Question, Score};

#[component]
pub fn ScoreRow(
    question_id: u32,
    cur_question_id: Signal<Option<u32>>,
    cur_student_idx: ReadSignal<usize>,
    qidx: usize,
    config: Signal<Config>,
    is_focused: bool,
    move_to_next: EventHandler<()>,
    move_to_prev: EventHandler<()>,
) -> Element {

    let student_id = config().students.get(cur_student_idx()).map(|s| s.id.to_string()).unwrap_or_default();
    let question = config().questions.iter().find(|q| q.id == question_id).cloned().unwrap_or(Question {
        id: 0,
        name: "Unknown".to_string(),
        full_score: 100,
        weight: 1.0,
        comment: "".to_string(),
    });
    let question_id = question.id;
    let q_name = question.name;
    let full = question.full_score;

    rsx! {
        div { class: "grid grid-cols-[1fr_auto_auto] md:grid-cols-[8rem_6rem_auto_auto] gap-2 items-center",
            div { class: "font-semibold truncate", "{q_name}" }
            input {
                id: "score-{qidx}",
                r#type: "number",
                value: config().scores
                    .iter()
                    .find(|sc| sc.question_id == question_id && sc.student_id == student_id )
                    .map(|sc| if let Some(scv) = sc.score { scv.to_string() } else { String::new() }),
                min: 0,
                max: full,
                required: true,
                class: "input validator",
                autofocus: is_focused,

                oninput: move |e| {
                    let mut s = e.value();
                    s.retain(|c| c.is_ascii_digit());
                    let mut binding = config.write();
                    let score_opt = binding.scores
                        .iter_mut()
                        .find(|sc| sc.question_id == question_id && sc.student_id == student_id);
                    if let Some(score) = score_opt {
                        if let Ok(num) = s.parse() {
                            if num <= full {
                                score.score = Some(num); 
                            } else {
                                score.score = None;
                            }
                        } else {
                            score.score = None;
                        }
                    } else {
                        let score;
                        if let Ok(num) = s.parse() { 
                            if num <= full {
                                score = Some(num);
                            } else {
                                score = None;
                            }
                        } else { 
                            score = None; 
                        };
                        binding.scores.push(Score {
                            student_id: student_id.clone(),
                            question_id,
                            score,
                        });
                    }
                },

                onkeydown: move |e| {
                    match e.code() {
                        Code::Enter | Code::NumpadEnter | Code::ArrowDown | Code::NumpadAdd => {
                            e.prevent_default();
                            move_to_next.call(());
                        },
                        Code::ArrowUp | Code::NumpadSubtract => {
                            e.prevent_default();
                            move_to_prev.call(());
                        },
                        _ => {}
                    }
                },
                onfocus: move |_e| { cur_question_id.set(Some(question_id)); }
            }
            div { class: "text-sm opacity-60", " / {full}" }
            p { class: "validator-hint", "Must be 0 to {full}" }
        }
    }
}
