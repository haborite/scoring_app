use dioxus::prelude::*;
use crate::models::Config;
use crate::ui::SaveBar;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    MasterQuestions,
    MasterStudents,
    Grading,
}

#[component]
pub fn HomePage(
    on_nav: EventHandler<Page>,
    config: Signal<Config>,
) -> Element {
    rsx! {
        div { style: "padding:16px; font-family: sans-serif;",
            h2 { "Grading App" }
            SaveBar { config }
            div { style: "display:flex; gap:12px; margin-top:12px;",
                button { class: "btn btn-primary btn-sm", onclick: move |_| on_nav.call(Page::MasterQuestions), "問題マスタ" }
                button { class: "btn btn-primary btn-sm", onclick: move |_| on_nav.call(Page::MasterStudents), "受験者マスタ" }
                button { class: "btn btn-primary btn-sm", onclick: move |_| on_nav.call(Page::Grading), "採点" }
            }
        }
    }
}
