# Phase 09c — API, State & Phone Storage

## Goal
Make the real app talk to the backend, manage async state and keep useful data
on the device — including credentials, cached schedules and offline behaviour.

## Learn
- **HTTP and JSON:** requests, status codes, timeouts, retries, idempotency,
  DTO ↔ model mapping, manual `fromJson` vs `json_serializable`/`freezed`.
- **Environment configuration:** `--dart-define`, dev/staging/prod base URLs,
  web CORS, Android emulator host `10.0.2.2`, Android `usesCleartextTraffic`
  for local HTTP, and (when the iOS track resumes) iOS App Transport Security.
- **Authentication lifecycle:** register/login, access + refresh tokens,
  attaching `Authorization`, refreshing once on 401, logout, session expiry and
  safe error messages.
- **State management in practice:** repository layer, dependency injection,
  loading/error/empty/data states, cancellation of stale requests, optimistic
  updates and rollback.
- **Phone storage choices:**
  - `flutter_secure_storage` / Keychain / Android Keystore for tokens and
    secrets — never plain `shared_preferences`;
  - `shared_preferences` for small preferences (locale, theme, last sync);
  - SQLite (`drift` or `sqflite`) for cached goals, plans and schedule slots;
  - schema migrations, cache freshness, invalidation and “clear local data”.
- **Offline and privacy:** cached read-only data, offline banner, queued writes
  only where safe, encrypted-at-rest expectations and what must never be logged.
- **Debugging the network:** backend logs, proxy tools (Charles/mitmproxy),
  `adb reverse`, and reproducing a request outside the app for comparison.

## Build
- Move the 08b `ApiClient` behind a repository (`GoalsRepository`,
  `ScheduleRepository`, `AuthRepository`) with a fake implementation for tests.
- Implement real auth, token storage and automatic refresh. Survive app restart.
- Replace mocked Today/Calendar/Roadmap data with API data for one seeded user.
  Onboarding creates a real goal and handles long-running decomposition with a
  progress/loading state.
- Cache the schedule and roadmap; show stale/offline state and a manual retry.
- Map API errors into user-facing categories (auth, validation, offline,
  server) and log only safe diagnostic context.
- Add a “clear local data” action in settings.

## Test focus
- Unit tests: DTO parsing, error mapping, token refresh (single-flight), cache
  freshness, repository behaviour with `MockClient`/fake HTTP.
- Widget tests with a fake repository: loading, error, empty, data and offline.
- Storage tests: token abstraction uses secure storage; DB migration from v1 to
  v2; clear-data removes tokens and cache.
- Contract tests: reuse `mobile/test/live_api_test.dart`; add a scheduled smoke
  run in Phase 10a.
- Manual checks: kill/relaunch, expire the access token, turn airplane mode on
  and off, switch environments.

## Checkpoints
- [ ] Login persists across restart and refresh works without logging the user out.
- [ ] Today/calendar/roadmap render real backend data.
- [ ] Cached data is visible offline with a clear stale/offline indicator.
- [ ] No token or personal data appears in logs or crash reports.
- [ ] The fake repository lets the UI tests run without a backend.
- [ ] Unit + widget + storage tests pass locally.
