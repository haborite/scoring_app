use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    MasterQuestions,
    MasterStudents,
    Grading,
}

#[component]
pub fn HomePage(on_nav: EventHandler<Page>) -> Element {
    rsx! {
        div { style: "padding:16px; font-family: sans-serif;",
            h2 { "Grading App" }
            div { style: "display:flex; gap:12px; margin-top:12px;",
                button { onclick: move |_| on_nav.call(Page::MasterQuestions), "問題マスタ" }
                button { onclick: move |_| on_nav.call(Page::MasterStudents), "受験者マスタ" }
                button { onclick: move |_| on_nav.call(Page::Grading), "採点" }
            }
            p { style: "margin-top:16px; color:#444;",
                "まずは問題/受験者を登録（CSVまたは手入力）→ 採点へ。"
            }
        }
    }
}
