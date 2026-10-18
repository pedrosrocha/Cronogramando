# Phase 03 — Plan / Steps / Activities

## Goal
Per-book detailed plan with editable steps (placeholder resources OK).

## Tasks
- [x] `steps` table (book_id, order, title, activity_kind, estimated_duration, references_step_id)
- [x] `activities` table (step_id, type, resource_url/embed_id, connector, language, completed)
- [x] Mocked per-book generator: quick-win (5–15 min) + main blocks + spaced reviews + varied types
- [x] `GET /books/:id/plan`, `POST /books/:id/steps`, `PATCH /steps/:id`, `DELETE /steps/:id`
- [x] `POST /steps/:id/regenerate-resource`, `PATCH /activities/:id/complete`
- [x] `POST /books/:id/generate-schedule`

## Exit criteria
- [x] Plan is reorderable/editable via API
- [x] Every day has one quick win; reviews reference earlier steps

## Notes
- Migration `backend/migrations/0003_steps_activities.sql`. `steps` has a
  self-FK `references_step_id` (ON DELETE SET NULL) and `UNIQUE(book_id, order_index)`;
  `activities` FK to steps (cascade). `activities.type` is one of
  video|podcast|pdf|article|quiz|task.
- Plan generation is behind the `BookPlanner` trait (`src/llm/plan.rs`,
  `MockBookPlanner`). It produces 3–12 steps from the book's planned duration;
  every 4th step is a 10-min quick win, every 3rd (non-quick-win) is a spaced
  review whose `references_step_id` points at an earlier step, and main blocks
  alternate resource/task with varied activity types. All resources are
  placeholders (`source_connector = "placeholder"`) until Phase 4.
- `POST /books/:id/generate-schedule` generates the plan (409 if the book
  already has steps) and flips the book to `in_progress`. Calendar slot
  allocation itself is Phase 5 — despite the route name, this only builds the
  book's step/activity plan for now.
- Reorder/delete renumber `order_index` contiguously via a two-phase update
  (negative offset) to respect the unique constraint.
- Ownership is enforced by resolving the step/activity up to its book and then
  its goal.
- Verified end-to-end: generate 201 (12 steps, first is a 10-min quick win,
  3 review steps with valid back-references, contiguous orders, book
  in_progress), re-generate 409, get plan 200, create step 201, patch
  reorder→0 200, regenerate resource 200 (video→pdf), complete activity 200,
  delete 204 + renumbered, bad activity_kind 400.
