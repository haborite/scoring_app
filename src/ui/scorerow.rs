use dioxus::prelude::*;

/// 1行（問題1つ）のUIを担当。フォーカスはUIに任せる（autofocus + remount）
#[component]
pub fn ScoreRow(
    sid: usize,
    qid: usize,
    q_name: String,
    full: i64,
    value: String,
    focused: bool,
    on_change: EventHandler<String>,
    move_to_next: EventHandler<()>,
    move_to_prev: EventHandler<()>,
) -> Element {

    rsx! {
        div { class: "grid grid-cols-[1fr_auto_auto] md:grid-cols-[12rem_1fr_auto_auto] gap-2 items-center",
            div { class: "font-semibold truncate", "{q_name}" }

            input {
                id: "score-{sid}-{qid}",
                r#type: "number",
                value: value,
                min: 0,
                max: full,
                required: true,
                class: "input validator",
                autofocus: focused,

                oninput: move |e| {
                    let mut s = e.value();
                    s.retain(|c| c.is_ascii_digit());
                    on_change.call(s);
                },

                onkeydown: move |e| {
                    match e.key() {
                        Key::Enter | Key::ArrowDown => {
                            e.prevent_default();
                            move_to_next.call(());
                        },
                        Key::ArrowUp => {
                            e.prevent_default();
                            move_to_prev.call(());
                        }
                        _ => {}
                    }
                },

                onblur: {
                    move |_| {}
                },

            }
            p { class: "validator-hint", "Must be between be 0 to {full}" }

            div { class: "text-sm opacity-60", " / {full}" }
        }
    }
}
