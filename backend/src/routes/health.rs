#[utoipa::path(
    get,
    path = "/health",
    tag = "system",
    responses((status = 200, description = "Service is healthy", body = String))
)]
pub async fn health() -> &'static str {
    "ok"
}
