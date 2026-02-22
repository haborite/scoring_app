// src/ui/grading.rs
use dioxus::prelude::*;
use std::rc::Rc;

use crate::db::{Db, QuestionRow, StudentRow};
use crate::ui::scorerow::ScoreRow;
use crate::ui::search::SearchWindow;

#[component]
pub fn GradingPage(on_back: EventHandler<()>) -> Element {

    // DB access context
    let db = use_context::<Db>();

    // master data
    let mut questions = use_signal(Vec::<QuestionRow>::new);
    let mut students = use_signal(Vec::<StudentRow>::new);

    // table rows
    let mut table_rows = use_signal(Vec::<TableRow>::new);

    // student page navigation
    let cur_student_idx = use_signal(|| 0usize);

    // inputs (string for UI; "" => NULL)
    let mut score_inputs = use_signal(Vec::<String>::new);

    // focus (0..questions.len())
    let mut focus_idx = use_signal(|| 0usize);

    // status message
    let mut msg = use_signal(|| String::new());

    // search popup
    let mut search_open = use_signal(|| false);

    // progress panel
    let total_students = use_signal(|| 0i64);
    let completed_students = use_signal(|| 0i64);    

    use_effect(move || {
        let ss = students.read().clone();
        let qs = questions.read().clone();
        if ss.is_empty() || qs.is_empty() {
            table_rows.set(vec![]);
            return;
        }
        // 初期は空欄で埋める（DBから取りたいなら別で一括ロード）
        let qids = qs.iter().map(|q| q.id.clone()).collect::<Vec<_>>();
        let mut rows = Vec::with_capacity(ss.len());
        for s in ss {
            rows.push(TableRow {
                student_id: s.id,
                student_name: s.name,
                scores: vec![String::new(); qids.len()],
                final_display: String::new(),
            });
        }
        table_rows.set(rows);
    });

    // ---------- initial load ----------
    let db_cloned = db.clone();
    use_effect(move || {
        let db_cloned2 = db_cloned.clone();
        spawn(async move {
            match db_cloned2.list_questions().await {
                Ok(qs) => questions.set(qs),
                Err(e) => {
                    msg.set(format!("questions load error: {e:#}"));
                    return;
                }
            }
            match db_cloned2.list_students().await {
                Ok(ss) => students.set(ss),
                Err(e) => {
                    msg.set(format!("students load error: {e:#}"));
                    return;
                }
            }
            msg.set("".to_string());
        });
    });

    // rebuild score_inputs when questions change
    use_effect(move || {
        score_inputs.set(vec![String::new(); questions().len()]);
        focus_idx.set(0);
    });    

    // refresh progress
    let refresh_progress = {
        let db = db.clone();
        move || {
            let db2 = db.clone();
            let mut total2 = total_students.clone();
            let mut done2 = completed_students.clone();
            let mut msg2 = msg.clone();
            spawn(async move {
                match db2.count_total_students().await {
                    Ok(n) => total2.set(n),
                    Err(e) => msg2.set(format!("progress error: {e:#}")),
                }
                match db2.count_completed_students().await {
                    Ok(n) => done2.set(n),
                    Err(e) => msg2.set(format!("progress error: {e:#}")),
                }
            });
        }
    };

    {
        let refresh_progress = refresh_progress.clone();
        use_effect(move || refresh_progress());
    }

    {
        use_effect(move || {
            let js = format!(
                r#"queueMicrotask(() => {{
                    const el = document.getElementById("score-{cur_student_idx}-{focus_idx}");
                    if (el) {{
                        el.focus();
                        if (el.select) el.select();
                    }}
                }});"#
            );
            let _ = document::eval(&js);
        });
    }

    // load scores for current student
    let load_current_student_scores = {
        let db = db.clone();
        let students = students.clone();
        let cur_student_idx = cur_student_idx.clone();
        let questions = questions.clone();
        let mut score_inputs = score_inputs.clone();
        let msg = msg.clone();

        move || {
            let ss = students.read().clone();
            let idx = *cur_student_idx.read();
            let qs = questions.read().clone();

            if ss.is_empty() || qs.is_empty() || idx >= ss.len() {
                score_inputs.set(vec![String::new(); qs.len()]);
                return;
            }

            let sid = ss[idx].id.clone();
            let db2 = db.clone();
            let mut score_inputs2 = score_inputs.clone();
            let mut msg2 = msg.clone();

            spawn(async move {
                match db2.get_scores_for_student(&sid).await {
                    Ok(pairs) => {
                        let mut v = vec![String::new(); qs.len()];
                        for (i, (_qid, score)) in pairs.into_iter().enumerate().take(qs.len()) {
                            v[i] = score.map(|x| x.to_string()).unwrap_or_default();
                        }
                        score_inputs2.set(v);
                        msg2.set("".to_string());
                    }
                    Err(e) => msg2.set(format!("load scores error: {e:#}")),
                }
            });
        }
    };

    {
        let mut load = load_current_student_scores.clone();
        let students = students.clone();
        let questions = questions.clone();
        let cur_student_idx = cur_student_idx.clone();

        use_effect(move || {
            let _ = students.read().len();
            let _ = questions.read().len();
            let _ = *cur_student_idx.read();
            load();
        });
    }

    // move student with completion mark
    let move_student: Rc<dyn Fn(i32)> = {
        let db = db.clone();
        let students = students.clone();
        let questions = questions.clone();
        let score_inputs = score_inputs.clone();
        let cur_student_idx = cur_student_idx.clone();
        let focus_idx = focus_idx.clone();
        let refresh_progress = refresh_progress.clone();

        Rc::new(move |delta: i32| {
            let mut cur_student_idx = cur_student_idx.clone();
            let mut focus_idx = focus_idx.clone();
            let ss = students.read().clone();
            if ss.is_empty() {
                return;
            }

            let old_idx = *cur_student_idx.read();
            let new_idx_i32 = old_idx as i32 + delta;
            let new_idx = if new_idx_i32 < 0 {
                0
            } else if new_idx_i32 as usize >= ss.len() {
                ss.len() - 1
            } else {
                new_idx_i32 as usize
            };

            // completion mark (ドメイン処理なのでRust側でOK)
            let qs = questions.read().clone();
            let inputs = score_inputs.read().clone();
            if old_idx < ss.len() && !qs.is_empty() && inputs.len() == qs.len() {
                if is_student_done(&qs, &inputs) {
                    let sid = ss[old_idx].id.clone();
                    let db2 = db.clone();
                    spawn(async move {
                        let _ = db2.mark_completed_once(&sid).await;
                    });
                }
            }

            cur_student_idx.set(new_idx);
            focus_idx.set(0); // 次の学生は先頭から
            refresh_progress();
        })
    };



    // ---------- derived ----------
    let ss = students.read();
    let qs = questions.read();

    let cur_student = ss.get(*cur_student_idx.read()).cloned();
    let cur_student_label = cur_student
        .as_ref()
        .map(|s| format!("{}  {}", s.id, s.name))
        .unwrap_or_else(|| "(受験者なし)".to_string());

    let mv_prev2 = move_student.clone();
    let mv_next2 = move_student.clone();
    let mv_prev3 = move_student.clone();
    let mv_next3 = move_student.clone();

    // ---------- UI ----------
    rsx! {
        div {
            class: "min-h-screen p-4 bg-base-200 text-base-content",
            tabindex: "0",

            // global hotkeys: F opens search
            onkeydown: move |e| {
                let k = e.key();
                if k == Key::Character("F".to_string()) || k == Key::Character("f".to_string()) {
                    e.prevent_default();
                    search_open.set(true);
                }
                else if k == Key::Character("L".to_string()) || k == Key::Character("l".to_string()) {
                    e.prevent_default();
                    mv_next2(1);
                }
                else if k == Key::Character("J".to_string()) || k == Key::Character("j".to_string()) {
                    e.prevent_default();
                    mv_prev2(-1);
                }
            },

            // Top bar / Navbar
            div { class: "navbar bg-base-100 rounded-box shadow mb-4",
                div { class: "navbar-start gap-2",
                    button { class: "btn btn-sm", onclick: move |_| on_back.call(()), "戻る" }
                    button { class: "btn btn-sm", onclick: move |_| (mv_prev3)(-1), "← 前" }
                    button { class: "btn btn-sm btn-primary", onclick: move |_| (mv_next3)(1), "次 →" }
                }
                div { class: "navbar-center",
                    div { class: "text-lg font-bold", "{cur_student_label}" }
                }
                div { class: "navbar-end",
                    div { class: "text-xs opacity-70 hidden md:block",
                        "F:検索 / J,K:受験者移動 / Enter:次問題"
                    }
                }
            }

            {(!msg.read().is_empty()).then(|| rsx! {
                div { class: "alert alert-error mb-4", "{msg}" }
            })}

            // Main area
            div { class: "grid grid-cols-1 xl:grid-cols-[1fr_18rem_20rem] gap-4",

                // grading inputs
                div { class: "card bg-base-100 shadow",
                    div { class: "card-body",
                        div { class: "card-title", "採点入力" }

                        {
                            if ss.is_empty() {
                                rsx!(div { class: "alert", "受験者が未登録です" })
                            } else if qs.is_empty() {
                                rsx!(div { class: "alert", "問題が未登録です" })
                            } else {
                                rsx! {
                                    div { class: "space-y-2",
                                        for (i, q) in qs.iter().enumerate() {
                                            {
                                                let full = q.full_score;
                                                let qid = q.id.clone();

                                                // 親: 状態&DB更新の責務
                                                let db_in = db.clone();
                                                let students_in = students.clone();
                                                let cur_idx_in = cur_student_idx.clone();

                                                let value = score_inputs.read().get(i).cloned().unwrap_or_default();
                                                let is_focused = *focus_idx.read() == i;
                                                let len_qs = qs.len();

                                                rsx! {
                                                    ScoreRow {
                                                        key: "row-{cur_student_idx}-{i}",
                                                        sid: cur_student_idx(),
                                                        qid: i,
                                                        q_name: q.name.clone(),
                                                        full: full,
                                                        value: value,
                                                        focused: is_focused,

                                                        // UIイベント -> 親が状態更新
                                                        on_change: move |new_str: String| {
                                                            // 1) 表示値更新
                                                            score_inputs.with_mut(|v| {
                                                                if i < v.len() { v[i] = new_str.clone(); }
                                                            });

                                                            // 2) DB書き込み
                                                            let mut s = new_str.clone();
                                                            s.retain(|c| c.is_ascii_digit());

                                                            if let Some(st) = students_in.read().get(*cur_idx_in.read()) {
                                                                let sid = st.id.clone();
                                                                let qid2 = qid.clone();
                                                                let db2 = db_in.clone();

                                                                if s.trim().is_empty() {
                                                                    spawn(async move { let _ = db2.set_score(&sid, &qid2, None).await; });
                                                                } else if let Ok(n) = s.parse::<i64>() {
                                                                    let mut m = Some(n);
                                                                    if n < 0 { m = Some(0); }
                                                                    if n > full { m = None; }
                                                                    spawn(async move { let _ = db2.set_score(&sid, &qid2, m).await; });
                                                                } else {
                                                                    spawn(async move { let _ = db2.set_score(&sid, &qid2, None).await; });
                                                                }
                                                            }

                                                            // 3) 一覧キャッシュ更新（現在学生の行だけ）
                                                            let cur_idx = *cur_idx_in.read();
                                                            let qs = questions.read().clone();
                                                            let new_val = new_str.clone();

                                                            table_rows.with_mut(|rows| {
                                                                if cur_idx >= rows.len() { return; }

                                                                // scores[qid] を更新
                                                                if i < rows[cur_idx].scores.len() {
                                                                    rows[cur_idx].scores[i] = new_val.clone();
                                                                }

                                                                // final を再計算（scores を Option<i64> に戻して calc_final へ）
                                                                let mut scores_opt = Vec::with_capacity(qs.len());
                                                                for (j, q) in qs.iter().enumerate() {
                                                                    let s = rows[cur_idx].scores.get(j).cloned().unwrap_or_default();
                                                                    let t = s.trim();
                                                                    if t.is_empty() {
                                                                        scores_opt.push(None);
                                                                    } else if let Ok(n) = t.parse::<i64>() {
                                                                        // clamp
                                                                        let n = n.clamp(0, q.full_score);
                                                                        scores_opt.push(Some(n));
                                                                    } else {
                                                                        scores_opt.push(None);
                                                                    }
                                                                }
                                                                let final_opt = calc_final(&qs, &scores_opt);
                                                                rows[cur_idx].final_display = final_opt.map(|x| format!("{:.4}", x)).unwrap_or_default();
                                                            });

                                                        },
                                                        move_to_next: move |_| {
                                                            let idx = std::cmp::min(i + 1, len_qs);
                                                            focus_idx.set(idx);
                                                        },
                                                        move_to_prev: move |_| {
                                                            let idx = if i <= 0 {0} else {i - 1};
                                                            focus_idx.set(idx);
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

                // comment panel card (placeholder)
                div { class: "card bg-base-100 shadow",
                    div { class: "card-body",
                        div { class: "card-title", "コメント" }
                        div { class: "opacity-60", "（未実装）" }
                    }
                }

                // progress panel card
                div { class: "card bg-base-100 shadow",
                    div { class: "card-body",
                        div { class: "card-title", "進捗" }
                        div { class: "flex justify-between",
                            span { "採点済み" }
                            span { class: "font-mono", "{completed_students}/{total_students}" }
                        }
                        div { class: "card-actions justify-end mt-4 gap-2",
                            button { class: "btn btn-sm", onclick: move |_| refresh_progress(), "更新" }
                            // button { class: "btn btn-sm btn-secondary", onclick: move |_| refresh_table(), "一覧更新" }
                        }
                    }
                }
            }

            // table panel card
            div { class: "card bg-base-100 shadow mt-4",
                div { class: "card-body",
                    div { class: "flex items-center gap-3",
                        div { class: "card-title", "一覧" }
                        div { class: "text-xs opacity-60", "（受験者×問題の表＋最終得点）" }
                    }

                    {
                        let qids = qs.iter().map(|q| q.id.clone()).collect::<Vec<_>>();
                        println!("qids: {:?}", qids);
                        rsx! {
                            div { class: "overflow-auto max-h-96 mt-3",
                                table { class: "table table-zebra table-sm",
                                    thead {
                                        tr {
                                            th { "id" }
                                            th { "name" }
                                            for qid in qids.iter() {
                                                th { "{qid}" }
                                            }
                                            th { "final" }
                                        }
                                    }
                                    tbody {
                                        for row in table_rows().iter() {
                                            tr {
                                                td { class: "font-mono", "{row.student_id}" }
                                                td { "{row.student_name}" }
                                                for s in row.scores.iter() {
                                                    td { class: "font-mono", "{s}" }
                                                }
                                                td { class: "font-mono font-semibold", "{row.final_display}" }
                                            }
                                        }
                                    }
                                }
                            },
                        }
                    }
                }
            }

            // search modal (DaisyUI)
            {
                search_open().then(|| 
                    rsx!( SearchWindow { search_open, msg, students, cur_student_idx, focus_idx } )
                )
            }
        }
    }
}


#[derive(Clone)]
struct TableRow {
    student_id: String,
    student_name: String,
    scores: Vec<String>,
    final_display: String,
}


// final = Σ(score/full*weight) / Σ(weight) * 100
fn calc_final(questions: &[QuestionRow], scores: &[Option<i64>]) -> Option<f64> {
    if questions.is_empty() {
        return None;
    }
    let mut num = 0.0f64;
    let mut den = 0.0f64;

    for (q, s) in questions.iter().zip(scores.iter()) {
        if q.weight <= 0.0 {
            continue;
        }
        den += q.weight;

        let sc = s.unwrap_or(0) as f64;
        let full = q.full_score.max(1) as f64;
        num += (sc / full) * q.weight;
    }

    if den == 0.0 { None } else { Some(num / den * 100.0) }
}


fn is_student_done(questions: &[QuestionRow], inputs: &[String]) -> bool {
    if questions.is_empty() || inputs.len() != questions.len() {
        return false;
    }
    for (q, s) in questions.iter().zip(inputs.iter()) {
        let t = s.trim();
        if t.is_empty() {
            return false;
        }
        if let Ok(n) = t.parse::<i64>() {
            if n < 0 || n > q.full_score {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}
