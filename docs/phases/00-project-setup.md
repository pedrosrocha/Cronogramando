# Phase 00 — Project Setup

## Goal
Monorepo skeleton + Compose skeleton + CI skeleton. Nothing functional yet.

## Tasks
- [ ] Create `/backend`, `/pdf-service`, `/mobile`, `/infra`, `/docs/phases`
- [ ] Root `docker-compose.yml` with `postgres`, `backend`, `pdf-service` services (placeholders OK)
- [ ] `.env.example`, `.gitignore`, `README.md` at root
- [ ] CI skeleton (`.github/workflows/ci.yml`) — `compose config` + placeholder jobs
- [ ] READMEs per service (how to run)

## Exit criteria
- [ ] `docker compose config` passes
- [ ] CI runs green on empty skeleton
