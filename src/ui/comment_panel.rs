use dioxus::prelude::*;
use crate::models::Config;

#[component]
pub fn CommentPanel(
    config: Signal<Config>,
    cur_question_id: Option<u32>,
) -> Element {

    let cfg = config();

    let (initial_text, question_name) = cur_question_id
        .and_then(|id| cfg.questions.iter().find(|q| q.id == id))
        .map(|q| (q.comment.as_str(), q.name.as_str()))
        .unwrap_or(("", "（問題未選択）"));
        
    let on_change = move |e: FormEvent| {
        if let Some(id) = cur_question_id {
            if let Some(q) = config.write().questions.iter_mut().find(|q| q.id == id) {
                q.comment = e.value();
            }
        }
    };

    rsx! {
        div { class: "card bg-base-100 shadow",
            div { class: "card-body gap-3",
                div { class: "card-title flex items-center justify-between",
                    span { {question_name} }
                }

                {cur_question_id.is_none().then(|| rsx!{
                    div { class: "text-sm opacity-70",
                        "点数入力欄にフォーカスすると、その問題のコメントをここで編集できます。"
                    }
                })}

                textarea {
                    class: "textarea textarea-bordered w-full h-full font-mono text-sm leading-snug",
                    placeholder: "各問題に対する採点上の注意点やメモ等を記述",
                    value: initial_text,
                    onchange: on_change,
                }

            }
        }
    }
}