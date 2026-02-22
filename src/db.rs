use anyhow::{Context, Result};
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct QuestionRow {
    pub id: u32,
    pub name: String,
    pub full_score: u32,
    pub weight: f32,
    pub comment: String,
}

#[derive(Debug, Clone)]
pub struct StudentRow {
    pub id: String,
    pub name: String,
}

#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    pub async fn connect(db_path: &Path) -> Result<Self> {
        let opts = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(opts).await?;

        // Ensure FK is on for this connection pool
        sqlx::query("PRAGMA foreign_keys = ON;")
            .execute(&pool)
            .await?;

        // Run embedded migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .context("migration failed")?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn rebuild_score_matrix(&self) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO scores(student_id, question_id, score)
            SELECT s.id, q.id, NULL
            FROM students s CROSS JOIN questions q
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ---------- Questions ----------
    pub async fn list_questions(&self) -> Result<Vec<QuestionRow>> {
        let rows = sqlx::query(r#"SELECT id, name, full_score, weight, comment FROM questions ORDER BY id"#)
            .fetch_all(&self.pool)
            .await?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(QuestionRow {
                id: r.try_get::<u32, _>("id")?,
                name: r.try_get::<String, _>("name")?,
                full_score: r.try_get::<u32, _>("full_score")?,
                weight: r.try_get::<f32, _>("weight")?,
                comment: r.try_get::<String, _>("comment")?
            });
        }
        Ok(out)
    }

    pub async fn upsert_questions(&self, qs: &[QuestionRow]) -> Result<u64> {
        let mut tx = self.pool.begin().await?;
        let mut affected = 0u64;

        for q in qs {
            validate_question(q)
                .with_context(|| format!("invalid question: id={}", q.id))?;

            let res = sqlx::query(
                r#"
                INSERT INTO questions(id, name, full_score, weight, comment)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(id) DO UPDATE SET
                  name = excluded.name,
                  full_score = excluded.full_score,
                  weight = excluded.weight,
                  comment = excluded.weight
                "#,
            )
            .bind(&q.id)
            .bind(&q.name)
            .bind(q.full_score)
            .bind(q.weight)
            .bind(&q.comment)
            .execute(&mut *tx)
            .await?;

            affected += res.rows_affected();
        }

        // Keep matrix complete
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO scores(student_id, question_id, score)
            SELECT s.id, q.id, NULL
            FROM students s CROSS JOIN questions q
            "#,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(affected)
    }

    pub async fn insert_question(&self, q: &QuestionRow) -> Result<()> {
        validate_question(q)?;

        sqlx::query(r#"INSERT INTO questions(id, name, full_score, weight, comment) VALUES (?1, ?2, ?3, ?4, ?5)"#)
            .bind(&q.id)
            .bind(&q.name)
            .bind(q.full_score)
            .bind(q.weight)
            .bind(&q.comment)
            .execute(&self.pool)
            .await?;

        self.rebuild_score_matrix().await?;
        Ok(())
    }

    pub async fn update_question(&self, q: &QuestionRow) -> Result<()> {
        validate_question(q)?;

        sqlx::query(r#"UPDATE questions SET name=?2, full_score=?3, weight=?4, comment=?5 WHERE id=?1"#)
            .bind(&q.id)
            .bind(&q.name)
            .bind(q.full_score)
            .bind(q.weight)
            .bind(&q.comment)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn delete_question(&self, id: &str) -> Result<()> {
        sqlx::query(r#"DELETE FROM questions WHERE id=?1"#)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---------- Students ----------
    pub async fn list_students(&self) -> Result<Vec<StudentRow>> {
        let rows = sqlx::query(r#"SELECT id, name FROM students ORDER BY id"#)
            .fetch_all(&self.pool)
            .await?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(StudentRow {
                id: r.try_get::<String, _>("id")?,
                name: r.try_get::<String, _>("name")?,
            });
        }
        Ok(out)
    }

    pub async fn upsert_students(&self, ss: &[StudentRow]) -> Result<u64> {
        let mut tx = self.pool.begin().await?;
        let mut affected = 0u64;

        for s in ss {
            validate_student(s).with_context(|| format!("invalid student: id={}", s.id))?;

            let res = sqlx::query(
                r#"
                INSERT INTO students(id, name)
                VALUES (?1, ?2)
                ON CONFLICT(id) DO UPDATE SET
                  name = excluded.name
                "#,
            )
            .bind(&s.id)
            .bind(&s.name)
            .execute(&mut *tx)
            .await?;

            affected += res.rows_affected();
        }

        // Keep matrix complete
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO scores(student_id, question_id, score)
            SELECT s.id, q.id, NULL
            FROM students s CROSS JOIN questions q
            "#,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(affected)
    }

    pub async fn insert_student(&self, s: &StudentRow) -> Result<()> {
        validate_student(s)?;

        sqlx::query(r#"INSERT INTO students(id, name) VALUES (?1, ?2)"#)
            .bind(&s.id)
            .bind(&s.name)
            .execute(&self.pool)
            .await?;

        self.rebuild_score_matrix().await?;
        Ok(())
    }

    pub async fn update_student(&self, s: &StudentRow) -> Result<()> {
        validate_student(s)?;

        sqlx::query(r#"UPDATE students SET name=?2 WHERE id=?1"#)
            .bind(&s.id)
            .bind(&s.name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_student(&self, id: &str) -> Result<()> {
        sqlx::query(r#"DELETE FROM students WHERE id=?1"#)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---------- Scores (grading) ----------
    pub async fn get_scores_for_student(
        &self,
        student_id: &str,
    ) -> Result<Vec<(String, Option<u32>)>> {
        let rows = sqlx::query(
            r#"
            SELECT q.id as qid, sc.score as score
            FROM questions q
            LEFT JOIN scores sc
              ON sc.question_id = q.id AND sc.student_id = ?1
            ORDER BY q.id
            "#,
        )
        .bind(student_id)
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let qid: String = r.try_get("qid")?;
            let score: Option<u32> = r.try_get("score")?;
            out.push((qid, score));
        }
        Ok(out)
    }

    pub async fn set_score(
        &self,
        student_id: String,
        question_id: u32,
        score: Option<u32>,
    ) -> Result<()> {
        let question_id = &question_id.to_string();
        sqlx::query(
            r#"
            INSERT INTO scores(student_id, question_id, score)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(student_id, question_id) DO UPDATE SET
              score = excluded.score,
              updated_at = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
            "#,
        )
        .bind(student_id)
        .bind(question_id)
        .bind(score)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn search_students(&self, q: &str, limit: u32) -> Result<Vec<StudentRow>> {
        let pat = format!("%{}%", q);

        let rows = sqlx::query(
            r#"
            SELECT id, name
            FROM students
            WHERE id LIKE ?1 ESCAPE '\' OR name LIKE ?1 ESCAPE '\'
            ORDER BY id
            LIMIT ?2
            "#,
        )
        .bind(pat)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(StudentRow {
                id: r.try_get::<String, _>("id")?,
                name: r.try_get::<String, _>("name")?,
            });
        }
        Ok(out)
    }

    pub async fn count_total_students(&self) -> Result<u32> {
        let r = sqlx::query(r#"SELECT COUNT(*) as n FROM students"#)
            .fetch_one(&self.pool)
            .await?;
        let n: u32 = r.try_get("n")?;
        Ok(n)
    }

    pub async fn count_completed_students(&self) -> Result<u32> {
        let r = sqlx::query(
            r#"
            SELECT COUNT(*) as n
            FROM students s
            WHERE NOT EXISTS (
              SELECT 1
              FROM questions q
              LEFT JOIN scores sc
                ON sc.student_id = s.id AND sc.question_id = q.id
              WHERE sc.score IS NULL
            )
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        let n: u32 = r.try_get("n")?;
        Ok(n)
    }

    pub async fn mark_completed_once(&self, student_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO grading_sessions(student_id)
            SELECT ?1
            WHERE NOT EXISTS (
              SELECT 1 FROM grading_sessions WHERE student_id = ?1
            )
            "#,
        )
        .bind(student_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_completion_times_latest(&self, limit: u32) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT finished_at
            FROM grading_sessions
            ORDER BY finished_at DESC
            LIMIT ?1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            // finished_at is TEXT
            out.push(r.try_get::<String, _>("finished_at")?);
        }
        Ok(out)
    }

    pub async fn fetch_table_join_all(
        &self,
    ) -> Result<Vec<(String, String, String, Option<u32>, u32, f32)>> {
        let rows = sqlx::query(
            r#"
            SELECT
              s.id   as sid,
              s.name as sname,
              q.id   as qid,
              sc.score as score,
              q.full_score as full_score,
              q.weight as weight
            FROM students s
            CROSS JOIN questions q
            LEFT JOIN scores sc
              ON sc.student_id = s.id AND sc.question_id = q.id
            ORDER BY s.id, q.id
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let sid: String = r.try_get("sid")?;
            let sname: String = r.try_get("sname")?;
            let qid: String = r.try_get("qid")?;
            let score: Option<u32> = r.try_get("score")?;
            let full_score: u32 = r.try_get("full_score")?;
            let weight: f32 = r.try_get("weight")?;
            out.push((sid, sname, qid, score, full_score, weight));
        }
        Ok(out)
    }
}

// ---- validation helpers ----
fn validate_question(q: &QuestionRow) -> Result<()> {
    if q.name.trim().is_empty() {
        anyhow::bail!("question.name is empty");
    }
    if q.full_score < 0 {
        anyhow::bail!("question.full_score < 0");
    }
    if q.weight < 0.0 {
        anyhow::bail!("question.weight < 0");
    }
    Ok(())
}

fn validate_student(s: &StudentRow) -> Result<()> {
    if s.id.trim().is_empty() {
        anyhow::bail!("student.id is empty");
    }
    if s.name.trim().is_empty() {
        anyhow::bail!("student.name is empty");
    }
    Ok(())
}
