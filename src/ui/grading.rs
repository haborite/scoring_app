use dioxus::prelude::*;
use crate::Page;
use crate::ui::{SearchWindow, MatrixTable, IndividualPanel, TopBar};
use crate::models::Config;

#[component]
pub fn GradingPage(
    on_nav: EventHandler<Page>,
    config: Signal<Config>,
) -> Element {
    
    let mut cur_student_idx = use_signal(|| 0usize);
    let search_open = use_signal(|| false);    
    let msg = use_signal(|| String::new());

    // Keep the index in range after students are deleted or another file is loaded
    use_effect(move || {
        let len = config.read().students.len();
        if cur_student_idx() >= len {
            cur_student_idx.set(len.saturating_sub(1));
        }
    });

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
            class: "min-h-screen p-2 bg-base-200 text-base-content",
            tabindex: "0",
            TopBar { config, on_nav }
            { (!msg().is_empty()).then(|| rsx! {
                div { class: "alert alert-error mb-2", "{msg}" }
            })}
            IndividualPanel { cur_student_idx, search_open, config }
            MatrixTable { config }
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