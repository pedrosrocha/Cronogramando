# Phase 09d — Emulators, Android/iOS, Media & Release Builds

## Goal
Run the real app on Android and iOS, handle media and app lifecycle differences,
and produce an installable release build. Learn where “works in Chrome” stops
being enough.

## Learn
- **Android toolchain:** Android Studio, SDK Manager, AVD Manager,
  `flutter emulators`, `adb devices`, emulator networking, `adb reverse`,
  permissions, Gradle, debug vs release signing, APK vs AAB.
- **iOS toolchain (macOS required):** Xcode, simulators, CocoaPods, bundle ID,
  signing/provisioning, TestFlight. If no macOS machine is available, document
  the limitation and use the web/Android paths for this phase.
- **Platform differences:** plugin registration, platform channels, app
  lifecycle (`resumed`/`paused`/`detached`), deep links, back button, status bar,
  network security config (Android) and ATS (iOS).
- **Media rendering:** YouTube via a native player or IFrame/WebView with a
  browser fallback; PDF via a native viewer or reflowed Markdown; file access,
  offline downloads and graceful failure when a plugin is unsupported.
- **Build/versioning:** `pubspec` version, Android `versionCode`/`versionName`,
  iOS build number, app icons, splash screen, environment banners and release
  logging.
- **Device QA basics:** cold start, rotation, background/foreground, low battery,
  denied permissions, no network, small screen and accessibility settings.

## Build
- Run the app on an Android emulator and a physical Android device (USB or
  wireless debugging). Record device/OS versions in the test matrix.
- Verify iOS on a simulator/device when macOS is available; otherwise complete
  the iOS setup checklist and record the blocker.
- Implement the activity media:
  - video with a native player or IFrame fallback;
  - PDF original/reflowed toggle with a native viewer fallback;
  - quiz and task interactions that survive app backgrounding.
- Add lifecycle handling for active timers/audio/video and refresh data when the
  app resumes.
- Produce a debug APK and a signed release APK/AAB; install both on a device.
- Add an app icon, splash screen and environment indicator (dev/staging).

## Test focus
- `integration_test` for one end-to-end flow on an Android emulator.
- Manual device checklist: install, launch, rotate, background, airplane mode,
  permission denial, media play/pause, back navigation.
- Smoke-test the release build, not only debug.
- Capture frame timings for scrolling and the media screen; note any jank.
- Verify deep links and app restart restore the expected route/session.

## Checkpoints
- [ ] `flutter install` or `adb install` succeeds on Android; app launches.
- [ ] iOS status is verified or explicitly documented with the blocker.
- [ ] Video and PDF have a working native path and a browser fallback.
- [ ] Release APK/AAB builds, installs and passes the smoke checklist.
- [ ] One integration test runs on the emulator.
- [ ] `docs/mobile-device-matrix.md` records tested devices, OS versions and
      known platform limitations.
