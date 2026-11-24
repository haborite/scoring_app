use dioxus::prelude::*;
use crate::csv_export::{self, ExportFormat, ExportMode};
use crate::csv_import;
use crate::models::{bump_data_gen, Config, Page, Part, Rating, DATA_GEN};
use crate::report::{self, Band, Status, StudentResult};
use crate::settings::{HistogramView, SETTINGS};
use crate::ui::GradingState;

// Which students the list shows; set from the table, the histogram or the list itself
#[derive(Clone, Debug, PartialEq)]
enum Filter {
    All,
    Rating(usize),
    Unrated,
    Incomplete,
    Regrade,
    Range(u32, u32),
}

impl Filter {
    fn matches(&self, r: &StudentResult) -> bool {
        match *self {
            Filter::All => true,
            Filter::Rating(i) => r.rating == Some(i),
            Filter::Unrated => matches!(r.status, Status::Final(_)) && r.rating.is_none(),
            Filter::Incomplete => r.status == Status::Incomplete,
            Filter::Regrade => r.status == Status::Regrade,
            Filter::Range(lo, hi) => r.score().is_some_and(|s| s >= lo && s <= hi),
        }
    }

    fn label(&self, ratings: &[Rating]) -> String {
        match *self {
            Filter::All => "全員".to_string(),
            Filter::Rating(i) => ratings.get(i).map(|r| r.label.clone()).unwrap_or_default(),
            Filter::Unrated => "区分なし".to_string(),
            Filter::Incomplete => "未完了".to_string(),
            Filter::Regrade => "要再採点".to_string(),
            Filter::Range(lo, hi) if lo == hi => format!("{lo} 点"),
            Filter::Range(lo, hi) => format!("{lo}〜{hi} 点"),
        }
    }
}

// Bar colors by rank of the rating (highest threshold first); the lowest gets the error color
const RANK_COLORS: [&str; 6] = ["bg-success", "bg-info", "bg-primary", "bg-accent", "bg-secondary", "bg-warning"];

fn rating_color(rating: Option<usize>, order: &[usize], colored: bool) -> &'static str {
    let Some(i) = rating else {
        return "bg-base-content/25";
    };
    if !colored {
        return "bg-primary";
    }
    let rank = order.iter().position(|&o| o == i).unwrap_or(0);
    if order.len() > 1 && rank + 1 == order.len() {
        "bg-error"
    } else {
        RANK_COLORS[rank % RANK_COLORS.len()]
    }
}

#[component]
pub fn RatingPage(
    config: Signal<Config>,
    page: Signal<Page>,
    grading: GradingState,
) -> Element {
    let mut msg = use_signal(String::new);
    let mut filter = use_signal(|| Filter::All);

    // A rating index may disappear when ratings are deleted or loaded
    use_effect(move || {
        let n = config.read().ratings.len();
        if matches!(filter(), Filter::Rating(i) if i >= n) {
            filter.set(Filter::All);
        }
    });

    let cfg = config.read();
    let results = report::results(&cfg);
    let ratings = cfg.ratings.clone();
    drop(cfg);

    rsx! {
        div { class: "space-y-2",
            {(!msg().is_empty()).then(|| rsx! {
                div { class: "alert alert-info py-2 whitespace-pre-wrap text-sm",
                    span { class: "flex-1", "{msg}" }
                    button { class: "btn btn-xs btn-ghost", onclick: move |_| msg.set(String::new()), "✕" }
                }
            })}
            div { class: "grid grid-cols-1 xl:grid-cols-2 gap-2 items-start",
                RatingTable { config, results: results.clone(), filter, msg }
                Histogram { results: results.clone(), ratings: ratings.clone(), filter }
            }
            StudentList { config, results, ratings, filter, page, grading, msg }
        }
    }
}

#[component]
fn RatingTable(
    config: Signal<Config>,
    results: Vec<StudentResult>,
    filter: Signal<Filter>,
    msg: Signal<String>,
) -> Element {
    let ratings = config.read().ratings.clone();
    let (bands, unrated) = report::bands(&ratings, &results);
    let warnings = report::rating_warnings(&ratings);
    let scored = results.iter().filter(|r| r.score().is_some()).count();
    let incomplete = results.iter().filter(|r| r.status == Status::Incomplete).count();
    let regrade = results.iter().filter(|r| r.status == Status::Regrade).count();
    let total = results.len();
    let gen = DATA_GEN();
    let pct = |x: f64| format!("{:.1}%", x * 100.0);

    rsx! {
        div { class: "card bg-base-100 shadow",
            div { class: "card-body p-4 gap-3",
                div { class: "flex flex-wrap items-center gap-2",
                    div { class: "card-title", "成績区分" }
                    div { class: "flex-1" }
                    button {
                        class: "btn btn-sm",
                        title: "成績区分だけを JSON ファイルに保存します",
                        onclick: move |_| Config::save_part(config, Part::Ratings, msg),
                        "保存"
                    }
                    button {
                        class: "btn btn-sm",
                        title: "保存ファイルから成績区分だけを読み込みます",
                        onclick: move |_| Config::load_part(config, Part::Ratings, msg),
                        "読み込み"
                    }
                }

                div { class: "overflow-x-auto",
                    table { class: "table table-sm",
                        thead {
                            tr {
                                th { "区分" }
                                th { class: "text-right", "下限" }
                                th { class: "text-right", "範囲" }
                                th { class: "text-right", "人数" }
                                th { class: "text-right", "割合" }
                                th { class: "text-right", "累積" }
                                th { class: "text-right", title: "この割合を超えると警告します（空欄なら警告なし）", "目安上限" }
                                th {}
                            }
                        }
                        tbody {
                            for (i, r) in ratings.iter().enumerate() {
                                {
                                    let band: Option<Band> = bands.iter().find(|b| b.rating == i).cloned();
                                    let range = match band.as_ref().and_then(|b| b.range) {
                                        Some((lo, hi)) if lo == hi => format!("{lo}"),
                                        Some((lo, hi)) => format!("{lo}–{hi}"),
                                        None => "—".to_string(),
                                    };
                                    let (count, ratio, cumulative, over) = band
                                        .map(|b| (b.count, pct(b.ratio), pct(b.cumulative), b.over_limit))
                                        .unwrap_or((0, String::new(), String::new(), false));
                                    let max_ratio = r.max_ratio.map(|m| m.to_string()).unwrap_or_default();
                                    rsx! {
                                        // Re-created when rows are re-ordered, deleted or loaded
                                        tr { key: "{gen}-{i}",
                                            td {
                                                input {
                                                    class: "input input-sm input-bordered w-24",
                                                    initial_value: "{r.label}",
                                                    oninput: move |e| {
                                                        if let Some(r) = config.write().ratings.get_mut(i) {
                                                            r.label = e.value();
                                                        }
                                                    }
                                                }
                                            }
                                            td { class: "text-right",
                                                input {
                                                    class: "input input-sm input-bordered w-20 text-right tabular-nums",
                                                    r#type: "number",
                                                    min: "0",
                                                    max: "100",
                                                    initial_value: "{r.min_score}",
                                                    oninput: move |e| {
                                                        if let Ok(v) = e.value().trim().parse::<u32>() {
                                                            if let Some(r) = config.write().ratings.get_mut(i) {
                                                                r.min_score = v.min(100);
                                                            }
                                                        }
                                                    },
                                                    // Reorder only after editing is committed; sorting per keystroke
                                                    // moves the row away from the input being typed into
                                                    onchange: move |_| {
                                                        let mut cfg = config.write();
                                                        let sorted = cfg.ratings.windows(2).all(|w| w[0].min_score >= w[1].min_score);
                                                        if !sorted {
                                                            cfg.ratings.sort_by(|a, b| b.min_score.cmp(&a.min_score));
                                                            drop(cfg);
                                                            bump_data_gen();
                                                        }
                                                    }
                                                }
                                            }
                                            td { class: "text-right tabular-nums opacity-70", "{range}" }
                                            td { class: "text-right tabular-nums",
                                                button {
                                                    class: "link link-hover",
                                                    title: "この区分の受験者を一覧に表示",
                                                    onclick: move |_| filter.set(Filter::Rating(i)),
                                                    "{count}"
                                                }
                                            }
                                            td {
                                                class: if over { "text-right tabular-nums text-error font-semibold" } else { "text-right tabular-nums" },
                                                title: if over { "目安上限を超えています" },
                                                if over { "⚠ " }
                                                "{ratio}"
                                            }
                                            td { class: "text-right tabular-nums opacity-70", "{cumulative}" }
                                            td { class: "text-right",
                                                label { class: "inline-flex items-center gap-1",
                                                    input {
                                                        class: "input input-sm input-bordered w-20 text-right tabular-nums",
                                                        r#type: "number",
                                                        min: "0",
                                                        max: "100",
                                                        step: "any",
                                                        placeholder: "—",
                                                        initial_value: "{max_ratio}",
                                                        oninput: move |e| {
                                                            let v = e.value().trim().parse::<f32>().ok().filter(|v| *v >= 0.0);
                                                            if let Some(r) = config.write().ratings.get_mut(i) {
                                                                r.max_ratio = v;
                                                            }
                                                        }
                                                    }
                                                    span { class: "text-sm opacity-60", "%" }
                                                }
                                            }
                                            td { class: "text-right",
                                                button {
                                                    class: "btn btn-xs btn-ghost",
                                                    onclick: move |_| {
                                                        let mut cfg = config.write();
                                                        if i < cfg.ratings.len() {
                                                            cfg.ratings.remove(i);
                                                            drop(cfg);
                                                            bump_data_gen();
                                                        }
                                                    },
                                                    "Del"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if unrated > 0 {
                                tr { class: "text-warning",
                                    td { "（区分なし）" }
                                    td {}
                                    td {}
                                    td { class: "text-right tabular-nums",
                                        button { class: "link link-hover", onclick: move |_| filter.set(Filter::Unrated), "{unrated}" }
                                    }
                                    td { class: "text-right tabular-nums", "{pct(unrated as f64 / scored.max(1) as f64)}" }
                                    td {}
                                    td {}
                                    td {}
                                }
                            }
                        }
                    }
                }

                div { class: "flex flex-wrap items-center gap-x-3 gap-y-1 text-sm",
                    span { class: "tabular-nums", "集計対象 {scored} / {total} 人" }
                    if incomplete > 0 {
                        button {
                            class: "badge badge-ghost cursor-pointer",
                            title: "未入力の問題がある受験者（集計対象外）",
                            onclick: move |_| filter.set(Filter::Incomplete),
                            "未完了 {incomplete} 人"
                        }
                    }
                    if regrade > 0 {
                        button {
                            class: "badge badge-warning cursor-pointer",
                            title: "満点が採点時から変わった点数がある受験者（集計対象外）",
                            onclick: move |_| filter.set(Filter::Regrade),
                            "⚠ 要再採点 {regrade} 人"
                        }
                    }
                    div { class: "flex-1" }
                    button {
                        class: "btn btn-xs",
                        onclick: move |_| {
                            config.write().ratings.push(Rating { label: "New".to_string(), min_score: 0, max_ratio: None });
                            bump_data_gen();
                        },
                        "区分を追加"
                    }
                }

                for w in warnings.iter() {
                    div { class: "alert alert-warning py-1 text-sm", "⚠ {w}" }
                }
            }
        }
    }
}

#[component]
fn Histogram(
    results: Vec<StudentResult>,
    ratings: Vec<Rating>,
    filter: Signal<Filter>,
) -> Element {
    let view = SETTINGS.read().histogram;
    let set_view = move |f: &dyn Fn(&mut HistogramView)| f(&mut SETTINGS.write().histogram);

    let bins = report::bins(view.bin_width);
    let order = report::rating_order(&ratings);
    // Per class: count per rating (None = below every threshold), stacked highest rating on top
    let mut stacks: Vec<Vec<(Option<usize>, usize)>> = vec![Vec::new(); bins.len()];
    let finals: Vec<u32> = results.iter().filter_map(|r| r.score()).collect();
    for r in results.iter() {
        let Some(s) = r.score() else { continue };
        let b = report::bin_index(s, &bins);
        match stacks[b].iter_mut().find(|(k, _)| *k == r.rating) {
            Some((_, n)) => *n += 1,
            None => stacks[b].push((r.rating, 1)),
        }
    }
    let rank = |k: Option<usize>| k.and_then(|i| order.iter().position(|&o| o == i)).unwrap_or(usize::MAX);
    for s in stacks.iter_mut() {
        s.sort_by_key(|(k, _)| rank(*k));
    }
    let max = stacks.iter().map(|s| s.iter().map(|(_, n)| n).sum::<usize>()).max().unwrap_or(0);
    // Round y axis with at most about 5 ticks
    let step = [1usize, 2, 5, 10, 20, 50, 100, 200, 500].into_iter().find(|s| max.div_ceil(*s) <= 5).unwrap_or(1000);
    let ymax = (max.div_ceil(step) * step).max(1);
    let ticks: Vec<usize> = (0..=ymax / step).map(|k| k * step).collect();
    // Scale of the x axis in points: the last class of widths > 1 also holds 100
    let span = if view.bin_width <= 1 { 101.0 } else { (bins.len() as u32 * view.bin_width) as f64 };
    let label_every = match view.bin_width { 1 => 10, 2 => 5, 5 => 2, _ => 1 };
    let stats = report::stats(&finals);

    rsx! {
        div { class: "card bg-base-100 shadow",
            div { class: "card-body p-4 gap-3",
                div { class: "flex flex-wrap items-center gap-x-4 gap-y-1",
                    div { class: "card-title", "分布" }
                    div { class: "flex-1" }
                    label { class: "flex items-center gap-1 text-sm",
                        "幅"
                        select {
                            class: "select select-xs w-20",
                            onchange: move |e| {
                                if let Ok(w) = e.value().parse::<u32>() {
                                    set_view(&|v| v.bin_width = w);
                                }
                            },
                            for w in HistogramView::WIDTHS {
                                option { value: "{w}", selected: w == view.bin_width, "{w} 点" }
                            }
                        }
                    }
                    label { class: "flex items-center gap-1 text-sm cursor-pointer",
                        input {
                            r#type: "checkbox",
                            class: "checkbox checkbox-xs",
                            checked: view.color_by_rating,
                            onchange: move |e| { let on = e.checked(); set_view(&|v| v.color_by_rating = on) },
                        }
                        "区分で色分け"
                    }
                    label { class: "flex items-center gap-1 text-sm cursor-pointer",
                        input {
                            r#type: "checkbox",
                            class: "checkbox checkbox-xs",
                            checked: view.show_thresholds,
                            onchange: move |e| { let on = e.checked(); set_view(&|v| v.show_thresholds = on) },
                        }
                        "閾値の線"
                    }
                    label { class: "flex items-center gap-1 text-sm cursor-pointer",
                        input {
                            r#type: "checkbox",
                            class: "checkbox checkbox-xs",
                            checked: view.show_counts,
                            onchange: move |e| { let on = e.checked(); set_view(&|v| v.show_counts = on) },
                        }
                        "人数"
                    }
                }

                match stats {
                    Some(s) => rsx! {
                        div { class: "flex flex-wrap gap-x-4 gap-y-1 text-sm tabular-nums",
                            span { "人数 {s.n}" }
                            span { "平均 {s.mean:.1}" }
                            span { "中央値 {s.median:.1}" }
                            span { "標準偏差 {s.sd:.1}" }
                            span { "最高 {s.max}" }
                            span { "最低 {s.min}" }
                        }
                    },
                    None => rsx! { div { class: "text-sm opacity-60", "最終得点の出ている受験者がいません" } },
                }

                div { class: "flex gap-1 mt-4",
                    // y axis
                    div { class: "relative w-7 h-52 text-xs opacity-60 tabular-nums",
                        for t in ticks.iter() {
                            div {
                                class: "absolute right-0 translate-y-1/2",
                                style: "bottom: {*t as f64 / ymax as f64 * 100.0}%;",
                                "{t}"
                            }
                        }
                    }
                    div { class: "flex-1 min-w-0",
                        div { class: "relative h-52 border-l border-b border-base-content/30",
                            for t in ticks.iter().filter(|t| **t > 0) {
                                div {
                                    class: "absolute left-0 right-0 border-t border-dashed border-base-content/10",
                                    style: "bottom: {*t as f64 / ymax as f64 * 100.0}%;",
                                }
                            }
                            div { class: "absolute inset-0 flex items-end gap-px px-px",
                                for (b, stack) in stacks.iter().enumerate() {
                                    {
                                        let (lo, hi) = bins[b];
                                        let n: usize = stack.iter().map(|(_, n)| n).sum();
                                        let range = if lo == hi { format!("{lo} 点") } else { format!("{lo}〜{hi} 点") };
                                        let detail: Vec<String> = stack.iter().map(|(k, c)| {
                                            let name = k.and_then(|i| ratings.get(i)).map(|r| r.label.as_str()).unwrap_or("区分なし");
                                            format!("{name} {c}")
                                        }).collect();
                                        let title = if n == 0 { format!("{range}: 0 人") } else { format!("{range}: {n} 人（{}）", detail.join(", ")) };
                                        let segs: Vec<(&'static str, f64)> = stack.iter().rev()
                                            .map(|(k, c)| (rating_color(*k, &order, view.color_by_rating), *c as f64 / ymax as f64 * 100.0))
                                            .collect();
                                        rsx! {
                                            div {
                                                class: "relative flex-1 h-full flex flex-col justify-end cursor-pointer hover:bg-base-content/5",
                                                title: "{title}",
                                                onclick: move |_| filter.set(Filter::Range(lo, hi)),
                                                if view.show_counts && n > 0 {
                                                    div {
                                                        class: "absolute left-0 right-0 text-center text-[10px] leading-none tabular-nums opacity-80",
                                                        style: "bottom: calc({n as f64 / ymax as f64 * 100.0}% + 2px);",
                                                        "{n}"
                                                    }
                                                }
                                                for (cls, h) in segs {
                                                    div { class: "{cls} w-full", style: "height: {h}%;" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if view.show_thresholds {
                                for r in ratings.iter().filter(|r| r.min_score > 0 && r.min_score <= 100) {
                                    div {
                                        class: "absolute -top-4 bottom-0 border-l-2 border-dashed border-error/70 pointer-events-none",
                                        style: "left: {r.min_score as f64 / span * 100.0}%;",
                                        span { class: "absolute -top-0.5 left-1 text-[10px] leading-none text-error whitespace-nowrap", "{r.label} {r.min_score}" }
                                    }
                                }
                            }
                        }
                        // x axis
                        div { class: "flex gap-px px-px text-[10px] opacity-60 tabular-nums mt-0.5",
                            for (b, (lo, _)) in bins.iter().enumerate() {
                                div { class: "flex-1 text-left overflow-visible whitespace-nowrap",
                                    if b % label_every == 0 { "{lo}" }
                                }
                            }
                        }
                    }
                }

                if view.color_by_rating && !ratings.is_empty() {
                    div { class: "flex flex-wrap gap-3 text-xs",
                        for i in order.iter().copied() {
                            span { class: "flex items-center gap-1",
                                span { class: "inline-block w-3 h-3 rounded-sm {rating_color(Some(i), &order, true)}" }
                                "{ratings[i].label}"
                            }
                        }
                    }
                }
                p { class: "text-xs opacity-60", "棒をクリックすると、その点数の受験者を一覧に表示します。" }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SortKey {
    Id,
    Name,
    Score,
    Rating,
}

#[component]
fn StudentList(
    config: Signal<Config>,
    results: Vec<StudentResult>,
    ratings: Vec<Rating>,
    filter: Signal<Filter>,
    page: Signal<Page>,
    grading: GradingState,
    msg: Signal<String>,
) -> Element {
    // (key, ascending); None = order of the student list
    let mut sort = use_signal(|| None::<(SortKey, bool)>);
    let order = report::rating_order(&ratings);
    let rank = |r: &StudentResult| r.rating.and_then(|i| order.iter().position(|&o| o == i));

    let mut rows: Vec<&StudentResult> = results.iter().filter(|r| filter.read().matches(r)).collect();
    if let Some((key, asc)) = sort() {
        // Students without a value go last whatever the direction
        rows.sort_by(|a, b| {
            use std::cmp::Ordering;
            let by = |x: Option<Ordering>| x.unwrap_or(Ordering::Equal);
            match key {
                SortKey::Id => if asc { a.id.cmp(&b.id) } else { b.id.cmp(&a.id) },
                SortKey::Name => if asc { a.name.cmp(&b.name) } else { b.name.cmp(&a.name) },
                SortKey::Score => match (a.score(), b.score()) {
                    (Some(x), Some(y)) => by(Some(if asc { x.cmp(&y) } else { y.cmp(&x) })),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => Ordering::Equal,
                },
                SortKey::Rating => match (rank(a), rank(b)) {
                    // Rank 0 is the highest rating
                    (Some(x), Some(y)) => if asc { y.cmp(&x) } else { x.cmp(&y) },
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => Ordering::Equal,
                },
            }
        });
    }
    let shown = rows.len();
    let total = results.len();
    let current_filter = filter();

    let header = move |key: SortKey, label: &'static str, right: bool| {
        let mark = match sort() {
            Some((k, true)) if k == key => " ▲",
            Some((k, false)) if k == key => " ▼",
            _ => "",
        };
        rsx! {
            th { class: if right { "text-right" } else { "" },
                button {
                    class: "link link-hover font-semibold",
                    title: "クリックで並べ替え",
                    onclick: move |_| {
                        let next = match sort() {
                            Some((k, true)) if k == key => Some((key, false)),
                            Some((k, false)) if k == key => None,
                            // Scores and ratings are most useful highest first
                            _ => Some((key, !matches!(key, SortKey::Score | SortKey::Rating))),
                        };
                        sort.set(next);
                    },
                    "{label}{mark}"
                }
            }
        }
    };

    rsx! {
        div { class: "card bg-base-100 shadow",
            div { class: "card-body p-4 gap-3",
                div { class: "flex flex-wrap items-center gap-2",
                    div { class: "card-title", "受験者一覧" }
                    select {
                        class: "select select-sm w-40",
                        title: "表示する受験者",
                        onchange: move |e| {
                            let v = e.value();
                            let f = match v.as_str() {
                                "all" => Filter::All,
                                "unrated" => Filter::Unrated,
                                "incomplete" => Filter::Incomplete,
                                "regrade" => Filter::Regrade,
                                other => other.strip_prefix("r:").and_then(|i| i.parse().ok()).map(Filter::Rating).unwrap_or(Filter::All),
                            };
                            filter.set(f);
                        },
                        option { value: "all", selected: current_filter == Filter::All, "全員" }
                        for i in order.iter().copied() {
                            option { value: "r:{i}", selected: current_filter == Filter::Rating(i), "{ratings[i].label}" }
                        }
                        option { value: "unrated", selected: current_filter == Filter::Unrated, "区分なし" }
                        option { value: "incomplete", selected: current_filter == Filter::Incomplete, "未完了" }
                        option { value: "regrade", selected: current_filter == Filter::Regrade, "要再採点" }
                        if let Filter::Range(..) = current_filter {
                            option { value: "range", selected: true, "{current_filter.label(&ratings)}" }
                        }
                    }
                    span { class: "text-sm opacity-70 tabular-nums", "{shown} / {total} 人" }
                    if current_filter != Filter::All {
                        button { class: "btn btn-xs btn-ghost", onclick: move |_| filter.set(Filter::All), "絞り込みを解除" }
                    }
                    div { class: "flex-1" }
                    ExportBar { config, msg }
                }

                div { class: "overflow-auto max-h-[70vh]",
                    table { class: "table table-sm table-zebra table-pin-rows",
                        thead {
                            tr {
                                {header(SortKey::Id, "id", false)}
                                {header(SortKey::Name, "name", false)}
                                {header(SortKey::Score, "score", true)}
                                {header(SortKey::Rating, "rating", false)}
                            }
                        }
                        tbody {
                            for r in rows.iter() {
                                {
                                    let idx = r.idx;
                                    let goto = move |_| {
                                        let mut g = grading;
                                        g.cur_student_idx.set(idx);
                                        g.section.set(crate::ui::Section::Scores);
                                        page.set(Page::Grading);
                                    };
                                    let score = r.score().map(|s| s.to_string()).unwrap_or_default();
                                    rsx! {
                                        tr { key: "{r.idx}",
                                            td {
                                                button { class: "link link-hover font-mono", title: "この受験者を採点画面で開く", onclick: goto, "{r.id}" }
                                            }
                                            td {
                                                button { class: "link link-hover text-left", title: "この受験者を採点画面で開く", onclick: goto, "{r.name}" }
                                            }
                                            td { class: "text-right tabular-nums font-semibold", "{score}" }
                                            td {
                                                match (r.status, r.rating) {
                                                    (Status::Final(_), Some(i)) => rsx! {
                                                        span { class: "flex items-center gap-1",
                                                            span { class: "inline-block w-2 h-2 rounded-full {rating_color(Some(i), &order, true)}" }
                                                            "{ratings[i].label}"
                                                        }
                                                    },
                                                    (Status::Final(_), None) => rsx! { span { class: "badge badge-sm badge-warning", "区分なし" } },
                                                    (Status::Incomplete, _) => rsx! { span { class: "badge badge-sm badge-ghost", "未完了" } },
                                                    (Status::Regrade, _) => rsx! { span { class: "badge badge-sm badge-warning", "⚠ 要再採点" } },
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if rows.is_empty() {
                        div { class: "text-sm opacity-60 p-2", "該当する受験者はいません" }
                    }
                }
            }
        }
    }
}

// The two export modes are separate functions: making a new CSV, and writing into a CSV that was
// downloaded from another system. Each has its own formats and its own button.
#[component]
fn ExportBar(config: Signal<Config>, msg: Signal<String>) -> Element {
    let formats = use_signal(Vec::<ExportFormat>::new);
    use_hook(move || reload_formats(formats, msg));

    rsx! {
        div { class: "flex flex-wrap items-center gap-x-2 gap-y-1",
            ExportGroup {
                mode: ExportMode::New,
                label: "新規作成",
                button_label: "CSV を作成...",
                hint: "指定した列だけの CSV を新しく作ります",
                formats,
                config,
                msg,
            }
            div { class: "divider divider-horizontal mx-0 h-6" }
            ExportGroup {
                mode: ExportMode::Fill,
                label: "既存ファイルに記入",
                button_label: "CSV に記入...",
                hint: "他のシステムからダウンロードした CSV を選び、受験者IDで照合して指定の列だけを書き込みます",
                formats,
                config,
                msg,
            }
            button {
                class: "btn btn-sm btn-ghost",
                title: "エクスポートのフォーマット定義ファイル (csv_export_formats.json) があるフォルダを開きます",
                onclick: move |_| match csv_export::ensure_user_formats_file() {
                    Ok(path) => {
                        csv_import::open_folder(&path);
                        msg.set(format!("エクスポートのフォーマット定義ファイル: {}", path.display()));
                    }
                    Err(e) => msg.set(e),
                },
                "定義..."
            }
        }
    }
}

// One mode: its formats and the button that runs it
#[component]
fn ExportGroup(
    mode: ExportMode,
    label: &'static str,
    button_label: &'static str,
    hint: &'static str,
    formats: Signal<Vec<ExportFormat>>,
    config: Signal<Config>,
    msg: Signal<String>,
) -> Element {
    let mut selected = use_signal(String::new);

    let list: Vec<ExportFormat> = formats.read().iter().filter(|f| f.mode == mode).cloned().collect();
    // Falls back to the first format of this mode while nothing is selected (or after an edit
    // of the definition file removed the selected one)
    let current = list.iter()
        .find(|f| f.name == selected())
        .or_else(|| list.first())
        .cloned();
    let title = match &current {
        Some(f) if !f.description.is_empty() => format!("{hint}\n\n{}: {}", f.name, f.description),
        _ => hint.to_string(),
    };

    rsx! {
        div { class: "flex items-center gap-1",
            span { class: "text-xs opacity-70 whitespace-nowrap", title: "{hint}", "{label}" }
            select {
                class: "select select-sm w-44",
                title: "{title}",
                // Re-read on focus so edits to csv_export_formats.json apply without restarting
                onfocus: move |_| reload_formats(formats, msg),
                onchange: move |e| selected.set(e.value()),
                for f in list.iter() {
                    option {
                        value: "{f.name}",
                        selected: Some(&f.name) == current.as_ref().map(|c| &c.name),
                        title: "{f.description}",
                        "{f.name}"
                    }
                }
            }
            button {
                class: "btn btn-sm btn-primary",
                title: "{title}",
                disabled: current.is_none(),
                onclick: move |_| {
                    let name = selected();
                    let all = formats.read();
                    // Same fallback as the select: the first format of this mode
                    let format = all.iter()
                        .find(|f| f.mode == mode && f.name == name)
                        .or_else(|| all.iter().find(|f| f.mode == mode))
                        .cloned();
                    drop(all);
                    match format {
                        Some(f) => run_export(f, config, msg),
                        None => msg.set("この方式のエクスポートフォーマットがありません".to_string()),
                    }
                },
                "{button_label}"
            }
        }
    }
}

fn reload_formats(mut formats: Signal<Vec<ExportFormat>>, mut msg: Signal<String>) {
    let (loaded, errors) = csv_export::load_formats();
    if !errors.is_empty() {
        msg.set(errors.join("\n"));
    }
    if *formats.peek() != loaded {
        formats.set(loaded);
    }
}

// Build the file (asking for the file to fill in when needed), confirm blanks, then ask where to save
fn run_export(format: ExportFormat, config: Signal<Config>, mut msg: Signal<String>) {
    spawn(async move {
        let cfg = config.peek().clone();
        let (built, default_name) = match format.mode {
            ExportMode::New => (format.export_new(&cfg), format!("{}.csv", format.name)),
            ExportMode::Fill => {
                let Some(handle) = rfd::AsyncFileDialog::new()
                    .set_title(format!("書き込む CSV ファイルを選択（{}）", format.name))
                    .add_filter("CSV", &["csv", "tsv", "txt"])
                    .pick_file()
                    .await
                else {
                    return;
                };
                let bytes = handle.read().await;
                let (imports, _) = csv_import::load_formats();
                let stem = handle.file_name();
                let stem = stem.rsplit_once('.').map(|(s, _)| s.to_string()).unwrap_or(stem);
                (format.export_fill(&cfg, &bytes, &imports), format!("{stem}_成績.csv"))
            }
        };
        let (bytes, report) = match built {
            Ok(v) => v,
            Err(e) => {
                msg.set(format!("エクスポートできませんでした: {e}"));
                return;
            }
        };
        if format.mode == ExportMode::Fill && report.written == 0 {
            msg.set(format!(
                "ファイルの中に受験者一覧と一致する受験者がいません。ファイルとフォーマット（{}）を確認してください。\n{}",
                format.name,
                report.summary(),
            ));
            return;
        }
        if !report.blank.is_empty() {
            let answer = rfd::AsyncMessageDialog::new()
                .set_level(rfd::MessageLevel::Warning)
                .set_title("Scoring App")
                .set_description(format!(
                    "{} 人は点数・区分が空欄で書き出されます（未完了・要再採点・区分なし）。\n\n書き出しますか？",
                    report.blank.len()
                ))
                .set_buttons(rfd::MessageButtons::OkCancel)
                .show()
                .await;
            if answer != rfd::MessageDialogResult::Ok {
                return;
            }
        }
        let Some(out) = rfd::AsyncFileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name(&default_name)
            .save_file()
            .await
        else {
            return;
        };
        match tokio::fs::write(out.path(), bytes).await {
            Ok(()) => msg.set(format!("{} に書き出しました（{}）。\n{}", out.file_name(), format.name, report.summary())),
            Err(e) => msg.set(format!("書き出しに失敗しました: {e}")),
        }
    });
}
