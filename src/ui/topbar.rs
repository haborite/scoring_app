use dioxus::prelude::*;
use std::time::Duration;

use crate::Page;
use crate::models::{Config, Status};
use crate::shortcuts::Shortcut;

// In workflow order; Ctrl+1..4 follow the same order
const PAGES: [(Page, &str); 4] = [
    (Page::MasterQuestions, "問題設定"),
    (Page::MasterStudents, "受験者設定"),
    (Page::Grading, "採点"),
    (Page::Rating, "成績評価"),
];

#[component]
pub fn TopBar(
    config: Signal<Config>,
    page: Signal<Page>,
    status: Signal<Status>,
) -> Element {

    // Info messages fade out on their own; errors stay until dismissed
    use_effect(move || {
        let current = status();
        if current.error || current.text.is_empty() {
            return;
        }
        spawn(async move {
            tokio::time::sleep(Duration::from_secs(4)).await;
            if *status.peek() == current {
                status.set(Status::default());
            }
        });
    });

    rsx! {
        div { class: "navbar bg-base-100 rounded-box shadow mb-2 gap-2 min-h-0",

            div { class: "navbar-start w-auto",
                div { role: "tablist", class: "tabs tabs-box tabs-sm",
                    for (p, label) in PAGES {
                        button {
                            role: "tab",
                            class: if page() == p { "tab tab-active" } else { "tab" },
                            title: "{Shortcut::Goto(p).label()}",
                            onclick: move |_| page.set(p),
                            "{label}"
                        }
                    }
                }
            }

            div { class: "flex-1 flex justify-center min-w-0",
                {(!status().text.is_empty()).then(|| {
                    let s = status();
                    let cls = if s.error { "alert alert-error" } else { "alert alert-success" };
                    rsx! {
                        div { class: "{cls} py-1 px-3 text-sm min-w-0",
                            span { class: "truncate", title: "{s.text}", "{s.text}" }
                            button {
                                class: "btn btn-xs btn-ghost",
                                onclick: move |_| status.set(Status::default()),
                                "✕"
                            }
                        }
                    }
                })}
            }

            div { class: "navbar-end w-auto gap-2",
                button {
                    class: "btn btn-sm btn-primary",
                    title: "上書き保存 ({Shortcut::Save.label()})",
                    onclick: move |_| Config::save(config, status),
                    "Save"
                }
                button {
                    class: "btn btn-sm",
                    title: "名前を付けて保存 ({Shortcut::SaveAs.label()})",
                    onclick: move |_| Config::save_as(config, status),
                    "Save as"
                }
                button {
                    class: "btn btn-sm",
                    title: "開く ({Shortcut::Load.label()})",
                    onclick: move |_| Config::load(config, status),
                    "Load"
                }
            }
        }
    }
}
