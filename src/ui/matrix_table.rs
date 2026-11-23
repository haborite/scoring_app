use dioxus::prelude::*;
use crate::models::{Config, TableRow};
use crate::settings::SETTINGS;
use crate::shortcuts::Action;

#[component]
pub fn MatrixTable(
    config: Signal<Config>,
    cur_student_idx: Signal<usize>,
) -> Element {

    let cfg = config.read();
    let score_map = cfg.score_map();
    let regrade = cfg.regrade_map();
    let regrade_students = Config::regrade_students(&regrade);
    let qnames: Vec<String> = cfg.questions.iter().map(|q| q.name.clone()).collect();
    let fulls: Vec<u32> = cfg.questions.iter().map(|q| q.full_score).collect();

    let mut completed = 0usize;
    let rows: Vec<TableRow> = cfg.students.iter().map(|student| {
        let sid = student.id.as_str();
        let scores = cfg.questions.iter()
            .map(|q| {
                score_map.get(&(sid, q.id)).copied().flatten()
                    .map(|scv| scv.to_string())
                    .unwrap_or_default()
            })
            .collect();
        let regrade_cells = cfg.questions.iter().map(|q| regrade.get(&(sid, q.id)).copied()).collect();
        let needs_regrade = regrade_students.contains(sid);
        let final_display = match cfg.final_score(&score_map, sid) {
            Some(f) if !needs_regrade => {
                completed += 1;
                f.to_string()
            }
            Some(f) => format!("({f})"),
            None => String::new(),
        };
        TableRow {
            student_id: student.id.clone(),
            student_name: student.name.clone(),
            scores,
            regrade: regrade_cells,
            final_display,
            needs_regrade,
        }
    }).collect();
    let total = cfg.students.len();
    let regrade_count = regrade_students.len();
    drop(regrade_students);
    drop(regrade);
    drop(score_map);
    drop(cfg);
    let next_regrade_key = SETTINGS.read().key_label(Action::NextRegrade);

    rsx! {
        div { class: "card bg-base-100 shadow h-full",
            div { class: "card-body p-4 gap-2 min-h-0",
                div { class: "flex items-center gap-3",
                    div { class: "card-title", "Completed: " }
                    div { class: "text-lg tabular-nums", "{completed} / {total}" }
                    if regrade_count > 0 {
                        span {
                            class: "badge badge-warning",
                            title: "満点が採点時から変わった点数があります（集計対象外）。{next_regrade_key} で順に移動",
                            "⚠ 要再採点 {regrade_count} 人"
                        }
                    }
                }

                div { class: "overflow-auto flex-1 min-h-0",
                    table { class: "table table-zebra table-sm table-pin-rows",
                        thead {
                            tr {
                                th { "id" }
                                th { "name" }
                                for qname in qnames.iter() {
                                    th { class: "text-right", "{qname}" }
                                }
                                th { class: "text-right", "score" }
                            }
                        }
                        tbody {
                            for (i, row) in rows.iter().enumerate() {
                                tr {
                                    id: "matrix-row-{i}",
                                    class: if i == cur_student_idx() { "bg-primary/15!" },
                                    td { class: "whitespace-nowrap",
                                        if row.needs_regrade {
                                            span { class: "text-warning mr-1", title: "要再採点", "⚠" }
                                        }
                                        button {
                                            class: "link link-hover font-mono",
                                            title: "この受験者を採点する",
                                            onclick: move |_| cur_student_idx.set(i),
                                            "{row.student_id}"
                                        }
                                    }
                                    td {
                                        button {
                                            class: "link link-hover text-left",
                                            title: "この受験者を採点する",
                                            onclick: move |_| cur_student_idx.set(i),
                                            "{row.student_name}"
                                        }
                                    }
                                    for (qi, sc) in row.scores.iter().enumerate() {
                                        match row.regrade[qi] {
                                            Some(graded) => {
                                                let tip = match graded {
                                                    Some(g) if g != fulls[qi] => format!("要再採点: 採点時の満点 {g} → 現在 {}", fulls[qi]),
                                                    _ => format!("要再採点: 満点 {} を超えています", fulls[qi]),
                                                };
                                                rsx! { td { class: "text-right tabular-nums bg-warning/40", title: "{tip}", "{sc}" } }
                                            }
                                            None => rsx! { td { class: "text-right tabular-nums", "{sc}" } },
                                        }
                                    }
                                    td {
                                        class: if row.needs_regrade { "text-right tabular-nums opacity-50" } else { "text-right tabular-nums font-semibold" },
                                        title: if row.needs_regrade { "要再採点のため集計対象外" },
                                        "{row.final_display}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
