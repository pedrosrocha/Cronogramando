# Cronogramando — Learning Scheduler App

Study project to learn mobile development and mobile testing, built to a
standard reviewable by senior devs / designers.

Type a learning goal + deadline (e.g. "learn Calculus II in 3 months"),
declare weekly availability, get an ordered plan of small activities
allocated onto a calendar. Main screen is "today's activities".

Full plan: `docs/learning-scheduler-app-plan.md`

## Stack

- Backend: Rust / Axum + sqlx + Postgres (`/backend`)
- PDF sidecar: Python — PyMuPDF / pdfplumber / GROBID (`/pdf-service`)
- Mobile: Flutter (`/mobile`), Flutter Web prototype first for design iteration
- LLM: NVIDIA NIM (mocked until key available)
- Infra: Docker + Docker Compose (`/infra`)

## Repo layout (planned monorepo)

```
/backend
/pdf-service
/mobile
/infra
/docs
```

## Build order

0. Project setup — 1. Auth + Swagger — 2. Goals & Books — 3. Plan/steps —
4. Content resolver + classifier — 5. Scheduler — 6. Deadline reconciliation —
7. PDF pipeline + translation — 8a. Flutter Web design prototype —
8b. Validation client — 9. Real mobile app — 10. Polish/demo

Per-phase checklists: `docs/phases/` (lean: Goal + Tasks + Exit criteria).
