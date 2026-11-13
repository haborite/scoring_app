// Import of students from CSV files described by a user-definable `CsvFormat`.
//
// Formats come from two places:
// - built-in formats (UTAS)
// - `csv_formats.json` in the app config directory, a JSON array of `CsvFormat`
//   that users edit by hand. A user format with the same name as a built-in one replaces it.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

use crate::models::Student;

const USER_FORMATS_FILE: &str = "csv_formats.json";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CsvEncoding {
    // UTF-8 if the file has a UTF-8 BOM or is valid UTF-8, otherwise Shift_JIS
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "utf-8")]
    Utf8,
    #[serde(rename = "shift_jis")]
    ShiftJis,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CsvDirection {
    // One student per row; the header is a row
    #[default]
    Rows,
    // One student per column; the header is a column (the table is transposed before reading)
    Columns,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct CsvFormat {
    pub name: String,
    pub description: String,
    pub encoding: CsvEncoding,
    pub delimiter: char,
    // None: quotes are treated as ordinary characters
    pub quote: Option<char>,
    pub direction: CsvDirection,
    // Rows (columns if direction = columns) to ignore before the header. Blank lines are not counted.
    pub skip_before_header: usize,
    // Rows to ignore between the header and the first student
    pub skip_after_header: usize,
    pub id_header: String,
    pub name_header: String,
    // Trim leading/trailing whitespace (including full-width spaces) of every cell
    pub trim_whitespace: bool,
}

impl Default for CsvFormat {
    fn default() -> Self {
        CsvFormat {
            name: String::new(),
            description: String::new(),
            encoding: CsvEncoding::Auto,
            delimiter: ',',
            quote: Some('"'),
            direction: CsvDirection::Rows,
            skip_before_header: 0,
            skip_after_header: 0,
            id_header: String::new(),
            name_header: String::new(),
            trim_whitespace: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CsvImport {
    pub students: Vec<Student>,
    // Rows that were skipped, for display
    pub warnings: Vec<String>,
}

pub fn builtin_formats() -> Vec<CsvFormat> {
    vec![CsvFormat {
        name: "UTAS".to_string(),
        description: "UTAS の履修者名簿（4 行目が見出し行）".to_string(),
        skip_before_header: 3,
        id_header: "学生証番号".to_string(),
        name_header: "氏名".to_string(),
        ..CsvFormat::default()
    }]
}

pub fn user_formats_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("scoring_app").join(USER_FORMATS_FILE))
}

// Built-in formats followed by user formats. Problems with the user file are returned as messages.
pub fn load_formats() -> (Vec<CsvFormat>, Vec<String>) {
    let mut formats = builtin_formats();
    let mut errors = Vec::new();

    let Some(path) = user_formats_path() else {
        return (formats, errors);
    };
    if !path.exists() {
        return (formats, errors);
    }

    let user: Vec<CsvFormat> = match std::fs::read_to_string(&path)
        .map_err(|e| e.to_string())
        .and_then(|txt| serde_json::from_str(txt.trim_start_matches('\u{feff}')).map_err(|e| e.to_string()))
    {
        Ok(v) => v,
        Err(e) => {
            errors.push(format!("{} を読み込めませんでした: {e}", path.display()));
            return (formats, errors);
        }
    };

    for f in user {
        if let Err(e) = f.validate() {
            errors.push(format!("CSV フォーマット「{}」を無視しました: {e}", f.name));
            continue;
        }
        match formats.iter_mut().find(|b| b.name == f.name) {
            Some(existing) => *existing = f,
            None => formats.push(f),
        }
    }
    (formats, errors)
}

// Create the user format file with an example entry if it does not exist. Returns its path.
pub fn ensure_user_formats_file() -> Result<PathBuf, String> {
    let path = user_formats_path().ok_or("設定フォルダが見つかりません")?;
    if !path.exists() {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let example = CsvFormat {
            name: "カスタム形式の例".to_string(),
            description: "UTAS 形式を元にした例。自由に編集・追加してください".to_string(),
            ..builtin_formats().remove(0)
        };
        let json = serde_json::to_string_pretty(&vec![example]).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
    }
    Ok(path)
}

impl CsvFormat {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("name が空です".to_string());
        }
        if !self.delimiter.is_ascii() {
            return Err("delimiter には半角文字 1 文字を指定してください".to_string());
        }
        if self.quote.is_some_and(|q| !q.is_ascii()) {
            return Err("quote には半角文字 1 文字を指定してください".to_string());
        }
        if self.id_header.trim().is_empty() || self.name_header.trim().is_empty() {
            return Err("id_header と name_header を指定してください".to_string());
        }
        Ok(())
    }

    pub fn parse(&self, bytes: &[u8]) -> Result<CsvImport, String> {
        self.validate()?;
        let text = decode(bytes, self.encoding)?;

        // (line number in the file, cells)
        let mut lines: Vec<(u64, Vec<String>)> = Vec::new();
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .delimiter(self.delimiter as u8)
            .quoting(self.quote.is_some())
            .quote(self.quote.unwrap_or('"') as u8)
            .from_reader(text.as_bytes());
        // csv's own line numbers are unreliable without headers, so derive them from byte offsets:
        // a record starts at the first non-newline byte after the previous record's end.
        let mut record = csv::StringRecord::new();
        let mut offset = 0usize;
        loop {
            let has_record = reader.read_record(&mut record)
                .map_err(|e| format!("CSV の解析に失敗しました: {e}"))?;
            if !has_record {
                break;
            }
            let start = offset + text.as_bytes()[offset..].iter()
                .take_while(|b| matches!(b, b'\r' | b'\n'))
                .count();
            let line = 1 + text.as_bytes()[..start].iter().filter(|&&b| b == b'\n').count() as u64;
            offset = (reader.position().byte() as usize).min(text.len());
            let cells = record.iter()
                .map(|c| if self.trim_whitespace { c.trim().to_string() } else { c.to_string() })
                .collect();
            lines.push((line, cells));
        }

        // For columns, transpose so that the rest works the same way. Labels become column numbers.
        let (lines, unit) = match self.direction {
            CsvDirection::Rows => (lines, "行目"),
            CsvDirection::Columns => (transpose(lines), "列目"),
        };

        let mut iter = lines.into_iter().skip(self.skip_before_header);
        let (_, header) = iter.next().ok_or("見出し行が見つかりません（skip_before_header が大きすぎます）")?;

        let find = |name: &str| header.iter().position(|h| h.trim() == name.trim());
        let (Some(id_idx), Some(name_idx)) = (find(&self.id_header), find(&self.name_header)) else {
            return Err(format!(
                "見出し「{}」「{}」が見つかりません。見出し行: {}",
                self.id_header,
                self.name_header,
                header.join(", "),
            ));
        };

        let mut students = Vec::new();
        let mut warnings = Vec::new();
        let mut seen = HashSet::new();

        for (line, cells) in iter.skip(self.skip_after_header) {
            if cells.iter().all(|c| c.trim().is_empty()) {
                continue;
            }
            let id = cells.get(id_idx).cloned().unwrap_or_default();
            let name = cells.get(name_idx).cloned().unwrap_or_default();
            if id.trim().is_empty() {
                warnings.push(format!("{line}{unit}: {} が空のため読み飛ばしました", self.id_header));
                continue;
            }
            if !seen.insert(id.clone()) {
                warnings.push(format!("{line}{unit}: {} {id}（{name}）は重複しているため読み飛ばしました", self.id_header));
                continue;
            }
            students.push(Student { id, name });
        }

        Ok(CsvImport { students, warnings })
    }
}

fn decode(bytes: &[u8], encoding: CsvEncoding) -> Result<String, String> {
    let utf8 = |b: &[u8]| {
        let b = b.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(b);
        String::from_utf8(b.to_vec()).map_err(|_| "UTF-8 として読めませんでした".to_string())
    };
    let sjis = |b: &[u8]| {
        let (s, _, had_errors) = encoding_rs::SHIFT_JIS.decode(b);
        if had_errors {
            Err("Shift_JIS として読めませんでした".to_string())
        } else {
            Ok(s.into_owned())
        }
    };
    match encoding {
        CsvEncoding::Utf8 => utf8(bytes),
        CsvEncoding::ShiftJis => sjis(bytes),
        CsvEncoding::Auto => utf8(bytes).or_else(|_| sjis(bytes)),
    }
}

// Column i becomes line i+1 (1-based column number)
fn transpose(lines: Vec<(u64, Vec<String>)>) -> Vec<(u64, Vec<String>)> {
    let width = lines.iter().map(|(_, c)| c.len()).max().unwrap_or(0);
    (0..width)
        .map(|i| {
            let col = lines.iter().map(|(_, c)| c.get(i).cloned().unwrap_or_default()).collect();
            (i as u64 + 1, col)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utas() -> CsvFormat {
        builtin_formats().remove(0)
    }

    fn ids(r: &CsvImport) -> Vec<(&str, &str)> {
        r.students.iter().map(|s| (s.id.as_str(), s.name.as_str())).collect()
    }

    #[test]
    fn utas_example_file() {
        let bytes = include_bytes!("../reference_files/utas_example_1.csv");
        let r = utas().parse(bytes).unwrap();
        assert_eq!(ids(&r), vec![
            ("09999999", "山田　花子"),
            ("01234567", "佐藤　次郎"),
            ("J2233445", "田中　葉子"),
            ("J2233446", "高橋　三郎"),
        ]);
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    }

    #[test]
    fn duplicate_ids_are_reported() {
        let text = "a\r\nb\r\nc\r\nNo,学生証番号,氏名\r\n1,J1,田中\r\n2,J1,高橋\r\n";
        let r = utas().parse(text.as_bytes()).unwrap();
        assert_eq!(ids(&r), vec![("J1", "田中")]);
        assert_eq!(r.warnings.len(), 1);
        assert!(r.warnings[0].starts_with("6行目"), "{:?}", r.warnings);
    }

    #[test]
    fn utas_in_shift_jis() {
        let text = "年度・学期,2026\r\n開講科目名,x\r\n採点者ID,1\r\nNo,学生証番号,氏名\r\n1, A001 ,山田　太郎\r\n";
        let (sjis, _, _) = encoding_rs::SHIFT_JIS.encode(text);
        let r = utas().parse(&sjis).unwrap();
        assert_eq!(ids(&r), vec![("A001", "山田　太郎")]);
    }

    #[test]
    fn options_are_applied() {
        let f = CsvFormat {
            name: "t".into(),
            delimiter: '\t',
            skip_after_header: 1,
            id_header: "id".into(),
            name_header: "name".into(),
            trim_whitespace: false,
            ..CsvFormat::default()
        };
        let r = f.parse("name\tid\n(sub header)\t\nAlice \tS1\n\n\tS2\n Bob\t\n".as_bytes()).unwrap();
        assert_eq!(ids(&r), vec![("S1", "Alice "), ("S2", "")]);
        // Line numbers count the blank line
        assert_eq!(r.warnings, vec!["6行目: id が空のため読み飛ばしました".to_string()]);
    }

    #[test]
    fn columns_direction() {
        let f = CsvFormat {
            name: "t".into(),
            direction: CsvDirection::Columns,
            skip_before_header: 1,
            id_header: "id".into(),
            name_header: "name".into(),
            ..CsvFormat::default()
        };
        // Column 1 is skipped, column 2 is the header, one student per column after that
        let r = f.parse("memo,id,S1,S2,\nx,name,Alice,Bob,Carol\n".as_bytes()).unwrap();
        assert_eq!(ids(&r), vec![("S1", "Alice"), ("S2", "Bob")]);
        assert_eq!(r.warnings, vec!["5列目: id が空のため読み飛ばしました".to_string()]);
    }

    #[test]
    fn missing_header_lists_available_headers() {
        let f = CsvFormat { id_header: "番号".into(), ..utas() };
        let err = f.parse(include_bytes!("../reference_files/utas_example_1.csv")).unwrap_err();
        assert!(err.contains("学生証番号"), "{err}");
    }

    #[test]
    fn user_json_uses_defaults_for_missing_fields() {
        let f: CsvFormat = serde_json::from_str(
            r#"{"name":"x","delimiter":";","encoding":"shift_jis","id_header":"ID","name_header":"Name"}"#,
        ).unwrap();
        assert_eq!(f.delimiter, ';');
        assert_eq!(f.encoding, CsvEncoding::ShiftJis);
        assert_eq!(f.quote, Some('"'));
        assert!(f.trim_whitespace);
        assert!(f.validate().is_ok());
    }
}
