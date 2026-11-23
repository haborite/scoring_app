use std::path::PathBuf;
use std::sync::Mutex;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::shortcuts::{Action, KeyOverrides};

// Preferences of this user, shared by every exam file. Stored next to the CSV formats
// in the user's config directory, never in the exam file.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    // Full score given to a newly added question
    pub default_full_score: u32,
    pub layout: Layout,
    pub keys: KeyOverrides,
    pub histogram: HistogramView,
}

// Display options of the histogram on the rating page
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct HistogramView {
    // Width of a class in points (1, 2, 5 or 10)
    pub bin_width: u32,
    pub color_by_rating: bool,
    pub show_thresholds: bool,
    pub show_counts: bool,
}

impl HistogramView {
    pub const WIDTHS: [u32; 4] = [1, 2, 5, 10];
}

impl Default for HistogramView {
    fn default() -> Self {
        HistogramView { bin_width: 5, color_by_rating: true, show_thresholds: true, show_counts: true }
    }
}

// Pane sizes of the grading page, in percent of the page
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Layout {
    // Width of the comment pane
    pub comment_width: f64,
    // Height of the comment / score input row (the student table gets the rest)
    pub top_height: f64,
}

impl Layout {
    pub const COMMENT_WIDTH: (f64, f64) = (15.0, 75.0);
    pub const TOP_HEIGHT: (f64, f64) = (20.0, 85.0);
}

impl Default for Layout {
    fn default() -> Self {
        Layout { comment_width: 35.0, top_height: 60.0 }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            default_full_score: 8,
            layout: Layout::default(),
            keys: KeyOverrides::new(),
            histogram: HistogramView::default(),
        }
    }
}

// Read from the settings file on first use
pub static SETTINGS: GlobalSignal<Settings> = Signal::global(load);
// JSON last written to (or read from) the settings file
static SAVED_SETTINGS: Mutex<String> = Mutex::new(String::new());
// Problem found while reading the settings file, shown once by App
static LOAD_ERROR: Mutex<Option<String>> = Mutex::new(None);
// Action whose new key is being recorded on the settings page
pub static RECORDING: GlobalSignal<Option<Action>> = Signal::global(|| None);

const SETTINGS_FILE: &str = "settings.json";

pub fn settings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("scoring_app").join(SETTINGS_FILE))
}

impl Settings {
    pub fn keys(&self, action: Action) -> Vec<String> {
        crate::shortcuts::keys_for(&self.keys, action)
    }

    pub fn key_label(&self, action: Action) -> String {
        crate::shortcuts::action_label(&self.keys, action)
    }
}

// Read the settings file. On a problem the defaults are used and the message is kept for App.
fn load() -> Settings {
    let loaded = match settings_path() {
        Some(path) if path.exists() => std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|t| serde_json::from_str::<Settings>(t.trim_start_matches('\u{feff}')).map_err(|e| e.to_string()))
            .map_err(|e| format!("設定ファイル {} を読み込めませんでした（初期設定で起動します）: {e}", path.display())),
        _ => Ok(Settings::default()),
    };
    let settings = loaded.unwrap_or_else(|e| {
        *LOAD_ERROR.lock().unwrap() = Some(e);
        Settings::default()
    });
    // A broken file is left as it is until the settings are changed
    *SAVED_SETTINGS.lock().unwrap() = serde_json::to_string_pretty(&settings).unwrap_or_default();
    settings
}

// The problem found while reading the settings file, once
pub fn take_load_error() -> Option<String> {
    let _ = SETTINGS.peek();
    LOAD_ERROR.lock().unwrap().take()
}

// Write the settings file if SETTINGS changed since the last write
pub fn save_if_changed() -> Result<(), String> {
    let json = serde_json::to_string_pretty(&*SETTINGS.read()).map_err(|e| e.to_string())?;
    if json == *SAVED_SETTINGS.lock().unwrap() {
        return Ok(());
    }
    let path = settings_path().ok_or("設定の保存先が見つかりません")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("設定を保存できませんでした: {e}"))?;
    }
    std::fs::write(&path, &json).map_err(|e| format!("設定を保存できませんでした: {e}"))?;
    *SAVED_SETTINGS.lock().unwrap() = json;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_file_falls_back_to_defaults() {
        let s: Settings = serde_json::from_str(r#"{"layout":{"top_height":50.0},"future_option":1}"#).unwrap();
        assert_eq!(s.default_full_score, 8);
        assert_eq!(s.layout.top_height, 50.0);
        assert_eq!(s.layout.comment_width, Layout::default().comment_width);
        assert!(s.keys.is_empty());
    }
}
