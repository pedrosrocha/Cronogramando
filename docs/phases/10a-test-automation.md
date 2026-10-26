# Phase 10a — API + Mobile Test Automation

## Goal
Build a test pyramid for the whole system and run it in CI: backend unit tests,
API contract checks, Flutter unit/widget tests and real integration flows on an
emulator.

## Learn
- **Test pyramid for mobile:** what belongs in unit, widget, integration/E2E,
  contract, visual and manual exploratory testing; what *not* to automate.
- **Flutter testing tools:** `flutter_test`, `integration_test`, `patrol` for
  native interactions, built-in golden tests, `mocktail`/`mockito`,
  `MockClient`, fakes and dependency injection. Add `golden_toolkit` or
  `alchemist` only if built-in goldens are not enough.
- **Stable selectors:** keys over text, semantics labels, page objects/helpers,
  deterministic test data, time/clock injection and avoiding arbitrary sleeps.
- **API automation:** existing `cargo test`, OpenAPI contract validation
  (e.g. Schemathesis), the Phase 08b live client test, Postman/Bruno collections
  for exploratory work, seed/cleanup scripts, and contract-drift detection.
- **Device automation:** Android emulator in CI
  (`reactivecircus/android-emulator-runner` + KVM), `flutter test integration_test`,
  `adb`, iOS simulator via `xcrun simctl` on macOS runners, and hosted device
  farms (Firebase Test Lab, BrowserStack, Sauce Labs) when local emulators are
  insufficient.
- **CI design:** path filters, dependency caching, JUnit/HTML reports,
  coverage, artifacts (screenshots, logs, APKs), timeouts, retries only for
  known-flaky infrastructure, and a quarantine policy.
- **Test data and environments:** disposable users/goals, seeded content,
  isolated databases, no production secrets in tests, safe fallback content for
  demo reliability.

## Build
- Add the Flutter suite:
  - unit tests for repositories, DTOs, token refresh, cache and formatting;
  - widget tests for navigation, states, localization and key components;
  - golden tests for the activity card, why banner and empty/error screens;
  - integration tests for auth → create goal → schedule → complete activity and
    offline/restart behaviour.
- Add `mobile/test/` helpers: fake repositories, a test app wrapper, page
  objects and a clock/time abstraction.
- Add API automation:
  - keep `mobile/test/live_api_test.dart` as the contract smoke;
  - add OpenAPI validation against the running backend;
  - extend backend `cargo test`/`cargo clippy` for scheduler, classifier and
    reconciliation edge cases;
  - document how to seed and clean the validation database.
- Update CI:
  - mobile: `flutter analyze`, `flutter test`, `flutter build web`, integration
    test on an Android emulator;
  - backend: `cargo test --locked`, `cargo clippy`, migrations against a
    disposable Postgres;
  - PDF: Python unit tests;
  - live smoke: scheduled/manual job with secrets, never on every PR.
- Write `docs/mobile-test-strategy.md`: scope, levels, environments, test data,
  device matrix, flake policy and ownership.
- Keep a short bug log: at least one defect found by automation, with the
  failing test and the fix.

## Test focus
- Verify tests fail for the right reason (mutation/negative checks), not just
  that they are green.
- Measure coverage but do not chase a number; cover risky behaviour and critical
  flows first.
- Run the suite from a clean checkout and a clean database.
- Confirm screenshots/logs are attached when a CI test fails.
- Do one manual exploratory session and compare it with the automated coverage.

## Checkpoints
- [ ] `flutter analyze` + unit/widget/golden tests pass in CI.
- [ ] One integration flow passes on an Android emulator in CI (or a documented
      hosted-device equivalent).
- [ ] Backend unit tests and API contract checks pass in CI.
- [ ] Live API smoke is scheduled and its failures are visible.
- [ ] Test strategy, device matrix and flake policy are documented.
- [ ] One real bug was found by automation and fixed.
