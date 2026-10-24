use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{types::Json as SqlJson, FromRow};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::extractor::AuthUser,
    error::AppError,
    llm::translator,
    models::pdf::{blocks_to_markdown, PdfBlock},
    pdf::{ExtractRequest, Extraction},
    routes::plan,
    state::AppState,
};

const EXTRACTION_COLUMNS: &str = "resource_url, connector, engine, page_count, blocks, markdown";
const TRANSLATION_COLUMNS: &str = "source_language, target_language, provider, blocks, markdown";
const TRANSLATION_BATCH_CHARS: usize = 4_000;
const TRANSLATION_BATCH_ITEMS: usize = 16;

#[derive(Debug, FromRow)]
struct PdfExtractionRow {
    resource_url: String,
    connector: String,
    engine: String,
    page_count: i32,
    blocks: SqlJson<Vec<PdfBlock>>,
    markdown: String,
}

#[derive(Debug, FromRow)]
struct PdfTranslationRow {
    source_language: String,
    target_language: String,
    provider: String,
    blocks: SqlJson<Vec<PdfBlock>>,
    markdown: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TranslationQuery {
    pub language: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PdfContentResponse {
    pub activity_id: Uuid,
    pub resource_url: String,
    pub connector: String,
    pub engine: String,
    pub page_count: i32,
    /// True when the extraction was served from the Postgres cache.
    pub cached: bool,
    pub blocks: Vec<PdfBlock>,
    pub markdown: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PdfTranslationResponse {
    pub activity_id: Uuid,
    pub resource_url: String,
    pub source_language: String,
    pub target_language: String,
    /// False when the source language already matches the requested one.
    pub translated: bool,
    /// True when the translation was served from the Postgres cache.
    pub cached: bool,
    pub provider: String,
    pub blocks: Vec<PdfBlock>,
    pub markdown: String,
}

/// Load the cached extraction or run the sidecar on first access. The cache
/// is keyed by resource URL, so every user sharing a resource shares the work.
async fn ensure_extraction(
    state: &AppState,
    resource_url: &str,
    connector: &str,
) -> Result<(PdfExtractionRow, bool), AppError> {
    if let Some(row) = fetch_extraction(state, resource_url).await? {
        return Ok((row, true));
    }

    let extraction: Extraction = state
        .pdf
        .extract(&ExtractRequest {
            resource_url: resource_url.to_string(),
            connector: connector.to_string(),
        })
        .await
        .map_err(|error| {
            tracing::warn!(%error, %resource_url, "PDF extraction failed");
            AppError::BadRequest(format!("could not extract PDF: {error}"))
        })?;

    let sql = format!(
        "INSERT INTO pdf_extractions (resource_url, connector, engine, page_count, blocks, markdown) \
         VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (resource_url) DO NOTHING \
         RETURNING {EXTRACTION_COLUMNS}"
    );
    let inserted = sqlx::query_as::<_, PdfExtractionRow>(&sql)
        .bind(resource_url)
        .bind(&extraction.connector)
        .bind(&extraction.engine)
        .bind(extraction.page_count)
        .bind(SqlJson(extraction.blocks.clone()))
        .bind(&extraction.markdown)
        .fetch_optional(&state.pool)
        .await?;

    match inserted {
        Some(row) => Ok((row, false)),
        // Another request cached this resource between our lookup and insert.
        None => {
            let row = fetch_extraction(state, resource_url)
                .await?
                .ok_or_else(|| AppError::Internal("PDF extraction cache race".into()))?;
            Ok((row, true))
        }
    }
}

async fn fetch_extraction(
    state: &AppState,
    resource_url: &str,
) -> Result<Option<PdfExtractionRow>, AppError> {
    let sql = format!("SELECT {EXTRACTION_COLUMNS} FROM pdf_extractions WHERE resource_url = $1");
    let row = sqlx::query_as::<_, PdfExtractionRow>(&sql)
        .bind(resource_url)
        .fetch_optional(&state.pool)
        .await?;
    Ok(row)
}

fn normalize_language(raw: &str) -> Result<String, AppError> {
    let value = raw.trim();
    if value.is_empty() || value.len() > 24 {
        return Err(AppError::BadRequest("invalid language tag".into()));
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        return Err(AppError::BadRequest("invalid language tag".into()));
    }
    Ok(value.replace('_', "-").to_ascii_lowercase())
}

async fn requested_language(
    state: &AppState,
    auth: &AuthUser,
    requested: Option<String>,
) -> Result<String, AppError> {
    match requested {
        Some(language) if !language.trim().is_empty() => normalize_language(&language),
        _ => {
            let language = plan::preferred_language(state, auth.id).await?;
            normalize_language(&language)
        }
    }
}

async fn translate_blocks(
    state: &AppState,
    blocks: Vec<PdfBlock>,
    source_language: &str,
    target_language: &str,
) -> Result<(Vec<PdfBlock>, String), AppError> {
    let mut translated_blocks = blocks;
    let mut indices = Vec::new();
    let mut texts = Vec::new();

    for (index, block) in translated_blocks.iter().enumerate() {
        if block.is_text() {
            if let Some(text) = block.text.as_deref() {
                indices.push(index);
                texts.push(text.to_string());
            }
        }
    }

    for range in translator::batch_ranges(&texts, TRANSLATION_BATCH_CHARS, TRANSLATION_BATCH_ITEMS)
    {
        let translated = state
            .translator
            .translate_batch(&texts[range.clone()], source_language, target_language)
            .await
            .map_err(|error| {
                tracing::warn!(%error, "PDF translation failed");
                AppError::Internal("translation failed".into())
            })?;
        if translated.len() != range.len() {
            return Err(AppError::Internal(
                "translator returned an unexpected segment count".into(),
            ));
        }
        for (offset, text) in translated.into_iter().enumerate() {
            translated_blocks[indices[range.start + offset]].text = Some(text);
        }
    }

    let markdown = blocks_to_markdown(&translated_blocks);
    Ok((translated_blocks, markdown))
}

fn pdf_resource(activity: &crate::models::activity::Activity) -> Result<String, AppError> {
    let url = activity
        .resource_url
        .clone()
        .ok_or_else(|| AppError::BadRequest("activity has no resource URL".into()))?;
    if url.trim().is_empty() {
        return Err(AppError::BadRequest("activity has no resource URL".into()));
    }
    Ok(url)
}

#[utoipa::path(
    get,
    path = "/activities/{id}/content",
    tag = "pdf",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Activity id")),
    responses(
        (status = 200, description = "Reflowed PDF content, original URL kept for the raw toggle", body = PdfContentResponse),
        (status = 400, description = "Activity is not an extractable PDF", body = ErrorResponse),
        (status = 404, description = "Activity not found", body = ErrorResponse),
    )
)]
pub async fn get_content(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(activity_id): Path<Uuid>,
) -> Result<Json<PdfContentResponse>, AppError> {
    let activity = plan::load_activity_owned(&state, activity_id, &auth).await?;
    let resource_url = pdf_resource(&activity)?;
    let connector = activity
        .source_connector
        .clone()
        .unwrap_or_else(|| "generic".to_string());

    let (row, cached) = ensure_extraction(&state, &resource_url, &connector).await?;

    Ok(Json(PdfContentResponse {
        activity_id: activity.id,
        resource_url: row.resource_url,
        connector: row.connector,
        engine: row.engine,
        page_count: row.page_count,
        cached,
        blocks: row.blocks.0,
        markdown: row.markdown,
    }))
}

#[utoipa::path(
    get,
    path = "/activities/{id}/translation",
    tag = "pdf",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Activity id"),
        ("language" = Option<String>, Query, description = "Target language (defaults to the user preference)"),
    ),
    responses(
        (status = 200, description = "Translated text panel; translated on first access and cached", body = PdfTranslationResponse),
        (status = 400, description = "Activity is not an extractable PDF or the language is invalid", body = ErrorResponse),
        (status = 404, description = "Activity not found", body = ErrorResponse),
    )
)]
pub async fn get_translation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(activity_id): Path<Uuid>,
    Query(query): Query<TranslationQuery>,
) -> Result<Json<PdfTranslationResponse>, AppError> {
    let activity = plan::load_activity_owned(&state, activity_id, &auth).await?;
    let resource_url = pdf_resource(&activity)?;
    let connector = activity
        .source_connector
        .clone()
        .unwrap_or_else(|| "generic".to_string());
    let target_language = requested_language(&state, &auth, query.language).await?;

    let (extraction, _) = ensure_extraction(&state, &resource_url, &connector).await?;

    let source_language =
        normalize_language(&activity.language).unwrap_or_else(|_| "en".to_string());

    if source_language == target_language {
        return Ok(Json(PdfTranslationResponse {
            activity_id: activity.id,
            resource_url: extraction.resource_url,
            source_language,
            target_language,
            translated: false,
            cached: true,
            provider: "none".to_string(),
            blocks: extraction.blocks.0,
            markdown: extraction.markdown,
        }));
    }

    if let Some(row) = fetch_translation(&state, &resource_url, &target_language).await? {
        return Ok(Json(translation_response(
            activity.id,
            resource_url,
            row,
            true,
        )));
    }

    let (blocks, markdown) = translate_blocks(
        &state,
        extraction.blocks.0,
        &source_language,
        &target_language,
    )
    .await?;
    let provider = state.translator.provider_name();

    let sql = format!(
        "INSERT INTO pdf_translations \
         (resource_url, source_language, target_language, provider, blocks, markdown) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (resource_url, target_language) DO NOTHING \
         RETURNING {TRANSLATION_COLUMNS}"
    );
    let inserted = sqlx::query_as::<_, PdfTranslationRow>(&sql)
        .bind(&resource_url)
        .bind(&source_language)
        .bind(&target_language)
        .bind(&provider)
        .bind(SqlJson(blocks))
        .bind(&markdown)
        .fetch_optional(&state.pool)
        .await?;

    let (row, cached) = match inserted {
        Some(row) => (row, false),
        None => {
            let row = fetch_translation(&state, &resource_url, &target_language)
                .await?
                .ok_or_else(|| AppError::Internal("PDF translation cache race".into()))?;
            (row, true)
        }
    };

    Ok(Json(translation_response(
        activity.id,
        resource_url,
        row,
        cached,
    )))
}

async fn fetch_translation(
    state: &AppState,
    resource_url: &str,
    target_language: &str,
) -> Result<Option<PdfTranslationRow>, AppError> {
    let sql = format!(
        "SELECT {TRANSLATION_COLUMNS} FROM pdf_translations \
         WHERE resource_url = $1 AND target_language = $2"
    );
    let row = sqlx::query_as::<_, PdfTranslationRow>(&sql)
        .bind(resource_url)
        .bind(target_language)
        .fetch_optional(&state.pool)
        .await?;
    Ok(row)
}

fn translation_response(
    activity_id: Uuid,
    resource_url: String,
    row: PdfTranslationRow,
    cached: bool,
) -> PdfTranslationResponse {
    PdfTranslationResponse {
        activity_id,
        resource_url,
        source_language: row.source_language,
        target_language: row.target_language,
        translated: true,
        cached,
        provider: row.provider,
        blocks: row.blocks.0,
        markdown: row.markdown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_language_tags() {
        assert_eq!(normalize_language("pt-BR").unwrap(), "pt-br");
        assert_eq!(normalize_language("pt_br").unwrap(), "pt-br");
        assert_eq!(normalize_language("EN").unwrap(), "en");
    }

    #[test]
    fn rejects_invalid_language_tags() {
        assert!(normalize_language("").is_err());
        assert!(normalize_language("pt br").is_err());
        assert!(normalize_language("pt/br").is_err());
        assert!(normalize_language(&"x".repeat(25)).is_err());
    }
}
