use sqlx::PgPool;

pub const YOUTUBE_SEARCH_UNITS: i64 = 100;

#[derive(Clone)]
pub struct QuotaTracker {
    pool: PgPool,
    youtube_daily_units: i64,
}

impl QuotaTracker {
    pub fn new(pool: PgPool, youtube_daily_units: i64) -> Self {
        Self {
            pool,
            youtube_daily_units,
        }
    }

    pub async fn consume_youtube_search(&self) -> bool {
        let result: Result<Option<i64>, sqlx::Error> = sqlx::query_scalar(
            "INSERT INTO connector_usage (connector, usage_date, units) \
             VALUES ('youtube', CURRENT_DATE, $1) \
             ON CONFLICT (connector, usage_date) DO UPDATE \
             SET units = connector_usage.units + EXCLUDED.units, updated_at = now() \
             WHERE connector_usage.units + EXCLUDED.units <= $2 \
             RETURNING units::BIGINT",
        )
        .bind(YOUTUBE_SEARCH_UNITS)
        .bind(self.youtube_daily_units)
        .fetch_optional(&self.pool)
        .await;

        match result {
            Ok(usage) => usage.is_some(),
            Err(error) => {
                tracing::warn!(%error, "failed to check YouTube quota; skipping connector");
                false
            }
        }
    }
}
