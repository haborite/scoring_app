use crate::models::Config;
use crate::ui::{CommentPanel, ScoreRows};
use dioxus::prelude::*;

#[component]
pub fn IndividualPanel(
    cur_student_idx: Signal<usize>,
    search_open: Signal<bool>,
    config: Signal<Config>
) -> Element {

    let focus_idx = use_signal(|| 0usize);
    let cur_question_id = use_signal(|| None::<u32>);

    use_effect(move || {
        let js = format!(
            r#"queueMicrotask(() => {{
                const el = document.getElementById("score-{focus_idx}");
                if (el) {{
                    el.focus();
                    if (el.select) el.select();
                }}
            }});"#
        );
        let _ = document::eval(&js);
    });

    rsx! {
        div { class: "grid grid-cols-1 xl:grid-cols-[1fr_60rem] gap-2",
            CommentPanel { config, cur_question_id: cur_question_id() }
            ScoreRows { cur_student_idx, cur_question_id, config, focus_idx, search_open }
        }
    }
}
