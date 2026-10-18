# Phase 01 — Auth + Swagger Foundation

## Goal
Real JWT auth from the start, Swagger docs from the first endpoint.

## Tasks
- [x] `users` table (id, email, password_hash, role user/admin, preferred_language)
- [x] Seed admin via migration/seed script
- [x] `argon2` hashing, `jsonwebtoken` (access + refresh)
- [x] `POST /auth/register`, `POST /auth/login`, `POST /auth/refresh`, `GET /auth/me`
- [x] `GET /admin/users`, `PATCH /admin/users/:id` (role-guarded)
- [x] Wire `utoipa` + swagger-ui from first endpoint

## Exit criteria
- [x] Register → login → me → refresh round-trips
- [x] Non-admin blocked from `/admin/*`
- [x] `/swagger-ui` renders all routes

## Notes
- Migration `backend/migrations/0001_init.sql`: `users` table + case-insensitive
  unique index on `lower(email)`.
- Admin is seeded at boot from `ADMIN_EMAIL` / `ADMIN_PASSWORD` (idempotent).
- Tokens are stateless JWTs; access and refresh distinguished by a `kind`
  claim (`ACCESS_TOKEN_TTL_SECS` default 900s, `REFRESH_TOKEN_TTL_SECS`
  default 30 days).
- `AuthUser` / `AdminUser` extractors in `src/auth/extractor.rs` enforce
  bearer auth and the `admin` role.
- Verified end-to-end: register 201, login 200, me 200, refresh 200,
  non-admin `/admin/users` 403, admin `/admin/users` 200, PATCH 200,
  `/swagger-ui` 200, `/api-docs/openapi.json` lists all routes.
