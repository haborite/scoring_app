use dioxus::prelude::*;
use crate::models::{Config, Student};

#[component]
pub fn SearchWindow(
    is_open: Signal<bool>,
    msg: Signal<String>,
    config: Signal<Config>,
    cur_student_idx: Signal<usize>,
    // focus_idx: Signal<usize>,
) -> Element {
    let mut search_q = use_signal(String::new);
    let mut search_results = use_signal(|| Vec::<Student>::new());

    // 追加: 検索結果内の選択位置
    let mut selected_idx = use_signal(|| 0usize);

    // incremental search (in-memory)
    let mut run_search = {
        let mut selected_idx = selected_idx;
        move |q: String| {
            let q_trim = q.trim().to_string();
            if q_trim.is_empty() {
                search_results.set(vec![]);
                selected_idx.set(0);
                return;
            }

            let q_upper = q_trim.to_uppercase();

            let all = config().students.clone();
            let mut hits: Vec<Student> = all
                .into_iter()
                .filter(|s| {
                    let id_hit = s.id.to_uppercase().contains(&q_upper);
                    let name_hit = s.name.contains(&q_trim);
                    id_hit || name_hit
                })
                .take(30)
                .collect();

            hits.sort_by(|a, b| a.id.cmp(&b.id));

            search_results.set(hits);
            selected_idx.set(0); // 新しい検索ごとに先頭を選択
        }
    };

    // 選択確定（クリック・Enter 共通）
    let mut select_student = {
        let mut is_open = is_open;
        let mut msg = msg;
        let mut cur_student_idx = cur_student_idx;
        // let mut focus_idx = focus_idx;
        move |sid: String| {
            if let Some(pos) = config().students.iter().position(|x| x.id == sid) {
                cur_student_idx.set(pos);
            } else {
                msg.set("選択した学生が見つかりません（一覧が更新された可能性）".to_string());
            }
            is_open.set(false);
            // focus_idx.set(0);
        }
    };

    rsx! {
        div { class: "modal modal-open",
            div { class: "modal-box w-11/12 max-w-3xl",
                div { class: "flex items-center gap-3",
                    h3 { class: "font-bold text-lg", "検索（学籍番号/氏名）" }
                    div { class: "ml-auto",
                        button {
                            class: "btn btn-sm",
                            onclick: move |_| is_open.set(false),
                            "閉じる"
                        }
                    }
                }

                input {
                    id: "search",
                    class: "input input-bordered w-full mt-3",
                    placeholder: "例: A001 / 山田",
                    value: "{search_q()}",
                    autofocus: true,

                    oninput: move |e| {
                        let v = e.value();
                        search_q.set(v.clone());
                        msg.set(String::new());
                        run_search(v);
                    },

                    // 追加: キー操作
                    onkeydown: move |e| {
                        match e.key() {
                            Key::Escape => {
                                e.prevent_default();
                                is_open.set(false);
                            }
                            Key::ArrowDown => {
                                e.prevent_default();
                                let n = search_results.read().len();
                                if n > 0 {
                                    selected_idx.set((selected_idx() + 1).min(n - 1));
                                }
                            }
                            Key::ArrowUp => {
                                e.prevent_default();
                                selected_idx.set(selected_idx().saturating_sub(1));
                            }
                            Key::Enter => {
                                e.prevent_default();
                                let n = search_results.read().len();
                                if n > 0 {
                                    let i = selected_idx().min(n - 1);
                                    if let Some(s) = search_results.read().get(i) {
                                        select_student(s.id.clone());
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }

                div { class: "mt-3 max-h-72 overflow-auto border border-base-300 rounded",
                    for (i, s) in search_results().iter().enumerate() {
                        {
                            let sid = s.id.clone();
                            let cls = if i == selected_idx() {
                                "px-3 py-2 bg-base-200 cursor-pointer"
                            } else {
                                "px-3 py-2 hover:bg-base-200 cursor-pointer"
                            };
                            rsx! {
                                div {
                                    class: "{cls}",
                                    onclick: move |_| {
                                        select_student(sid.clone());
                                    },
                                    span { class: "font-mono", "{s.id}" }
                                    span { class: "ml-3", "{s.name}" }
                                }
                            }
                        }
                    }

                    {(!search_q.read().trim().is_empty() && search_results.read().is_empty()).then(|| rsx! {
                        div { class: "px-3 py-2 opacity-60", "該当なし" }
                    })}
                }
            }

            // click outside to close
            div { class: "modal-backdrop",
                onclick: move |_| is_open.set(false),
            }
        }
    }
}