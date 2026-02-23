use dioxus::prelude::*;
use crate::ui::{SearchWindow, MatrixTable, IndividualPanel};
use crate::models::Config;

#[component]
pub fn GradingPage(
    on_back: EventHandler<()>,
    config: Signal<Config>,
) -> Element {

    // navigation
    let cur_student_idx = use_signal(|| 0usize);
    let mut search_open = use_signal(|| false);    

    // message
    let msg = use_signal(|| String::new());

    use_effect(move || {
        let el_id = if search_open() { "search" } else { "score-0" };
        let js = format!(
            r#"queueMicrotask(() => {{
                const el = document.getElementById("{el_id}");
                if (el) {{
                    el.focus();
                    if (el.select) el.select();
                }}
            }});"#
        );
        let _ = document::eval(&js);
    });

    rsx! {
        div {
            class: "min-h-screen p-4 bg-base-200 text-base-content",
            tabindex: "0",

            // global hotkeys: F opens search
            onkeydown: move |e| {
                if let Key::Character(chr) = e.key() {
                    match chr.as_str() {
                        "f" | "F" => {
                            e.prevent_default();
                            search_open.set(true);
                        }
                        "j" | "J" => {
                            e.prevent_default();
                            mv_prev_student(cur_student_idx);
                        }
                        "l" | "L" => {
                            e.prevent_default();
                            mv_next_student(cur_student_idx, config().students.len());
                        }
                        _ => {}
                    }
                } else {
                    match e.key() {
                        Key::Escape => {
                            e.prevent_default();
                            search_open.set(false);
                        }
                        _ => {}
                    }
                }
            },

            // Top bar / Navbar
            div { class: "navbar bg-base-100 rounded-box shadow mb-4",
                div { class: "navbar-start gap-2",
                    button { class: "btn btn-sm btn-primary", onclick: move |_| on_back.call(()), "戻る" }
                    button { class: "btn btn-sm", onclick: move |_| { mv_prev_student(cur_student_idx) }, "← 前" }
                    button { class: "btn btn-sm", onclick: move |_| { mv_next_student(cur_student_idx, config().students.len()) }, "次 →" }
                }
                div { class: "navbar-center",
                    div { class: "text-lg font-bold", "cur_student_label(config, cur_student_idx)" }
                }
                div { class: "navbar-end",
                    div { class: "text-xs opacity-70 hidden md:block",
                        "F:検索 / J,K:受験者移動 / Enter:次問題"
                    }
                }
            }

            {(!msg.read().is_empty()).then(|| rsx! {
                div { class: "alert alert-error mb-4", "{msg}" }
            })}

            // Individual area
            IndividualPanel { cur_student_idx, config }

            // table panel card
            MatrixTable { config }

            // search modal
            {
                search_open().then(|| 
                    rsx!{
                        SearchWindow { 
                            is_open: search_open, 
                            msg, 
                            config, 
                            cur_student_idx, 
                        }
                    }
                )
            }
        }
    }
}

fn mv_prev_student(
    mut cur_student_idx: Signal<usize>
) {
    let current_idx = cur_student_idx();
    let idx = current_idx.saturating_sub(1);
    cur_student_idx.set(idx);
}

fn mv_next_student(
    mut cur_student_idx: Signal<usize>, 
    student_count: usize
) {
    let current_idx = cur_student_idx();
    let max_idx = student_count.saturating_sub(1);
    let idx = std::cmp::min(current_idx + 1, max_idx);
    cur_student_idx.set(idx);
}

