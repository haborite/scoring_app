use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use anyhow::{Result, Context};
use dioxus::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, Store)]
pub struct Config {
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    MasterQuestions,
    MasterStudents,
    Grading,
    Rating,
}

// ----------- implementation --------------

impl Config {

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

    // Save config to a stored filepath
    pub fn save(config: Signal<Config>, mut msg: Signal<String>) {
        let cfg = config().clone();
        let Some(path_str) = cfg.save_path.as_deref() else {
            Config::save_as(config, msg);
            return
        };
        let path = PathBuf::from(path_str);
        spawn(async move {
            msg.set(format!("Saving to {:?} ...", path));
            match cfg.save_to_filepath(&path).await {
                Ok(()) => {
                    msg.set(format!("Saved: {:?}", path));
                }
                Err(e) => {
                    msg.set(format!("Save failed: {:#}", e));
                }
            }
        });
    }

    // Open filedialog, select filepath to be stored, stored to the path, and save the path
    pub fn save_as(mut config: Signal<Config>, mut msg: Signal<String>) {
        spawn(async move {
            let handle = rfd::AsyncFileDialog::new()
                .add_filter("JSON", &["json"])
                .set_file_name("config.json")
                .save_file()
                .await;

            let Some(handle) = handle else {
                return;
            };

            let path: PathBuf = handle.path().to_path_buf();
            let cfg_snapshot = config();
            msg.set(format!("Saving to {:?} ...", path));

            match cfg_snapshot.save_to_filepath(&path).await {
                Ok(()) => {
                    let mut c = config.write();
                    c.save_path = Some(path.to_string_lossy().to_string());
                    msg.set(format!("Saved: {:?}", path));
                }
                Err(e) => {
                    msg.set(format!("Save failed: {:#}", e));
                }
            }
        });
    }

    // Load config from a selected filepath
    pub fn load(mut config: Signal<Config>, mut msg: Signal<String>) {
        spawn(async move {
            let handle = rfd::AsyncFileDialog::new()
                .add_filter("JSON", &["json"])
                .pick_file()
                .await;

            let Some(handle) = handle else {
                return;
            };

            let path = handle.path().to_path_buf();

            let bytes = handle.read().await;

            let txt = match String::from_utf8(bytes) {
                Ok(s) => s,
                Err(e) => {
                    msg.set(format!("Invalid UTF-8: {e}"));
                    return;
                }
            };

            if txt.trim().is_empty() {
                msg.set("Empty file.".to_string());
                return;
            }

            match serde_json::from_str::<Config>(&txt) {
                Ok(mut cfg) => {
                    cfg.save_path = Some(path.to_string_lossy().to_string());
                    config.set(cfg);
                    msg.set("Loaded.".to_string());
                }
                Err(e) => {
                    msg.set(format_json_error(&txt, e));
                }
            }
        });
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


