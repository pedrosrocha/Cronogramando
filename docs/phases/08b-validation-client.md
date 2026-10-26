# Phase 08b — Validation Client

## Goal
Plain Flutter forms hitting every endpoint to catch integration issues early.

## Tasks
- [x] Unstyled forms for auth, goals, books, plan, schedule, complete/reconcile
- [x] Expose all routes in §14 of the plan through the Flutter HTTP client
- [x] Exercise every route against a live backend from the client
- [x] Log integration gaps back into phase docs

## Implementation notes
- Entry point: `mobile/lib/main.dart` — a drawer selects one form per route
  group.
- `mobile/lib/api_client.dart` wraps every live route (including the routes
  added after §14 was first written: redecompose, module patch, lesson,
  feedback, trace, PDF content and translation).
- On Flutter web the base URL is derived from the page host (port `8000`); on
  other platforms it defaults to `http://localhost:8000` and can be overridden
  in the Auth form or with
  `--dart-define=API_BASE_URL=http://10.0.2.2:8080`.
- The Raw Console form can hit any method/path, including `/health` (which
  returns `text/plain`, not JSON).
- The Phase 08a prototype lives in `mobile/lib/prototype/` and is reachable
  from the `8a Prototype` drawer item.
- `mobile/web/` is committed so Flutter web works. Native platform folders are
  not committed; run `flutter create --platforms=android,ios .` in `/mobile`
  before validating on an emulator/device.
- Run the UI: `cd mobile && flutter pub get && flutter run -d web-server
  --web-hostname 0.0.0.0 --web-port 8090` (or `-d chrome`).
- Run the live contract test (sources `.env` for admin credentials):
  `mobile/tool/run_live_validation.sh`. The test itself is
  `mobile/test/live_api_test.dart`; it is skipped when `LIVE_API_BASE_URL` or
  credentials are missing, so plain `flutter test` stays offline.

## Live validation run — 2026-10-26
Command: `mobile/tool/run_live_validation.sh` → `All tests passed!`
(33 route/method pairs exercised through `ApiClient`).

Covered:
- Health: `GET /health`
- Auth: `POST /auth/register`, `POST /auth/login`, `POST /auth/refresh`,
  `GET /auth/me`
- Admin: `GET /admin/users`, `PATCH /admin/users/{id}`
- Goals: `POST /goals`, `GET /goals`, `GET /goals/{id}`, `PATCH /goals/{id}`,
  `DELETE /goals/{id}`, `POST /goals/{id}/redecompose`,
  `GET /goals/{id}/books`
- Modules: `GET /books/{id}`, `PATCH /books/{id}`,
  `POST /books/{id}/generate-schedule`, `GET /books/{id}/plan`,
  `POST /books/{id}/complete`, `POST /books/{id}/reconcile-deadline`
- Steps/activities: `POST /books/{id}/steps`, `PATCH /steps/{id}`,
  `DELETE /steps/{id}`, `POST /steps/{id}/regenerate-resource`,
  `POST /steps/{id}/generate-lesson`, `GET /activities/{id}`,
  `PATCH /activities/{id}/complete`, `POST /activities/{id}/feedback`,
  `GET /activities/{id}/trace`, `GET /activities/{id}/content`,
  `GET /activities/{id}/translation`
- Schedule: `GET /schedule`, `PATCH /schedule/{slot_id}`

The test creates its own user and goal, walks auth → decomposition → plan →
schedule → completion/reconciliation → delete, and cleans the goal up in
`finally`. It exercises success and guard paths (400 for non-PDF content, 500
for an unavailable LLM tutor, and either a 200 reconciliation or its 409 guard).
The recorded run forced the behind-schedule branch and reconciled successfully.

## Integration gaps / notes
1. **LLM tutor availability** — `POST /steps/{id}/generate-lesson` returned
   HTTP 500 during validation because every configured OpenRouter free model
   was rate-limited or returned an empty completion. The route and Flutter
   client are wired correctly; the backend has no deterministic lesson fallback
   (unlike decomposition/planning). This is a provider-capacity gap, not a
   client gap.
2. **Health is plain text** — `/health` returns `ok` with `text/plain`, so it is
   only usable through the Raw Console; documented instead of adding a typed
   method.
3. **Move-slot target days** — `PATCH /schedule/{slot_id}` rejects dates outside
   the goal's availability. The validation client lets any date be typed; in
   Phase 09b the real calendar must constrain the target date to available
   weekdays.
4. **PDF routes need a resource URL** — `GET /activities/{id}/content` and
   `/translation` return 400 for task/lesson activities without a URL. The
   forms require an activity ID from a real PDF resource; the live test uses a
   task probe to assert the intended 400.
5. **§14 drift** — the planned route table was missing routes added by phases
   2b/4b/7. §14 of `docs/learning-scheduler-app-plan.md` has been updated so it
   matches the running backend.
6. Each live run registers a throwaway `phase8-*@example.com` user because
   there is no user-delete route; goals are cleaned up by the test. Use a
   disposable database for repeated validation runs.

## Exit criteria
- [x] Every endpoint round-trips from a real mobile HTTP client
- [x] Backend declared "done enough" only after this passes (hard gate for Phase 09)
