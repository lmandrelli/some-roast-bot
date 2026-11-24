use std::path::Path;

use async_trait::async_trait;
use chrono::Utc;
use sqlx::AnyPool;
use sqlx::any::AnyPoolOptions;
use sqlx::migrate::Migrator;

use crate::db::MemoryRepository;
use crate::error::DbError;

static SQLITE_MIGRATOR: Migrator = sqlx::migrate!("./migrations/sqlite");
static POSTGRES_MIGRATOR: Migrator = sqlx::migrate!("./migrations/postgres");

/// Memory repository on SQLite or Postgres through the sqlx `Any` driver.
/// Queries are written once in the SQL subset both backends accept
/// (`$N` placeholders, `ON CONFLICT`, `CAST(... AS BIGINT)`).
pub struct SqlMemoryRepository {
    pool: AnyPool,
}

impl SqlMemoryRepository {
    /// Connect to `database_url` when set, otherwise to the SQLite file at
    /// `sqlite_path`, then apply the migrations of that backend.
    pub async fn connect(database_url: Option<&str>, sqlite_path: &Path) -> Result<Self, DbError> {
        sqlx::any::install_default_drivers();

        let url = match database_url {
            Some(url) => url.to_string(),
            None => {
                if let Some(parent) = sqlite_path.parent() {
                    std::fs::create_dir_all(parent).map_err(sqlx::Error::Io)?;
                }
                format!("sqlite://{}?mode=rwc", sqlite_path.display())
            }
        };

        let (backend, migrator, max_connections) = if url.starts_with("sqlite:") {
            ("sqlite", &SQLITE_MIGRATOR, 1)
        } else if url.starts_with("postgres:") || url.starts_with("postgresql:") {
            ("postgres", &POSTGRES_MIGRATOR, 5)
        } else {
            return Err(DbError(sqlx::Error::Configuration(
                "DATABASE_URL must start with postgres:// or sqlite:".into(),
            )));
        };

        let pool = AnyPoolOptions::new()
            .max_connections(max_connections)
            .connect(&url)
            .await?;
        migrator.run(&pool).await.map_err(sqlx::Error::from)?;

        match database_url {
            Some(_) => tracing::info!("Memory database initialised ({backend})"),
            None => tracing::info!("Memory database initialised at {}", sqlite_path.display()),
        }

        Ok(SqlMemoryRepository { pool })
    }
}

#[async_trait]
impl MemoryRepository for SqlMemoryRepository {
    async fn record_roast(
        &self,
        triggerer_id: &str,
        target_id: Option<&str>,
        roast_type: &str,
    ) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO roast_stats (triggerer_id, target_id, roast_type, count) VALUES ($1, $2, $3, 1)
             ON CONFLICT (triggerer_id, target_id, roast_type) DO UPDATE SET count = roast_stats.count + 1",
        )
        .bind(triggerer_id)
        .bind(target_id)
        .bind(roast_type)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn recent_topics(&self, limit: usize) -> Result<Vec<String>, DbError> {
        Ok(
            sqlx::query_scalar("SELECT topic FROM news ORDER BY id DESC LIMIT $1")
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await?,
        )
    }

    async fn remember_topic(&self, topic: &str) -> Result<(), DbError> {
        sqlx::query("INSERT INTO news (topic, used_at) VALUES ($1, $2)")
            .bind(topic)
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?;
        // Keep only the 10 most recent topics to prevent prompt bloat
        sqlx::query(
            "DELETE FROM news WHERE id NOT IN (SELECT id FROM news ORDER BY id DESC LIMIT 10)",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_stats(&self) -> Result<(i64, i64), DbError> {
        Ok(
            sqlx::query_as("SELECT microsoft, quoi_feur FROM stats WHERE id = 1")
                .fetch_one(&self.pool)
                .await?,
        )
    }

    async fn increment_microsoft_count(&self) -> Result<(), DbError> {
        sqlx::query("UPDATE stats SET microsoft = microsoft + 1 WHERE id = 1")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn increment_quoi_feur_count(&self) -> Result<(), DbError> {
        sqlx::query("UPDATE stats SET quoi_feur = quoi_feur + 1 WHERE id = 1")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_roast_count(&self, roast_type: &str) -> Result<i64, DbError> {
        Ok(sqlx::query_scalar(
            "SELECT CAST(COALESCE(SUM(count), 0) AS BIGINT) FROM roast_stats WHERE roast_type = $1",
        )
        .bind(roast_type)
        .fetch_one(&self.pool)
        .await?)
    }

    async fn get_top_triggerers(
        &self,
        roast_type: &str,
        limit: usize,
    ) -> Result<Vec<(String, i64)>, DbError> {
        Ok(sqlx::query_as(
            "SELECT triggerer_id, CAST(SUM(count) AS BIGINT) AS total
             FROM roast_stats
             WHERE roast_type = $1
             GROUP BY triggerer_id
             ORDER BY total DESC
             LIMIT $2",
        )
        .bind(roast_type)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn get_top_targets(
        &self,
        roast_type: &str,
        limit: usize,
    ) -> Result<Vec<(String, i64)>, DbError> {
        Ok(sqlx::query_as(
            "SELECT target_id, CAST(SUM(count) AS BIGINT) AS total
             FROM roast_stats
             WHERE roast_type = $1 AND target_id IS NOT NULL
             GROUP BY target_id
             ORDER BY total DESC
             LIMIT $2",
        )
        .bind(roast_type)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique(prefix: &str) -> String {
        format!(
            "{prefix}-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        )
    }

    fn temp_sqlite_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("{}.sqlite", unique("some-roast-bot")))
    }

    /// Runs every repository operation. Keys are unique per run so the same
    /// Postgres database can be reused across runs.
    async fn exercise(repository: &SqlMemoryRepository) {
        let roast_type = unique("roast");
        assert_eq!(repository.get_roast_count(&roast_type).await.unwrap(), 0);
        for _ in 0..3 {
            repository
                .record_roast("alice", None, &roast_type)
                .await
                .unwrap();
        }
        for _ in 0..2 {
            repository
                .record_roast("bob", Some("alice"), &roast_type)
                .await
                .unwrap();
        }
        repository
            .record_roast("alice", Some("bob"), &roast_type)
            .await
            .unwrap();
        assert_eq!(repository.get_roast_count(&roast_type).await.unwrap(), 6);
        assert_eq!(
            repository.get_top_triggerers(&roast_type, 3).await.unwrap(),
            vec![("alice".to_string(), 4), ("bob".to_string(), 2)]
        );
        assert_eq!(
            repository.get_top_targets(&roast_type, 1).await.unwrap(),
            vec![("alice".to_string(), 2)]
        );

        let topics: Vec<String> = (0..12).map(|i| unique(&format!("topic{i}"))).collect();
        for topic in &topics {
            repository.remember_topic(topic).await.unwrap();
        }
        let expected: Vec<String> = topics.iter().rev().take(10).cloned().collect();
        assert_eq!(repository.recent_topics(20).await.unwrap(), expected);

        let (microsoft, quoi_feur) = repository.get_stats().await.unwrap();
        repository.increment_microsoft_count().await.unwrap();
        repository.increment_quoi_feur_count().await.unwrap();
        repository.increment_quoi_feur_count().await.unwrap();
        assert_eq!(
            repository.get_stats().await.unwrap(),
            (microsoft + 1, quoi_feur + 2)
        );
    }

    #[tokio::test]
    async fn sqlite_backend() {
        let path = temp_sqlite_path();
        let repository = SqlMemoryRepository::connect(None, &path).await.unwrap();
        exercise(&repository).await;
    }

    #[tokio::test]
    async fn sqlite_keeps_a_database_created_before_migrations() {
        let path = temp_sqlite_path();
        sqlx::any::install_default_drivers();
        let pool = AnyPool::connect(&format!("sqlite://{}?mode=rwc", path.display()))
            .await
            .unwrap();
        sqlx::raw_sql(include_str!("../../migrations/sqlite/0001_init.sql"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE stats SET microsoft = 5 WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;

        let repository = SqlMemoryRepository::connect(None, &path).await.unwrap();
        assert_eq!(repository.get_stats().await.unwrap(), (5, 0));
    }

    /// Set `TEST_DATABASE_URL=postgres://...` to run against Postgres.
    #[tokio::test]
    async fn postgres_backend() {
        let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
            eprintln!("TEST_DATABASE_URL not set, skipping");
            return;
        };
        let repository = SqlMemoryRepository::connect(Some(&url), Path::new(""))
            .await
            .unwrap();
        exercise(&repository).await;
    }
}
