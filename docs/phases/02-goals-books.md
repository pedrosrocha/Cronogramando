# Phase 02 — Goals & Books

## Goal
Goal CRUD + one-shot LLM decomposition into books (mocked NIM for now).

## Tasks
- [x] `goals` table (user_id, title, why_statement, deadline, target_deadline, availability JSON)
- [x] `books` table (goal_id, order_index, title, description, status, planned/min/actual_duration)
- [x] `POST /goals` triggers mocked decomposition → Book 1 `unlocked`, rest `locked`
- [x] `GET /goals`, `GET /goals/:id`, `PATCH /goals/:id`, `DELETE /goals/:id`
- [x] `GET /goals/:id/books`, `GET /books/:id`

## Exit criteria
- [x] Creating a goal yields ordered books with weights/floors
- [x] Broad goal → multiple books; narrow goal → single book

## Notes
- Migration `backend/migrations/0002_goals_books.sql`. `availability` is JSONB
  (`{days: [...], minutes_per_day}`), default mon/wed/fri @ 60 min.
- `books.status` is `locked|unlocked|completed|in_progress`; book index 0 is
  created `unlocked`, the rest `locked` (unlock flow is Phase 6).
- Decomposition is behind the `GoalDecomposer` trait (`src/llm/`); the current
  `MockDecomposer` picks book count from breadth keywords + horizon
  (broad or >45 days → 3, broad and >120 days → 4, else 1) and assigns
  decreasing weights.
- Per-book durations are computed in the handler from the goal's availability
  and horizon (`capacity_minutes`); `planned` from normalized weight,
  `min_duration_mins` = 60% of planned (mock compression floor — the real NIM
  call will supply the floor per plan §10).
- Ownership: every goal/book read or write is scoped to the requesting user
  (admins may access any). Deleting a goal cascades to its books.
- Verified end-to-end: narrow goal → 1 book, broad goal → 4 ordered books
  (statuses unlocked/locked…), list/get/books/get-book/patch 200, delete 204,
  subsequent get 404, past deadline 400, unauthenticated 401.

## Follow-up

Book decomposition is redefined as measurable module objectives in
[`02b-objective-decomposition.md`](02b-objective-decomposition.md). That phase
adds `goals.outcome`, `books.objective/scope/assessment`, and the new
decomposer contract.
