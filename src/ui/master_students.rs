use dioxus::prelude::*;
use rfd::FileDialog;

use crate::{
    csv_import::{preview_students, read_students_csv},
    db::{Db, StudentRow},
};

#[component]
pub fn MasterStudentsPage(on_back: EventHandler<()>) -> Element {
    let db = use_context::<Db>();
    let students = use_signal(Vec::<StudentRow>::new);
    let msg = use_signal(|| String::new());

    let mut form_id = use_signal(|| String::new());
    let mut form_name = use_signal(|| String::new());

    // 初期ロード
    {
        let db = db.clone();
        let students = students.clone();
        let msg = msg.clone();
        use_effect(move || {
            let db = db.clone();
            let mut students = students.clone();
            let mut msg = msg.clone();
            spawn(async move {
                match db.list_students().await {
                    Ok(ss) => students.set(ss),
                    Err(e) => msg.set(format!("DB load error: {e:#}")),
                }
            });
        });
    }

    let refresh = {
        let db = db.clone();
        let students = students.clone();
        let msg = msg.clone();
        move || {
            let db = db.clone();
            let mut students = students.clone();
            let mut msg = msg.clone();
            spawn(async move {
                match db.list_students().await {
                    Ok(ss) => {
                        students.set(ss);
                        msg.set("".to_string());
                    }
                    Err(e) => msg.set(format!("DB load error: {e:#}")),
                }
            });
        }
    };

    let on_import = {
        let db = db.clone();
        let students = students.clone();
        let mut msg = msg.clone();
        move |_| {
            if let Some(path) = FileDialog::new()
                .add_filter("CSV", &["csv"])
                .pick_file()
            {
                let path2 = path.clone();
                match read_students_csv(&path2) {
                    Ok(rows) => {
                        let preview = preview_students(&rows, 5);
                        msg.set(format!(
                            "CSV loaded: {} rows. Preview: {} rows. Importing...",
                            rows.len(),
                            preview.len()
                        ));

                        // ★外側は db をムーブしない。cloneをasyncへ
                        let db2 = db.clone();
                        let mut students2 = students.clone();
                        let mut msg2 = msg.clone();

                        spawn(async move {
                            match db2.upsert_students(&rows).await {
                                Ok(affected) => {
                                    msg2.set(format!("Import done. affected rows: {affected}"));
                                    if let Ok(ss) = db2.list_students().await {
                                        students2.set(ss);
                                    }
                                }
                                Err(e) => msg2.set(format!("Import error: {e:#}")),
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
        let msg = msg.clone();
        let students = students.clone();
        let form_id = form_id.clone();
        let form_name = form_name.clone();
        move |_| {
            let row = StudentRow {
                id: form_id.read().trim().to_string(),
                name: form_name.read().trim().to_string(),
            };
            let db = db.clone();
            let mut msg = msg.clone();
            let mut students = students.clone();
            spawn(async move {
                match db.insert_student(&row).await {
                    Ok(_) => {
                        msg.set("added".to_string());
                        if let Ok(ss) = db.list_students().await {
                            students.set(ss);
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
                h2 { "受験者マスタ" }
                button { onclick: on_import, "CSVインポート（upsert）" }
                button { onclick: move |_| refresh(), "再読み込み" }
            }

            if !msg.read().is_empty() {
                p { style: "color:#b00; margin-top:8px;", "{msg}" }
            }

            h3 { style: "margin-top:16px;", "手入力で追加（idは追加時のみ）" }
            div { style: "display:flex; gap:8px; flex-wrap:wrap; align-items:center;",
                input {
                    placeholder: "id (例: A001)",
                    value: "{form_id}",
                    oninput: move |e| form_id.set(e.value()),
                }
                input {
                    placeholder: "name (例: 山田 太郎)",
                    value: "{form_name}",
                    oninput: move |e| form_name.set(e.value()),
                }
                button { onclick: on_add, "追加" }
            }

            h3 { style: "margin-top:16px;", "一覧（編集は次ステップで追加）" }
            table { border: "1", padding: "6",
                thead {
                    tr {
                        th { "id" }
                        th { "name" }
                    }
                }
                tbody {
                    for s in students.read().iter() {
                        tr {
                            td { "{s.id}" }
                            td { "{s.name}" }
                        }
                    }
                }
            }
        }
    }
}
