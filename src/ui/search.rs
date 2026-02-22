use dioxus::prelude::*;
use crate::db::StudentRow;
use crate::db::Db;

#[component]
pub fn SearchWindow(
    search_open: Signal<bool>,
    msg: Signal<String>,
    students: Signal<Vec<StudentRow>>,
    cur_student_idx: Signal<usize>,
    focus_idx: Signal<usize>,
) -> Element {

    // DB access context
    let db = use_context::<Db>();

    let mut search_q = use_signal(|| String::new());
    let search_results = use_signal(Vec::<StudentRow>::new);

    // incremental search
    let run_search = {
        move |q: String| {
            let db_cloned = db.clone();
            let mut res_cloned = search_results.clone();
            let mut msg_cloned = msg.clone();
            spawn(async move {
                if q.trim().is_empty() {
                    res_cloned.set(vec![]);
                    return;
                }
                match db_cloned.search_students(&q, 30).await {
                    Ok(r) => res_cloned.set(r),
                    Err(e) => msg_cloned.set(format!("search error: {e:#}")),
                }
            });
        }
    };

    rsx! {
        div { class: "modal modal-open",
            div { class: "modal-box w-11/12 max-w-3xl",
                div { class: "flex items-center gap-3",
                    h3 { class: "font-bold text-lg", "検索（学籍番号/氏名）" }
                    div { class: "ml-auto",
                        button { class: "btn btn-sm", onclick: move |_| search_open.set(false), "閉じる" }
                    }
                }

                input {
                    class: "input input-bordered w-full mt-3",
                    placeholder: "例: A001 / 山田",
                    value: "{search_q}",
                    autofocus: true,
                    oninput: move |e| {
                        let v = e.value();
                        search_q.set(v.clone());
                        run_search(v);
                    }
                }

                div { class: "mt-3 max-h-72 overflow-auto border border-base-300 rounded",
                    for s in search_results().iter() {
                        div {
                            class: "px-3 py-2 hover:bg-base-200 cursor-pointer",
                            onclick: {
                                let sid = s.id.clone();
                                move |_| {
                                    if let Some(pos) = students().iter().position(|x| x.id == sid) {
                                        cur_student_idx.set(pos);
                                    }
                                    search_open.set(false);
                                    focus_idx.set(0);
                                }
                            },
                            span { class: "font-mono", "{s.id}" }
                            span { class: "ml-3", "{s.name}" }
                        }
                    }

                    {(!search_q.read().trim().is_empty() && search_results.read().is_empty()).then(|| rsx! {
                        div { class: "px-3 py-2 opacity-60", "該当なし" }
                    })}
                }
                
            }

            // click outside to close
            div { class: "modal-backdrop",
                onclick: move |_| search_open.set(false),
            }
        }
    }
}
