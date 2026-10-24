# Phase 07 — PDF Pipeline + Translation

## Goal
Python sidecar extracts reflowable blocks; NIM translates with cache.

## Tasks
- [x] Sidecar endpoints: extract (normal vs academic 2-col), deterministic engine pick by connector
- [x] Unified block JSON (`heading|paragraph|image|table_image|formula_image`) → Markdown
- [x] Tables/formulas rasterized as images; raw-PDF toggle as fallback
- [x] Translate-on-first-access via NIM, cache in Postgres; PDFs show translated text panel
- [x] Cache extracted JSON per resource (shared across users)

## Exit criteria
- [x] OpenStax + arXiv samples render reflowed + toggle to original
- [x] Second view of same translation hits cache, no re-call

## Notes
- `pdf-service/app/extraction.py` is the pure extraction layer:
  - `select_engine(connector)` is deterministic — `arxiv`, `doaj` and
    `openalex` use the **academic** engine (column-aware reading order); every
    other connector uses the **normal** engine (top-to-bottom order).
  - Both engines return the same block schema: `heading`, `paragraph`,
    `image`, `table_image`, `formula_image`. Blocks carry `page`, `bbox` and
    image blocks carry a self-contained data URI, so a
    cached extraction needs no separate image store.
  - PyMuPDF is imported lazily. `blocks_to_markdown`, engine selection and
    reading-order helpers are unit-tested without the native library.
  - Tables are rasterized instead of reconstructed. Small documents use
    PyMuPDF's accurate `page.find_tables()`; larger books switch to a fast
    line-grid detector built from `page.get_drawings()` because the finder
    costs roughly a second per page on a 769-page OpenStax textbook. The mode
    is controlled by `PDF_TABLE_DETECTION` (`auto` default, `lines`,
    `pymupdf`) and `PDF_TABLE_FINDER_MAX_PAGES` (default 20). Both paths emit
    the same `table_image` block. Formula detection is the documented
    symbol-density heuristic; matching text is rasterized too. Both are
    best-effort, which is why the raw-PDF toggle remains the fallback.
  - Photo/image blocks are JPEG-compressed; tables and formulas stay PNG so
    text and symbols remain crisp. Every block still carries a self-contained
    data URI.
  - The academic engine sorts left column, then right column, and treats
    full-width blocks as band separators so headings spanning both columns
    stay in reading order.
- `pdf-service/app/main.py` exposes `POST /extract` (multipart PDF +
  `connector` form field) and keeps `GET /health` reporting both engines.
  Uploads are rejected when they are not a PDF, exceed `PDF_MAX_BYTES`
  (default 64 MB in the sidecar) or extract to no content.
- Backend client `src/pdf/`:
  - Downloads the resource, verifies the `%PDF` magic bytes and enforces
    `PDF_MAX_BYTES` (default 64 MB), then posts it to the sidecar.
  - Rewrites arXiv abstract URLs (`/abs/<id>`) to their PDF form
    (`/pdf/<id>.pdf`) so the arXiv connector works without changing Phase 04.
  - `PDF_SERVICE_URL` defaults to `http://localhost:8001` and
    `PDF_SERVICE_TIMEOUT_SECS` to 180 (a full 769-page OpenStax book takes
    about 45–55 s on first access); Compose overrides the URL to
    `http://pdf-service:8001`.
- Migration `backend/migrations/0007_pdf_translation.sql` adds:
  - `pdf_extractions` — unique `resource_url`, engine, page count, `blocks`
    JSONB and Markdown. Shared across users: extraction runs once per resource.
  - `pdf_translations` — unique `(resource_url, target_language)`, source
    language, provider, translated `blocks` JSONB and Markdown.
- Endpoints:
  - `GET /activities/:id/content` — reflows a PDF on first access, caches the
    extraction, and returns `resource_url` for the original/raw toggle.
    `cached` is true on later views. It is not limited to `type = 'pdf'`;
    the download check is the authority, so a PDF attached to an `article`
    activity still works.
  - `GET /activities/:id/translation?language=pt-BR` — translates headings and
    paragraphs on first access and caches the text panel. Images/table/formula
    blocks are carried through unchanged. `language` defaults to the user's
    `preferred_language`. If source and target match, the original content is
    returned with `translated: false`, `provider: "none"` and no provider call.
- Translation uses the existing NIM configuration: `NIM_API_KEY`, `NIM_MODEL`
  (default `meta/llama-3.1-8b-instruct`) and `NIM_BASE_URL` (default
  `https://integrate.api.nvidia.com/v1`). Text blocks are batched
  (4,000 chars / 16 segments) and sent as one OpenAI-compatible chat request
  per batch. With no key, an identity mock keeps the cache path testable and
  logs that real translation is disabled.
- Cache writes use `ON CONFLICT ... DO NOTHING RETURNING`; a concurrent
  first-access request that loses the insert race reads the winner's row, so
  both callers get a consistent response.
- Verified end-to-end with the Compose sidecar and local Postgres:
  - Real OpenStax Calculus Volume 1 (769 pages, 52 MB) extracted in 52 s via
    the backend on first access (13,495 blocks: 11,163 paragraphs, 850
    headings, 801 table images, 680 images, 1 formula image); the second view
    returned `cached: true` in 3.3 s.
  - Real arXiv `1706.03762` (15 pages) used the academic engine and produced
    200 blocks with headings, table images and formula images in 3.7 s.
  - A generated normal PDF produced heading/paragraph/table/formula blocks in
    Markdown; a synthetic academic sample read left before right; the first
    translation stored a `pt-br` row and the second returned `cached: true`; a
    second user sharing the same resource URL hit the shared extraction cache;
    and an `en → en` request skipped translation entirely.
  - Rust tests (58 total) and the sidecar's stdlib `unittest` suite (13 tests)
    pass; CI runs both.
