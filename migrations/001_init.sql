PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS questions (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  full_score  INTEGER NOT NULL CHECK(full_score >= 0),
  weight      REAL NOT NULL CHECK(weight >= 0),
  comment     TEXT
);

CREATE TABLE IF NOT EXISTS students (
  id    TEXT PRIMARY KEY,
  name  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS scores (
  student_id  TEXT NOT NULL,
  question_id TEXT NOT NULL,
  score       INTEGER NULL CHECK(score IS NULL OR score >= 0),
  updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  PRIMARY KEY(student_id, question_id),
  FOREIGN KEY(student_id) REFERENCES students(id) ON DELETE CASCADE,
  FOREIGN KEY(question_id) REFERENCES questions(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS grading_sessions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  student_id TEXT NOT NULL,
  finished_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  FOREIGN KEY(student_id) REFERENCES students(id) ON DELETE CASCADE
);
