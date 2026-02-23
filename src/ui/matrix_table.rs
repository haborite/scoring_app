use dioxus::prelude::*;
use crate::models::{Config, TableRow};

#[component]
pub fn MatrixTable(
    config: Signal<Config>,
) -> Element {

    // Table row
    let mut table_rows: Signal<Vec<TableRow>> = use_signal(|| Vec::new());

    use_effect(move || {

        let students = config().students.clone();
        let questions = config().questions.clone();
        let cfg = config.read().clone();

        let mut rows: Vec<TableRow> = Vec::with_capacity(students.len());

        for student in &students {
            let mut filled = true;
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
                    filled = false;
                }
                weighted_full += q.full_score as f32 * q.weight;
            }

            let final_display = if weighted_full > 0.0 && filled {
                format!("{:.0}", weighted_sum / weighted_full * 100.0)
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

    rsx! {
        div { class: "card bg-base-100 shadow mt-4",
            div { class: "card-body",
                div { class: "flex items-center gap-3",
                    div { class: "card-title", "一覧" }
                    div { class: "text-xs opacity-60", "（受験者×問題の表＋最終得点）" }
                }

                {
                    // let qids = config().questions.iter().map(|q| q.id.clone()).collect::<Vec<_>>();
                    let qnames = config().questions.iter().map(|q| q.name.clone()).collect::<Vec<_>>();
                    rsx! {
                        div { class: "overflow-auto max-h-96 mt-3",
                            table { class: "table table-zebra table-sm",
                                thead {
                                    tr {
                                        th { "id" }
                                        th { "name" }
                                        for qname in qnames.iter() {
                                            th { "{qname}" }
                                        }
                                        th { "final" }
                                    }
                                }
                                tbody {
                                    for row in table_rows().iter() {
                                        tr {
                                            td { class: "font-mono", "{row.student_id}" }
                                            td { "{row.student_name}" }
                                            for sc in row.scores.iter() {
                                                td { class: "font-mono", "{sc}" }
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
    }
}


