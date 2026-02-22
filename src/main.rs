use anyhow::Result;
use dioxus::prelude::*;
use rfd::FileDialog;
use std::path::PathBuf;

mod csv_import;
mod db;
mod ui;

use db::Db;
use ui::home::{HomePage, Page};
use ui::master_questions::MasterQuestionsPage;
use ui::master_students::MasterStudentsPage;
use ui::grading::GradingPage;

fn main() {
    // Dioxus Desktopは同期mainでOK（内部でruntimeを使う）
    launch(App);
}

#[component]
fn App() -> Element {
    let mut page = use_signal(|| Page::Home);
    let db_state = use_signal(|| Option::<Db>::None);
    let msg = use_signal(|| String::new());

    // 起動時にDB選択
    {
        let mut db_state = db_state.clone();
        let mut msg = msg.clone();
        use_effect(move || {
            spawn(async move {
                match pick_or_create_db().await {
                    Ok(db) => db_state.set(Some(db)),
                    Err(e) => msg.set(format!("DB open error: {e:#}")),
                }
            });
        });
    }

    let Some(db) = db_state.read().clone() else {
        return rsx! {
            div { style: "padding:16px; font-family:sans-serif;",
                h2 { "Starting..." }
                if !msg.read().is_empty() {
                    p { style: "color:#b00;", "{msg}" }
                }
                p {
                    "DBファイル選択ダイアログが開かない場合は、ウィンドウの背面を確認してください。"
                }
            }
        };
    };

    use_context_provider(|| db.clone());

    rsx! {
        match *page.read() {
            Page::Home => rsx! {
                HomePage { on_nav: move |p| page.set(p) }
            },
            Page::MasterQuestions => rsx! {
                MasterQuestionsPage { on_back: move |_| page.set(Page::Home) }
            },
            Page::MasterStudents => rsx! {
                MasterStudentsPage { on_back: move |_| page.set(Page::Home) }
            },
            Page::Grading => rsx! {
                document::Stylesheet { href: asset!("assets/tailwind.css") }
                GradingPage { on_back: move |_| page.set(Page::Home) }
            },
        }
    }
}

async fn pick_or_create_db() -> Result<Db> {
    let path: PathBuf = FileDialog::new()
        .add_filter("SQLite DB", &["db", "sqlite", "sqlite3"])
        .set_title("採点DBファイルを選択（なければ新規作成の場所を指定）")
        .save_file()
        .ok_or_else(|| anyhow::anyhow!("DB path not selected"))?;

    let db = Db::connect(&path).await?;
    Ok(db)
}
