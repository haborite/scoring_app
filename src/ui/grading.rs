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
    let search_open = use_signal(|| false);    

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
            // Top bar / Navbar
            div { class: "navbar bg-base-100 rounded-box shadow mb-4",
                div { class: "navbar-start gap-2",
                    button { class: "btn btn-sm btn-primary", onclick: move |_| on_back.call(()), "戻る" }
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
            IndividualPanel { cur_student_idx, search_open, config }

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