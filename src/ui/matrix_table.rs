use dioxus::prelude::*;
use crate::models::{Config, TableRow};
use std::collections::HashMap;

#[component]
pub fn MatrixTable(
    config: Signal<Config>,
) -> Element {

    // Table row
    let mut table_rows: Signal<Vec<TableRow>> = use_signal(|| Vec::new());
    let mut completed = use_signal(||0);
    let mut total = use_signal(||0);

    use_effect(move || {
        let cfg = config.read();
        let students = &cfg.students;
        let questions = &cfg.questions;

        let score_map: HashMap<(&str, u32), Option<u32>> = cfg.scores
            .iter()
            .map(|s| ((s.student_id.as_str(), s.question_id), s.score))
            .collect();

        let total_weight: f32 = questions.iter().map(|q| q.weight).sum();

        let mut completed_student_count = 0usize;
        let mut rows: Vec<TableRow> = Vec::with_capacity(students.len());

        for student in students.iter() {
            let mut filled = true;
            let mut score_strings = Vec::with_capacity(questions.len());

            let mut weighted_rate_sum: f32 = 0.0;

            for q in questions.iter() {
                match score_map
                    .get(&(student.id.as_str(), q.id))
                    .copied()
                    .flatten()
                {
                    Some(scv) => {
                        score_strings.push(scv.to_string());
                        if q.full_score > 0 {
                            let rate = scv as f32 / q.full_score as f32;
                            weighted_rate_sum += rate * q.weight;
                        } else {
                            filled = false;
                        }
                    }
                    None => {
                        score_strings.push(String::new());
                        filled = false;
                    }
                }
            }

            let final_display = if filled && total_weight > 0.0 {
                completed_student_count += 1;
                format!("{:.0}", weighted_rate_sum / total_weight * 100.0)
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

        completed.set(completed_student_count);
        total.set(students.len());
        table_rows.set(rows);
    });

    rsx! {
        div { class: "card bg-base-100 shadow mt-2",
            div { class: "card-body",
                div { class: "flex items-center gap-3",
                    div { class: "card-title", "Completed: " }
                    div { class: "text-lg", "{completed} / {total}" }
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
                                        th { "score" }
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