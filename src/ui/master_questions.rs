use dioxus::prelude::*;
use rfd::FileDialog;

use crate::{
    csv_import::{preview_questions, read_questions_csv},
    db::{Db, QuestionRow},
};

#[component]
pub fn MasterQuestionsPage(on_back: EventHandler<()>) -> Element {
    let db = use_context::<Db>();
    let questions = use_signal(Vec::<QuestionRow>::new);
    let msg = use_signal(|| String::new());

    // 手入力フォーム
    let mut form_id = use_signal(|| String::new());
    let mut form_name = use_signal(|| String::new());
    let mut form_full = use_signal(|| String::new());
    let mut form_weight = use_signal(|| String::new());
    let mut form_comment = use_signal(|| String::new());

    // 初期ロード
    {
        let db = db.clone();
        let questions = questions.clone();
        let msg = msg.clone();
        use_effect(move || {
            let db = db.clone();
            let mut questions = questions.clone();
            let mut msg = msg.clone();
            spawn(async move {
                match db.list_questions().await {
                    Ok(qs) => questions.set(qs),
                    Err(e) => msg.set(format!("DB load error: {e:#}")),
                }
            });
        });
    }

    let refresh = {
        let db = db.clone();
        let questions = questions.clone();
        let msg = msg.clone();
        move || {
            let db = db.clone();
            let mut questions = questions.clone();
            let mut msg = msg.clone();
            spawn(async move {
                match db.list_questions().await {
                    Ok(qs) => {
                        questions.set(qs);
                        msg.set("".to_string());
                    }
                    Err(e) => msg.set(format!("DB load error: {e:#}")),
                }
            });
        }
    };

    let on_import = {
        let db = db.clone();
        let questions = questions.clone();
        let mut msg = msg.clone();
        move |_| {
            if let Some(path) = FileDialog::new()
                .add_filter("CSV", &["csv"])
                .pick_file()
            {
                let path2 = path.clone();
                // CSV読み込みは同期なので別スレでも良いが、まずは単純に
                match read_questions_csv(&path2) {
                    Ok(rows) => {
                        let preview = preview_questions(&rows, 5);
                        msg.set(format!(
                            "CSV loaded: {} rows. Preview: {} rows. Importing...",
                            rows.len(),
                            preview.len()
                        ));
                        let db = db.clone();
                        let mut questions = questions.clone();
                        let mut msg = msg.clone();
                        spawn(async move {
                            match db.upsert_questions(&rows).await {
                                Ok(affected) => {
                                    msg.set(format!("Import done. affected rows: {affected}"));
                                    if let Ok(qs) = db.list_questions().await {
                                        questions.set(qs);
                                    }
                                }
                                Err(e) => msg.set(format!("Import error: {e:#}")),
                            }
                        });
                    }
                    Err(e) => msg.set(format!("CSV parse error: {e:#}")),
                }
            }
        }
    };

    let on_add = {
        let db = db.clone();
        let mut msg = msg.clone();
        let questions = questions.clone();
        let form_id = form_id.clone();
        let form_name = form_name.clone();
        let form_full = form_full.clone();
        let form_weight = form_weight.clone();
        let form_comment = form_comment.clone();

        move |_| {
            let id = form_id.read().trim().to_string();
            let name = form_name.read().trim().to_string();
            let full_score: i64 = match form_full.read().trim().parse() {
                Ok(v) => v,
                Err(_) => {
                    msg.set("full_score は整数で入力してください".to_string());
                    return;
                }
            };
            let weight: f64 = match form_weight.read().trim().parse() {
                Ok(v) => v,
                Err(_) => {
                    msg.set("weight は数値で入力してください".to_string());
                    return;
                }
            };
            let comment = form_comment.read().trim().to_string();

            let row = QuestionRow { id, name, full_score, weight, comment };
            let db = db.clone();
            let mut msg = msg.clone();
            let mut questions = questions.clone();
            spawn(async move {
                match db.insert_question(&row).await {
                    Ok(_) => {
                        msg.set("added".to_string());
                        if let Ok(qs) = db.list_questions().await {
                            questions.set(qs);
                        }
                    }
                    Err(e) => msg.set(format!("add error: {e:#}")),
                }
            });
        }
    };

    rsx! {
        div { style: "padding:16px; font-family: sans-serif;",
            div { style: "display:flex; align-items:center; gap:12px;",
                button { onclick: move |_| on_back.call(()), "戻る" }
                h2 { "問題マスタ" }
                button { onclick: on_import, "CSVインポート（upsert）" }
                button { onclick: move |_| refresh(), "再読み込み" }
            }

            if !msg.read().is_empty() {
                p { style: "color:#b00; margin-top:8px;", "{msg}" }
            }

            h3 { style: "margin-top:16px;", "手入力で追加（idは追加時のみ）" }
            div { style: "display:flex; gap:8px; flex-wrap:wrap; align-items:center;",
                input {
                    placeholder: "id (例: Q1)",
                    value: "{form_id}",
                    oninput: move |e| form_id.set(e.value()),
                }
                input {
                    placeholder: "name (例: 問1)",
                    value: "{form_name}",
                    oninput: move |e| form_name.set(e.value()),
                }
                input {
                    placeholder: "full_score (例: 10)",
                    value: "{form_full}",
                    oninput: move |e| form_full.set(e.value()),
                }
                input {
                    placeholder: "weight (例: 1.0)",
                    value: "{form_weight}",
                    oninput: move |e| form_weight.set(e.value()),
                }
                input {
                    placeholder: "comment",
                    value: "{form_comment}",
                    oninput: move |e| form_comment.set(e.value()),
                }
                button { onclick: on_add, "追加" }
            }

            h3 { style: "margin-top:16px;", "一覧（編集は次ステップで追加）" }
            table { border: "1", padding: "6",
                thead {
                    tr {
                        th { "id" }
                        th { "name" }
                        th { "full" }
                        th { "weight" }
                        th { "comment" }
                    }
                }
                tbody {
                    for q in questions.read().iter() {
                        tr {
                            td { "{q.id}" }
                            td { "{q.name}" }
                            td { "{q.full_score}" }
                            td { "{q.weight}" }
                            td { "{q.comment}" }
                        }
                    }
                }
            }
        }
    }
}
