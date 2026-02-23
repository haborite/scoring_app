use dioxus::prelude::*;

mod ui;
mod models;

use ui::{HomePage, Page};
use ui::{MasterQuestionsPage, MasterStudentsPage, GradingPage};
use models::Config;

fn main() {
    launch(App);
}

#[component]
fn App() -> Element {
    let mut page = use_signal(|| Page::Home);
    // let msg = use_signal(|| String::new());
    let config = use_signal(|| Config::new());

    rsx! {
        document::Stylesheet { href: asset!("assets/tailwind.css") }
        match *page.read() {
            Page::Home => rsx! {
                HomePage { 
                    on_nav: move |p| page.set(p),
                    config 
                }
            },
            Page::MasterQuestions => rsx! {
                MasterQuestionsPage { 
                    on_back: move |_| page.set(Page::Home),
                    config,
                }
            },
            Page::MasterStudents => rsx! {
                MasterStudentsPage {
                    on_back: move |_| page.set(Page::Home),
                    config,
                }
            },
            Page::Grading => rsx! {
                GradingPage { 
                    on_back: move |_| page.set(Page::Home),
                    config,                    
                }
            },
        }
    }
}
