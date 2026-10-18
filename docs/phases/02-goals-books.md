# Phase 02 — Goals & Books

## Goal
Goal CRUD + one-shot LLM decomposition into books (mocked NIM for now).

## Tasks
- [ ] `goals` table (user_id, title, why_statement, deadline, target_deadline, availability JSON)
- [ ] `books` table (goal_id, order_index, title, description, status, planned/min/actual_duration)
- [ ] `POST /goals` triggers mocked decomposition → Book 1 `unlocked`, rest `locked`
- [ ] `GET /goals`, `GET /goals/:id`, `PATCH /goals/:id`, `DELETE /goals/:id`
- [ ] `GET /goals/:id/books`, `GET /books/:id`

## Exit criteria
- [ ] Creating a goal yields ordered books with weights/floors
- [ ] Broad goal → multiple books; narrow goal → single book
