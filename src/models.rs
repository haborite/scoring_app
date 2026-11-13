use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::fs;
use anyhow::{Result, Context};
use dioxus::prelude::*;

// Serialized form of the last saved/loaded config, used to detect unsaved changes
pub static SAVED_SNAPSHOT: GlobalSignal<String> = Signal::global(|| Config::new().snapshot());

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
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Store)]
pub struct Rating {
    pub label: String,
    pub min_score: u32,
}

// ----------- for UI display -----------

#[derive(Clone)]
pub struct TableRow {
    pub student_id: String,
    pub student_name: String,
    pub scores: Vec<String>,
    pub final_display: String,
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
                    status.set(Status::info(format!("読み込みました: {}", display_name(&path))));
                }
                Ok(None) => {}
                Err(e) => status.set(Status::error(e)),
            }
        });
    }

    // Open filedialog and parse the selected file. Ok(None) if canceled.
    pub async fn pick_file() -> std::result::Result<Option<(PathBuf, Config)>, String> {
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

        serde_json::from_str::<Config>(&txt)
            .map(|cfg| Some((path, cfg)))
            .map_err(|e| format_json_error(&txt, e))
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
        Score { student_id: student_id.to_string(), question_id, score }
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
}
