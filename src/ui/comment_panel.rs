use dioxus::prelude::*;
use crate::models::Config;

#[component]
pub fn CommentPanel(
    config: Signal<Config>,
    /// ScoreRows 側で「いまフォーカスされている問題」をここに流し込む
    cur_question_id: Signal<Option<u32>>,
) -> Element {
    // 現在フォーカス中の問題（参照）
    let q_opt = {
        let cfg = config.clone();
        let qid = cur_question_id();
        qid.and_then(|id| cfg().questions.into_iter().find(|q| q.id == id))
    };

    // 表示用 Markdown（configから毎回引く: 1ソースに寄せる）
    let md_text = q_opt.clone()
        .and_then(|q| Some(q.comment.clone()))
        .unwrap_or_default();

    // textarea 編集 -> config.questions[*].comment を即時更新
    let on_input = move |e: FormEvent| {
        let new_text = e.value();

        let qid = match cur_question_id() {
            Some(id) => id,
            None => return, // フォーカス問題なし
        };

        // Config を更新（Signal は set で丸ごと差し替え）
        // ※ Config が Clone 可能である前提（既存コードがそう）
        let mut cfg = config.read().clone();
        if let Some(q) = cfg.questions.iter_mut().find(|q| q.id == qid) {
            q.comment = new_text; // 空文字なら「削除」と同義
        }
        config.set(cfg);
    };

    rsx! {
        div { class: "card bg-base-100 shadow",
            div { class: "card-body gap-3",
                div { class: "card-title flex items-center justify-between",
                    span { {q_opt.map(|q| q.name.clone()).unwrap_or_else(|| "（問題未選択）".to_string())} }
                }

                // フォーカスなしの案内
                {cur_question_id().is_none().then(|| rsx!{
                    div { class: "text-sm opacity-70",
                        "点数入力欄にフォーカスすると、その問題のコメントをここで編集できます。"
                    }
                })}

                // editor
                textarea {
                    class: "textarea textarea-bordered w-full h-full font-mono text-sm leading-snug",
                    placeholder: "各問題に対する採点上の注意点やメモ等を記述",
                    value: "{md_text}",
                    oninput: on_input,
                }

            }
        }
    }
}