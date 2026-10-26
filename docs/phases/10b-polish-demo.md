# Phase 10b — Polish, Device QA & Demo

## Goal
Make the app credible: accessibility, performance, crash handling, release
hygiene and a demo that shows both the product and the QA practice behind it.

## Learn
- **Accessibility:** `Semantics`, labels/hints, focus order, text scaling,
  contrast, 44 px targets, TalkBack/VoiceOver, reduced motion.
- **Performance:** frame timings, jank from rebuilds/images, memory, network
  payload size, startup time, `flutter run --profile` and DevTools performance
  views.
- **Crash/observability:** crash reporting (Sentry/Firebase Crashlytics),
  structured logs, privacy scrubbing, version/build metadata. Keep it optional
  and documented if the demo must stay offline.
- **Release hygiene:** app name/icon/splash, package/bundle IDs, versioning,
  environment config, store listing text and screenshots (even if not published).
- **QA storytelling:** demo narrative, test evidence, bug log, flaky-test
  history, coverage summary and what you would do next.
- **Design wrap-up:** apply the design direction that has emerged; do not chase
  a perfect visual system at the expense of a working, tested app.

## Build
- Run an accessibility pass and fix labels, order, tap targets and contrast.
- Profile the Today and activity-detail screens; remove obvious jank and
  oversized images.
- Add safe crash/error handling and a privacy-scrubbed diagnostic path.
- Add app icon/splash, versioning and a dev/staging/prod environment indicator.
- Update `README.md`, architecture diagram, setup instructions, test strategy
  and device matrix.
- Build a short demo script with curated fallback content so external APIs or
  LLM quotas cannot break the presentation.
- Record a 3–5 minute demo (screenshots or video) and capture the final
  retrospective.

## Test focus
- Full regression: backend tests, API contract, Flutter unit/widget/golden,
  emulator integration, release-build smoke.
- Accessibility checks with TalkBack and/or VoiceOver; text scaling at 200%.
- Offline, expired session, denied permission and low-quota scenarios.
- Run the demo twice: once online and once with the curated fallback path.
- Collect final evidence: test report, coverage, bug log, device list and a
  short “what I learned about mobile testing” note.

## Checkpoints
- [ ] Accessibility and performance checklists completed and documented.
- [ ] Release build installs and passes the smoke checklist.
- [ ] README, demo script and architecture diagram are current.
- [ ] Demo runs end to end in under five minutes.
- [ ] Final retrospective answers: what I built, what I tested, what I would
      change in the architecture or test strategy, and what I would learn next.
