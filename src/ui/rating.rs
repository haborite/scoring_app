use dioxus::prelude::*;
use std::collections::HashMap;
use crate::models::{Config, Rating};
use crate::ui::TopBar;
use crate::models::Page;

#[component]
pub fn RatingPage(
    on_nav: EventHandler<Page>,
    config: Signal<Config>,
) -> Element {

    // ===== final スコア一覧（未入力除外） =====
    let final_scores = {
        let cfg = config.read();
        cfg.students.iter()
            .filter_map(|s| calc_final_percent(&cfg, &s.id)) // Option<u32>
            .collect::<Vec<u32>>()
    };

    // ===== rating 割当結果 =====
    let rating_stats = {
        let cfg = config.read();
        compute_rating_stats(&final_scores, &cfg.ratings)
    };

    rsx! {
        div { class: "min-h-screen p-2 bg-base-200",

            TopBar { config, on_nav }
            
            div { class: "grid grid-cols-1 xl:grid-cols-[26rem_1fr] gap-2",
                RatingEditorCard { config }
                RatingStatsCard { stats: rating_stats.clone() }
            }
            div { class: "mt-2",
                HistogramCard { scores: final_scores, ratings: config().ratings.clone() }
            }
        }
    }
}

#[component]
fn RatingEditorCard(config: Signal<Config>) -> Element {

    let ratings = config().ratings.clone();

    rsx! {
        div { class: "card bg-base-100 shadow",
            div { class: "card-body gap-3",

                div { class: "card-title", "区分・閾値設定" }

                {
                    ratings.iter().enumerate().map(|(i, r)| {
                        rsx! {
                            div { class: "flex gap-2 items-center",

                                input {
                                    class: "input input-sm input-bordered w-24",
                                    value: "{r.label}",
                                    oninput: move |e| {
                                        let mut cfg = config.read().clone();
                                        cfg.ratings[i].label = e.value();
                                        config.set(cfg);
                                    }
                                }

                                input {
                                    class: "input input-sm input-bordered w-24",
                                    r#type: "number",
                                    min: "0",
                                    max: "100",
                                    value: "{r.min_score}",
                                    oninput: move |e| {
                                        if let Ok(v) = e.value().parse::<u32>() {
                                            let mut cfg = config.read().clone();
                                            cfg.ratings[i].min_score = v.min(100);
                                            cfg.ratings.sort_by(|a,b| b.min_score.cmp(&a.min_score));
                                            config.set(cfg);
                                        }
                                    }
                                }

                                span { class: "text-sm opacity-60", "以上" }
                            }
                        }
                    })
                }

                button {
                    class: "btn btn-xs mt-2",
                    onclick: move |_| {
                        let mut cfg = config.read().clone();
                        cfg.ratings.push(Rating {
                            label: "New".to_string(),
                            min_score: 0,
                        });
                        config.set(cfg);
                    },
                    "区分を追加"
                }
            }
        }
    }
}

#[component]
fn RatingStatsCard(stats: Vec<RatingStats>) -> Element {
    rsx! {
        div { class: "card bg-base-100 shadow",
            div { class: "card-body",
                div { class: "card-title", "人数・割合" }

                table { class: "table table-sm",
                    thead {
                        tr {
                            th { "Rating" }
                            th { "人数" }
                            th { "割合" }
                        }
                    }
                    tbody {
                        {
                            stats.iter().map(|s| rsx! {
                                tr {
                                    td { "{s.label}" }
                                    td { "{s.count}" }
                                    td { "{s.ratio * 100.0}%" }
                                }
                            })
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn HistogramCard(
    scores: Vec<u32>,
    ratings: Vec<Rating>,
) -> Element {

    let bins = histogram(&scores, 5); // 5点刻み

    let max = bins.iter().copied().max().unwrap_or(1);

    rsx! {
        div { class: "card bg-base-100 shadow",
            div { class: "card-body",
                div { class: "card-title", "ヒストグラム（5点刻み）" }

                div { class: "flex items-end gap-1 h-40",

                    {
                        bins.iter().enumerate().map(|(_i, &count)| {
                            let height = (count as f32 / max as f32) * 100.0;
                            rsx! {
                                div {
                                    class: "bg-primary w-3",
                                    style: "height: {height}%;"
                                }
                            }
                        })
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RatingStats {
    pub label: String,
    pub count: usize,
    pub ratio: f32,
}

fn compute_rating_stats(
    scores: &[u32],
    ratings: &[Rating],
) -> Vec<RatingStats> {

    let mut counts = vec![0usize; ratings.len()];

    for &s in scores {
        if let Some((i, _)) = ratings.iter()
            .enumerate()
            .find(|(_, r)| s >= r.min_score)
        {
            counts[i] += 1;
        }
    }

    let total = scores.len().max(1);

    ratings.iter().enumerate().map(|(i, r)| {
        RatingStats {
            label: r.label.clone(),
            count: counts[i],
            ratio: counts[i] as f32 / total as f32,
        }
    }).collect()
}

fn calc_final_percent( config: &Config, student_id: &str) -> Option<u32> {
    // let cfg = config;
    let questions = &config.questions;

    let score_map: HashMap<(&str, u32), Option<u32>> = config.scores
        .iter()
        .map(|s| ((s.student_id.as_str(), s.question_id), s.score))
        .collect();

    let total_weight: f32 = questions.iter().map(|q| q.weight).sum();
    let mut weighted_rate_sum: f32 = 0.0;

    for q in questions.iter() {
        match score_map
            .get(&(student_id, q.id))
            .copied()
            .flatten()
        {
            Some(scv) => {
                if q.full_score > 0 {
                    let rate = scv as f32 / q.full_score as f32;
                    weighted_rate_sum += rate * q.weight;
                } else {
                    return None; // full_score=0 は未入力扱い
                }
            }
            None => return None, // 未入力
        }
    }
    if total_weight > 0.0 {
        Some((weighted_rate_sum / total_weight * 100.0) as u32)
    } else {
        None
    }
}

fn histogram(scores: &[u32], bin_width: u32) -> Vec<usize> {
    let bins = (100 / bin_width) + 1; // 例: bin_width=5 => 21
    let mut h = vec![0usize; bins as usize];
    for &s in scores {
        let idx = (s / bin_width).min(bins - 1) as usize;
        h[idx] += 1;
    }
    h
}
