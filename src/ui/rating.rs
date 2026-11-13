use dioxus::prelude::*;
use crate::models::{Config, Rating};

#[component]
pub fn RatingPage(
    config: Signal<Config>,
) -> Element {

    // ===== final スコア一覧（未入力除外） =====
    let final_scores = {
        let cfg = config.read();
        let score_map = cfg.score_map();
        cfg.students.iter()
            .filter_map(|s| cfg.final_score(&score_map, &s.id))
            .collect::<Vec<u32>>()
    };
    let student_count = config.read().students.len();

    // ===== rating 割当結果 =====
    let rating_stats = compute_rating_stats(&final_scores, &config.read().ratings);

    rsx! {
        div {

            div { class: "grid grid-cols-1 xl:grid-cols-[26rem_1fr] gap-2",
                RatingEditorCard { config }
                RatingStatsCard {
                    stats: rating_stats,
                    scored: final_scores.len(),
                    student_count,
                }
            }
            div { class: "mt-2",
                HistogramCard { scores: final_scores }
            }
        }
    }
}

#[component]
fn RatingEditorCard(mut config: Signal<Config>) -> Element {

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
                                        if let Some(r) = config.write().ratings.get_mut(i) {
                                            r.label = e.value();
                                        }
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
                                            if let Some(r) = config.write().ratings.get_mut(i) {
                                                r.min_score = v.min(100);
                                            }
                                        }
                                    },
                                    // Reorder only after editing is committed; sorting per keystroke
                                    // moves the row away from the input being typed into
                                    onchange: move |_| {
                                        config.write().ratings.sort_by(|a, b| b.min_score.cmp(&a.min_score));
                                    }
                                }

                                span { class: "text-sm opacity-60", "以上" }

                                button {
                                    class: "btn btn-xs btn-ghost ml-auto",
                                    onclick: move |_| {
                                        let mut cfg = config.write();
                                        if i < cfg.ratings.len() {
                                            cfg.ratings.remove(i);
                                        }
                                    },
                                    "Del"
                                }
                            }
                        }
                    })
                }

                button {
                    class: "btn btn-xs mt-2",
                    onclick: move |_| {
                        config.write().ratings.push(Rating {
                            label: "New".to_string(),
                            min_score: 0,
                        });
                    },
                    "区分を追加"
                }
            }
        }
    }
}

#[component]
fn RatingStatsCard(
    stats: Vec<RatingStats>,
    scored: usize,
    student_count: usize,
) -> Element {
    rsx! {
        div { class: "card bg-base-100 shadow",
            div { class: "card-body",
                div { class: "card-title", "人数・割合" }
                div { class: "text-sm opacity-70",
                    "集計対象: {scored} / {student_count} 人（未入力のある受験者は除外）"
                }

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
                            stats.iter().map(|s| {
                                let pct = format!("{:.1}", s.ratio * 100.0);
                                rsx! {
                                    tr {
                                        td { "{s.label}" }
                                        td { "{s.count}" }
                                        td { "{pct}%" }
                                    }
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
) -> Element {

    let bin_width = 5; // 5点刻み
    let bins = histogram(&scores, bin_width);

    let max = bins.iter().copied().max().unwrap_or(0).max(1);

    rsx! {
        div { class: "card bg-base-100 shadow",
            div { class: "card-body",
                div { class: "card-title", "ヒストグラム（5点刻み）" }

                div { class: "flex items-end gap-1 h-40",
                    {
                        bins.iter().enumerate().map(|(i, &count)| {
                            let height = count as f32 / max as f32 * 100.0;
                            let lo = i as u32 * bin_width;
                            let range = if lo >= 100 {
                                "100".to_string()
                            } else {
                                format!("{lo}-{}", lo + bin_width - 1)
                            };
                            rsx! {
                                div {
                                    class: "bg-primary w-6",
                                    style: "height: {height}%;",
                                    title: "{range}: {count} 人",
                                }
                            }
                        })
                    }
                }
                div { class: "flex gap-1 text-xs opacity-60",
                    {
                        bins.iter().enumerate().map(|(i, _)| {
                            let lo = i as u32 * bin_width;
                            let label = if lo % 10 == 0 { lo.to_string() } else { String::new() };
                            rsx! { div { class: "w-6 text-left", "{label}" } }
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

    // Highest threshold first, regardless of the display order
    let mut order: Vec<usize> = (0..ratings.len()).collect();
    order.sort_by(|&a, &b| ratings[b].min_score.cmp(&ratings[a].min_score));

    let mut counts = vec![0usize; ratings.len()];
    let mut unrated = 0usize;

    for &s in scores {
        match order.iter().find(|&&i| s >= ratings[i].min_score) {
            Some(&i) => counts[i] += 1,
            None => unrated += 1,
        }
    }

    let total = scores.len().max(1);

    let mut stats: Vec<RatingStats> = order.iter().map(|&i| {
        RatingStats {
            label: ratings[i].label.clone(),
            count: counts[i],
            ratio: counts[i] as f32 / total as f32,
        }
    }).collect();

    if unrated > 0 {
        stats.push(RatingStats {
            label: "（区分なし）".to_string(),
            count: unrated,
            ratio: unrated as f32 / total as f32,
        });
    }
    stats
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

#[cfg(test)]
mod tests {
    use super::*;

    fn r(label: &str, min_score: u32) -> Rating {
        Rating { label: label.to_string(), min_score }
    }

    #[test]
    fn stats_ignore_display_order_and_count_unrated() {
        // Not sorted: as while the user is still editing
        let ratings = vec![r("C", 60), r("A", 80), r("B", 70)];
        let stats = compute_rating_stats(&[85, 75, 65, 50], &ratings);
        let got: Vec<(&str, usize)> = stats.iter().map(|s| (s.label.as_str(), s.count)).collect();
        assert_eq!(got, vec![("A", 1), ("B", 1), ("C", 1), ("（区分なし）", 1)]);
    }

    #[test]
    fn histogram_puts_100_in_last_bin() {
        let h = histogram(&[0, 4, 5, 100], 5);
        assert_eq!(h.len(), 21);
        assert_eq!((h[0], h[1], h[20]), (2, 1, 1));
    }
}
