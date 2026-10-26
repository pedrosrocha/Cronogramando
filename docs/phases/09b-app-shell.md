# Phase 09b — App Shell, Navigation & Provisional Design

## Goal
Turn the 08a wireframe into a real app shell: routing, theming, responsive
layout and pt-BR/en localization. Keep the design provisional and replaceable.

## Learn
- **Material 3:** `ThemeData`, `ColorScheme`, typography, spacing tokens,
  component themes, light/dark mode, and why a token layer makes redesigns
  cheap.
- **Navigation:** `Navigator` basics vs a router such as `go_router`; named
  routes, nested navigation, back-stack behaviour, deep links, and restoring a
  route after restart.
- **Responsive Flutter:** phone vs tablet vs web widths, `LayoutBuilder`,
  `MediaQuery`, safe areas, orientation, and keyboard insets.
- **Localization:** `flutter_localizations` + `gen_l10n`, ARB files, pt-BR as
  default, English fallback, locale switching, plurals and date/number formats.
- **State-management decision:** compare `setState`, `Provider`, `Riverpod` and
  `Bloc`; choose one based on testability and team familiarity. Write down the
  choice and the trade-off.
- **Component library:** small reusable widgets for loading, empty, error,
  activity cards and section headers; avoid one giant screen widget.
- **Design documentation:** low-fidelity wireframes, design tokens and a
  decision log are enough at this stage; pixel polish is not the goal.

## Build
- Create `mobile/lib/app/` as the real app shell. Keep the 08a prototype under
  `mobile/lib/prototype/` and the 08b forms under `mobile/lib/screens/` until
  the real screens replace them.
- Add routes for onboarding, today, calendar, roadmap, activity detail and
  settings. Add a debug menu entry for the validation client.
- Move strings into ARB files and implement a locale switch. Dates and numbers
  must use the active locale.
- Build a provisional theme (colour, type scale, spacing, card/button styles)
  and reusable state widgets.
- Port the 08a screens as provisional UI, clearly marked in the design log.
  Completing a module/unlocking the next is still mocked until 09c.
- Add a settings screen with locale, theme mode and API environment display.

## Test focus
- Widget tests for each route, back navigation and the debug-menu path.
- Locale tests: switch pt-BR ↔ en, assert a few labels and a formatted date.
- Responsive tests at 320 px, 360 px and tablet width; catch overflow.
- Golden tests for the activity card, why banner and empty/error states.
- Accessibility smoke: semantic labels, tap-target size, text scaling at
  200%.

## Checkpoints
- [ ] `flutter analyze` clean and all widget/golden tests pass.
- [ ] Every main route is reachable and back navigation is predictable.
- [ ] pt-BR is the default; English fallback works.
- [ ] No overflow at the smallest target width.
- [ ] The validation client is still reachable behind the debug menu.
- [ ] `docs/mobile-design-log.md` records the chosen state-management package
      and the provisional visual direction.
