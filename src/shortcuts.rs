use crate::models::Page;

// Application-wide keyboard shortcuts. All of them use Ctrl (Cmd on macOS), so they
// work wherever the focus is without getting in the way of typing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shortcut {
    Save,
    SaveAs,
    Load,
    Goto(Page),
    Search,
    PrevStudent,
    NextStudent,
}

// (combo sent by the listener, action). The combo is built from KeyboardEvent.code,
// so it does not depend on the keyboard layout or IME state.
const BINDINGS: &[(&str, Shortcut)] = &[
    ("KeyS", Shortcut::Save),
    ("Shift+KeyS", Shortcut::SaveAs),
    ("KeyO", Shortcut::Load),
    ("Digit1", Shortcut::Goto(Page::MasterQuestions)),
    ("Digit2", Shortcut::Goto(Page::MasterStudents)),
    ("Digit3", Shortcut::Goto(Page::Grading)),
    ("Digit4", Shortcut::Goto(Page::Rating)),
    ("KeyF", Shortcut::Search),
    ("KeyJ", Shortcut::PrevStudent),
    ("KeyL", Shortcut::NextStudent),
];

pub const MOD_LABEL: &str = if cfg!(target_os = "macos") { "⌘" } else { "Ctrl" };

impl Shortcut {
    pub fn from_combo(combo: &str) -> Option<Self> {
        BINDINGS.iter().find(|(c, _)| *c == combo).map(|(_, s)| *s)
    }

    // Label shown in the UI, e.g. "Ctrl+Shift+S"
    pub fn label(self) -> String {
        let (combo, _) = BINDINGS.iter().find(|(_, s)| *s == self).expect("every shortcut is bound");
        let key = combo.rsplit('+').next().unwrap_or(combo);
        let key = key.trim_start_matches("Key").trim_start_matches("Digit");
        if combo.starts_with("Shift+") {
            format!("{MOD_LABEL}+Shift+{key}")
        } else {
            format!("{MOD_LABEL}+{key}")
        }
    }
}

// Document-level listener (capture phase) that forwards bound combos to Rust and keeps the
// webview from handling them (e.g. its own find bar on Ctrl+F).
pub fn listener_js() -> String {
    let combos: Vec<&str> = BINDINGS.iter().map(|(c, _)| *c).collect();
    let combos = serde_json::to_string(&combos).unwrap_or_default();
    format!(
        r#"const combos = new Set({combos});
        document.addEventListener("keydown", (e) => {{
            if (e.isComposing || e.altKey || !(e.ctrlKey || e.metaKey)) return;
            // Synthesized key events (remote desktop, automation tools) may lack e.code
            let code = e.code;
            if (!code && /^[a-z]$/i.test(e.key)) code = "Key" + e.key.toUpperCase();
            if (!code && /^[0-9]$/.test(e.key)) code = "Digit" + e.key;
            const combo = (e.shiftKey ? "Shift+" : "") + code;
            if (!combos.has(combo)) return;
            e.preventDefault();
            e.stopPropagation();
            if (e.repeat) return;
            // Commit a field that is still being edited (its change event) before saving
            const el = document.activeElement;
            if (el && el !== document.body && el.blur) {{
                el.blur();
                el.focus();
            }}
            dioxus.send(combo);
        }}, true);
        // Keep the channel open for the lifetime of the page
        await new Promise(() => {{}});"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combos_map_to_shortcuts() {
        assert_eq!(Shortcut::from_combo("KeyS"), Some(Shortcut::Save));
        assert_eq!(Shortcut::from_combo("Shift+KeyS"), Some(Shortcut::SaveAs));
        assert_eq!(Shortcut::from_combo("Digit3"), Some(Shortcut::Goto(Page::Grading)));
        assert_eq!(Shortcut::from_combo("KeyX"), None);
    }

    #[test]
    fn labels() {
        assert_eq!(Shortcut::SaveAs.label(), format!("{MOD_LABEL}+Shift+S"));
        assert_eq!(Shortcut::Goto(Page::Rating).label(), format!("{MOD_LABEL}+4"));
    }
}
