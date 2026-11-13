use dioxus::prelude::*;
use dioxus::desktop::{tao, use_wry_event_handler, window, WindowCloseBehaviour};
use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

mod ui;
mod models;

use ui::{MasterQuestionsPage, MasterStudentsPage, GradingPage, RatingPage};
use models::{Config, Page};

fn main() {
    let window = tao::window::WindowBuilder::new()
        .with_title("Scoring App")
        .with_resizable(true);
    dioxus::LaunchBuilder::new()
        .with_cfg(
            dioxus::desktop::Config::new()
                .with_window(window)
                .with_menu(None)
                // Closing is decided in App's CloseRequested handler (unsaved changes check)
                .with_close_behaviour(WindowCloseBehaviour::WindowHides)
        )
        .launch(App);
}

#[component]
fn App() -> Element {
    let mut page = use_signal(|| Page::Grading);
    let mut config = use_signal(|| Config::new());

    // Window title: file name and unsaved mark
    use_effect(move || {
        let cfg = config.read();
        let name = cfg.save_path.as_deref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "無題".to_string());
        let mark = if cfg.is_dirty() { "* " } else { "" };
        window().set_title(&format!("{mark}{name} - Scoring App"));
    });

    use_wry_event_handler(move |event, _| {
        let tao::event::Event::WindowEvent {
            event: tao::event::WindowEvent::CloseRequested,
            window_id,
            ..
        } = event else {
            return;
        };
        let w = window();
        if *window_id != w.id() {
            return;
        }
        if confirm_close(&mut config) {
            w.set_close_behavior(WindowCloseBehaviour::WindowCloses);
        } else {
            // dioxus hides the window right after this handler returns; bring it back
            spawn(async move {
                let w = window();
                w.set_visible(true);
                w.set_focus();
            });
        }
    });

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
            Page::Rating => rsx! {
                RatingPage {
                    on_nav: move |p| page.set(p),
                    config,
                }
            },
        }
    }
}

// Ask what to do with unsaved changes. true = the window may close.
fn confirm_close(config: &mut Signal<Config>) -> bool {
    if !config.peek().is_dirty() {
        return true;
    }

    // Plain Yes/No/Cancel: custom button labels are not supported on every platform
    let answer = MessageDialog::new()
        .set_level(MessageLevel::Warning)
        .set_title("Scoring App")
        .set_description(
            "変更が保存されていません。保存してから終了しますか？

             はい: 保存して終了
             いいえ: 保存せずに終了
             キャンセル: 終了しない",
        )
        .set_buttons(MessageButtons::YesNoCancel)
        .set_parent(&*window().window)
        .show();

    match answer {
        MessageDialogResult::Yes => {
            let mut cfg = config.peek().clone();
            match cfg.save_blocking() {
                Ok(true) => true,
                Ok(false) => false,
                Err(e) => {
                    MessageDialog::new()
                        .set_level(MessageLevel::Error)
                        .set_title("Scoring App")
                        .set_description(format!("保存に失敗しました。
{e:#}"))
                        .set_parent(&*window().window)
                        .show();
                    false
                }
            }
        }
        MessageDialogResult::No => true,
        _ => false,
    }
}
