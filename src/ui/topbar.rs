use dioxus::prelude::*;

use crate::Page;
use crate::models::Config;

#[component]
pub fn TopBar(
    config: Signal<Config>,
    on_nav: EventHandler<Page>,
) -> Element {

    let msg = use_signal(String::new);

    rsx! {
        div { class: "navbar bg-base-100 rounded-box shadow mb-2",

            div { class: "navbar-start gap-2",
                button { class: "btn btn-sm", onclick: move |_| on_nav.call(Page::MasterQuestions), "問題設定" }
                button { class: "btn btn-sm", onclick: move |_| on_nav.call(Page::MasterStudents), "受験者設定" }
                button { class: "btn btn-sm", onclick: move |_| on_nav.call(Page::Rating), "成績評価" }
                button { class: "btn btn-sm", onclick: move |_| on_nav.call(Page::Grading), "採点" }
            }

            div { class: "navbar-center gap-2",
                {(!msg().is_empty()).then(|| rsx!{
                    div { class: "alert alert-info mb-2 py-2",
                        span { class: "text-sm", "{msg()}" }
                    }
                })}
            }

            div { class: "navbar-end gap-2",
                button { class: "btn btn-sm btn-primary", onclick: move |_| config().save(msg), "Save" }
                button { class: "btn btn-sm", onclick: move |_| Config::save_as(config, msg), "Save as" }
                button { class: "btn btn-sm", onclick: move |_| Config::load(config, msg), "Load" }
            }
        }
    }
}

