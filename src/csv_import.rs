// Import of students from CSV files described by a user-definable `CsvFormat`.
//
// The formats shipped with the app are not written in Rust: they are the JSON array in
// `formats/import_formats.json`, which is copied to `csv_formats.json` in the app config directory
// the first time the app needs it. From then on that file *is* the list of formats, so the formats
// that come with the app can be edited and removed like any other. `*.default.json` next to it
// keeps a copy of the shipped formats for reference (it is overwritten, so never edit it).

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::models::Student;

const USER_FORMATS_FILE: &str = "csv_formats.json";

// The formats shipped with the app
pub const DEFAULT_FORMATS_JSON: &str = include_str!("../formats/import_formats.json");

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

// The name column, or several columns joined with `name_separator` (family name + given name)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum NameHeader {
    One(String),
    Many(Vec<String>),
}

impl Default for NameHeader {
    fn default() -> Self {
        NameHeader::One(String::new())
    }
}

impl From<&str> for NameHeader {
    fn from(h: &str) -> Self {
        NameHeader::One(h.to_string())
    }
}

impl NameHeader {
    pub fn headers(&self) -> Vec<&str> {
        match self {
            NameHeader::One(h) => vec![h.as_str()],
            NameHeader::Many(hs) => hs.iter().map(String::as_str).collect(),
        }
    }

    fn is_empty(&self) -> bool {
        self.headers().iter().all(|h| h.trim().is_empty())
    }

    // For messages
    fn label(&self) -> String {
        self.headers().join(" + ")
    }
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
    // One header, or several that are joined (e.g. ["Surname", "First name"])
    pub name_header: NameHeader,
    // Between the parts of a name made of several columns
    pub name_separator: String,
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
            name_header: NameHeader::default(),
            name_separator: " ".to_string(),
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

// The formats shipped with the app, as read from `formats/import_formats.json`. Used by the
// tests: at run time the formats come from the user's file, which is created from the same JSON.
#[cfg(test)]
pub fn builtin_formats() -> Vec<CsvFormat> {
    parse_defaults(DEFAULT_FORMATS_JSON, "formats/import_formats.json")
}

// The shipped formats of an embedded JSON file. A test checks every one of them, so a failure here
// means the binary was built from a broken file.
pub fn parse_defaults<T: DeserializeOwned>(json: &str, what: &str) -> Vec<T> {
    serde_json::from_str(json.trim_start_matches('\u{feff}'))
        .unwrap_or_else(|e| panic!("{what} is not a valid format list: {e}"))
}

pub fn user_formats_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("scoring_app").join(USER_FORMATS_FILE))
}

// The formats of `csv_formats.json`, created from the shipped ones when it is missing.
// Problems with the file are returned as messages.
pub fn load_formats() -> (Vec<CsvFormat>, Vec<String>) {
    load_user_file(DEFAULT_FORMATS_JSON, user_formats_path(), |f| f.name.clone(), CsvFormat::validate)
}

// The entries of a user JSON file (an array), which is created from `defaults_json` when missing.
// Invalid entries are skipped and reported; if nothing usable is left, the shipped formats are
// used so that the app keeps working whatever is in the file.
pub fn load_user_file<T: DeserializeOwned>(
    defaults_json: &str,
    path: Option<PathBuf>,
    name_of: fn(&T) -> String,
    validate: fn(&T) -> Result<(), String>,
) -> (Vec<T>, Vec<String>) {
    let mut errors = Vec::new();
    let fallback = || parse_defaults::<T>(defaults_json, "the formats included with the app");
    let Some(path) = path else {
        errors.push("設定フォルダが見つかりません".to_string());
        return (fallback(), errors);
    };
    if let Err(e) = ensure_user_file(Some(path.clone()), defaults_json) {
        errors.push(e);
    }
    let read = std::fs::read_to_string(&path)
        .map_err(|e| e.to_string())
        .and_then(|txt| {
            serde_json::from_str::<Vec<T>>(txt.trim_start_matches('\u{feff}')).map_err(|e| e.to_string())
        });
    let user = match read {
        Ok(v) => v,
        Err(e) => {
            errors.push(format!("{} を読み込めませんでした（付属のフォーマットを使います）: {e}", path.display()));
            return (fallback(), errors);
        }
    };
    let mut items = Vec::new();
    for item in user {
        match validate(&item) {
            Ok(()) => items.push(item),
            Err(e) => errors.push(format!("フォーマット「{}」を無視しました: {e}", name_of(&item))),
        }
    }
    if items.is_empty() {
        errors.push(format!("{} に使えるフォーマットがないため、付属のフォーマットを使います", path.display()));
        return (fallback(), errors);
    }
    (items, errors)
}

// A CSV in the shape `format` expects: the rows it skips, a header row of the id and name columns
// plus `extra_headers`, and one student. Used by the tests of this module and of `csv_export`.
#[cfg(test)]
pub fn sample_file(format: &CsvFormat, extra_headers: &[&str], id: &str, name_part: &str) -> String {
    let sep = format.delimiter.to_string();
    let names = format.name_header.headers();
    let mut headers: Vec<String> = vec![format.id_header.clone()];
    headers.extend(names.iter().map(|h| h.to_string()));
    headers.extend(extra_headers.iter().map(|h| h.to_string()));

    let mut lines: Vec<String> = (0..format.skip_before_header).map(|i| format!("前書き {i}")).collect();
    lines.push(headers.join(&sep));
    lines.extend((0..format.skip_after_header).map(|_| vec![""; headers.len()].join(&sep)));
    let mut cells = vec![id.to_string()];
    cells.extend(names.iter().enumerate().map(|(i, _)| format!("{name_part}{i}")));
    cells.extend(extra_headers.iter().map(|_| String::new()));
    lines.push(cells.join(&sep));
    lines.join("\r\n") + "\r\n"
}

// `<name>.default.json`, a read-only copy of the shipped formats next to the user's file
fn defaults_copy_path(path: &Path) -> PathBuf {
    path.with_extension("default.json")
}

// Create the user format file from `defaults_json` if it does not exist, and keep the reference
// copy of the shipped formats next to it up to date. Returns the user file's path.
pub fn ensure_user_file(path: Option<PathBuf>, defaults_json: &str) -> Result<PathBuf, String> {
    let path = path.ok_or("設定フォルダが見つかりません")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    if !path.exists() {
        std::fs::write(&path, defaults_json).map_err(|e| e.to_string())?;
    }
    // Rewritten when the app ships new formats, so that they can be copied into the user's file
    let copy = defaults_copy_path(&path);
    if std::fs::read_to_string(&copy).ok().as_deref() != Some(defaults_json) {
        std::fs::write(&copy, defaults_json).map_err(|e| e.to_string())?;
    }
    Ok(path)
}

// Show the folder of a file in the OS file manager
pub fn open_folder(path: &Path) {
    let dir = path.parent().unwrap_or(path);
    #[cfg(target_os = "windows")]
    let cmd = std::process::Command::new("explorer").arg(dir).spawn();
    #[cfg(target_os = "macos")]
    let cmd = std::process::Command::new("open").arg(dir).spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let cmd = std::process::Command::new("xdg-open").arg(dir).spawn();
    let _ = cmd;
}

// Create the user format file from the shipped formats if it does not exist. Returns its path.
pub fn ensure_user_formats_file() -> Result<PathBuf, String> {
    ensure_user_file(user_formats_path(), DEFAULT_FORMATS_JSON)
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
        if self.id_header.trim().is_empty() || self.name_header.is_empty() {
            return Err("id_header と name_header を指定してください".to_string());
        }
        Ok(())
    }

    // Records of the text with where they are, cells not trimmed
    pub fn read_records(&self, text: &str) -> Result<Vec<RawRecord>, String> {
        let mut records = Vec::new();
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
            let bytes = text.as_bytes();
            let start = offset + bytes[offset..].iter()
                .take_while(|b| matches!(b, b'\r' | b'\n'))
                .count();
            let line = 1 + bytes[..start].iter().filter(|&&b| b == b'\n').count() as u64;
            offset = (reader.position().byte() as usize).min(text.len());
            let end = start.max(offset - bytes[start..offset].iter().rev()
                .take_while(|b| matches!(b, b'\r' | b'\n'))
                .count());
            let cells = record.iter().map(str::to_string).collect();
            records.push(RawRecord { line, start, end, cells });
        }
        Ok(records)
    }

    pub fn clean_cell(&self, c: &str) -> String {
        if self.trim_whitespace { c.trim().to_string() } else { c.to_string() }
    }

    // The name cells joined with `name_separator`, leaving out the parts that are empty
    fn join_name(&self, cells: &[String], idx: &[usize]) -> String {
        let parts: Vec<&str> = idx.iter()
            .filter_map(|&i| cells.get(i))
            .map(String::as_str)
            .filter(|p| !p.trim().is_empty())
            .collect();
        parts.join(&self.name_separator)
    }

    pub fn parse(&self, bytes: &[u8]) -> Result<CsvImport, String> {
        self.validate()?;
        let text = decode(bytes, self.encoding)?;

        // (line number in the file, cells)
        let lines: Vec<(u64, Vec<String>)> = self.read_records(&text)?
            .into_iter()
            .map(|r| (r.line, r.cells.iter().map(|c| self.clean_cell(c)).collect()))
            .collect();

        // For columns, transpose so that the rest works the same way. Labels become column numbers.
        let (lines, unit) = match self.direction {
            CsvDirection::Rows => (lines, "行目"),
            CsvDirection::Columns => (transpose(lines), "列目"),
        };

        let mut iter = lines.into_iter().skip(self.skip_before_header);
        let (_, header) = iter.next().ok_or("見出し行が見つかりません（skip_before_header が大きすぎます）")?;

        let find = |name: &str| header.iter().position(|h| h.trim() == name.trim());
        let name_idx: Option<Vec<usize>> = self.name_header.headers().into_iter().map(find).collect();
        let (Some(id_idx), Some(name_idx)) = (find(&self.id_header), name_idx) else {
            return Err(format!(
                "見出し「{}」「{}」が見つかりません。見出し行: {}",
                self.id_header,
                self.name_header.label(),
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
            let name = self.join_name(&cells, &name_idx);
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
    decode_detect(bytes, encoding).map(|(s, _)| s)
}

// A record and the byte range of its text (without the line break)
#[derive(Debug, Clone, PartialEq)]
pub struct RawRecord {
    pub line: u64,
    pub start: usize,
    pub end: usize,
    pub cells: Vec<String>,
}

// How a file was actually encoded, so that it can be written back the same way
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detected {
    Utf8 { bom: bool },
    ShiftJis,
}

pub fn decode_detect(bytes: &[u8], encoding: CsvEncoding) -> Result<(String, Detected), String> {
    let utf8 = |b: &[u8]| {
        let stripped = b.strip_prefix(b"\xEF\xBB\xBF");
        let bom = stripped.is_some();
        String::from_utf8(stripped.unwrap_or(b).to_vec())
            .map(|s| (s, Detected::Utf8 { bom }))
            .map_err(|_| "UTF-8 として読めませんでした".to_string())
    };
    let sjis = |b: &[u8]| {
        let (s, _, had_errors) = encoding_rs::SHIFT_JIS.decode(b);
        if had_errors {
            Err("Shift_JIS として読めませんでした".to_string())
        } else {
            Ok((s.into_owned(), Detected::ShiftJis))
        }
    };
    match encoding {
        CsvEncoding::Utf8 => utf8(bytes),
        CsvEncoding::ShiftJis => sjis(bytes),
        CsvEncoding::Auto => utf8(bytes).or_else(|_| sjis(bytes)),
    }
}

pub fn encode(text: &str, encoding: Detected) -> Result<Vec<u8>, String> {
    match encoding {
        Detected::Utf8 { bom } => {
            let mut out = if bom { b"\xEF\xBB\xBF".to_vec() } else { Vec::new() };
            out.extend_from_slice(text.as_bytes());
            Ok(out)
        }
        Detected::ShiftJis => {
            let (b, _, had_errors) = encoding_rs::SHIFT_JIS.encode(text);
            if had_errors {
                Err("Shift_JIS で表せない文字が含まれています".to_string())
            } else {
                Ok(b.into_owned())
            }
        }
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
    fn shipped_formats_are_valid_and_uniquely_named() {
        let formats = builtin_formats();
        assert!(formats.len() > 2, "formats/import_formats.json がほとんど空です");
        for f in &formats {
            assert_eq!(f.validate(), Ok(()), "{}", f.name);
        }
        let mut names: Vec<&str> = formats.iter().map(|f| f.name.as_str()).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total, "name が重複しています");
    }

    // Every shipped format is tried on a file in the shape it describes, so that a wrong
    // skip_before_header / skip_after_header or a missing header cannot go unnoticed
    #[test]
    fn shipped_formats_read_a_file_of_their_own_shape() {
        for f in builtin_formats() {
            assert_eq!(f.direction, CsvDirection::Rows, "{}", f.name);
            let text = sample_file(&f, &["その他"], "A001", "氏名");
            let r = f.parse(text.as_bytes()).unwrap_or_else(|e| panic!("{}: {e}\n{text}", f.name));
            let expected: Vec<String> = (0..f.name_header.headers().len()).map(|i| format!("氏名{i}")).collect();
            assert_eq!(
                ids(&r),
                vec![("A001", expected.join(&f.name_separator).as_str())],
                "{}: {:?}",
                f.name,
                r.warnings,
            );
        }
    }

    #[test]
    fn user_file_is_created_from_the_shipped_formats_and_then_decides() {
        let dir = std::env::temp_dir().join("scoring_app_test_user_formats_file");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("csv_formats.json");
        let load = || load_user_file(
            DEFAULT_FORMATS_JSON,
            Some(path.clone()),
            |f: &CsvFormat| f.name.clone(),
            CsvFormat::validate,
        );

        // Missing: written out as it is shipped, together with the reference copy
        let (formats, errors) = load();
        assert_eq!(errors, Vec::<String>::new());
        assert_eq!(formats, builtin_formats());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), DEFAULT_FORMATS_JSON);
        assert_eq!(
            std::fs::read_to_string(dir.join("csv_formats.default.json")).unwrap(),
            DEFAULT_FORMATS_JSON,
        );

        // Present: the file is the list of formats (a shipped format can be removed), and an
        // entry that cannot be used is skipped with a message
        std::fs::write(&path, r#"[{"name":"自作","id_header":"ID","name_header":"氏名"},{"name":"壊れた"}]"#).unwrap();
        let (formats, errors) = load();
        assert_eq!(formats.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), vec!["自作"]);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("壊れた"), "{errors:?}");

        // Unreadable: the shipped formats keep the app usable
        std::fs::write(&path, "{").unwrap();
        let (formats, errors) = load();
        assert_eq!(formats, builtin_formats());
        assert_eq!(errors.len(), 1, "{errors:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn name_can_come_from_several_columns() {
        let f = CsvFormat {
            name: "t".into(),
            id_header: "ID number".into(),
            name_header: NameHeader::Many(vec!["Surname".into(), "First name".into()]),
            ..CsvFormat::default()
        };
        let text = "First name,Surname,ID number\nHanako,Yamada,A1\n,Sato,A2\n";
        let r = f.parse(text.as_bytes()).unwrap();
        // The parts are joined in the order they are written, and an empty part is left out
        assert_eq!(ids(&r), vec![("A1", "Yamada Hanako"), ("A2", "Sato")]);

        // A missing name column is reported with every header it needs
        let err = f.parse("First name,ID number\nHanako,A1\n".as_bytes()).unwrap_err();
        assert!(err.contains("Surname + First name"), "{err}");
    }

    #[test]
    fn utas_example_file() {
        let bytes = include_bytes!("../reference_files/utas_example_1.csv");
        let r = utas().parse(bytes).unwrap();
        assert_eq!(ids(&r), vec![
            ("09999999", "山田　花子"),
            ("01234567", "佐藤　次郎"),
            ("J2233445", "田中　葉子"),
        ]);
        // The last row repeats J2233445: skipped, with its line number in the warning
        assert_eq!(r.warnings, vec![
            "8行目: 学生証番号 J2233445（高橋　三郎）は重複しているため読み飛ばしました".to_string(),
        ]);
    }

    #[test]
    fn utol_example_file() {
        let utol = builtin_formats().into_iter().find(|f| f.name == "UTOL 課題").unwrap();
        let bytes = include_bytes!("../reference_files/utol_example_1.csv");
        let r = utol.parse(bytes).unwrap();
        assert_eq!(ids(&r), vec![
            ("01122334", "鈴木 一郎"),
            ("01122335", "山田 花子"),
            ("01122336", "田中 次郎"),
        ]);
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    }

    #[test]
    fn records_know_their_byte_ranges() {
        let f = CsvFormat { name: "t".into(), id_header: "id".into(), name_header: "n".into(), ..CsvFormat::default() };
        let text = "a,b\r\n\r\n\"x,1\",y\r\nlast";
        let recs = f.read_records(text).unwrap();
        let spans: Vec<(&str, u64)> = recs.iter().map(|r| (&text[r.start..r.end], r.line)).collect();
        assert_eq!(spans, vec![("a,b", 1), ("\"x,1\",y", 3), ("last", 4)]);
        assert_eq!(recs[1].cells, vec!["x,1", "y"]);
    }

    #[test]
    fn decode_and_encode_keep_the_encoding() {
        let bom = b"\xEF\xBB\xBFa,b";
        let (t, d) = decode_detect(bom, CsvEncoding::Auto).unwrap();
        assert_eq!((t.as_str(), d), ("a,b", Detected::Utf8 { bom: true }));
        assert_eq!(encode(&t, d).unwrap(), bom.to_vec());
        let (sjis, _, _) = encoding_rs::SHIFT_JIS.encode("評語");
        let (t, d) = decode_detect(&sjis, CsvEncoding::Auto).unwrap();
        assert_eq!(d, Detected::ShiftJis);
        assert_eq!(encode(&t, d).unwrap(), sjis.to_vec());
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
