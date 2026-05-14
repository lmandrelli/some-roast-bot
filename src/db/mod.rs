pub mod sqlite;

use crate::error::DbError;

/// Repository for bot memory / statistics storage.
pub trait MemoryRepository: Send + Sync {
    /// Record a roast event.
    fn record_roast(
        &self,
        triggerer_id: &str,
        target_id: Option<&str>,
        roast_type: &str,
    ) -> Result<(), DbError>;

    /// Return the last `limit` used news topics, most recent first.
    fn recent_topics(&self, limit: usize) -> Result<Vec<String>, DbError>;

    /// Store a news topic to avoid repeating it.
    fn remember_topic(&self, topic: &str) -> Result<(), DbError>;

    /// Get the global microsoft + quoi_feur counts.
    fn get_stats(&self) -> Result<(i64, i64), DbError>;

    /// Increment the Microsoft roast counter.
    fn increment_microsoft_count(&self) -> Result<(), DbError>;

    /// Increment the quoi-feur counter.
    fn increment_quoi_feur_count(&self) -> Result<(), DbError>;

    /// Total count for a specific roast type.
    fn get_roast_count(&self, roast_type: &str) -> Result<i64, DbError>;

    /// Top triggerers for a roast type.
    fn get_top_triggerers(
        &self,
        roast_type: &str,
        limit: usize,
    ) -> Result<Vec<(String, i64)>, DbError>;

    /// Top targets for a roast type.
    fn get_top_targets(
        &self,
        roast_type: &str,
        limit: usize,
    ) -> Result<Vec<(String, i64)>, DbError>;
}
