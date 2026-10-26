# Phase 09 — Real Mobile App (learning path)

## Goal
Build the production Flutter app in small, testable increments while learning how
a mobile app actually works: the framework, the tools, the device, the network,
local storage and the test seams.

This phase is **not** a UI-polish sprint. The goal is a working app you can
explain, run on a device and test at every layer.

## Design status
- The Phase 08a prototype is a **provisional wireframe**, not a locked visual
  spec. There is no final-design gate.
- Design decisions are expected to change. Keep screens small, composable and
  theme-driven so a redesign is cheap.
- Record design changes in the phase docs or a short `docs/mobile-design-log.md`
  as they happen.
- Final visual polish, store assets and accessibility polish belong to Phase
  10b.

## Why sub-phases
Phase 09 is split so each session has one learning objective and one shippable
checkpoint:

| Sub-phase | Question it answers |
|---|---|
| [09a — Flutter anatomy](09a-flutter-anatomy.md) | What is a Flutter app made of, and how do I run a “hello world”? |
| [09b — App shell](09b-app-shell.md) | How do navigation, theming and i18n work? |
| [09c — API, state & storage](09c-api-state-storage.md) | How does the app talk to the API and keep data on the phone? |
| [09d — Devices, media & release](09d-devices-media.md) | How does it run on Android/iPhone, render media and ship a build? |

Phase 08b’s live client remains available as the “contract console” while the
real app grows. Keep it behind a debug menu until Phase 10a replaces it with
automated coverage.

## Exit criteria
- [ ] The real app runs on Flutter Web and an Android emulator (iOS simulator
      where macOS is available; otherwise the limitation is documented)
- [ ] Onboarding → today → calendar → roadmap → activity detail works against
      the real backend for one seeded user
- [ ] A session survives app restart and expired access tokens refresh cleanly
- [ ] Data is cached locally and the app shows useful offline/error states
- [ ] Each sub-phase has unit/widget/integration tests at the right level
- [ ] You can explain the widget tree, layout constraints, state flow, API
      layer, storage choice and device build process without hand-waving
