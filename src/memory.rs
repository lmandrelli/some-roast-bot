use chrono::Utc;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

static DB: Mutex<Option<Connection>> = Mutex::new(None);

/// Default path for the SQLite database file.
const DEFAULT_DB_PATH: &str = "data/memory.db";

/// Initialise the SQLite database (creates the file + table if needed).
/// Call this once at startup.
pub fn init() {
    let db_path = std::env::var("MEMORY_DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_string());

    // Ensure parent directory exists
    if let Some(parent) = Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let conn = Connection::open(&db_path).expect("failed to open memory database");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS news (
            id       INTEGER PRIMARY KEY AUTOINCREMENT,
            topic    TEXT    NOT NULL,
            used_at  TEXT    NOT NULL
        );",
    )
    .expect("failed to create news table");

    *DB.lock().unwrap() = Some(conn);
    tracing::info!("Memory database initialised at {db_path}");
}

/// Return the last `limit` news topics that were already used, most recent first.
pub fn recent_topics(limit: usize) -> Vec<String> {
    let guard = DB.lock().unwrap();
    let conn = guard.as_ref().expect("memory not initialised");

    let mut stmt = conn
        .prepare("SELECT topic FROM news ORDER BY id DESC LIMIT ?1")
        .expect("failed to prepare query");

    stmt.query_map([limit as i64], |row| row.get::<_, String>(0))
        .expect("query failed")
        .filter_map(Result::ok)
        .collect()
}

/// Store a news topic so it won't be repeated.
pub fn remember_topic(topic: &str) {
    let guard = DB.lock().unwrap();
    let conn = guard.as_ref().expect("memory not initialised");

    conn.execute(
        "INSERT INTO news (topic, used_at) VALUES (?1, ?2)",
        (topic, Utc::now().to_rfc3339()),
    )
    .expect("failed to insert topic");
}
