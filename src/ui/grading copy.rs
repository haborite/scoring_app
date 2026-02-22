// src/ui/grading.rs
use dioxus::prelude::*;

use crate::ui::scorerows::ScoreRows;
use crate::ui::search::SearchWindow;
use crate::models::{Config, Question, Student};

#[component]
pub fn GradingPage(
    on_back: EventHandler<()>,
    config: Signal<Config>,
) -> Element {

    // master data
    let questions = use_signal(|| config.read().questions.clone());
    let students  = use_signal(|| config.read().students.clone());

    // navigation
    let mut cur_student_idx = use_signal(|| 0usize);

    // search popup
    let search_open = use_signal(|| false);
    let search_q = use_signal(|| String::new());

    use_effect(move || {
        let js = format!(
            r#"queueMicrotask(() => {{
                const el = document.getElementById("score-{cur_student_idx}-{focus_idx}");
                if (el) {{
                    el.focus();
                    if (el.select) el.select();
                }}
            }});"#
        );
        let _ = document::eval(&js);
    });
    

    // load scores for current student
    let load_current_student_scores = {
        move || {
            if students().is_empty() || questions().is_empty() || cur_student_idx() >= students().len() {
                score_inputs.set(vec![String::new(); questions().len()]);
                return;
            }
            spawn(async move {
                let sid = &cur_student_idx().to_string();
                match db.get_scores_for_student(&sid).await {
                    Ok(pairs) => {
                        let mut v = vec![String::new(); questions().len()];
                        for (i, (_qid, score)) in pairs.into_iter().enumerate().take(questions().len()) {
                            v[i] = score.map(|x| x.to_string()).unwrap_or_default();
                        }
                        score_inputs.set(v);
                        msg.set("".to_string());
                    }
                    Err(e) => msg.set(format!("load scores error: {e:#}")),
                }
            });
        }
    };

    {
        let mut load = load_current_student_scores.clone();
        let students = students.clone();
        let questions = questions.clone();

        use_effect(move || {
            let _ = students.read().len();
            let _ = questions.read().len();
            let _ = cur_student_idx();
            load();
        });
    }

    // ---------- derived ----------
    let ss = students.read();
    let qs = questions.read();

    let cur_student = ss.get(cur_student_idx()).cloned();
    let cur_student_label = cur_student
        .as_ref()
        .map(|s| format!("{}  {}", s.id, s.name))
        .unwrap_or_else(|| "(受験者なし)".to_string());


    
    // ---------- UI ----------

    rsx! {
        div {
            class: "min-h-screen p-4 bg-base-200 text-base-content",
            tabindex: "0",

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
                            mv_prev(focus_idx);
                        }
                        "l" | "L" => {
                            e.prevent_default();
                            mv_next(focus_idx, questions().len());
                        }
                        _ => {}
                    }
                }
            },

            // Top bar / Navbar
            div { class: "navbar bg-base-100 rounded-box shadow mb-4",
                div { class: "navbar-start gap-2",
                    button { class: "btn btn-sm", onclick: move |_| on_back.call(()), "戻る" }
                    button { class: "btn btn-sm", onclick: move |_| { mv_prev(focus_idx) }, "← 前" }
                    button { class: "btn btn-sm btn-primary", onclick: move |_| { mv_next(focus_idx, questions().len()) }, "次 →" }
                }
                div { class: "navbar-center",
                    div { class: "text-lg font-bold", "{cur_student_label}" }
                }
                div { class: "navbar-end",
                    div { class: "text-xs opacity-70 hidden md:block",
                        "F:検索 / J,K:受験者移動 / Enter:次問題"
                    }
                }
            }

            {(!msg.read().is_empty()).then(|| rsx! {
                div { class: "alert alert-error mb-4", "{msg}" }
            })}

            // Main area
            div { class: "grid grid-cols-1 xl:grid-cols-[1fr_18rem_20rem] gap-4",


                // comment panel card (placeholder)
                div { class: "card bg-base-100 shadow",
                    div { class: "card-body",
                        div { class: "card-title", "コメント" }
                        div { class: "opacity-60", "（未実装）" }
                    }
                }

                // progress panel card
                div { class: "card bg-base-100 shadow",
                    div { class: "card-body",
                        div { class: "card-title", "進捗" }
                        div { class: "flex justify-between",
                            span { "採点済み" }
                            span { class: "font-mono", "{completed_students}/{total_students}" }
                        }
                    }
                }
            }

            // table panel card
            div { class: "card bg-base-100 shadow mt-4",
                div { class: "card-body",
                    div { class: "flex items-center gap-3",
                        div { class: "card-title", "一覧" }
                        div { class: "text-xs opacity-60", "（受験者×問題の表＋最終得点）" }
                    }

                    {
                        let qids = qs.iter().map(|q| q.id.clone()).collect::<Vec<_>>();
                        println!("qids: {:?}", qids);
                        rsx! {
                            div { class: "overflow-auto max-h-96 mt-3",
                                table { class: "table table-zebra table-sm",
                                    thead {
                                        tr {
                                            th { "id" }
                                            th { "name" }
                                            for qid in qids.iter() {
                                                th { "{qid}" }
                                            }
                                            th { "final" }
                                        }
                                    }
                                    tbody {
                                        for row in table_rows().iter() {
                                            tr {
                                                td { class: "font-mono", "{row.student_id}" }
                                                td { "{row.student_name}" }
                                                for s in row.scores.iter() {
                                                    td { class: "font-mono", "{s}" }
                                                }
                                                td { class: "font-mono font-semibold", "{row.final_display}" }
                                            }
                                        }
                                    }
                                }
                            },
                        }
                    }
                }
            }

            // search modal
            {
                search_open().then(|| 
                    rsx!{ SearchWindow { search_open, msg, students, cur_student_idx, focus_idx } }
                )
            }
        }
    }
}


#[derive(Clone)]
struct TableRow {
    student_id: String,
    student_name: String,
    scores: Vec<String>,
    final_display: String,
}


// final = Σ(score/full*weight) / Σ(weight) * 100
fn calc_final(questions: &[QuestionRow], scores: &[Option<u32>]) -> Option<f32> {
    if questions.is_empty() {
        return None;
    }
    let mut num = 0.0f32;
    let mut den = 0.0f32;

    for (q, s) in questions.iter().zip(scores.iter()) {
        if q.weight <= 0.0 {
            continue;
        }
        den += q.weight;

        let sc = s.unwrap_or(0) as f32;
        let full = q.full_score.max(1) as f32;
        num += (sc / full) * q.weight;
    }

    if den == 0.0 { None } else { Some(num / den * 100.0) }
}


fn is_student_done(questions: &[QuestionRow], inputs: &[String]) -> bool {
    if questions.is_empty() || inputs.len() != questions.len() {
        return false;
    }
    for (q, s) in questions.iter().zip(inputs.iter()) {
        let t = s.trim();
        if t.is_empty() {
            return false;
        }
        if let Ok(n) = t.parse::<u32>() {
            if n < 0 || n > q.full_score {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

fn mv_next(
    mut focus_idx: Signal<usize>, 
    qs_len: usize
) {
    let current = *focus_idx.read();
    let max_idx = qs_len.saturating_sub(1);
    let idx = std::cmp::min(current + 1, max_idx);
    focus_idx.set(idx);
}

fn mv_prev(
    mut focus_idx: Signal<usize>
) {
    let current = *focus_idx.read();
    let idx = current.saturating_sub(1);
    focus_idx.set(idx);
}

