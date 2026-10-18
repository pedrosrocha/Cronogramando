# Phase 04 — Content Resolver + Classifier

## Goal
Route each step to the right free connectors; YouTube + Wikipedia as universal fallback.

## Tasks
- [ ] Closed-label source-category classifier (Manual, STEM, Business, Humanities, Arts, Health)
- [ ] Connectors: YouTube + Wikipedia first, then Archive, arXiv, OpenAlex, DOAJ, Gutenberg
- [ ] Language-aware query: `pt.wikipedia.org`, YouTube `relevanceLanguage=pt` + `regionCode=BR`
- [ ] Tag English-only results with language badge (no silent switch)
- [ ] Curated fallback list for demo reliability; respect ~100 YouTube searches/day quota

## Exit criteria
- [ ] Each step resolves to ≥1 playable/ readable resource or generic task
- [ ] PT query prefers PT results where they exist
