# Phase 08a — Flutter Web Design Prototype

## Goal
Clickable design spike to lock UX before native polish. Mocked data, no backend.

## Tasks
- [x] Flutter Web scaffold (`/mobile`, pt-BR default, en fallback)
- [x] Mocked JSON: 1 goal, 2 books, ~10 steps, 1 week of slots
- [x] Screens: onboarding (goal/deadline/availability/why) → today → calendar → roadmap/books → activity detail (video/PDF-toggle/quiz/task)
- [x] Streak chip, why-reminder banner, book-complete celebration placeholder
- [x] Decision log: what to keep/change for Phase 09

## Implementation notes
- Entry point: `mobile/lib/prototype/prototype_shell.dart`, reachable from the
  `8a Prototype` drawer item in the validation client.
- `mobile/assets/mock/prototype.json` is the single mocked dataset: one goal
  (`Passar em Cálculo II`), two modules, ten steps and seven days of slots.
  Slot dates are relative (`day_offset`), so "today" always has activities.
- Screens:
  - `onboarding_page.dart` — goal, why, deadline, weekday availability and
    minutes per day.
  - `today_page.dart` — why banner, streak, daily progress, today's slots.
  - `calendar_page.dart` — one-week strip with availability and per-day list.
  - `roadmap_page.dart` — module progress, locked/unlocked states and the
    celebration preview.
  - `activity_detail_page.dart` — video placeholder, reflowed-PDF/original
    toggle, quiz feedback and task checklist.
- State is local (`PrototypeShell`): completing every activity in a module
  unlocks the next one and opens the book-complete celebration placeholder.
- Run: `cd mobile && flutter pub get && flutter run -d web-server
  --web-hostname 0.0.0.0 --web-port 8090`. `flutter build web --release` and
  `flutter test test/prototype_test.dart` both pass.
- Widget tests cover onboarding → today → video detail → calendar → roadmap →
  celebration, plus the PDF toggle and quiz feedback.

## Decision log (for Phase 09)
Keep:
- Three-tab shell (`Hoje`, `Calendário`, `Roadmap`) with activity detail pushed
  as a full page; one primary action per screen.
- Persistent why-reminder on Today and a compact streak chip (not a separate
  gamification screen).
- Mixed activity types inside one day: quick win first, then the main block.
- Module roadmap that shows locked modules and unlocks the next one only after
  the previous module is complete.
- PDF reflowed/original toggle as the human fallback for extraction errors.
- Celebration preview behind an explicit prototype-only affordance so design
  reviewers can inspect it without completing a module.

Change for later phases:
- 09c: replace the mocked JSON repository with the API-backed repository and
  move completion/streak state to the backend.
- 09b: swap the hand-rolled week strip for `table_calendar`; restrict the
  move-slot target date to the goal's available weekdays (the backend rejects
  others).
- 09d: use the real YouTube IFrame player and a maintained Markdown renderer for
  reflowed PDFs.
- 09b: add `intl`/`easy_localization` string extraction; the prototype is PT-BR
  and the scaffold already declares `en` as fallback.
- 10b: replace the celebration placeholder with animation/sound and a module
  summary.
- 09c: add explicit loading/empty/error states per screen; the prototype only
  mocks the happy path.
- 10b: check accessibility — minimum 44 px targets, semantic labels for activity
  completion, and contrast on the gradient why banner.

## Exit criteria
- [x] `flutter build web --release` succeeds and widget tests render every screen with mocks (Chrome is not installed on this host)
- [x] Design decisions recorded; Phase 09 can start from this provisional wireframe
- [ ] Final visual sign-off — **not a Phase 09 blocker**; the design will evolve while the real app is built

> Design status: provisional. The owner does not have a final design yet. Keep
> the app shell theme-driven and cheap to restyle, and record changes in
> `docs/mobile-design-log.md` as they emerge.
