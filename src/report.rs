// Results of the exam: final scores, ratings, counts and statistics.
// Shared by the rating page and the CSV export.

use crate::models::{Config, Rating};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    // Every question scored and nothing to regrade
    Final(u32),
    // Some question has no score
    Incomplete,
    // Some score needs regrading (full score changed)
    Regrade,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StudentResult {
    // Index in Config::students
    pub idx: usize,
    pub id: String,
    pub name: String,
    pub status: Status,
    // Index in Config::ratings; None when not Final or below every threshold
    pub rating: Option<usize>,
}

impl StudentResult {
    pub fn score(&self) -> Option<u32> {
        match self.status {
            Status::Final(s) => Some(s),
            _ => None,
        }
    }
}

// Rating indices from the highest threshold down, whatever the stored order
pub fn rating_order(ratings: &[Rating]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..ratings.len()).collect();
    order.sort_by(|&a, &b| ratings[b].min_score.cmp(&ratings[a].min_score).then(a.cmp(&b)));
    order
}

pub fn rate(score: u32, ratings: &[Rating], order: &[usize]) -> Option<usize> {
    order.iter().copied().find(|&i| score >= ratings[i].min_score)
}

pub fn results(cfg: &Config) -> Vec<StudentResult> {
    let score_map = cfg.score_map();
    let regrade = cfg.regrade_map();
    let regrade_students = Config::regrade_students(&regrade);
    let order = rating_order(&cfg.ratings);
    cfg.students
        .iter()
        .enumerate()
        .map(|(idx, s)| {
            let status = if regrade_students.contains(s.id.as_str()) {
                Status::Regrade
            } else {
                match cfg.final_score(&score_map, &s.id) {
                    Some(f) => Status::Final(f),
                    None => Status::Incomplete,
                }
            };
            let rating = match status {
                Status::Final(f) => rate(f, &cfg.ratings, &order),
                _ => None,
            };
            StudentResult { idx, id: s.id.clone(), name: s.name.clone(), status, rating }
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct Band {
    // Index in Config::ratings
    pub rating: usize,
    // Scores that get this rating; None when another rating with the same threshold takes them
    pub range: Option<(u32, u32)>,
    pub count: usize,
    // Share of the rated-or-unrated final scores (0.0-1.0)
    pub ratio: f64,
    // Share of this rating and every higher one
    pub cumulative: f64,
    pub over_limit: bool,
}

// One row per rating, highest threshold first, plus the number of final scores below every threshold
pub fn bands(ratings: &[Rating], results: &[StudentResult]) -> (Vec<Band>, usize) {
    let order = rating_order(ratings);
    let finals: Vec<&StudentResult> = results.iter().filter(|r| r.score().is_some()).collect();
    let total = finals.len().max(1) as f64;
    let mut cumulative = 0.0;
    let mut upper: Option<u32> = None; // threshold of the previous (higher) rating
    let bands = order
        .iter()
        .map(|&i| {
            let min = ratings[i].min_score;
            let hi = match upper {
                None => 100,
                Some(u) => u.saturating_sub(1),
            };
            let range = (upper.is_none_or(|u| min < u) && min <= hi).then_some((min, hi.max(min)));
            upper = Some(upper.map_or(min, |u| u.min(min)));
            let count = finals.iter().filter(|r| r.rating == Some(i)).count();
            let ratio = count as f64 / total;
            cumulative += ratio;
            let over_limit = ratings[i].max_ratio.is_some_and(|m| ratio * 100.0 > m as f64 + 1e-9);
            Band { rating: i, range, count, ratio, cumulative, over_limit }
        })
        .collect();
    let unrated = finals.iter().filter(|r| r.rating.is_none()).count();
    (bands, unrated)
}

// Problems with the thresholds themselves
pub fn rating_warnings(ratings: &[Rating]) -> Vec<String> {
    let mut w = Vec::new();
    if ratings.is_empty() {
        return w;
    }
    let order = rating_order(ratings);
    for pair in order.windows(2) {
        let (a, b) = (&ratings[pair[0]], &ratings[pair[1]]);
        if a.min_score == b.min_score {
            w.push(format!("「{}」と「{}」の下限が同じ {} 点です（「{}」は使われません）", a.label, b.label, a.min_score, b.label));
        }
    }
    if ratings.iter().all(|r| r.min_score > 0) {
        let lowest = &ratings[*order.last().unwrap()];
        w.push(format!("{} 点未満の受験者はどの区分にも入りません（下限 0 点の区分を作ると全員が区分に入ります）", lowest.min_score));
    }
    let mut labels: Vec<&str> = ratings.iter().map(|r| r.label.as_str()).collect();
    labels.sort();
    if labels.windows(2).any(|p| p[0] == p[1]) {
        w.push("同じ名前の区分があります".to_string());
    }
    w
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stats {
    pub n: usize,
    pub mean: f64,
    pub median: f64,
    // Population standard deviation
    pub sd: f64,
    pub min: u32,
    pub max: u32,
}

pub fn stats(scores: &[u32]) -> Option<Stats> {
    if scores.is_empty() {
        return None;
    }
    let mut v = scores.to_vec();
    v.sort_unstable();
    let n = v.len();
    let mean = v.iter().map(|&s| s as f64).sum::<f64>() / n as f64;
    let median = if n % 2 == 1 { v[n / 2] as f64 } else { (v[n / 2 - 1] + v[n / 2]) as f64 / 2.0 };
    let var = v.iter().map(|&s| (s as f64 - mean).powi(2)).sum::<f64>() / n as f64;
    Some(Stats { n, mean, median, sd: var.sqrt(), min: v[0], max: v[n - 1] })
}

// Histogram classes (lo, hi) covering 0-100. 100 is put in the last class, except for width 1.
pub fn bins(width: u32) -> Vec<(u32, u32)> {
    let w = width.clamp(1, 100);
    if w == 1 {
        return (0..=100).map(|s| (s, s)).collect();
    }
    let n = 100u32.div_ceil(w);
    (0..n).map(|i| (i * w, if i + 1 == n { 100 } else { i * w + w - 1 })).collect()
}

pub fn bin_index(score: u32, bins: &[(u32, u32)]) -> usize {
    bins.iter().position(|&(lo, hi)| score >= lo && score <= hi).unwrap_or(bins.len().saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Question, Score, Student};

    fn r(label: &str, min_score: u32) -> Rating {
        Rating { label: label.to_string(), min_score, max_ratio: None }
    }

    fn result(score: Option<u32>, ratings: &[Rating]) -> StudentResult {
        let order = rating_order(ratings);
        StudentResult {
            idx: 0,
            id: String::new(),
            name: String::new(),
            status: score.map_or(Status::Incomplete, Status::Final),
            rating: score.and_then(|s| rate(s, ratings, &order)),
        }
    }

    #[test]
    fn bands_ignore_display_order_and_count_unrated() {
        // Not sorted: as while the user is still editing
        let mut ratings = vec![r("C", 60), r("A", 80), r("B", 70)];
        ratings[1].max_ratio = Some(20.0);
        let res: Vec<_> = [85, 75, 65, 50].iter().map(|&s| result(Some(s), &ratings))
            .chain([result(None, &ratings)])
            .collect();
        let (bands, unrated) = bands(&ratings, &res);
        let got: Vec<(&str, Option<(u32, u32)>, usize)> =
            bands.iter().map(|b| (ratings[b.rating].label.as_str(), b.range, b.count)).collect();
        assert_eq!(got, vec![("A", Some((80, 100)), 1), ("B", Some((70, 79)), 1), ("C", Some((60, 69)), 1)]);
        assert_eq!(unrated, 1);
        // Incomplete students are not part of the ratio
        assert_eq!(bands[0].ratio, 0.25);
        assert_eq!(bands[2].cumulative, 0.75);
        assert!(bands[0].over_limit);
        assert!(!bands[1].over_limit);
    }

    #[test]
    fn duplicate_thresholds_and_missing_zero_are_warned() {
        let ratings = vec![r("A", 80), r("B", 80), r("C", 60)];
        let res = vec![result(Some(90), &ratings)];
        let (bands, _) = bands(&ratings, &res);
        assert_eq!(bands[0].range, Some((80, 100)));
        assert_eq!(bands[1].range, None);
        assert_eq!(bands[2].range, Some((60, 79)));
        let w = rating_warnings(&ratings);
        assert_eq!(w.len(), 2, "{w:?}");
        assert!(rating_warnings(&[r("A", 50), r("F", 0)]).is_empty());
    }

    #[test]
    fn stats_of_scores() {
        let s = stats(&[60, 80, 70, 90]).unwrap();
        assert_eq!((s.n, s.min, s.max), (4, 60, 90));
        assert_eq!(s.mean, 75.0);
        assert_eq!(s.median, 75.0);
        assert!((s.sd - 125f64.sqrt()).abs() < 1e-9);
        assert_eq!(stats(&[]), None);
    }

    #[test]
    fn bins_cover_0_to_100() {
        let b5 = bins(5);
        assert_eq!(b5.len(), 20);
        assert_eq!((b5[0], b5[19]), ((0, 4), (95, 100)));
        assert_eq!(bin_index(100, &b5), 19);
        assert_eq!(bin_index(4, &b5), 0);
        assert_eq!(bins(1).len(), 101);
        assert_eq!(bins(10)[9], (90, 100));
    }

    #[test]
    fn results_mark_incomplete_and_regrade() {
        let mut cfg = Config::new();
        cfg.questions = vec![Question { id: 1, name: "Q".into(), full_score: 10, weight: 1.0, comment: String::new() }];
        cfg.students = ["A", "B", "C"].iter().map(|id| Student { id: id.to_string(), name: String::new() }).collect();
        cfg.ratings = vec![r("優", 80), r("不可", 0)];
        cfg.set_score("A", 1, Some(9));
        cfg.set_score("C", 1, Some(5));
        cfg.scores.push(Score { student_id: "B".into(), question_id: 1, score: None, graded_full_score: None });
        cfg.questions[0].full_score = 10;
        let res = results(&cfg);
        assert_eq!(res[0].status, Status::Final(90));
        assert_eq!(res[0].rating, Some(0));
        assert_eq!(res[1].status, Status::Incomplete);
        assert_eq!(res[2].rating, Some(1));
        cfg.questions[0].full_score = 8;
        assert_eq!(results(&cfg)[0].status, Status::Regrade);
    }
}
