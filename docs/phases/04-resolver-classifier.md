# Phase 04 — Content Resolver + Classifier

## Goal
Route each step to the right free connectors; YouTube + Wikipedia as universal fallback.

## Tasks
- [x] Closed-label source-category classifier (Manual, STEM, Business, Humanities, Arts, Health)
- [x] Connectors: YouTube + Wikipedia first, then Archive, arXiv, OpenAlex, DOAJ, Gutenberg
- [x] Language-aware query: `pt.wikipedia.org`, YouTube `relevanceLanguage=pt` + `regionCode=BR`
- [x] Tag English-only results with language badge (no silent switch)
- [x] Curated fallback list for demo reliability; respect ~100 YouTube searches/day quota

## Exit criteria
- [x] Each step resolves to ≥1 playable/ readable resource or generic task
- [x] PT query prefers PT results where they exist

## Notes
- Migration `backend/migrations/0004_content_resolver.sql`: `activities.source_category`
  plus the `connector_usage` daily counter used for YouTube quota.
- `src/content/` is split into `classifier`, `types`, `fallback`, `quota`, and
  `connectors/{youtube,wikipedia,internet_archive,arxiv,openalex,doaj,gutenberg}`.
  Each connector is language-aware: `pt-*` queries hit `pt.wikipedia.org`,
  YouTube uses `relevanceLanguage=pt` + `regionCode=BR`, and every result carries
  its own `language` (`pt-BR`/`en`) so the client can show a badge instead of
  silently switching. Non-PT-only connectors (arXiv, OpenStax-style PDFs) are
  tagged `en`.
- `HttpContentResolver` orders connectors by the classifier's primary category,
  always appends Wikipedia + YouTube as universal fallbacks, dedupes by URL, and
  caps results at 12 (5 per connector).
- YouTube search costs 100 units; `connector_usage` enforces
  `YOUTUBE_DAILY_UNITS` (default 10,000 → ~100 searches/day). Missing/failed
  quota skips YouTube and falls through to the next connector.
- When every connector returns nothing (offline demo, exhausted quota), the
  curated fallback list per category is used, preferring PT entries for PT users.
  If even that is empty a generic `task` activity is produced, so every step
  always resolves to something actionable.
- `POST /books/:id/generate-schedule` and `POST /steps/:id/regenerate-resource`
  now call the resolver and record `source_connector`, `source_category` and
  `language` on the activity. Step titles are normalized (`Quick win:`,
  `Study:`, `Practice:`, `Spaced review of earlier material in`) before querying.
- Unit tests cover classifier labels, language helpers, every fallback category,
  PT preference, connector parsing, and quota-safe behavior (33 tests green).
