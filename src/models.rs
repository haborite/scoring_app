use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::fs;
use anyhow::{Result, Context};
use dioxus::prelude::*;

// Serialized form of the last saved/loaded config, used to detect unsaved changes
pub static SAVED_SNAPSHOT: GlobalSignal<String> = Signal::global(|| Config::new().snapshot());

// Bumped whenever the data is replaced from outside the inputs (file load, import).
// Text inputs are uncontrolled while typing and are re-created when this changes.
pub static DATA_GEN: GlobalSignal<u64> = Signal::global(|| 0);

pub fn bump_data_gen() {
    *DATA_GEN.write() += 1;
}

#[derive(Debug, Clone, Serialize, Deserialize, Store)]
#[serde(default)]
pub struct Config {
    // Runtime only: not written to the file so a moved/shared file never points to a stale path
    #[serde(skip)]
    pub save_path: Option<String>,
    pub questions: Vec<Question>,
    pub students: Vec<Student>,
    pub scores: Vec<Score>,
    pub ratings: Vec<Rating>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Store)]
pub struct Question {
    pub id: u32,
    pub name: String,
    pub full_score: u32,
    pub weight: f32,
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Store)]
pub struct Student {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Store)]
pub struct Score {
    pub student_id: String,
    pub question_id: u32,
    pub score: Option<u32>,
    // Full score of the question when this score was entered (or confirmed).
    // None for files written before this was recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graded_full_score: Option<u32>,
}

impl Score {
    // Whether this score has to be looked at again against the question's current full score
    pub fn needs_regrade(&self, full_score: u32) -> bool {
        let Some(scv) = self.score else {
            return false;
        };
        self.graded_full_score.is_some_and(|g| g != full_score) || scv > full_score
    }
}

// Scores that have to be looked at again: (student_id, question_id) -> full score at grading time
pub type RegradeMap<'a> = HashMap<(&'a str, u32), Option<u32>>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Store)]
pub struct Rating {
    pub label: String,
    pub min_score: u32,
    // Guideline upper limit of the share of this rating, in percent. Exceeding it is only warned about.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_ratio: Option<f32>,
}

// One of the lists that can be saved to / loaded from its own file
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Questions,
    Students,
    Ratings,
}

impl Part {
    // Key in the (common) JSON format
    pub fn key(self) -> &'static str {
        match self {
            Part::Questions => "questions",
            Part::Students => "students",
            Part::Ratings => "ratings",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Part::Questions => "問題",
            Part::Students => "受験者",
            Part::Ratings => "成績区分",
        }
    }

    fn len(self, cfg: &Config) -> usize {
        match self {
            Part::Questions => cfg.questions.len(),
            Part::Students => cfg.students.len(),
            Part::Ratings => cfg.ratings.len(),
        }
    }
}

// ----------- for UI display -----------

#[derive(Clone)]
pub struct TableRow {
    pub student_id: String,
    pub student_name: String,
    pub scores: Vec<String>,
    // Per question: Some(full score at grading time) when the score needs regrading
    pub regrade: Vec<Option<Option<u32>>>,
    pub final_display: String,
    pub needs_regrade: bool,
}

// Message shown in the top bar
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Status {
    pub text: String,
    pub error: bool,
}

impl Status {
    pub fn info(text: impl Into<String>) -> Self {
        Status { text: text.into(), error: false }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Status { text: text.into(), error: true }
    }
}

// Keeps keyboard shortcuts from stacking up file dialogs while one is open
static DIALOG_OPEN: AtomicBool = AtomicBool::new(false);

struct DialogGuard;

impl DialogGuard {
    fn acquire() -> Option<Self> {
        (!DIALOG_OPEN.swap(true, Ordering::SeqCst)).then_some(DialogGuard)
    }
}

impl Drop for DialogGuard {
    fn drop(&mut self) {
        DIALOG_OPEN.store(false, Ordering::SeqCst);
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    MasterQuestions,
    MasterStudents,
    Grading,
    Rating,
    Settings,
}

// ----------- implementation --------------

impl Default for Config {
    fn default() -> Self {
        Config::new()
    }
}

impl Config {

    // (student_id, question_id) -> score
    pub fn score_map(&self) -> HashMap<(&str, u32), Option<u32>> {
        self.scores
            .iter()
            .map(|s| ((s.student_id.as_str(), s.question_id), s.score))
            .collect()
    }

    // Weighted final score (0-100, rounded). None if any question is unscored.
    pub fn final_score(
        &self,
        score_map: &HashMap<(&str, u32), Option<u32>>,
        student_id: &str,
    ) -> Option<u32> {
        let total_weight: f32 = self.questions.iter().map(|q| q.weight).sum();
        if self.questions.is_empty() || total_weight <= 0.0 {
            return None;
        }
        let mut weighted_rate_sum: f32 = 0.0;
        for q in self.questions.iter() {
            let scv = score_map.get(&(student_id, q.id)).copied().flatten()?;
            if q.full_score == 0 {
                return None;
            }
            weighted_rate_sum += scv as f32 / q.full_score as f32 * q.weight;
        }
        Some((weighted_rate_sum / total_weight * 100.0).round() as u32)
    }

    // Scores that need regrading: the full score changed since grading, or the score is above it.
    // Derived every time, so changing the full score back clears the mark by itself.
    pub fn regrade_map(&self) -> RegradeMap<'_> {
        let full: HashMap<u32, u32> = self.questions.iter().map(|q| (q.id, q.full_score)).collect();
        self.scores
            .iter()
            .filter(|s| full.get(&s.question_id).is_some_and(|&cur| s.needs_regrade(cur)))
            .map(|s| ((s.student_id.as_str(), s.question_id), s.graded_full_score))
            .collect()
    }

    // Students with at least one score that needs regrading
    pub fn regrade_students<'a>(regrade: &RegradeMap<'a>) -> HashSet<&'a str> {
        regrade.keys().map(|(sid, _)| *sid).collect()
    }

    // Store a score and remember the full score it was given against
    pub fn set_score(&mut self, student_id: &str, question_id: u32, score: Option<u32>) {
        let full = self.questions.iter().find(|q| q.id == question_id).map(|q| q.full_score);
        let graded_full_score = score.and(full);
        match self.scores.iter_mut()
            .find(|sc| sc.question_id == question_id && sc.student_id == student_id)
        {
            Some(sc) => {
                sc.score = score;
                sc.graded_full_score = graded_full_score;
            }
            None => self.scores.push(Score {
                student_id: student_id.to_string(),
                question_id,
                score,
                graded_full_score,
            }),
        }
    }

    // Keep the score as it is and accept it for the current full score.
    // false if there is nothing to confirm or the score is above the full score.
    pub fn confirm_score(&mut self, student_id: &str, question_id: u32) -> bool {
        let Some(full) = self.questions.iter().find(|q| q.id == question_id).map(|q| q.full_score) else {
            return false;
        };
        match self.scores.iter_mut()
            .find(|sc| sc.question_id == question_id && sc.student_id == student_id)
        {
            Some(sc) if sc.score.is_some_and(|v| v <= full) && sc.graded_full_score != Some(full) => {
                sc.graded_full_score = Some(full);
                true
            }
            _ => false,
        }
    }

    // Next student (after `from`, wrapping around) that needs regrading, with the first such question index
    pub fn next_regrade(&self, from: usize) -> Option<(usize, usize)> {
        let regrade = self.regrade_map();
        let n = self.students.len();
        (1..=n).map(|k| (from + k) % n).find_map(|i| {
            let sid = self.students[i].id.as_str();
            self.questions.iter()
                .position(|q| regrade.contains_key(&(sid, q.id)))
                .map(|qi| (i, qi))
        })
    }

    // Change a student id and carry its scores over. Err if the new id is empty or taken.
    pub fn rename_student(&mut self, idx: usize, new_id: &str) -> Result<(), String> {
        let new_id = new_id.trim();
        let Some(old_id) = self.students.get(idx).map(|s| s.id.clone()) else {
            return Err("Student not found".to_string());
        };
        if new_id == old_id {
            return Ok(());
        }
        if new_id.is_empty() {
            return Err("学籍番号は空にできません".to_string());
        }
        if self.students.iter().any(|s| s.id == new_id) {
            return Err(format!("学籍番号 {new_id} は既に使われています"));
        }
        let old_id_shared = self.students.iter().filter(|s| s.id == old_id).count() > 1;
        if !old_id_shared {
            for sc in self.scores.iter_mut().filter(|sc| sc.student_id == old_id) {
                sc.student_id = new_id.to_string();
            }
        }
        self.students[idx].id = new_id.to_string();
        Ok(())
    }

    // Change a question id and carry its scores over. Err if the new id is taken.
    pub fn rename_question(&mut self, idx: usize, new_id: u32) -> Result<(), String> {
        let Some(old_id) = self.questions.get(idx).map(|q| q.id) else {
            return Err("Question not found".to_string());
        };
        if new_id == old_id {
            return Ok(());
        }
        if self.questions.iter().any(|q| q.id == new_id) {
            return Err(format!("問題 id {new_id} は既に使われています"));
        }
        let old_id_shared = self.questions.iter().filter(|q| q.id == old_id).count() > 1;
        if !old_id_shared {
            for sc in self.scores.iter_mut().filter(|sc| sc.question_id == old_id) {
                sc.question_id = new_id;
            }
        }
        self.questions[idx].id = new_id;
        Ok(())
    }

    // JSON holding only one part, in the common format: {"questions": [...]}
    pub fn part_json(&self, part: Part) -> Result<String> {
        let value = match part {
            Part::Questions => serde_json::to_value(&self.questions),
            Part::Students => serde_json::to_value(&self.students),
            Part::Ratings => serde_json::to_value(&self.ratings),
        }?;
        let mut map = serde_json::Map::new();
        map.insert(part.key().to_string(), value);
        Ok(serde_json::to_string_pretty(&serde_json::Value::Object(map))?)
    }

    // Read one part from a file in the common format (a whole saved file is fine too).
    // Err if the file does not contain that part, so that nothing is replaced by mistake.
    pub fn parse_part(txt: &str, part: Part) -> std::result::Result<Config, String> {
        let txt = txt.trim_start_matches('\u{feff}');
        let value: serde_json::Value = serde_json::from_str(txt).map_err(|e| format_json_error(txt, e))?;
        if value.get(part.key()).is_none() {
            return Err(format!("このファイルには {}（{}）が含まれていません", part.label(), part.key()));
        }
        serde_json::from_value::<Config>(value).map_err(|e| format!("{}（{}）を読み込めませんでした: {e}", part.label(), part.key()))
    }

    // Replace one part with the one in `from`
    pub fn replace_part(&mut self, from: Config, part: Part) {
        match part {
            Part::Questions => self.questions = from.questions,
            Part::Students => self.students = from.students,
            Part::Ratings => {
                self.ratings = from.ratings;
                self.ratings.sort_by(|a, b| b.min_score.cmp(&a.min_score));
            }
        }
    }

    // Entered scores that would no longer belong to any question/student after replacing a part
    pub fn scores_lost_by(&self, from: &Config, part: Part) -> usize {
        let entered = self.scores.iter().filter(|s| s.score.is_some());
        match part {
            Part::Questions => {
                let ids: HashSet<u32> = from.questions.iter().map(|q| q.id).collect();
                let current: HashSet<u32> = self.questions.iter().map(|q| q.id).collect();
                entered.filter(|s| current.contains(&s.question_id) && !ids.contains(&s.question_id)).count()
            }
            Part::Students => {
                let ids: HashSet<&str> = from.students.iter().map(|s| s.id.as_str()).collect();
                let current: HashSet<&str> = self.students.iter().map(|s| s.id.as_str()).collect();
                entered.filter(|s| current.contains(s.student_id.as_str()) && !ids.contains(s.student_id.as_str())).count()
            }
            Part::Ratings => 0,
        }
    }

    // Save one part to a file chosen by the user. The result is reported through `msg`.
    pub fn save_part(config: Signal<Config>, part: Part, mut msg: Signal<String>) {
        let Some(guard) = DialogGuard::acquire() else {
            return;
        };
        spawn(async move {
            let handle = rfd::AsyncFileDialog::new()
                .add_filter("JSON", &["json"])
                .set_file_name(format!("{}.json", part.key()))
                .save_file()
                .await;
            drop(guard);
            let Some(handle) = handle else {
                return;
            };
            let path = handle.path().to_path_buf();
            let result = match config.peek().part_json(part) {
                Ok(json) => fs::write(&path, json).await.map_err(anyhow::Error::from),
                Err(e) => Err(e),
            };
            match result {
                Ok(()) => msg.set(format!(
                    "{}（{} 件）を保存しました: {}",
                    part.label(),
                    part.len(&config.peek()),
                    display_name(&path)
                )),
                Err(e) => msg.set(format!("保存に失敗しました: {e:#}")),
            }
        });
    }

    // Replace one part with the one in a file chosen by the user, after confirming
    pub fn load_part(mut config: Signal<Config>, part: Part, mut msg: Signal<String>) {
        spawn(async move {
            let (path, txt) = match Config::pick_text().await {
                Ok(Some(v)) => v,
                Ok(None) => return,
                Err(e) => {
                    msg.set(e);
                    return;
                }
            };
            let from = match Config::parse_part(&txt, part) {
                Ok(c) => c,
                Err(e) => {
                    msg.set(format!("{}: {e}", display_name(&path)));
                    return;
                }
            };
            let current = part.len(&config.peek());
            if current > 0 {
                let lost = config.peek().scores_lost_by(&from, part);
                let mut text = format!(
                    "今の{} {} 件を、{} の {} 件で置き換えます。",
                    part.label(),
                    current,
                    display_name(&path),
                    part.len(&from),
                );
                if lost > 0 {
                    text.push_str(&format!(
                        "\n\n入力済みの点数のうち {lost} 件は、対応する{}がなくなるため表示されなくなります（{}を元に戻せば再び表示されます）。",
                        part.label(),
                        part.label(),
                    ));
                }
                let answer = rfd::AsyncMessageDialog::new()
                    .set_level(if lost > 0 { rfd::MessageLevel::Warning } else { rfd::MessageLevel::Info })
                    .set_title("Scoring App")
                    .set_description(text)
                    .set_buttons(rfd::MessageButtons::OkCancel)
                    .show()
                    .await;
                if answer != rfd::MessageDialogResult::Ok {
                    return;
                }
            }
            let count = part.len(&from);
            config.write().replace_part(from, part);
            bump_data_gen();
            msg.set(format!("{}を {} 件読み込みました: {}", part.label(), count, display_name(&path)));
        });
    }

    // Create blunk config
    pub fn new() -> Config {
        Config {
            save_path: None,
            questions: Vec::new(),
            students: Vec::new(),
            scores: Vec::new(),
            ratings: Vec::new(),
        }
    }

    // Compact JSON used only for change detection (save_path is not serialized)
    pub fn snapshot(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    // Whether there are changes since the last save/load
    pub fn is_dirty(&self) -> bool {
        self.snapshot() != *SAVED_SNAPSHOT.read()
    }

    // Record this config as the saved state
    pub fn mark_saved(&self) {
        *SAVED_SNAPSHOT.write() = self.snapshot();
    }

    // Save config to a given filepath
    pub async fn save_to_filepath<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .await
                    .with_context(|| format!("Failed to create directory: {:?}", parent))?;
            }
        }
        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize Config to JSON")?;
        fs::write(path, json)
            .await
            .with_context(|| format!("Failed to write file: {:?}", path))?;
        Ok(())
    }

    // Blocking save for use while the window is closing, where nothing can be awaited.
    // Asks for a path if none is stored. Ok(false) if the dialog was canceled.
    pub fn save_blocking(&mut self) -> Result<bool> {
        let path = match self.save_path.as_deref() {
            Some(p) => PathBuf::from(p),
            None => {
                let picked = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .set_file_name("config.json")
                    .save_file();
                let Some(p) = picked else {
                    return Ok(false);
                };
                p
            }
        };
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {:?}", parent))?;
            }
        }
        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize Config to JSON")?;
        std::fs::write(&path, json)
            .with_context(|| format!("Failed to write file: {:?}", path))?;
        self.save_path = Some(path.to_string_lossy().to_string());
        self.mark_saved();
        Ok(true)
    }

    // Save config to a stored filepath
    pub fn save(config: Signal<Config>, mut status: Signal<Status>) {
        let cfg = config();
        let Some(path_str) = cfg.save_path.as_deref() else {
            Config::save_as(config, status);
            return
        };
        let path = PathBuf::from(path_str);
        spawn(async move {
            match cfg.save_to_filepath(&path).await {
                Ok(()) => {
                    cfg.mark_saved();
                    status.set(Status::info(format!("保存しました: {}", display_name(&path))));
                }
                Err(e) => status.set(Status::error(format!("保存に失敗しました: {e:#}"))),
            }
        });
    }

    // Open filedialog, select filepath to be stored, stored to the path, and save the path
    pub fn save_as(mut config: Signal<Config>, mut status: Signal<Status>) {
        let Some(guard) = DialogGuard::acquire() else {
            return;
        };
        spawn(async move {
            let handle = rfd::AsyncFileDialog::new()
                .add_filter("JSON", &["json"])
                .set_file_name("config.json")
                .save_file()
                .await;
            drop(guard);

            let Some(handle) = handle else {
                return;
            };

            let path: PathBuf = handle.path().to_path_buf();
            let cfg_snapshot = config();

            match cfg_snapshot.save_to_filepath(&path).await {
                Ok(()) => {
                    cfg_snapshot.mark_saved();
                    let mut c = config.write();
                    c.save_path = Some(path.to_string_lossy().to_string());
                    status.set(Status::info(format!("保存しました: {}", display_name(&path))));
                }
                Err(e) => status.set(Status::error(format!("保存に失敗しました: {e:#}"))),
            }
        });
    }

    // Load config from a selected filepath
    pub fn load(mut config: Signal<Config>, mut status: Signal<Status>) {
        spawn(async move {
            if config.peek().is_dirty() {
                let answer = rfd::AsyncMessageDialog::new()
                    .set_level(rfd::MessageLevel::Warning)
                    .set_title("Scoring App")
                    .set_description("保存していない変更があります。変更を破棄して別のファイルを読み込みますか？")
                    .set_buttons(rfd::MessageButtons::OkCancel)
                    .show()
                    .await;
                if answer != rfd::MessageDialogResult::Ok {
                    return;
                }
            }
            match Config::pick_file().await {
                Ok(Some((path, mut cfg))) => {
                    cfg.save_path = Some(path.to_string_lossy().to_string());
                    cfg.mark_saved();
                    config.set(cfg);
                    bump_data_gen();
                    status.set(Status::info(format!("読み込みました: {}", display_name(&path))));
                }
                Ok(None) => {}
                Err(e) => status.set(Status::error(e)),
            }
        });
    }

    // Open filedialog and parse the selected file. Ok(None) if canceled.
    pub async fn pick_file() -> std::result::Result<Option<(PathBuf, Config)>, String> {
        let Some((path, txt)) = Config::pick_text().await? else {
            return Ok(None);
        };
        serde_json::from_str::<Config>(&txt)
            .map(|cfg| Some((path, cfg)))
            .map_err(|e| format_json_error(&txt, e))
    }

    // Open filedialog and read the selected JSON file as text. Ok(None) if canceled.
    async fn pick_text() -> std::result::Result<Option<(PathBuf, String)>, String> {
        let Some(guard) = DialogGuard::acquire() else {
            return Ok(None);
        };
        let handle = rfd::AsyncFileDialog::new()
            .add_filter("JSON", &["json"])
            .pick_file()
            .await;
        drop(guard);

        let Some(handle) = handle else {
            return Ok(None);
        };

        let path = handle.path().to_path_buf();
        let bytes = handle.read().await;
        let txt = String::from_utf8(bytes).map_err(|e| format!("Invalid UTF-8: {e}"))?;

        if txt.trim().is_empty() {
            return Err("Empty file.".to_string());
        }
        Ok(Some((path, txt)))
    }

}

fn format_json_error(src: &str, e: serde_json::Error) -> String {
    let (line, col) = (e.line(), e.column());

    let kind = if e.is_syntax() {
        "JSON syntax error"
    } else if e.is_data() {
        "JSON structure/type mismatch"
    } else if e.is_eof() {
        "Unexpected end of file"
    } else {
        "JSON parse error"
    };

    let line_text = src.lines().nth(line.saturating_sub(1)).unwrap_or("");
    let caret_pad = " ".repeat(col.saturating_sub(1));
    let caret = format!("{caret_pad}^");

    let hint = if e.is_data() {
        "Hint: field name misspelled? wrong type? missing field?"
    } else {
        ""
    };

    if line > 0 {
        format!(
            "{kind} at line {line}, column {col}: {e}\n\
             {line_text}\n\
             {caret}\n\
             {hint}"
        )
    } else {
        format!("{kind}: {e}\n{hint}")
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    fn q(id: u32, full_score: u32, weight: f32) -> Question {
        Question { id, name: format!("Q{id}"), full_score, weight, comment: String::new() }
    }

    fn sc(student_id: &str, question_id: u32, score: Option<u32>) -> Score {
        Score { student_id: student_id.to_string(), question_id, score, graded_full_score: None }
    }

    fn st(id: &str) -> Student {
        Student { id: id.to_string(), name: String::new() }
    }

    #[test]
    fn final_score_is_weighted_and_rounded() {
        let mut cfg = Config::new();
        cfg.questions = vec![q(1, 10, 0.1), q(2, 10, 0.1), q(3, 10, 0.1)];
        cfg.scores = vec![sc("A", 1, Some(10)), sc("A", 2, Some(10)), sc("A", 3, Some(10))];
        // 0.1 weights accumulate float error; must still be exactly 100
        assert_eq!(cfg.final_score(&cfg.score_map(), "A"), Some(100));

        cfg.questions = vec![q(1, 3, 1.0)];
        cfg.scores = vec![sc("A", 1, Some(2))]; // 66.67
        assert_eq!(cfg.final_score(&cfg.score_map(), "A"), Some(67));
    }

    #[test]
    fn final_score_none_when_incomplete() {
        let mut cfg = Config::new();
        cfg.questions = vec![q(1, 10, 1.0), q(2, 10, 1.0)];
        cfg.scores = vec![sc("A", 1, Some(5)), sc("A", 2, None)];
        assert_eq!(cfg.final_score(&cfg.score_map(), "A"), None);
        assert_eq!(cfg.final_score(&cfg.score_map(), "B"), None);
    }

    #[test]
    fn rename_student_moves_scores_and_rejects_duplicates() {
        let mut cfg = Config::new();
        cfg.students = vec![st("A1"), st("A2")];
        cfg.scores = vec![sc("A1", 1, Some(3))];

        assert!(cfg.rename_student(0, "A2").is_err());
        assert!(cfg.rename_student(0, "  ").is_err());
        assert_eq!(cfg.students[0].id, "A1");

        cfg.rename_student(0, "B1").unwrap();
        assert_eq!(cfg.students[0].id, "B1");
        assert_eq!(cfg.scores[0].student_id, "B1");
    }

    #[test]
    fn rename_question_moves_scores_and_rejects_duplicates() {
        let mut cfg = Config::new();
        cfg.questions = vec![q(1, 10, 1.0), q(2, 10, 1.0)];
        cfg.scores = vec![sc("A", 1, Some(3))];

        assert!(cfg.rename_question(0, 2).is_err());
        cfg.rename_question(0, 5).unwrap();
        assert_eq!(cfg.questions[0].id, 5);
        assert_eq!(cfg.scores[0].question_id, 5);
    }

    #[test]
    fn deserialize_partial_file_and_skip_save_path() {
        let cfg: Config = serde_json::from_str(r#"{"students":[{"id":"A1","name":"x"}],"save_path":"C:/old.json"}"#).unwrap();
        assert_eq!(cfg.students.len(), 1);
        assert!(cfg.questions.is_empty());
        assert_eq!(cfg.save_path, None);

        let mut cfg = Config::new();
        cfg.save_path = Some("C:/x.json".to_string());
        assert!(!serde_json::to_string(&cfg).unwrap().contains("save_path"));
    }

    #[test]
    fn regrade_follows_full_score_changes_and_reverts() {
        let mut cfg = Config::new();
        cfg.questions = vec![q(1, 5, 1.0), q(2, 5, 1.0)];
        cfg.students = vec![st("A"), st("B")];
        cfg.set_score("A", 1, Some(4));
        cfg.set_score("A", 2, Some(5));
        assert!(cfg.regrade_map().is_empty());

        cfg.questions[0].full_score = 8;
        {
            let map = cfg.regrade_map();
            assert_eq!(map.get(&("A", 1)), Some(&Some(5)));
            assert_eq!(map.len(), 1);
            let students = Config::regrade_students(&map);
            assert!(students.contains("A"));
        }

        // Back to the value at grading time: no longer flagged
        cfg.questions[0].full_score = 5;
        assert!(cfg.regrade_map().is_empty());

        // Confirming accepts the new full score
        cfg.questions[0].full_score = 8;
        assert!(cfg.confirm_score("A", 1));
        assert!(cfg.regrade_map().is_empty());
        assert_eq!(cfg.scores[0].graded_full_score, Some(8));
    }

    #[test]
    fn regrade_rules_for_unscored_old_and_too_high_scores() {
        let mut cfg = Config::new();
        cfg.questions = vec![q(1, 10, 1.0)];
        // Old file: no grading-time full score, flagged only when the score is too high
        cfg.scores = vec![sc("A", 1, Some(7)), sc("B", 1, None)];
        cfg.questions[0].full_score = 6;
        {
            let map = cfg.regrade_map();
            assert!(map.contains_key(&("A", 1)));
            assert!(!map.contains_key(&("B", 1)));
        }
        // A score above the full score cannot be confirmed
        assert!(!cfg.confirm_score("A", 1));
        cfg.questions[0].full_score = 7;
        assert!(cfg.regrade_map().is_empty());
        // Clearing a score stores no grading-time full score
        cfg.set_score("A", 1, None);
        assert_eq!(cfg.scores[0].graded_full_score, None);
    }

    #[test]
    fn next_regrade_wraps_around() {
        let mut cfg = Config::new();
        cfg.questions = vec![q(1, 5, 1.0), q(2, 5, 1.0)];
        cfg.students = vec![st("A"), st("B"), st("C")];
        cfg.set_score("A", 2, Some(3));
        cfg.set_score("C", 1, Some(3));
        assert_eq!(cfg.next_regrade(0), None);
        cfg.questions[1].full_score = 6;
        cfg.questions[0].full_score = 6;
        assert_eq!(cfg.next_regrade(0), Some((2, 0)));
        assert_eq!(cfg.next_regrade(2), Some((0, 1)));
    }

    #[test]
    fn part_round_trip_and_missing_part() {
        let mut cfg = Config::new();
        cfg.questions = vec![q(1, 10, 1.0)];
        cfg.students = vec![st("A")];
        cfg.scores = vec![sc("A", 1, Some(3))];
        cfg.ratings = vec![Rating { label: "可".into(), min_score: 60, max_ratio: None }];

        let json = cfg.part_json(Part::Ratings).unwrap();
        assert!(json.contains("ratings") && !json.contains("questions") && !json.contains("students"));
        let back = Config::parse_part(&json, Part::Ratings).unwrap();
        assert_eq!(back.ratings, cfg.ratings);

        // A ratings-only file must not empty the question list
        assert!(Config::parse_part(&json, Part::Questions).unwrap_err().contains("questions"));
        // A whole saved file works for any part
        let whole = serde_json::to_string(&cfg).unwrap();
        assert_eq!(Config::parse_part(&whole, Part::Students).unwrap().students, cfg.students);
    }

    #[test]
    fn replacing_parts_reports_lost_scores() {
        let mut cfg = Config::new();
        cfg.questions = vec![q(1, 10, 1.0), q(2, 10, 1.0)];
        cfg.students = vec![st("A"), st("B")];
        cfg.scores = vec![sc("A", 1, Some(3)), sc("A", 2, Some(4)), sc("B", 2, None)];

        let mut from = Config::new();
        from.questions = vec![q(1, 5, 1.0)];
        assert_eq!(cfg.scores_lost_by(&from, Part::Questions), 1);
        from.students = vec![st("B")];
        assert_eq!(cfg.scores_lost_by(&from, Part::Students), 2);

        from.ratings = vec![
            Rating { label: "可".into(), min_score: 60, max_ratio: None },
            Rating { label: "優".into(), min_score: 80, max_ratio: Some(30.0) },
        ];
        cfg.replace_part(from, Part::Ratings);
        assert_eq!(cfg.ratings[0].label, "優");
        assert_eq!(cfg.questions.len(), 2);
    }
}
