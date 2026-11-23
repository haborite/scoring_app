use dioxus::prelude::*;
use crate::models::{bump_data_gen, Config, Part, Student, DATA_GEN};
use crate::csv_import::{self, CsvFormat};

#[component]
pub fn MasterStudentsPage(
    config: Signal<Config>,
) -> Element {

    let mut msg = use_signal(String::new);
    let csv_formats = use_signal(Vec::<CsvFormat>::new);
    let mut csv_format_name = use_signal(|| "UTAS".to_string());

    use_hook(move || reload_csv_formats(csv_formats, msg));

    rsx! {
        div { class: "space-y-2",

            {(!msg().is_empty()).then(|| rsx! {
                div { class: "alert alert-warning py-2 whitespace-pre-wrap font-mono text-sm",
                    span { class: "flex-1", "{msg}" }
                    button { class: "btn btn-xs btn-ghost", onclick: move |_| msg.set(String::new()), "✕" }
                }
            })}

            // import panel
            div { class: "card bg-base-100 shadow",
                div { class: "card-body p-2 space-y-3",

                    div { class: "flex flex-wrap items-center gap-2",
                        button {
                            class: "btn btn-sm",
                            title: "受験者だけを JSON ファイルに保存します",
                            onclick: move |_| Config::save_part(config, Part::Students, msg),
                            "保存"
                        }
                        button {
                            class: "btn btn-sm",
                            title: "保存ファイルから受験者だけを読み込みます（全体の保存ファイルからも読み込めます）",
                            onclick: move |_| Config::load_part(config, Part::Students, msg),
                            "読み込み"
                        }

                        div { class: "flex items-center gap-1",
                            select {
                                class: "select select-sm w-40",
                                title: "CSV フォーマット",
                                // Re-read on focus so edits to csv_formats.json apply without restarting
                                onfocus: move |_| reload_csv_formats(csv_formats, msg),
                                onchange: move |e| csv_format_name.set(e.value()),
                                for f in csv_formats() {
                                    option {
                                        value: "{f.name}",
                                        selected: f.name == csv_format_name(),
                                        title: "{f.description}",
                                        "{f.name}"
                                    }
                                }
                            }
                            button {
                                class: "btn btn-sm btn-primary",
                                onclick: move |_| {
                                    spawn(async move {
                                        let format = csv_formats.read().iter()
                                            .find(|f| f.name == csv_format_name())
                                            .cloned();
                                        let Some(format) = format else {
                                            msg.set("CSV フォーマットを選択してください".to_string());
                                            return;
                                        };
                                        let Some(handle) = rfd::AsyncFileDialog::new()
                                            .add_filter("CSV", &["csv", "tsv", "txt"])
                                            .pick_file()
                                            .await
                                        else {
                                            return;
                                        };
                                        let bytes = handle.read().await;
                                        match format.parse(&bytes) {
                                            Ok(result) => {
                                                let mut m = format!(
                                                    "{} 人をインポートしました（{} 形式）",
                                                    result.students.len(),
                                                    format.name,
                                                );
                                                for w in &result.warnings {
                                                    m.push('\n');
                                                    m.push_str(w);
                                                }
                                                msg.set(m);
                                                config.write().students = result.students;
                                                bump_data_gen();
                                            }
                                            Err(e) => msg.set(format!("{}: {e}", handle.file_name())),
                                        }
                                    });
                                },
                                "Import CSV"
                            }
                        }
                        button {
                            class: "btn btn-sm btn-ghost",
                            title: "CSV フォーマットの定義ファイル (csv_formats.json) があるフォルダを開きます",
                            onclick: move |_| {
                                match csv_import::ensure_user_formats_file() {
                                    Ok(path) => {
                                        csv_import::open_folder(&path);
                                        msg.set(format!("CSV フォーマットの定義ファイル: {}", path.display()));
                                    }
                                    Err(e) => msg.set(e),
                                }
                            },
                            "CSV フォーマット定義..."
                        }

                        div { class: "flex-1" }

                        button {
                            class: "btn btn-sm", 
                            onclick: move |_| {
                                let mut ss = config.read().students.clone();
                                let next_id = ss.last()
                                    .map(|s| next_student_id(&s.id))
                                    .unwrap_or_else(|| "S1".to_string());
                                ss.push(Student {
                                    id: next_id,
                                    name: String::new(),
                                });
                                config.write().students = ss;
                            }, 
                            "Add row" 
                        }
                        button {
                            class: "btn btn-sm btn-ghost",
                            onclick: move |_| {
                                config.write().students = Vec::new();
                            },
                            "Clear"
                        }
                    }

                    p { class: "text-sm opacity-70",
                        "保存 / 読み込み: 受験者一覧だけを JSON ファイル（"
                        code { "students" }
                        " だけを含む、全体の保存ファイルと同じ形式）でやり取りします。Import CSV: 選んだフォーマットで CSV を読み込みます。どちらも受験者一覧を置き換えます（点数は学籍番号で対応付けられるため保持されます）。"
                    }
                }
            }

            // table
            div { class: "card bg-base-100 shadow",
                div { class: "card-body p-2",
                    div { class: "overflow-x-auto",
                        table { class: "table table-zebra w-full",
                            thead {
                                tr {
                                    th { class: "w-64", "id" }
                                    th { "name" }
                                    th { class: "w-24", "" }
                                }
                            }
                            tbody {
                                for (idx, _s) in config.read().students.iter().cloned().enumerate() {
                                    StudentRow { key: "{DATA_GEN()}-{idx}", idx, config, msg }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn StudentRow(
    idx: usize,
    config: Signal<Config>,
    msg: Signal<String>,
) -> Element {
    // 表示用にその場で読む（クローンを作らない）
    let s = config.read().students.get(idx).cloned();

    // 既に消えていたら何も出さない（削除直後など）
    let Some(s) = s else {
        return rsx! {};
    };
    let old_id = s.id.clone();

    rsx! {
        tr {
            td {
                input {
                    id: "student-id-{idx}",
                    class: "input input-bordered input-sm w-full",
                    value: "{s.id}",
                    // Applied on commit (not per keystroke) so scores can be carried over safely
                    onchange: move |ev| {
                        let result = config.write().rename_student(idx, &ev.value());
                        match result {
                            Ok(()) => msg.set(String::new()),
                            Err(e) => {
                                msg.set(e);
                                let old = serde_json::to_string(&old_id).unwrap_or_default();
                                let _ = document::eval(&format!(
                                    r#"document.getElementById("student-id-{idx}").value = {old};"#
                                ));
                            }
                        }
                    }
                }
            }
            td {
                input {
                    class: "input input-bordered input-sm w-full",
                    // Not controlled while typing (rows are re-created when the list is replaced)
                    initial_value: "{s.name}",
                    oninput: move |ev| {
                        let v = ev.value();
                        if let Some(s) = config.write().students.get_mut(idx) { s.name = v; }
                    }
                }
            }
            td { class: "text-right",
                button {
                    class: "btn btn-sm btn-ghost",
                    onclick: move |_| {
                        let mut config_write = config.write();
                        if idx < config_write.students.len() {
                            config_write.students.remove(idx);
                            drop(config_write);
                            // Rows below move up: re-create them with their new values
                            bump_data_gen();
                        }
                    },
                    "Del"
                }
            }
        }
    }
}

// "A009" -> "A010", "S1" -> "S2" (keeps the prefix and zero padding)
fn next_student_id(id: &str) -> String {
    let prefix: String = id.chars().take_while(|c| !c.is_ascii_digit()).collect();
    let digits: String = id.chars().skip(prefix.chars().count()).take_while(|c| c.is_ascii_digit()).collect();
    let next = digits.parse::<u64>().unwrap_or(0).saturating_add(1);
    format!("{prefix}{next:0width$}", width = digits.len())
}

fn reload_csv_formats(mut formats: Signal<Vec<CsvFormat>>, mut msg: Signal<String>) {
    let (loaded, errors) = csv_import::load_formats();
    if !errors.is_empty() {
        msg.set(errors.join("\n"));
    }
    if *formats.peek() != loaded {
        formats.set(loaded);
    }
}

#[cfg(test)]
mod tests {
    use super::next_student_id;

    #[test]
    fn keeps_prefix_and_padding() {
        assert_eq!(next_student_id("A001"), "A002");
        assert_eq!(next_student_id("A009"), "A010");
        assert_eq!(next_student_id("S1"), "S2");
        assert_eq!(next_student_id("A99"), "A100");
        assert_eq!(next_student_id("ABC"), "ABC1");
    }
}
