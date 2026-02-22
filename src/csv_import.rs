use anyhow::{Context, Result};
use csv::ReaderBuilder;
use std::path::Path;

use crate::db::{QuestionRow, StudentRow};

pub fn read_questions_csv(path: &Path) -> Result<Vec<QuestionRow>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_path(path)
        .with_context(|| format!("failed to open questions csv: {}", path.display()))?;

    let mut out = vec![];
    for (i, rec) in rdr.records().enumerate() {
        let rec = rec.with_context(|| format!("csv read error at line {}", i + 2))?;
        let id = rec.get(0).context("missing id")?.to_string();
        let name = rec.get(1).context("missing name")?.to_string();
        let full_score: i64 = rec.get(2).context("missing full_score")?.parse()
            .with_context(|| format!("invalid full_score at line {}", i + 2))?;
        let weight: f64 = rec.get(3).context("missing weight")?.parse()
            .with_context(|| format!("invalid weight at line {}", i + 2))?;
        let comment: String = rec.get(4).context("")?.parse()
            .with_context(|| format!("invalid comment at line {}", i + 2))?;
        out.push(QuestionRow { id, name, full_score, weight, comment });
    }
    Ok(out)
}

pub fn read_students_csv(path: &Path) -> Result<Vec<StudentRow>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_path(path)
        .with_context(|| format!("failed to open students csv: {}", path.display()))?;

    let mut out = vec![];
    for (i, rec) in rdr.records().enumerate() {
        let rec = rec.with_context(|| format!("csv read error at line {}", i + 2))?;
        let id = rec.get(0).context("missing id")?.to_string();
        let name = rec.get(1).context("missing name")?.to_string();
        out.push(StudentRow { id, name });
    }
    Ok(out)
}

/// プレビュー用: 先頭n件だけ返す（UIで表示しやすい）
pub fn preview_questions(rows: &[QuestionRow], n: usize) -> Vec<QuestionRow> {
    rows.iter().take(n).cloned().collect()
}
pub fn preview_students(rows: &[StudentRow], n: usize) -> Vec<StudentRow> {
    rows.iter().take(n).cloned().collect()
}
