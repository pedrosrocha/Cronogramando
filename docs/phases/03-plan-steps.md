# Phase 03 — Plan / Steps / Activities

## Goal
Per-book detailed plan with editable steps (placeholder resources OK).

## Tasks
- [ ] `steps` table (book_id, order, title, activity_kind, estimated_duration, references_step_id)
- [ ] `activities` table (step_id, type, resource_url/embed_id, connector, language, completed)
- [ ] Mocked per-book generator: quick-win (5–15 min) + main blocks + spaced reviews + varied types
- [ ] `GET /books/:id/plan`, `POST /books/:id/steps`, `PATCH /steps/:id`, `DELETE /steps/:id`
- [ ] `POST /steps/:id/regenerate-resource`, `PATCH /activities/:id/complete`
- [ ] `POST /books/:id/generate-schedule`

## Exit criteria
- [ ] Plan is reorderable/editable via API
- [ ] Every day has one quick win; reviews reference earlier steps
