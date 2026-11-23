use dioxus::prelude::*;
use dioxus::desktop::{tao, use_wry_event_handler, window, WindowCloseBehaviour};
use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

mod ui;
mod models;
mod csv_import;
mod shortcuts;
mod settings;

use ui::{MasterQuestionsPage, MasterStudentsPage, GradingPage, RatingPage, SettingsPage, use_grading_state, TopBar};
use models::{Config, Page, Status};
use settings::{RECORDING, SETTINGS};
use shortcuts::{Action, KeyMsg};

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
    let page = use_signal(|| Page::Grading);
    let mut config = use_signal(|| Config::new());
    let mut status = use_signal(Status::default);
    // Lives here so the current student survives page switches
    let grading = use_grading_state();

    // App settings: read once, written back whenever they change
    use_effect(move || {
        if let Some(e) = settings::take_load_error() {
            status.set(Status::error(e));
        }
    });
    use_effect(move || {
        if let Err(e) = settings::save_if_changed() {
            status.set(Status::error(e));
        }
    });

    // Keyboard shortcuts, wherever the focus is
    use_future(move || async move {
        let mut listener = document::eval(&shortcuts::listener_js());
        while let Ok(msg) = listener.recv::<String>().await {
            match KeyMsg::parse(&msg) {
                Some(KeyMsg::Action(action)) => run_action(action, page, config, status, grading),
                Some(KeyMsg::Record(combo)) => ui::record_key(&combo),
                None => {}
            }
        }
    });
    // Bindings and recording mode are pushed to the listener whenever they change
    use_effect(move || {
        let js = shortcuts::keymap_js(&SETTINGS.read().keys);
        let _ = document::eval(&js);
    });
    use_effect(move || {
        let _ = document::eval(&shortcuts::recording_js(RECORDING.read().is_some()));
    });
    // Leaving the settings page stops recording
    use_effect(move || {
        if page() != Page::Settings && RECORDING.peek().is_some() {
            *RECORDING.write() = None;
        }
    });

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
        // Fits the window: each page scrolls inside its own area
        div { class: "h-screen flex flex-col gap-2 p-2 overflow-hidden bg-base-200 text-base-content",
            TopBar { config, page, status }
            div { class: if page() == Page::Grading { "flex-1 min-h-0" } else { "flex-1 min-h-0 overflow-auto" },
                match page() {
                    Page::MasterQuestions => rsx! { MasterQuestionsPage { config } },
                    Page::MasterStudents => rsx! { MasterStudentsPage { config } },
                    Page::Grading => rsx! { GradingPage { config, state: grading } },
                    Page::Rating => rsx! { RatingPage { config } },
                    Page::Settings => rsx! { SettingsPage { status } },
                }
            }
        }
    }
}

fn run_action(
    action: Action,
    mut page: Signal<Page>,
    mut config: Signal<Config>,
    mut status: Signal<Status>,
    grading: ui::GradingState,
) {
    let on_grading = page() == Page::Grading;
    let qlen = config.peek().questions.len();
    match action {
        Action::Save => Config::save(config, status),
        Action::SaveAs => Config::save_as(config, status),
        Action::Load => Config::load(config, status),
        Action::GotoQuestions => page.set(Page::MasterQuestions),
        Action::GotoStudents => page.set(Page::MasterStudents),
        Action::GotoGrading => page.set(Page::Grading),
        Action::GotoRating => page.set(Page::Rating),
        Action::GotoSettings => page.set(Page::Settings),
        Action::Search | Action::FieldSearch => {
            page.set(Page::Grading);
            grading.open_search();
        }
        Action::NextRegrade => {
            page.set(Page::Grading);
            let from = *grading.cur_student_idx.peek();
            match config.peek().next_regrade(from) {
                Some((s, q)) => grading.goto(s, q),
                None => status.set(Status::info("要再採点の受験者はいません")),
            }
        }
        _ if !on_grading => {}
        Action::PrevStudent | Action::FieldPrevStudent => grading.prev_student(),
        Action::NextStudent | Action::FieldNextStudent => {
            grading.next_student(config.peek().students.len())
        }
        Action::ToggleSection => grading.toggle_section(),
        Action::NextQuestion => grading.move_question(1, qlen),
        Action::PrevQuestion => grading.move_question(-1, qlen),
        Action::ConfirmScore => {
            let cfg = config.peek();
            let sid = cfg.students.get(*grading.cur_student_idx.peek()).map(|s| s.id.clone());
            let qid = cfg.questions.get(*grading.focus_idx.peek()).map(|q| q.id);
            drop(cfg);
            if let (Some(sid), Some(qid)) = (sid, qid) {
                config.write().confirm_score(&sid, qid);
            }
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
