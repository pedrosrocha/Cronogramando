# Phase 07 — PDF Pipeline + Translation

## Goal
Python sidecar extracts reflowable blocks; NIM translates with cache.

## Tasks
- [ ] Sidecar endpoints: extract (normal vs academic 2-col), deterministic engine pick by connector
- [ ] Unified block JSON (`heading|paragraph|image|table_image|formula_image`) → Markdown
- [ ] Tables/formulas rasterized as images; raw-PDF toggle as fallback
- [ ] Translate-on-first-access via NIM, cache in Postgres; PDFs show translated text panel
- [ ] Cache extracted JSON per resource (shared across users)

## Exit criteria
- [ ] OpenStax + arXiv samples render reflowed + toggle to original
- [ ] Second view of same translation hits cache, no re-call
