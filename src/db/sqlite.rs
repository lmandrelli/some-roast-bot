use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;
use rusqlite::Connection;

use crate::db::MemoryRepository;
use crate::error::DbError;

pub struct SqliteMemoryRepository {
    conn: Mutex<Connection>,
}

impl SqliteMemoryRepository {
    pub fn new(db_path: &Path) -> Result<Self, DbError> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DbError(rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                ))
            })?;
        }

        let conn = Connection::open(db_path).map_err(DbError)?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS news (
                id       INTEGER PRIMARY KEY AUTOINCREMENT,
                topic    TEXT    NOT NULL,
                used_at  TEXT    NOT NULL
            );

            CREATE TABLE IF NOT EXISTS stats (
                id          INTEGER PRIMARY KEY CHECK (id = 1),
                microsoft   INTEGER NOT NULL DEFAULT 0,
                quoi_feur   INTEGER NOT NULL DEFAULT 0
            );

            INSERT OR IGNORE INTO stats (id, microsoft, quoi_feur) VALUES (1, 0, 0);

            CREATE TABLE IF NOT EXISTS roast_stats (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                triggerer_id TEXT NOT NULL,
                target_id    TEXT,
                roast_type   TEXT NOT NULL,
                count        INTEGER NOT NULL DEFAULT 1,
                UNIQUE(triggerer_id, target_id, roast_type)
            );
            ",
        )
        .map_err(DbError)?;

        tracing::info!("Memory database initialised at {}", db_path.display());

        Ok(SqliteMemoryRepository {
            conn: Mutex::new(conn),
        })
    }
}

impl MemoryRepository for SqliteMemoryRepository {
    fn record_roast(
        &self,
        triggerer_id: &str,
        target_id: Option<&str>,
        roast_type: &str,
    ) -> Result<(), DbError> {
        let guard = self.conn.lock().unwrap();

        if let Some(tid) = target_id {
            guard.execute(
                "INSERT INTO roast_stats (triggerer_id, target_id, roast_type, count) VALUES (?1, ?2, ?3, 1)
                 ON CONFLICT(triggerer_id, target_id, roast_type) DO UPDATE SET count = count + 1",
                (triggerer_id, tid, roast_type),
            )?;
        } else {
            guard.execute(
                "INSERT INTO roast_stats (triggerer_id, target_id, roast_type, count) VALUES (?1, NULL, ?2, 1)
                 ON CONFLICT(triggerer_id, target_id, roast_type) DO UPDATE SET count = count + 1",
                (triggerer_id, roast_type),
            )?;
        }
        Ok(())
    }

    fn recent_topics(&self, limit: usize) -> Result<Vec<String>, DbError> {
        let guard = self.conn.lock().unwrap();
        let mut stmt = guard.prepare("SELECT topic FROM news ORDER BY id DESC LIMIT ?1")?;

        let topics = stmt
            .query_map([limit as i64], |row| row.get::<_, String>(0))?
            .filter_map(Result::ok)
            .collect();

        Ok(topics)
    }

    fn remember_topic(&self, topic: &str) -> Result<(), DbError> {
        let guard = self.conn.lock().unwrap();
        guard.execute(
            "INSERT INTO news (topic, used_at) VALUES (?1, ?2)",
            (topic, Utc::now().to_rfc3339()),
        )?;
        Ok(())
    }

    fn get_stats(&self) -> Result<(i64, i64), DbError> {
        let guard = self.conn.lock().unwrap();
        let mut stmt = guard.prepare("SELECT microsoft, quoi_feur FROM stats WHERE id = 1")?;
        let stats = stmt.query_row([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(stats)
    }

    fn increment_microsoft_count(&self) -> Result<(), DbError> {
        let guard = self.conn.lock().unwrap();
        guard.execute(
            "UPDATE stats SET microsoft = microsoft + 1 WHERE id = 1",
            [],
        )?;
        Ok(())
    }

    fn increment_quoi_feur_count(&self) -> Result<(), DbError> {
        let guard = self.conn.lock().unwrap();
        guard.execute(
            "UPDATE stats SET quoi_feur = quoi_feur + 1 WHERE id = 1",
            [],
        )?;
        Ok(())
    }

    fn get_roast_count(&self, roast_type: &str) -> Result<i64, DbError> {
        let guard = self.conn.lock().unwrap();
        let mut stmt = guard
            .prepare("SELECT COALESCE(SUM(count), 0) FROM roast_stats WHERE roast_type = ?1")?;

        let count = stmt.query_row([roast_type], |row| row.get(0))?;
        Ok(count)
    }

    fn get_top_triggerers(
        &self,
        roast_type: &str,
        limit: usize,
    ) -> Result<Vec<(String, i64)>, DbError> {
        let guard = self.conn.lock().unwrap();
        let mut stmt = guard.prepare(
            "SELECT triggerer_id, SUM(count) as total 
             FROM roast_stats 
             WHERE roast_type = ?1 
             GROUP BY triggerer_id 
             ORDER BY total DESC 
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(rusqlite::params![roast_type, limit as i64], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(results)
    }

    fn get_top_targets(
        &self,
        roast_type: &str,
        limit: usize,
    ) -> Result<Vec<(String, i64)>, DbError> {
        let guard = self.conn.lock().unwrap();
        let mut stmt = guard.prepare(
            "SELECT target_id, SUM(count) as total 
             FROM roast_stats 
             WHERE roast_type = ?1 AND target_id IS NOT NULL 
             GROUP BY target_id 
             ORDER BY total DESC 
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(rusqlite::params![roast_type, limit as i64], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(results)
    }
}
