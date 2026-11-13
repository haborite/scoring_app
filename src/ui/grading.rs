use dioxus::prelude::*;
use crate::ui::{SearchWindow, MatrixTable, IndividualPanel};
use crate::models::Config;
use crate::shortcuts::Shortcut;

// Where the grader is: kept by App so it survives page switches
#[derive(Clone, Copy, PartialEq)]
pub struct GradingState {
    pub cur_student_idx: Signal<usize>,
    // Index of the score input that has (or gets) the focus
    pub focus_idx: Signal<usize>,
    pub search_open: Signal<bool>,
}

pub fn use_grading_state() -> GradingState {
    GradingState {
        cur_student_idx: use_signal(|| 0),
        focus_idx: use_signal(|| 0),
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
}

#[component]
pub fn GradingPage(
    config: Signal<Config>,
    state: GradingState,
) -> Element {

    let GradingState { mut cur_student_idx, mut focus_idx, search_open } = state;
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

    // Focus the search box, or the current score input whenever the student or question changes
    use_effect(move || {
        let student = cur_student_idx();
        let el_id = if search_open() { "search".to_string() } else { format!("score-{}", focus_idx()) };
        let js = format!(
            r#"queueMicrotask(() => {{
                const el = document.getElementById("{el_id}");
                if (el) {{
                    el.focus();
                    if (el.select) el.select();
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

    rsx! {
        div {
            { (!msg().is_empty()).then(|| rsx! {
                div { class: "alert alert-error mb-2", "{msg}" }
            })}
            IndividualPanel { cur_student_idx, focus_idx, search_open, config }
            MatrixTable { config, cur_student_idx }
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

#[component]
fn ShortcutHelp() -> Element {
    let items = [
        (vec!["Enter".to_string(), "↓".to_string()], "次の問題"),
        (vec!["↑".to_string()], "前の問題"),
        (vec![Shortcut::PrevStudent.label()], "前の受験者"),
        (vec![Shortcut::NextStudent.label()], "次の受験者"),
        (vec![Shortcut::Search.label()], "受験者を検索"),
        (vec![Shortcut::Save.label()], "保存"),
    ];
    rsx! {
        div { class: "flex flex-wrap gap-x-4 gap-y-1 mt-2 px-2 text-xs opacity-70",
            for (keys, label) in items {
                span { class: "flex items-center gap-1",
                    for (i, k) in keys.iter().enumerate() {
                        if i > 0 { "/" }
                        kbd { class: "kbd kbd-xs", "{k}" }
                    }
                    span { "{label}" }
                }
            }
            span { class: "ml-auto", "テンキー: + / − で問題、/ * で受験者、. で検索" }
        }
    }
}
