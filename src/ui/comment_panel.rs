use dioxus::prelude::*;
use crate::models::Config;

#[component]
pub fn CommentPanel(
    config: Signal<Config>,
    cur_question_id: Option<u32>,
) -> Element {

    let q_opt = {
        cur_question_id
            .and_then(
                |id| config().questions
                .into_iter()
                .find(|q| q.id == id)
            )
    };

    let initial_text = q_opt.clone()
        .and_then(|q| Some(q.comment.clone()))
        .unwrap_or_default();

    let on_change = move |e: FormEvent| {
        let new_text = e.value();
        let qid = match cur_question_id {
            Some(id) => id,
            None => return,
        };
        if let Some(q) = config.write().questions.iter_mut().find(|q| q.id == qid) {
            q.comment = new_text;
        }
    };

    rsx! {
        div { class: "card bg-base-100 shadow",
            div { class: "card-body gap-3",
                div { class: "card-title flex items-center justify-between",
                    span { {q_opt.map(|q| q.name.clone()).unwrap_or_else(|| "（問題未選択）".to_string())} }
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