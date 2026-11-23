// Export of the results as CSV, in formats described by a user-definable `ExportFormat`.
//
// Formats come from two places, like the import formats:
// - built-in formats (a plain list, UTAS, UTOL assignments)
// - `csv_export_formats.json` in the app config directory, a JSON array of `ExportFormat`
//
// Two modes:
// - "new": write a new file with the given columns, one row per student
// - "fill": write values into a CSV downloaded from another system (e.g. the UTAS roster),
//   matching students by id and leaving everything else of the file as it is

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use crate::csv_import::{self, CsvDirection, CsvFormat, Detected};
use crate::models::Config;
use crate::report::{self, StudentResult};

const USER_FORMATS_FILE: &str = "csv_export_formats.json";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExportMode {
    #[default]
    New,
    Fill,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum OutEncoding {
    #[serde(rename = "utf-8")]
    Utf8,
    // With a BOM, so that Excel reads it as UTF-8
    #[default]
    #[serde(rename = "utf-8-bom")]
    Utf8Bom,
    #[serde(rename = "shift_jis")]
    ShiftJis,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LineEnding {
    #[default]
    Crlf,
    Lf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ExportFormat {
    pub name: String,
    pub description: String,
    pub mode: ExportMode,
    // fill: name of the import format used to read the file to fill in (header position, id column)
    pub read_as: String,
    // new: how the file is written (fill keeps the encoding and line breaks of the original file)
    pub encoding: OutEncoding,
    pub delimiter: char,
    pub line_ending: LineEnding,
    pub columns: Vec<ExportColumn>,
}

impl Default for ExportFormat {
    fn default() -> Self {
        ExportFormat {
            name: String::new(),
            description: String::new(),
            mode: ExportMode::New,
            read_as: String::new(),
            encoding: OutEncoding::Utf8Bom,
            delimiter: ',',
            line_ending: LineEnding::Crlf,
            columns: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct ExportColumn {
    // new: header written; fill: header of the column to write into
    pub header: String,
    // "id", "name", "score", "rating", "q:<question id>" or "literal:<text>"
    pub value: String,
    // Replaces values: exact match first, then "*" for any non-empty value
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub map: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
enum Source {
    Id,
    Name,
    Score,
    Rating,
    Question(u32),
    Literal(String),
}

impl Source {
    fn parse(s: &str) -> Result<Source, String> {
        let s = s.trim();
        if let Some(text) = s.strip_prefix("literal:") {
            return Ok(Source::Literal(text.to_string()));
        }
        if let Some(id) = s.strip_prefix("q:") {
            return id.trim().parse().map(Source::Question)
                .map_err(|_| format!("\"{s}\": q: の後には問題 id（整数）を書いてください"));
        }
        match s {
            "id" => Ok(Source::Id),
            "name" => Ok(Source::Name),
            "score" => Ok(Source::Score),
            "rating" => Ok(Source::Rating),
            _ => Err(format!("value \"{s}\" は使えません（id, name, score, rating, q:<問題id>, literal:<文字列>）")),
        }
    }

    // Depends on the student's result, so may be blank while grading is not finished
    fn is_result(&self) -> bool {
        matches!(self, Source::Score | Source::Rating)
    }
}

pub fn builtin_formats() -> Vec<ExportFormat> {
    let col = |header: &str, value: &str| ExportColumn {
        header: header.to_string(),
        value: value.to_string(),
        map: BTreeMap::new(),
    };
    vec![
        ExportFormat {
            name: "成績一覧".to_string(),
            description: "id, name, score, rating の 4 列の CSV を新しく作ります".to_string(),
            columns: vec![col("id", "id"), col("name", "name"), col("score", "score"), col("rating", "rating")],
            ..ExportFormat::default()
        },
        ExportFormat {
            name: "UTAS".to_string(),
            description: "UTAS の履修者名簿 CSV の「評語」列に区分を書き込みます".to_string(),
            mode: ExportMode::Fill,
            read_as: "UTAS".to_string(),
            columns: vec![col("評語", "rating")],
            ..ExportFormat::default()
        },
        ExportFormat {
            name: "UTOL 課題".to_string(),
            description: "UTOL の課題提出状況 CSV の「評価」列に最終得点を書き込みます".to_string(),
            mode: ExportMode::Fill,
            read_as: "UTOL 課題".to_string(),
            columns: vec![col("評価", "score")],
            ..ExportFormat::default()
        },
    ]
}

pub fn user_formats_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("scoring_app").join(USER_FORMATS_FILE))
}

pub fn load_formats() -> (Vec<ExportFormat>, Vec<String>) {
    csv_import::load_with_user_file(builtin_formats(), user_formats_path(), |f| f.name.clone(), ExportFormat::validate)
}

pub fn ensure_user_formats_file() -> Result<PathBuf, String> {
    let mut map = BTreeMap::new();
    map.insert("不可".to_string(), "否".to_string());
    map.insert("*".to_string(), "合".to_string());
    let example = ExportFormat {
        name: "カスタム形式の例".to_string(),
        description: "列の並びと値の置き換えの例。自由に編集・追加してください".to_string(),
        encoding: OutEncoding::ShiftJis,
        columns: vec![
            ExportColumn { header: "学生証番号".into(), value: "id".into(), map: BTreeMap::new() },
            ExportColumn { header: "氏名".into(), value: "name".into(), map: BTreeMap::new() },
            ExportColumn { header: "得点".into(), value: "score".into(), map: BTreeMap::new() },
            ExportColumn { header: "評語".into(), value: "rating".into(), map: BTreeMap::new() },
            ExportColumn { header: "合否".into(), value: "rating".into(), map },
        ],
        ..ExportFormat::default()
    };
    csv_import::ensure_user_file(user_formats_path(), &[example])
}

// What an export did, for the confirmation and the message afterwards
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExportReport {
    // Students written
    pub written: usize,
    // Students written with a blank score/rating (not graded to the end, or to be regraded)
    pub blank: Vec<String>,
    // fill: students of the app that are not in the file
    pub not_in_file: Vec<String>,
    // fill: ids in the file that are not students of the app (left as they are)
    pub not_in_app: Vec<String>,
}

impl ExportReport {
    pub fn summary(&self) -> String {
        let mut lines = vec![format!("{} 人分を書き出しました。", self.written)];
        let list = |ids: &[String]| {
            let shown: Vec<&str> = ids.iter().take(10).map(String::as_str).collect();
            let more = if ids.len() > 10 { format!(" ほか {} 人", ids.len() - 10) } else { String::new() };
            format!("{}{more}", shown.join(", "))
        };
        if !self.blank.is_empty() {
            lines.push(format!("空欄（未完了・要再採点・区分なし）: {} 人 — {}", self.blank.len(), list(&self.blank)));
        }
        if !self.not_in_file.is_empty() {
            lines.push(format!("ファイルにない受験者（書き出していません）: {} 人 — {}", self.not_in_file.len(), list(&self.not_in_file)));
        }
        if !self.not_in_app.is_empty() {
            lines.push(format!("受験者一覧にない行（変更していません）: {} 行 — {}", self.not_in_app.len(), list(&self.not_in_app)));
        }
        lines.join("\n")
    }
}

// Values of one student, per source
struct Values<'a> {
    cfg: &'a Config,
    scores: HashMap<(&'a str, u32), u32>,
}

impl<'a> Values<'a> {
    fn new(cfg: &'a Config) -> Self {
        let scores = cfg.scores.iter()
            .filter_map(|s| s.score.map(|v| ((s.student_id.as_str(), s.question_id), v)))
            .collect();
        Values { cfg, scores }
    }

    fn get(&self, r: &StudentResult, source: &Source) -> String {
        match source {
            Source::Id => r.id.clone(),
            Source::Name => r.name.clone(),
            Source::Score => r.score().map(|s| s.to_string()).unwrap_or_default(),
            Source::Rating => r.rating.map(|i| self.cfg.ratings[i].label.clone()).unwrap_or_default(),
            Source::Question(q) => self.scores.get(&(r.id.as_str(), *q)).map(|v| v.to_string()).unwrap_or_default(),
            Source::Literal(t) => t.clone(),
        }
    }
}

fn apply_map(value: String, map: &BTreeMap<String, String>) -> String {
    if let Some(v) = map.get(&value) {
        return v.clone();
    }
    match map.get("*") {
        Some(v) if !value.is_empty() => v.clone(),
        _ => value,
    }
}

impl ExportFormat {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("name が空です".to_string());
        }
        if self.columns.is_empty() {
            return Err("columns が空です".to_string());
        }
        for c in &self.columns {
            Source::parse(&c.value)?;
            if self.mode == ExportMode::Fill && c.header.trim().is_empty() {
                return Err("fill では columns の header（書き込む列の見出し）が必要です".to_string());
            }
        }
        if self.mode == ExportMode::Fill && self.read_as.trim().is_empty() {
            return Err("fill では read_as（元のファイルを読む CSV フォーマットの名前）が必要です".to_string());
        }
        if !self.delimiter.is_ascii() {
            return Err("delimiter には半角文字 1 文字を指定してください".to_string());
        }
        Ok(())
    }

    fn sources(&self) -> Vec<(Source, &ExportColumn)> {
        self.columns.iter().filter_map(|c| Source::parse(&c.value).ok().map(|s| (s, c))).collect()
    }

    // Values of the columns for one student, and whether a result column came out blank
    fn row(&self, values: &Values, r: &StudentResult) -> (Vec<String>, bool) {
        let mut blank = false;
        let cells = self.sources().into_iter().map(|(src, col)| {
            let v = apply_map(values.get(r, &src), &col.map);
            if src.is_result() && v.is_empty() {
                blank = true;
            }
            v
        }).collect();
        (cells, blank)
    }

    // A new file with a header row and one row per student
    pub fn export_new(&self, cfg: &Config) -> Result<(Vec<u8>, ExportReport), String> {
        self.validate()?;
        let values = Values::new(cfg);
        let mut report = ExportReport::default();
        let mut w = csv::WriterBuilder::new()
            .delimiter(self.delimiter as u8)
            .terminator(match self.line_ending {
                LineEnding::Crlf => csv::Terminator::CRLF,
                LineEnding::Lf => csv::Terminator::Any(b'\n'),
            })
            .from_writer(Vec::new());
        let err = |e: csv::Error| format!("CSV を作れませんでした: {e}");
        w.write_record(self.columns.iter().map(|c| c.header.as_str())).map_err(err)?;
        for r in report::results(cfg) {
            let (cells, blank) = self.row(&values, &r);
            if blank {
                report.blank.push(r.id.clone());
            }
            w.write_record(&cells).map_err(err)?;
            report.written += 1;
        }
        let bytes = w.into_inner().map_err(|e| format!("CSV を作れませんでした: {e}"))?;
        let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        let encoding = match self.encoding {
            OutEncoding::Utf8 => Detected::Utf8 { bom: false },
            OutEncoding::Utf8Bom => Detected::Utf8 { bom: true },
            OutEncoding::ShiftJis => Detected::ShiftJis,
        };
        Ok((csv_import::encode(&text, encoding)?, report))
    }

    // Write the values into `original`, read with the import format `read_as`.
    // Only the target cells of matched rows change; other text is kept byte for byte.
    pub fn export_fill(&self, cfg: &Config, original: &[u8], import_formats: &[CsvFormat]) -> Result<(Vec<u8>, ExportReport), String> {
        self.validate()?;
        let reader = import_formats.iter().find(|f| f.name == self.read_as)
            .ok_or_else(|| format!("CSV フォーマット「{}」が見つかりません（read_as）", self.read_as))?;
        if reader.direction != CsvDirection::Rows {
            return Err("書き込みは 1 行に 1 人の CSV（direction = rows）にだけ対応しています".to_string());
        }
        let (text, encoding) = csv_import::decode_detect(original, reader.encoding)?;
        let records = reader.read_records(&text)?;

        // Counted as when importing (empty lines are not records)
        let mut iter = records.iter().skip(reader.skip_before_header);
        let header = iter.next().ok_or("見出し行が見つかりません")?;
        let header_cells: Vec<String> = header.cells.iter().map(|c| c.trim().to_string()).collect();
        let find = |name: &str| header_cells.iter().position(|h| h == name.trim());
        let id_idx = find(&reader.id_header)
            .ok_or_else(|| format!("見出し「{}」が見つかりません。見出し行: {}", reader.id_header, header_cells.join(", ")))?;
        let targets: Vec<usize> = self.columns.iter()
            .map(|c| find(&c.header).ok_or_else(|| format!("書き込む列「{}」が見つかりません。見出し行: {}", c.header, header_cells.join(", "))))
            .collect::<Result<_, _>>()?;

        let results = report::results(cfg);
        let by_id: HashMap<&str, &StudentResult> = results.iter().map(|r| (r.id.as_str(), r)).collect();
        let values = Values::new(cfg);
        let mut report = ExportReport::default();
        let mut seen: HashSet<String> = HashSet::new();
        // (byte range in `text`, new text)
        let mut edits: Vec<(usize, usize, String)> = Vec::new();

        for rec in iter.skip(reader.skip_after_header) {
            let id = reader.clean_cell(rec.cells.get(id_idx).map(String::as_str).unwrap_or(""));
            if id.trim().is_empty() {
                continue;
            }
            let Some(r) = by_id.get(id.as_str()) else {
                report.not_in_app.push(id);
                continue;
            };
            if !seen.insert(id.clone()) {
                continue;
            }
            let (cells, blank) = self.row(&values, r);
            if blank {
                report.blank.push(id.clone());
            }
            let mut row = rec.cells.clone();
            for (&t, v) in targets.iter().zip(cells) {
                if row.len() <= t {
                    row.resize(t + 1, String::new());
                }
                row[t] = v;
            }
            edits.push((rec.start, rec.end, write_row(&row, reader)?));
            report.written += 1;
        }
        report.not_in_file = results.iter().filter(|r| !seen.contains(&r.id)).map(|r| r.id.clone()).collect();

        let mut out = String::with_capacity(text.len() + 64);
        let mut pos = 0;
        for (start, end, new) in edits {
            out.push_str(&text[pos..start]);
            out.push_str(&new);
            pos = end;
        }
        out.push_str(&text[pos..]);
        Ok((csv_import::encode(&out, encoding)?, report))
    }
}

// One record in the style of the format it was read with, without a line break
fn write_row(cells: &[String], format: &CsvFormat) -> Result<String, String> {
    let mut w = csv::WriterBuilder::new()
        .delimiter(format.delimiter as u8)
        .quote(format.quote.unwrap_or('"') as u8)
        .quote_style(if format.quote.is_some() { csv::QuoteStyle::Necessary } else { csv::QuoteStyle::Never })
        .terminator(csv::Terminator::Any(b'\n'))
        .from_writer(Vec::new());
    w.write_record(cells).map_err(|e| e.to_string())?;
    let bytes = w.into_inner().map_err(|e| e.to_string())?;
    let mut s = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    if s.ends_with('\n') {
        s.pop();
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Question, Rating, Student};

    fn cfg() -> Config {
        let mut c = Config::new();
        c.questions = vec![Question { id: 1, name: "Q".into(), full_score: 10, weight: 1.0, comment: String::new() }];
        c.students = [("09999999", "山田　花子"), ("01234567", "佐藤　次郎"), ("X1", "外部")]
            .iter().map(|(id, n)| Student { id: id.to_string(), name: n.to_string() }).collect();
        c.ratings = vec![
            Rating { label: "優".into(), min_score: 80, max_ratio: None },
            Rating { label: "不可".into(), min_score: 0, max_ratio: None },
        ];
        c.set_score("09999999", 1, Some(9));
        c.set_score("X1", 1, Some(3));
        c
    }

    fn fmt(name: &str) -> ExportFormat {
        builtin_formats().into_iter().find(|f| f.name == name).unwrap()
    }

    #[test]
    fn builtin_formats_are_valid() {
        for f in builtin_formats() {
            assert_eq!(f.validate(), Ok(()), "{}", f.name);
        }
        let imports = csv_import::builtin_formats();
        for f in builtin_formats().iter().filter(|f| f.mode == ExportMode::Fill) {
            assert!(imports.iter().any(|i| i.name == f.read_as), "{}", f.name);
        }
    }

    #[test]
    fn new_file_has_header_and_one_row_per_student() {
        let (bytes, report) = fmt("成績一覧").export_new(&cfg()).unwrap();
        let text = String::from_utf8(bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap().to_vec()).unwrap();
        assert_eq!(text, "id,name,score,rating\r\n09999999,山田　花子,90,優\r\n01234567,佐藤　次郎,,\r\nX1,外部,30,不可\r\n");
        assert_eq!(report.written, 3);
        assert_eq!(report.blank, vec!["01234567"]);
    }

    #[test]
    fn fill_utas_keeps_everything_else() {
        let original = include_bytes!("../reference_files/utas_example_1.csv");
        let (bytes, report) = fmt("UTAS").export_fill(&cfg(), original, &csv_import::builtin_formats()).unwrap();
        let before = String::from_utf8_lossy(original);
        let after = String::from_utf8_lossy(&bytes);
        assert!(bytes.starts_with(b"\xEF\xBB\xBF"));
        assert!(after.contains("3,神学部経典デザイン科,3,09,09999999,山田　花子,優,"), "{after}");
        assert!(after.contains("5,神学部応用座禅科,3,04,01234567,佐藤　次郎,,"), "{after}");
        // Only the 評語 cell of the first student changed
        assert_eq!(after.len(), before.len() + "優".len());
        assert_eq!(after.lines().count(), before.lines().count());
        assert_eq!(report.written, 2);
        assert_eq!(report.blank, vec!["01234567"]);
        assert_eq!(report.not_in_file, vec!["X1"]);
        assert_eq!(report.not_in_app, vec!["J2233445", "J2233445"]);
    }

    #[test]
    fn fill_utol_writes_scores() {
        let mut c = cfg();
        c.students.push(Student { id: "01122334".into(), name: "鈴木 一郎".into() });
        c.set_score("01122334", 1, Some(7));
        let original = include_bytes!("../reference_files/utol_example_1.csv");
        let (bytes, report) = fmt("UTOL 課題").export_fill(&c, original, &csv_import::builtin_formats()).unwrap();
        let after = String::from_utf8_lossy(&bytes);
        assert!(after.contains("01122334,1111111111,Fグループ,鈴木 一郎,学生,70,,"), "{after}");
        assert_eq!(report.written, 1);
        assert_eq!(report.not_in_app.len(), 2);
    }

    #[test]
    fn map_and_sources() {
        let mut map = BTreeMap::new();
        map.insert("不可".to_string(), "否".to_string());
        map.insert("*".to_string(), "合".to_string());
        assert_eq!(apply_map("不可".into(), &map), "否");
        assert_eq!(apply_map("優".into(), &map), "合");
        assert_eq!(apply_map(String::new(), &map), "");
        assert_eq!(Source::parse("q:3"), Ok(Source::Question(3)));
        assert_eq!(Source::parse("literal:済"), Ok(Source::Literal("済".into())));
        assert!(Source::parse("total").is_err());
        let bad = ExportFormat { name: "x".into(), mode: ExportMode::Fill, columns: vec![ExportColumn { header: "a".into(), value: "id".into(), map: BTreeMap::new() }], ..ExportFormat::default() };
        assert!(bad.validate().unwrap_err().contains("read_as"));
    }

    #[test]
    fn fill_reports_missing_target_column() {
        let mut f = fmt("UTAS");
        f.columns[0].header = "成績".into();
        let err = f.export_fill(&cfg(), include_bytes!("../reference_files/utas_example_1.csv"), &csv_import::builtin_formats()).unwrap_err();
        assert!(err.contains("成績") && err.contains("評語"), "{err}");
    }
}
