use sqlx::PgPool;

use crate::{auth::password, state::AppState};

pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

pub async fn seed_admin(state: &AppState) -> anyhow::Result<()> {
    let email = state.cfg.admin_email.trim().to_lowercase();
    let existing: Option<(uuid::Uuid,)> =
        sqlx::query_as("SELECT id FROM users WHERE lower(email) = $1")
            .bind(&email)
            .fetch_optional(&state.pool)
            .await?;

    if existing.is_none() {
        let hash = password::hash_password(&state.cfg.admin_password)
            .map_err(|e| anyhow::anyhow!("failed to hash admin password: {e}"))?;
        sqlx::query(
            "INSERT INTO users (email, password_hash, role, preferred_language) \
             VALUES ($1, $2, 'admin', 'pt-BR')",
        )
        .bind(&email)
        .bind(&hash)
        .execute(&state.pool)
        .await?;
        tracing::info!(%email, "seeded admin user");
    }

    Ok(())
}
