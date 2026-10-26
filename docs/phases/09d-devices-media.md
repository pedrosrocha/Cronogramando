# Phase 09d — Android, Emulators, Media & Release Builds

## Goal
Run the real app on Android, handle media and app lifecycle differences, and
produce an installable release build. Learn where “works in Chrome” stops being
enough. **Android is the release target for this phase; iOS is parked.**

## Learn
- **Android toolchain:** Android Studio, SDK Manager, AVD Manager,
  `flutter emulators`, `adb devices`, emulator networking, `adb reverse`,
  permissions, Gradle, debug vs release signing, APK vs AAB.
- **Android emulator vs device:** hardware acceleration, cold boot, snapshots,
  API levels, screen sizes, rotation, developer options and wireless debugging.
- **Platform differences:** plugin registration, platform channels, app
  lifecycle (`resumed`/`paused`/`detached`), deep links, back button, status bar
  and network security config.
- **Media rendering:** YouTube via a native player or IFrame/WebView with a
  browser fallback; PDF via a native viewer or reflowed Markdown; file access,
  offline downloads and graceful failure when a plugin is unsupported.
- **Build/versioning:** `pubspec` version, Android `versionCode`/`versionName`,
  app icons, splash screen, environment banners and release logging.
- **Device QA basics:** cold start, rotation, background/foreground, low battery,
  denied permissions, no network, small screen and accessibility settings.
- **iOS is parked:** keep the code platform-neutral and record what would be
  needed later (Xcode, CocoaPods, signing, ATS). Revisit after Android is stable.

## Build
- Run the app on an Android emulator and, when available, a physical Android
  device via USB or wireless debugging. Record device/OS versions in
  `docs/mobile-device-matrix.md`.
- Implement the activity media for Android:
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
- [ ] Debug and release APK/AAB build, install and pass the smoke checklist.
- [ ] Video and PDF have a working Android path and a web fallback.
- [ ] One integration test runs on the Android emulator.
- [ ] `docs/mobile-device-matrix.md` records tested Android devices/API levels.
- [ ] iOS is explicitly parked with the future work documented (not a blocker).
