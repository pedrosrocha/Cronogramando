# Phase 06 — Deadline Reconciliation

## Goal
Detect drift on book completion; enforce LLM compression floor.

## Tasks
- [x] `POST /books/:id/complete` → mark completed, unlock next book
- [x] Compare planned vs actual duration for finished book
- [x] Behind → prompt stay-on-deadline (compress) vs push-deadline
- [x] `POST /books/:id/reconcile-deadline` with `min_duration` floor check
- [x] Floor exceeded → force push + warning explaining why

## Exit criteria
- [x] On-time book unlocks next silently
- [x] Over-compression is overridden with clear warning

## Notes
- Migration `backend/migrations/0006_reconciliation.sql` adds `books.completed_at`,
  `books.reconciliation_status` (`on_time|pending|stay_on_deadline|push_deadline|forced_push`)
  and `books.reconciliation_warning`. `BookResponse` exposes all three.
- Pure logic lives in `src/reconciliation/` (`available_minutes`, `deadline_for`,
  `remaining_demand`, `compress`) with unit tests; handlers are in
  `src/routes/reconciliation.rs`.
- `actual_duration_mins` on completion is the sum of the book's step
  `estimated_duration_mins` — the study time the realized plan actually asked
  for. `drift = actual − planned`; a positive drift means the book overran.
- A completed book is `behind` when either the plan overran (`drift > 0`) or
  the remaining books' planned minutes exceed the availability left before
  `target_deadline`. On-time completion sets `reconciliation_status = 'on_time'`,
  unlocks the next book, and returns `requires_decision: false` with no prompt.
- Behind completion sets `reconciliation_status = 'pending'`, unlocks the next
  book, and returns `requires_decision: true` plus the options
  `stay_on_deadline` / `push_deadline` and a warning explaining the drift and/or
  the remaining-capacity gap. While a completed book is `pending`,
  `POST /books/:id/generate-schedule` for a later book returns 409, so the
  choice is made before the next plan consumes the compressed budget; completing
  a later book is blocked with 409 for the same reason.
- `POST /books/:id/reconcile-deadline` accepts `{"choice": "stay_on_deadline"}`
  or `{"choice": "push_deadline"}`.
  - **stay_on_deadline**: the budget is the availability left until
    `target_deadline` minus the completed book's positive drift. If the
    remaining books already fit, nothing changes. Otherwise each remaining
    book's `planned_duration_mins` is scaled proportionally and written back, so
    the next plan generation uses the compressed duration.
  - **floor check**: if proportional compression would push any remaining book
    below its `min_duration_mins`, the user's choice is overridden. The goal's
    `target_deadline` is pushed to the earliest date whose availability covers
    the remaining planned minutes, `reconciliation_status` becomes
    `forced_push`, and the response carries a warning naming the book, the
    compressed value, its floor, and the new deadline.
  - **push_deadline**: moves `target_deadline` to the earliest date that fits
    the remaining planned minutes (never earlier than the current target) and
    leaves planned durations untouched.
- Reconcile is idempotent-safe by state: `on_time` and already-reconciled books
  return 409; only `pending` (or a legacy `NULL`) completed book can be
  reconciled.
- Verified end-to-end against Postgres: an overrun book returned `behind` and
  unlocked the next book, generation was blocked with 409 until reconciliation,
  stay-on-deadline absorbed the drift without compression, a compressed
  scenario scaled remaining plans (386 → 200 + 100, both above their floors), a
  zero-availability target forced a push with a warning (target moved from
  2026-10-24 to 2026-11-09), and an on-time book unlocked the next book silently
  (`requires_decision: false`) while a redundant reconcile returned 409.
