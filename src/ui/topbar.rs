use dioxus::prelude::*;
use std::time::Duration;

use crate::Page;
use crate::models::{Config, Status};
use crate::settings::SETTINGS;
use crate::shortcuts::Action;

// In workflow order; the default Ctrl+1..5 follow the same order
const PAGES: [(Page, &str, Action); 5] = [
    (Page::MasterQuestions, "問題設定", Action::GotoQuestions),
    (Page::MasterStudents, "受験者設定", Action::GotoStudents),
    (Page::Grading, "採点", Action::GotoGrading),
    (Page::Rating, "成績評価", Action::GotoRating),
    (Page::Settings, "設定", Action::GotoSettings),
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

    let settings = SETTINGS.read();
    let key = |a: Action| settings.key_label(a);
    let save_title = format!("上書き保存 ({})", key(Action::Save));
    let save_as_title = format!("名前を付けて保存 ({})", key(Action::SaveAs));
    let load_title = format!("開く ({})", key(Action::Load));
    let tabs: Vec<(Page, &str, String)> = PAGES.iter().map(|&(p, label, a)| (p, label, key(a))).collect();
    drop(settings);

    rsx! {
        div { class: "navbar bg-base-100 rounded-box shadow gap-2 min-h-0",

            div { class: "navbar-start w-auto",
                div { role: "tablist", class: "tabs tabs-box tabs-sm",
                    for (p, label, keys) in tabs {
                        button {
                            role: "tab",
                            class: if page() == p { "tab tab-active" } else { "tab" },
                            title: "{keys}",
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
                    title: "{save_title}",
                    onclick: move |_| Config::save(config, status),
                    "Save"
                }
                button {
                    class: "btn btn-sm",
                    title: "{save_as_title}",
                    onclick: move |_| Config::save_as(config, status),
                    "Save as"
                }
                button {
                    class: "btn btn-sm",
                    title: "{load_title}",
                    onclick: move |_| Config::load(config, status),
                    "Load"
                }
            }
        }
    }
}
