use dioxus::prelude::*;
use crate::ui::{SearchWindow, MatrixTable, CommentPanel, ScoreRows};
use crate::models::{Config, DATA_GEN};
use crate::settings::{Layout, SETTINGS};
use crate::shortcuts::Action;

// Which part of the grading page has the keyboard
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Scores,
    Comment,
}

// Where the grader is: kept by App so it survives page switches
#[derive(Clone, Copy, PartialEq)]
pub struct GradingState {
    pub cur_student_idx: Signal<usize>,
    // Index of the current question (its score input has or gets the focus)
    pub focus_idx: Signal<usize>,
    pub section: Signal<Section>,
    pub search_open: Signal<bool>,
}

pub fn use_grading_state() -> GradingState {
    GradingState {
        cur_student_idx: use_signal(|| 0),
        focus_idx: use_signal(|| 0),
        section: use_signal(|| Section::Scores),
        search_open: use_signal(|| false),
    }
}

impl GradingState {
    pub fn prev_student(mut self) {
        let idx = self.cur_student_idx.peek().saturating_sub(1);
        self.cur_student_idx.set(idx);
    }

    pub fn next_student(mut self, student_count: usize) {
        let idx = (*self.cur_student_idx.peek() + 1).min(student_count.saturating_sub(1));
        self.cur_student_idx.set(idx);
    }

    pub fn open_search(mut self) {
        self.search_open.set(true);
    }

    pub fn goto(mut self, student_idx: usize, question_idx: usize) {
        self.cur_student_idx.set(student_idx);
        self.focus_idx.set(question_idx);
        self.section.set(Section::Scores);
    }

    pub fn toggle_section(mut self) {
        let next = match *self.section.peek() {
            Section::Scores => Section::Comment,
            Section::Comment => Section::Scores,
        };
        self.section.set(next);
    }

    pub fn move_question(mut self, delta: isize, question_count: usize) {
        let cur = *self.focus_idx.peek() as isize;
        let idx = (cur + delta).clamp(0, question_count.saturating_sub(1) as isize) as usize;
        self.focus_idx.set(idx);
    }
}

// Track sizes for two panes with a splitter between them, as fractions so gaps need no care
fn tracks(first: f64) -> String {
    format!("minmax(0, {first}fr) 0.5rem minmax(0, {}fr)", 100.0 - first)
}

#[component]
pub fn GradingPage(
    config: Signal<Config>,
    state: GradingState,
) -> Element {

    let GradingState { mut cur_student_idx, mut focus_idx, section, search_open } = state;
    let msg = use_signal(String::new);

    // Keep the indices in range after students/questions are deleted or another file is loaded
    use_effect(move || {
        let len = config.read().students.len();
        if cur_student_idx() >= len {
            cur_student_idx.set(len.saturating_sub(1));
        }
    });
    use_effect(move || {
        let len = config.read().questions.len();
        if focus_idx() >= len {
            focus_idx.set(len.saturating_sub(1));
        }
    });

    // Focus the search box, the comment, or the current score input whenever one of them changes
    use_effect(move || {
        let student = cur_student_idx();
        let question = focus_idx();
        let _ = DATA_GEN();
        let (el_id, select) = if search_open() {
            ("search".to_string(), true)
        } else if section() == Section::Comment {
            ("comment".to_string(), false)
        } else {
            (format!("score-{question}"), true)
        };
        let js = format!(
            r#"queueMicrotask(() => {{
                const el = document.getElementById("{el_id}");
                if (el && document.activeElement !== el) {{
                    el.focus();
                    if ({select} && el.select) el.select();
                }} else if (el && {select} && el.select) {{
                    el.select();
                }}
                // Scroll the student's row of the score table into view (only the table)
                const row = document.getElementById("matrix-row-{student}");
                const box = row && row.closest(".overflow-auto");
                if (box) {{
                    const head = box.querySelector("thead");
                    const top = box.getBoundingClientRect().top + (head ? head.offsetHeight : 0);
                    const r = row.getBoundingClientRect();
                    const bottom = box.getBoundingClientRect().bottom;
                    if (r.top < top) box.scrollTop -= top - r.top;
                    else if (r.bottom > bottom) box.scrollTop += r.bottom - bottom;
                }}
            }});"#
        );
        let _ = document::eval(&js);
    });

    let layout = SETTINGS.read().layout;
    let style = format!(
        "grid-template-columns: {}; grid-template-rows: {};",
        tracks(layout.comment_width),
        tracks(layout.top_height),
    );

    rsx! {
        div { class: "h-full flex flex-col gap-1",
            { (!msg().is_empty()).then(|| rsx! {
                div { class: "alert alert-error", "{msg}" }
            })}
            div { id: "grading-layout", class: "flex-1 min-h-0 grid", style,
                div { class: "min-h-0 min-w-0",
                    CommentPanel { config, state }
                }
                Splitter { vertical: true }
                div { class: "min-h-0 min-w-0",
                    ScoreRows { config, state }
                }
                Splitter { vertical: false }
                div { class: "min-h-0 min-w-0 col-span-3",
                    MatrixTable { config, cur_student_idx }
                }
            }
            ShortcutHelp {}
            { search_open().then(|| rsx!{
                SearchWindow {
                    is_open: search_open,
                    msg,
                    config,
                    cur_student_idx,
                }
            })}
        }
    }
}

// Drag handle between two panes. The drag itself runs in JS; the size is stored on release.
#[component]
fn Splitter(vertical: bool) -> Element {
    let (min, max) = if vertical { Layout::COMMENT_WIDTH } else { Layout::TOP_HEIGHT };
    let start = move |e: MouseEvent| {
        e.prevent_default();
        let js = format!(
            r#"const box = document.getElementById("grading-layout");
            if (!box) {{ dioxus.send(null); return; }}
            const vertical = {vertical};
            const tracks = (p) => `minmax(0, ${{p}}fr) 0.5rem minmax(0, ${{100 - p}}fr)`;
            let last = null;
            const move = (e) => {{
                const r = box.getBoundingClientRect();
                let p = vertical ? (e.clientX - r.left) / r.width * 100 : (e.clientY - r.top) / r.height * 100;
                last = Math.round(Math.min({max}, Math.max({min}, p)) * 10) / 10;
                if (vertical) box.style.gridTemplateColumns = tracks(last);
                else box.style.gridTemplateRows = tracks(last);
            }};
            document.body.style.userSelect = "none";
            document.body.style.cursor = vertical ? "col-resize" : "row-resize";
            window.addEventListener("mousemove", move);
            await new Promise((resolve) => window.addEventListener("mouseup", resolve, {{ once: true }}));
            window.removeEventListener("mousemove", move);
            document.body.style.userSelect = "";
            document.body.style.cursor = "";
            dioxus.send(last);"#
        );
        spawn(async move {
            let mut drag = document::eval(&js);
            if let Ok(Some(p)) = drag.recv::<Option<f64>>().await {
                let mut s = SETTINGS.write();
                if vertical { s.layout.comment_width = p } else { s.layout.top_height = p }
            }
        });
    };
    let reset = move |_| {
        let d = Layout::default();
        let mut s = SETTINGS.write();
        if vertical { s.layout.comment_width = d.comment_width } else { s.layout.top_height = d.top_height }
    };
    rsx! {
        div {
            class: if vertical { "group flex justify-center cursor-col-resize" } else { "group col-span-3 flex flex-col justify-center cursor-row-resize" },
            title: "ドラッグで大きさを変更（ダブルクリックで元に戻す）",
            onmousedown: start,
            ondoubleclick: reset,
            div { class: if vertical { "w-1 h-full rounded bg-base-300 group-hover:bg-primary/50" } else { "h-1 w-full rounded bg-base-300 group-hover:bg-primary/50" } }
        }
    }
}

#[component]
fn ShortcutHelp() -> Element {
    let settings = SETTINGS.read();
    let items: Vec<(Vec<String>, &str)> = [
        Action::NextQuestion,
        Action::PrevQuestion,
        Action::PrevStudent,
        Action::NextStudent,
        Action::ToggleSection,
        Action::Search,
        Action::NextRegrade,
        Action::ConfirmScore,
        Action::Save,
    ]
    .into_iter()
    .map(|a| {
        let keys = settings.keys(a).iter().map(|k| crate::shortcuts::combo_label(k)).collect();
        (keys, a.label())
    })
    .filter(|(keys, _): &(Vec<String>, &str)| !keys.is_empty())
    .collect();
    drop(settings);
    rsx! {
        div { class: "flex flex-wrap gap-x-4 gap-y-1 px-2 text-xs opacity-70",
            for (keys, label) in items {
                span { class: "flex items-center gap-1",
                    for (i, k) in keys.iter().enumerate() {
                        if i > 0 { "/" }
                        kbd { class: "kbd kbd-xs", "{k}" }
                    }
                    span { "{label}" }
                }
            }
            span { class: "flex items-center gap-1",
                kbd { class: "kbd kbd-xs", "Esc" }
                span { "コメントから点数入力へ戻る" }
            }
        }
    }
}
