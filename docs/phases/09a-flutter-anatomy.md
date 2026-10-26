# Phase 09a — Flutter Anatomy & “Hello World”

## Goal
Understand what `flutter create` gives you, how a widget description becomes
pixels, and how to run, debug and test a tiny app before touching the scheduler
UI.

## Learn
- **Dart you need first:** null safety, classes, collections, `Future`,
  `async`/`await`, exceptions and imports. Enough to read Flutter code, not a
  full language course.
- **Flutter mental model:** the widget → element → render-object trees,
  declarative UI, `build()`, `BuildContext`, `StatelessWidget` vs
  `StatefulWidget`, `setState`, composition over inheritance.
- **Layout rules:** “constraints go down, sizes go up, parents set position”;
  `Row`/`Column`/`Stack`/`ListView`, `Expanded`, overflow errors, keyboard
  insets and `SafeArea`.
- **Project anatomy:** `pubspec.yaml`, `lib/`, `test/`, `web/`, `android/`,
  `ios/`, `.dart_tool/`, `build/`; which folders are generated and which are
  yours to edit.
- **Toolchain:** `flutter doctor`, `flutter devices`, `flutter emulators`,
  `flutter create`, `flutter run`, `flutter analyze`, `flutter test`,
  `flutter pub`, Dart DevTools (widget inspector, layout explorer, performance
  overlay), IDE Flutter/Dart extensions.
- **Build targets:** what changes between `-d chrome`, an Android emulator and
  an iOS simulator.

## Build
- Create a disposable lab entry point, not a new repo:
  `mobile/lib/labs/hello_flutter/main.dart`.
- Run it with `flutter run -t lib/labs/hello_flutter/main.dart -d chrome`
  (and later `-d <android-emulator>`), so the validation client and prototype
  are untouched.
- Exercise hot reload and hot restart deliberately:
  - change text/colour and reload;
  - add a counter with `setState`;
  - add a second screen with `Navigator.push`;
  - add a scrollable list;
  - break a layout on purpose and read the overflow/constraint error.
- Add one widget test in `mobile/test/labs/hello_flutter_test.dart` that finds a
  text/key, taps a button and asserts the result.
- Use the widget inspector to find the same widget by text, key and semantics.

## QA focus
- Learn the three finder styles: by text, by key (`ValueKey`/`GlobalKey`) and by
  semantics/accessibility label. Prefer stable keys for testable controls.
- Read a failing widget test’s widget tree and stack trace.
- Notice what hot reload preserves and what requires a restart; make this part
  of your debugging checklist.
- Write one failing test first, then make it pass (red → green loop).

## Checkpoints
- Explain the difference between the widget, element and render trees.
- Explain why a `Column` overflows and how `Expanded`/`Flexible` fix it.
- Show a successful `flutter analyze`, `flutter test`, hot reload and a run on
  Chrome plus one emulator.
- Add a short note to `docs/mobile-design-log.md` describing what you learned
  about the toolchain and which IDE/emulator you will use.
