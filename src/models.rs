use serde::{Serialize, Deserialize};
use std::path::Path;
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

    pub fn new() -> Config {
        Config {
            save_path: None,
            questions: Vec::new(),
            students: Vec::new(),
            scores: Vec::new(),
            ratings: Vec::new(),
        }
    }

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
    
}
