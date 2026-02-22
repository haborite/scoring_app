// src/ui/grading.rs
use dioxus::prelude::*;

use crate::ui::scorerows::ScoreRows;
use crate::ui::search::SearchWindow;
use crate::models::{Config, Question, Student};

#[component]
pub fn GradingPage(
    on_back: EventHandler<()>,
    config: Signal<Config>,
) -> Element {
    // master data (signal から毎回読むと重い/揺れるので、ページ内 state に固定)
    let questions = use_signal(|| config.read().questions.clone());
    let students  = use_signal(|| config.read().students.clone());

    // navigation
    let mut cur_student_idx = use_signal(|| 0usize);

    // search popup
    let search_open = use_signal(|| false);
    let search_q = use_signal(|| String::new());

    // helper: 現在の student
    let cur_student = students
        .read()
        .get(cur_student_idx())
        .cloned();

    // back
    let on_back_click = move |_| on_back.call(());

    // open search
    let on_open_search = {
        let mut search_open = search_open;
        move |_| {
            search_q.set(String::new());
            search_open.set(true);
        }
    };

    // move prev/next
    let on_prev = {
        move |_| {
            let n = students.read().len();
            if n == 0 { return; }
            let i = cur_student_idx();
            cur_student_idx.set(if i == 0 { 0 } else { i - 1 });
        }
    };
    let on_next = {
        move |_| {
            let n = students.read().len();
            if n == 0 { return; }
            let i = cur_student_idx();
            cur_student_idx.set((i + 1).min(n - 1));
        }
    };

    rsx! {
        div { class: "p-4 space-y-4",

            // header
            div { class: "flex items-center gap-2",
                button { class: "btn btn-sm", onclick: on_back_click, "Back" }
                div { class: "flex-1" }
                button { class: "btn btn-sm", onclick: on_prev, "Prev" }
                button { class: "btn btn-sm", onclick: on_next, "Next" }
                button { class: "btn btn-sm btn-primary", onclick: on_open_search, "Search" }
            }

            // current student card
            div { class: "card bg-base-100 shadow",
                div { class: "card-body p-4",
                    if let Some(s) = cur_student {
                        div { class: "flex flex-wrap items-baseline gap-3",
                            h2 { class: "text-lg font-bold", "{s.name}" }
                            span { class: "badge badge-outline", "{s.id}" }
                            span { class: "opacity-70 text-sm",
                                "({cur_student_idx() + 1}/{students.read().len()})",
                            }
                        }
                    } else {
                        div { class: "opacity-70", "No students. Import students first." }
                    }
                }
            }

            // score rows
            if let Some(s) = cur_student {
                ScoreRows {
                    config,
                    student_id: s.id.clone(),
                    questions,
                }
            }

            // search window (modal)
            if search_open() {
                SearchWindow {
                    is_open: search_open,
                    query: search_q,
                    students,
                    on_select: {
                        let mut cur_student_idx = cur_student_idx;
                        let mut search_open = search_open;
                        move |idx: usize| {
                            cur_student_idx.set(idx);
                            search_open.set(false);
                        }
                    }
                }
            }
        }
    }
}