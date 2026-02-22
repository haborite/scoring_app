use dioxus::prelude::*;
use crate::ui::scorerows::ScoreRows;
use crate::ui::search::SearchWindow;
use crate::models::{Config, Question};

#[component]
pub fn GradingPage(
    on_back: EventHandler<()>,
    config: Signal<Config>,
) -> Element {

    // navigation
    let cur_student_idx = use_signal(|| 0usize);

    // search popup
    let mut search_open = use_signal(|| false);
    // let search_q = use_signal(|| String::new());

    // message
    let mut msg = use_signal(|| String::new());

    // Table row
    let mut table_rows: Signal<Vec<TableRow>> = use_signal(|| Vec::new());



    use_effect(move || {

        let mut filled = true;
        let students = config().students.clone();
        let questions = config().questions.clone();
        let cfg = config.read().clone();

        let mut rows: Vec<TableRow> = Vec::with_capacity(students.len());

        for student in &students {
            let mut score_strings: Vec<String> = Vec::with_capacity(questions.len());

            let mut weighted_sum: f32 = 0.0;
            let mut weighted_full: f32 = 0.0;

            for q in &questions {
                let score_opt = cfg.scores.iter().find(|s|
                    s.student_id == student.id && s.question_id == q.id
                );

                if let Some(sc) = score_opt {
                    if let Some(scv) = sc.score {
                        score_strings.push(scv.to_string());
                        weighted_sum += scv as f32 * q.weight;
                    } else {
                        score_strings.push(String::new());
                        filled = false;
                    }
                } else {
                    score_strings.push(String::new());
                }
                weighted_full += q.full_score as f32 * q.weight;
            }

            // 100点換算（必要なら）
            let final_display = if weighted_full > 0.0 && filled {
                format!("{:.1}", weighted_sum / weighted_full * 100.0)
            } else {
                String::new()
            };

            rows.push(TableRow {
                student_id: student.id.clone(),
                student_name: student.name.clone(),
                scores: score_strings,
                final_display,
            });
        }

        table_rows.set(rows);
    });

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

            // Top bar / Navbar
            div { class: "navbar bg-base-100 rounded-box shadow mb-4",
                div { class: "navbar-start gap-2",
                    button { class: "btn btn-sm btn-primary", onclick: move |_| on_back.call(()), "戻る" }
                    button { class: "btn btn-sm", onclick: move |_| { mv_prev_student(cur_student_idx) }, "← 前" }
                    button { class: "btn btn-sm", onclick: move |_| { mv_next_student(cur_student_idx, config().students.len()) }, "次 →" }
                }
                div { class: "navbar-center",
                    div { class: "text-lg font-bold", "{cur_student_label(config, cur_student_idx)}" }
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

                ScoreRows {
                    cur_student_idx,
                    // cur_question_idx,
                    config,
                }

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
                            span { class: "font-mono", "completed_students/total_students" }
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
                        let qids = config().questions.iter().map(|q| q.id.clone()).collect::<Vec<_>>();
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
                    rsx!{
                        SearchWindow { 
                            is_open: search_open, 
                            msg, 
                            config, 
                            cur_student_idx, 
                            // focus_idx: cur_question_idx
                        }
                    }
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
/*
fn calc_final(questions: &[Question], scores: &[Option<u32>]) -> Option<f32> {
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
*/

fn is_student_done(questions: &[Question], inputs: &[String]) -> bool {
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



fn cur_student_label(
    config: Signal<Config>,
    cur_student_idx: Signal<usize>,
) -> String {
    config().students
        .get(cur_student_idx())
        .map(|s| format!("{} ({})", s.name, s.id))
        .unwrap_or_else(|| "No student".to_string())
}
