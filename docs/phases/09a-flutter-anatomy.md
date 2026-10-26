# Phase 09a — Flutter Anatomy & “Hello World”

## Goal
Understand what `flutter create` gives you, how a widget description becomes
pixels, and how to run, debug and test a tiny app before touching the scheduler
UI.

## Setup (Windows + Android-first)
- Use the Windows PC as the primary Flutter machine. Enable virtualization so
  the Android emulator runs with hardware acceleration.
- Install Git for Windows, Flutter stable, Android Studio + Android SDK +
  platform tools, and an AVD. Install Chrome only if you want browser checks;
  Android is the primary target.
- Verify the toolchain:
  ```powershell
  flutter doctor
  flutter doctor --android-licenses
  flutter emulators
  flutter emulators --launch <emulator_id>
  adb devices
  ```
- If `flutter doctor` complains about Chrome or Linux desktop, ignore those for
  now; only the Android toolchain and connected device need to be green.
- The backend is not needed for this phase. When 09c needs it, use
  `http://10.0.2.2:8000` if the backend runs on the same Windows machine, or
  the server IP if it stays remote (see `docs/mobile-device-matrix.md`).

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
- **Build targets:** what changes between a quick Chrome run, the Android
  emulator and a physical Android device (iOS is parked for later).

## Build
- Create a disposable lab entry point, not a new repo:
  `mobile/lib/labs/hello_flutter/main.dart`.
- Run it on Android first:
  `flutter run -t lib/labs/hello_flutter/main.dart` (with the emulator running),
  then optionally in Chrome with `-d chrome`, so the validation client and
  prototype are untouched.
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
  the Android emulator (Chrome is optional).
- Add a short note to `docs/mobile-design-log.md` describing what you learned
  about the toolchain and which IDE/emulator you will use.
- Add the Android emulator/device details to `docs/mobile-device-matrix.md`.
