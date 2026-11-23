use dioxus::prelude::*;
use crate::models::{Config, DATA_GEN};
use crate::ui::{GradingState, Section};

#[component]
pub fn CommentPanel(
    config: Signal<Config>,
    state: GradingState,
) -> Element {

    let cfg = config.read();
    let question = cfg.questions.get((state.focus_idx)()).map(|q| (q.id, q.name.clone(), q.comment.clone()));
    drop(cfg);
    let gen = DATA_GEN();

    rsx! {
        div { class: "card bg-base-100 shadow h-full",
            div { class: "card-body gap-2 p-4 min-h-0",
                div { class: "card-title text-base truncate",
                    match &question {
                        Some((_, name, _)) => rsx! { "{name} のコメント" },
                        None => rsx! { "（問題未登録）" },
                    }
                }
                // Re-created per question (and per loaded file) so that typing never has its
                // text overwritten by a late update from Rust
                for (id, _, comment) in question.into_iter() {
                    CommentBox { key: "{gen}-{id}", question_id: id, initial: comment, config, state }
                }
            }
        }
    }
}

#[component]
fn CommentBox(
    question_id: u32,
    initial: String,
    config: Signal<Config>,
    state: GradingState,
) -> Element {
    let initial = use_hook(|| initial);
    let mut section = state.section;
    rsx! {
        textarea {
            id: "comment",
            class: "textarea textarea-bordered w-full flex-1 min-h-0 resize-none font-mono text-sm leading-snug",
            placeholder: "各問題に対する採点上の注意点やメモ等を記述",
            initial_value: initial,
            // Per keystroke so that a save (Ctrl+S) always includes it
            oninput: move |e: FormEvent| {
                if let Some(q) = config.write().questions.iter_mut().find(|q| q.id == question_id) {
                    q.comment = e.value();
                }
            },
            onfocus: move |_| {
                if *section.peek() != Section::Comment {
                    section.set(Section::Comment);
                }
            },
        }
    }
}
