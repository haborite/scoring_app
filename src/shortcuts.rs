use std::collections::BTreeMap;

// Keyboard actions. Global ones work wherever the focus is and must use Ctrl (Cmd on macOS)
// so they never get in the way of typing; score-field ones only work in a score input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Action {
    Save,
    SaveAs,
    Load,
    GotoQuestions,
    GotoStudents,
    GotoGrading,
    GotoRating,
    GotoSettings,
    Search,
    PrevStudent,
    NextStudent,
    NextRegrade,
    ToggleSection,
    NextQuestion,
    PrevQuestion,
    FieldPrevStudent,
    FieldNextStudent,
    FieldSearch,
    ConfirmScore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Global,
    ScoreField,
}

pub const MOD_LABEL: &str = if cfg!(target_os = "macos") { "⌘" } else { "Ctrl" };

impl Action {
    pub const ALL: [Action; 19] = [
        Action::Save,
        Action::SaveAs,
        Action::Load,
        Action::GotoQuestions,
        Action::GotoStudents,
        Action::GotoGrading,
        Action::GotoRating,
        Action::GotoSettings,
        Action::Search,
        Action::PrevStudent,
        Action::NextStudent,
        Action::NextRegrade,
        Action::ToggleSection,
        Action::NextQuestion,
        Action::PrevQuestion,
        Action::FieldPrevStudent,
        Action::FieldNextStudent,
        Action::FieldSearch,
        Action::ConfirmScore,
    ];

    // Name used in the settings file and in messages from the key listener
    pub fn id(self) -> String {
        format!("{self:?}")
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Action::ALL.into_iter().find(|a| a.id() == id)
    }

    pub fn scope(self) -> Scope {
        use Action::*;
        match self {
            NextQuestion | PrevQuestion | FieldPrevStudent | FieldNextStudent | FieldSearch
            | ConfirmScore => Scope::ScoreField,
            _ => Scope::Global,
        }
    }

    pub fn label(self) -> &'static str {
        use Action::*;
        match self {
            Save => "上書き保存",
            SaveAs => "名前を付けて保存",
            Load => "ファイルを開く",
            GotoQuestions => "問題設定画面へ",
            GotoStudents => "受験者設定画面へ",
            GotoGrading => "採点画面へ",
            GotoRating => "成績評価画面へ",
            GotoSettings => "設定画面へ",
            Search => "受験者を検索",
            PrevStudent => "前の受験者",
            NextStudent => "次の受験者",
            NextRegrade => "次の要再採点の受験者",
            ToggleSection => "点数入力 ⇄ コメント",
            NextQuestion => "次の問題",
            PrevQuestion => "前の問題",
            FieldPrevStudent => "前の受験者",
            FieldNextStudent => "次の受験者",
            FieldSearch => "受験者を検索",
            ConfirmScore => "要再採点を確認済みにする",
        }
    }

    // Combos are "Ctrl+Alt+Shift+<KeyboardEvent.code>", so they do not depend on the layout or IME
    pub fn default_keys(self) -> &'static [&'static str] {
        use Action::*;
        match self {
            Save => &["Ctrl+KeyS"],
            SaveAs => &["Ctrl+Shift+KeyS"],
            Load => &["Ctrl+KeyO"],
            GotoQuestions => &["Ctrl+Digit1"],
            GotoStudents => &["Ctrl+Digit2"],
            GotoGrading => &["Ctrl+Digit3"],
            GotoRating => &["Ctrl+Digit4"],
            GotoSettings => &["Ctrl+Digit5"],
            Search => &["Ctrl+KeyF"],
            PrevStudent => &["Ctrl+KeyJ"],
            NextStudent => &["Ctrl+KeyL"],
            NextRegrade => &["Ctrl+Shift+KeyL"],
            ToggleSection => &["Ctrl+KeyE"],
            NextQuestion => &["Enter", "NumpadEnter", "ArrowDown", "NumpadAdd"],
            PrevQuestion => &["ArrowUp", "NumpadSubtract"],
            FieldPrevStudent => &["NumpadDivide"],
            FieldNextStudent => &["NumpadMultiply"],
            FieldSearch => &["NumpadDecimal"],
            ConfirmScore => &["Ctrl+Enter"],
        }
    }
}

// User overrides on top of the defaults, keyed by Action::id. Unknown ids are kept
// (and ignored) so that a settings file from another version still loads.
pub type KeyOverrides = BTreeMap<String, Vec<String>>;

pub fn keys_for(overrides: &KeyOverrides, action: Action) -> Vec<String> {
    match overrides.get(&action.id()) {
        Some(keys) => keys.clone(),
        None => action.default_keys().iter().map(|k| k.to_string()).collect(),
    }
}

// The action already using a combo, if any
pub fn owner_of(overrides: &KeyOverrides, combo: &str) -> Option<Action> {
    Action::ALL.into_iter().find(|&a| keys_for(overrides, a).iter().any(|k| k == combo))
}

// Whether `combo` may be bound to `action`
pub fn check_combo(overrides: &KeyOverrides, action: Action, combo: &str) -> Result<(), String> {
    let (mods, code) = split_combo(combo);
    let ctrl = mods.contains(&"Ctrl");
    let alt = mods.contains(&"Alt");
    let shift = mods.contains(&"Shift");

    if ctrl && !alt && matches!(code, "KeyA" | "KeyC" | "KeyV" | "KeyX" | "KeyY" | "KeyZ") {
        return Err(format!("{} は文字の編集に使うため割り当てられません", combo_label(combo)));
    }
    match action.scope() {
        Scope::Global if !ctrl => {
            return Err(format!(
                "どこでも使える操作には {MOD_LABEL} を含むキーを割り当ててください（文字入力の妨げになるため）"
            ));
        }
        Scope::ScoreField if !ctrl && !alt => {
            let editing = code.starts_with("Digit")
                || (code.starts_with("Numpad") && code[6..].chars().all(|c| c.is_ascii_digit()))
                || matches!(code, "Backspace" | "Delete" | "Tab" | "ArrowLeft" | "ArrowRight" | "Home" | "End");
            if editing && !(shift && code.starts_with("Arrow")) {
                return Err(format!("{} は点数の入力に使うため割り当てられません", combo_label(combo)));
            }
        }
        _ => {}
    }
    match owner_of(overrides, combo) {
        Some(other) if other == action => Err(format!("{} は既に割り当てられています", combo_label(combo))),
        Some(other) => Err(format!("{} は「{}」で使われています", combo_label(combo), other.label())),
        None => Ok(()),
    }
}

fn split_combo(combo: &str) -> (Vec<&str>, &str) {
    let mut parts: Vec<&str> = combo.split('+').collect();
    // "NumpadAdd" never contains '+', but a trailing empty part would mean the key itself is '+'
    let code = parts.pop().unwrap_or("");
    (parts, code)
}

// Label shown in the UI, e.g. "Ctrl+Shift+S", "テンキー+"
pub fn combo_label(combo: &str) -> String {
    let (mods, code) = split_combo(combo);
    let key = match code {
        "ArrowUp" => "↑".to_string(),
        "ArrowDown" => "↓".to_string(),
        "ArrowLeft" => "←".to_string(),
        "ArrowRight" => "→".to_string(),
        "Escape" => "Esc".to_string(),
        "NumpadEnter" => "テンキーEnter".to_string(),
        "NumpadAdd" => "テンキー+".to_string(),
        "NumpadSubtract" => "テンキー−".to_string(),
        "NumpadMultiply" => "テンキー*".to_string(),
        "NumpadDivide" => "テンキー/".to_string(),
        "NumpadDecimal" => "テンキー.".to_string(),
        "Semicolon" => ";".to_string(),
        "Quote" => "'".to_string(),
        "Comma" => ",".to_string(),
        "Period" => ".".to_string(),
        "Slash" => "/".to_string(),
        "Backslash" | "IntlYen" | "IntlRo" => "\\".to_string(),
        "BracketLeft" => "[".to_string(),
        "BracketRight" => "]".to_string(),
        "Minus" => "-".to_string(),
        "Equal" => "=".to_string(),
        "Backquote" => "`".to_string(),
        c => {
            if let Some(n) = c.strip_prefix("Numpad") {
                format!("テンキー{n}")
            } else {
                c.trim_start_matches("Key").trim_start_matches("Digit").to_string()
            }
        }
    };
    let mut out: Vec<String> = mods
        .iter()
        .map(|m| if *m == "Ctrl" { MOD_LABEL.to_string() } else { m.to_string() })
        .collect();
    out.push(key);
    out.join("+")
}

// Labels of every key bound to an action, joined with " / "
pub fn action_label(overrides: &KeyOverrides, action: Action) -> String {
    keys_for(overrides, action).iter().map(|k| combo_label(k)).collect::<Vec<_>>().join(" / ")
}

// JS statement that installs the current bindings for the listener
pub fn keymap_js(overrides: &KeyOverrides) -> String {
    let mut global = serde_json::Map::new();
    let mut field = serde_json::Map::new();
    for a in Action::ALL {
        let target = match a.scope() {
            Scope::Global => &mut global,
            Scope::ScoreField => &mut field,
        };
        for k in keys_for(overrides, a) {
            target.insert(k, serde_json::Value::String(a.id()));
        }
    }
    let maps = serde_json::json!({ "global": global, "field": field });
    format!("window.__keyMaps = {maps};")
}

pub fn recording_js(on: bool) -> String {
    format!("window.__keyRecording = {on};")
}

// Message from the listener
pub enum KeyMsg {
    Action(Action),
    // A combo pressed while a binding is being recorded
    Record(String),
}

impl KeyMsg {
    pub fn parse(msg: &str) -> Option<Self> {
        if let Some(a) = msg.strip_prefix("action:") {
            return Action::from_id(a).map(KeyMsg::Action);
        }
        msg.strip_prefix("record:").map(|c| KeyMsg::Record(c.to_string()))
    }
}

// Document-level listener (capture phase). Bound combos are sent to Rust and kept from the
// webview (e.g. its own find bar on Ctrl+F). The bindings live in window.__keyMaps.
pub fn listener_js() -> String {
    r#"window.__keyMaps = window.__keyMaps || { global: {}, field: {} };
        window.__keyRecording = window.__keyRecording || false;
        const comboOf = (e) => {
            // Synthesized key events (remote desktop, automation tools) may lack e.code
            let code = e.code;
            if (!code && /^[a-z]$/i.test(e.key)) code = "Key" + e.key.toUpperCase();
            if (!code && /^[0-9]$/.test(e.key)) code = "Digit" + e.key;
            if (!code || /^(Control|Shift|Alt|Meta|OS)(Left|Right)?$/.test(code)) return null;
            return (e.ctrlKey || e.metaKey ? "Ctrl+" : "") + (e.altKey ? "Alt+" : "")
                + (e.shiftKey ? "Shift+" : "") + code;
        };
        document.addEventListener("keydown", (e) => {
            if (e.isComposing || e.keyCode === 229) return;
            const combo = comboOf(e);
            if (!combo) return;
            if (window.__keyRecording) {
                e.preventDefault();
                e.stopPropagation();
                if (!e.repeat) dioxus.send("record:" + combo);
                return;
            }
            const el = document.activeElement;
            const maps = window.__keyMaps;
            let action = maps.global[combo];
            const inScore = !action && el && el.dataset && el.dataset.scoreInput !== undefined;
            if (inScore) action = maps.field[combo];
            if (!action && el && el.id === "comment" && combo === "Escape") action = "ToggleSection";
            if (!action) return;
            e.preventDefault();
            e.stopPropagation();
            if (!inScore) {
                if (e.repeat) return;
                // Commit a field that is still being edited (its change event) before saving
                if (el && el !== document.body && el.blur) {
                    el.blur();
                    el.focus();
                }
            }
            dioxus.send("action:" + action);
        }, true);
        // Keep the channel open for the lifetime of the page
        await new Promise(() => {});"#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_have_no_conflicts_and_are_valid() {
        let none = KeyOverrides::new();
        let mut seen = std::collections::HashSet::new();
        for a in Action::ALL {
            for k in a.default_keys() {
                assert!(seen.insert(*k), "{k} bound twice");
                // Valid for its action once it is not counted as already bound
                let mut without = KeyOverrides::new();
                without.insert(a.id(), vec![]);
                assert_eq!(check_combo(&without, a, k), Ok(()), "{k}");
            }
        }
        assert_eq!(owner_of(&none, "Ctrl+KeyS"), Some(Action::Save));
    }

    #[test]
    fn check_combo_rules() {
        let none = KeyOverrides::new();
        assert!(check_combo(&none, Action::Save, "KeyQ").is_err()); // global needs Ctrl
        assert!(check_combo(&none, Action::Save, "Ctrl+KeyC").is_err()); // editing key
        assert!(check_combo(&none, Action::Save, "Ctrl+KeyL").is_err()); // used elsewhere
        assert!(check_combo(&none, Action::Save, "Ctrl+KeyQ").is_ok());
        assert!(check_combo(&none, Action::NextQuestion, "Digit3").is_err());
        assert!(check_combo(&none, Action::NextQuestion, "Numpad3").is_err());
        assert!(check_combo(&none, Action::NextQuestion, "Backspace").is_err());
        assert!(check_combo(&none, Action::NextQuestion, "Space").is_ok());
    }

    #[test]
    fn overrides_replace_defaults() {
        let mut o = KeyOverrides::new();
        o.insert(Action::Save.id(), vec!["Ctrl+KeyQ".to_string()]);
        o.insert("RemovedAction".to_string(), vec!["Ctrl+KeyS".to_string()]);
        assert_eq!(keys_for(&o, Action::Save), vec!["Ctrl+KeyQ"]);
        assert_eq!(owner_of(&o, "Ctrl+KeyS"), None);
        assert_eq!(Action::from_id("SaveAs"), Some(Action::SaveAs));
    }

    #[test]
    fn labels() {
        assert_eq!(combo_label("Ctrl+Shift+KeyS"), format!("{MOD_LABEL}+Shift+S"));
        assert_eq!(combo_label("Ctrl+Digit4"), format!("{MOD_LABEL}+4"));
        assert_eq!(combo_label("NumpadAdd"), "テンキー+");
        assert_eq!(combo_label("ArrowDown"), "↓");
    }
}
