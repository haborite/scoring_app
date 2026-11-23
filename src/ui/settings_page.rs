use dioxus::prelude::*;
use crate::models::Status;
use crate::settings::{settings_path, Layout, RECORDING, SETTINGS};
use crate::shortcuts::{self, combo_label, Action, Scope, MOD_LABEL};

// Why the last key pressed while recording was not accepted
static RECORD_MSG: GlobalSignal<String> = Signal::global(String::new);

// A combo pressed while RECORDING is set: add it to that action, or say why not
pub fn record_key(combo: &str) {
    let Some(action) = *RECORDING.peek() else {
        return;
    };
    if combo == "Escape" {
        stop_recording();
        return;
    }
    let checked = shortcuts::check_combo(&SETTINGS.peek().keys, action, combo);
    match checked {
        Ok(()) => {
            let mut keys = SETTINGS.peek().keys(action);
            keys.push(combo.to_string());
            set_keys(action, keys);
            stop_recording();
        }
        // Keep recording so another key can be tried
        Err(e) => *RECORD_MSG.write() = e,
    }
}

fn stop_recording() {
    *RECORDING.write() = None;
    *RECORD_MSG.write() = String::new();
}

// Store the keys of an action; the same as the defaults means no override
fn set_keys(action: Action, keys: Vec<String>) {
    let mut s = SETTINGS.write();
    if keys.iter().map(String::as_str).eq(action.default_keys().iter().copied()) {
        s.keys.remove(&action.id());
    } else {
        s.keys.insert(action.id(), keys);
    }
}

#[component]
pub fn SettingsPage(status: Signal<Status>) -> Element {
    let settings = SETTINGS.read().clone();
    let path = settings_path().map(|p| p.display().to_string()).unwrap_or_default();
    let layout = settings.layout;
    let layout_is_default = layout == Layout::default();
    let any_override = !settings.keys.is_empty();

    rsx! {
        div { class: "space-y-2 max-w-5xl",
            div { class: "card bg-base-100 shadow",
                div { class: "card-body p-4 gap-3",
                    div { class: "card-title", "一般" }
                    label { class: "flex items-center gap-3",
                        span { class: "w-56", "新しい問題の満点（初期値）" }
                        input {
                            class: "input input-bordered input-sm w-24 text-right tabular-nums",
                            r#type: "number",
                            min: 1,
                            initial_value: "{settings.default_full_score}",
                            oninput: move |e| {
                                if let Ok(v) = e.value().trim().parse::<u32>() {
                                    SETTINGS.write().default_full_score = v;
                                }
                            }
                        }
                        span { class: "text-sm opacity-60", "問題設定画面の「Add row」で使います。登録済みの問題は変わりません。" }
                    }
                }
            }

            div { class: "card bg-base-100 shadow",
                div { class: "card-body p-4 gap-3",
                    div { class: "card-title", "採点画面のレイアウト" }
                    div { class: "flex items-center gap-4 text-sm",
                        span { class: "tabular-nums", "コメント欄の幅 {layout.comment_width:.0}%" }
                        span { class: "tabular-nums", "上段の高さ {layout.top_height:.0}%" }
                        button {
                            class: "btn btn-sm",
                            disabled: layout_is_default,
                            onclick: move |_| SETTINGS.write().layout = Layout::default(),
                            "レイアウトを初期化"
                        }
                    }
                    p { class: "text-sm opacity-60",
                        "採点画面の区切り線をドラッグすると大きさを変えられます（ダブルクリックでその区切りだけ元に戻ります）。"
                    }
                }
            }

            div { class: "card bg-base-100 shadow",
                div { class: "card-body p-4 gap-3",
                    div { class: "flex items-center gap-2",
                        div { class: "card-title", "キー割り当て" }
                        div { class: "flex-1" }
                        button {
                            class: "btn btn-sm btn-ghost",
                            disabled: !any_override,
                            onclick: move |_| {
                                stop_recording();
                                SETTINGS.write().keys.clear();
                                status.set(Status::info("キー割り当てを初期値に戻しました"));
                            },
                            "すべて初期値に戻す"
                        }
                    }
                    p { class: "text-sm opacity-60",
                        "「追加」を押してから割り当てたいキーを押してください（Esc で取り消し）。"
                        "どこでも使える操作には {MOD_LABEL} を含むキーが必要です。"
                    }
                    {(!RECORD_MSG().is_empty()).then(|| rsx! {
                        div { class: "alert alert-warning py-2 text-sm", "{RECORD_MSG}" }
                    })}
                    KeyTable { scope: Scope::Global, title: "どこでも使える操作" }
                    KeyTable { scope: Scope::ScoreField, title: "採点画面の点数入力欄で使う操作" }
                }
            }

            p { class: "text-xs opacity-60 px-2",
                "設定は自動で保存され、どの採点ファイルでも共通です: "
                span { class: "font-mono", "{path}" }
            }
        }
    }
}

#[component]
fn KeyTable(scope: Scope, title: String) -> Element {
    let settings = SETTINGS.read().clone();
    let recording = RECORDING();
    rsx! {
        div {
            div { class: "font-semibold text-sm mt-2", "{title}" }
            table { class: "table table-sm",
                tbody {
                    for action in Action::ALL.into_iter().filter(|a| a.scope() == scope) {
                        {
                            let keys = settings.keys(action);
                            let customized = settings.keys.contains_key(&action.id());
                            let is_recording = recording == Some(action);
                            rsx! {
                                tr { key: "{action.id()}",
                                    td { class: "w-56", "{action.label()}" }
                                    td {
                                        div { class: "flex flex-wrap items-center gap-1",
                                            if keys.is_empty() {
                                                span { class: "text-sm opacity-50", "（なし）" }
                                            }
                                            for (i, k) in keys.iter().cloned().enumerate() {
                                                span { class: "flex items-center",
                                                    kbd { class: "kbd kbd-sm", "{combo_label(&k)}" }
                                                    button {
                                                        class: "btn btn-xs btn-ghost px-1",
                                                        title: "このキーを外す",
                                                        onclick: {
                                                            let keys = keys.clone();
                                                            move |_| {
                                                                let mut keys = keys.clone();
                                                                keys.remove(i);
                                                                set_keys(action, keys);
                                                            }
                                                        },
                                                        "✕"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    td { class: "w-72 text-right whitespace-nowrap",
                                        if is_recording {
                                            span { class: "text-sm text-primary mr-2", "キーを押してください…" }
                                            button { class: "btn btn-xs", onclick: move |_| stop_recording(), "取消" }
                                        } else {
                                            button {
                                                class: "btn btn-xs",
                                                onclick: move |_| {
                                                    *RECORD_MSG.write() = String::new();
                                                    *RECORDING.write() = Some(action);
                                                },
                                                "追加"
                                            }
                                        }
                                        button {
                                            class: "btn btn-xs btn-ghost ml-1",
                                            disabled: !customized,
                                            title: "{action_defaults(action)}",
                                            onclick: move |_| set_keys(action, action.default_keys().iter().map(|k| k.to_string()).collect()),
                                            "初期値"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn action_defaults(action: Action) -> String {
    let keys: Vec<String> = action.default_keys().iter().map(|k| combo_label(k)).collect();
    format!("初期値: {}", if keys.is_empty() { "なし".to_string() } else { keys.join(" / ") })
}
