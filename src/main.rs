use dioxus::prelude::*;

mod ui;
mod models;

use ui::{MasterQuestionsPage, MasterStudentsPage, GradingPage};
use models::{Config, Page};

fn main() {

    use dioxus::desktop::tao;
    let window = tao::window::WindowBuilder::new().with_resizable(true);
    dioxus::LaunchBuilder::new().with_cfg(dioxus::desktop::Config::new().with_window(window).with_menu(None)).launch(App);

    // launch(App);
}

#[component]
fn App() -> Element {
    let mut page = use_signal(|| Page::Grading);
    let config = use_signal(|| Config::new());

    rsx! {
        document::Stylesheet { href: asset!("assets/tailwind.css") }
        match *page.read() {
            Page::MasterQuestions => rsx! {
                MasterQuestionsPage {
                    on_nav: move |p| page.set(p),
                    config,
                }
            },
            Page::MasterStudents => rsx! {
                MasterStudentsPage {
                    on_nav: move |p| page.set(p),
                    config,
                }
            },
            Page::Grading => rsx! {
                GradingPage { 
                    on_nav: move |p| page.set(p),
                    config,                    
                }
            },
        }
    }
}
