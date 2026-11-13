use dioxus::prelude::*;
use crate::models::{Config, TableRow};

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

        let score_map = cfg.score_map();

        let mut completed_student_count = 0usize;
        let mut rows: Vec<TableRow> = Vec::with_capacity(students.len());

        for student in students.iter() {
            let score_strings = questions.iter()
                .map(|q| {
                    score_map.get(&(student.id.as_str(), q.id))
                        .copied()
                        .flatten()
                        .map(|scv| scv.to_string())
                        .unwrap_or_default()
                })
                .collect();

            let final_display = match cfg.final_score(&score_map, &student.id) {
                Some(f) => {
                    completed_student_count += 1;
                    f.to_string()
                }
                None => String::new(),
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