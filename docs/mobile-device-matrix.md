# Mobile Device & Environment Matrix

Android-first working matrix for the learning path. Update this file as real
devices, emulators and CI runners are added.

## Machines
| Machine | Role | Status |
|---|---|---|
| Windows PC | Primary Flutter/Android development, emulator, physical Android, Chrome checks | Active |
| Mac | Optional iOS track later; can also run Flutter/Android | Parked |
| Server (this host) | Backend, Postgres, PDF service, Docker Compose, CI | Active |

## Current target matrix
| Target | Environment | Status | Notes |
|---|---|---|---|
| Android emulator | Windows + Android Studio AVD | Primary | Must pass for Phase 09d |
| Physical Android | Windows + USB/wireless debugging | When available | Preferred release smoke device |
| Flutter Web | Chrome or `web-server` | Secondary | Fast UI/state checks; not a release target |
| iOS simulator/device | Mac + Xcode | Parked | Documented future track, not a Phase 09 blocker |

## Android matrix to fill in
| Device | API level | Screen | Android version | Emulator/physical | Tested |
|---|---|---|---|---|---|
| AVD — small phone | current stable | 360 × 640 dp | TBD | emulator | [ ] |
| AVD — large phone | current stable | 412 × 915 dp | TBD | emulator | [ ] |
| AVD — older supported API | one level below stable | 360 × 640 dp | TBD | emulator | [ ] |
| Physical device | TBD | TBD | TBD | physical | [ ] |

## Network / API base URL
| Client | Backend location | Base URL |
|---|---|---|
| Android emulator | Same Windows machine | `http://10.0.2.2:8000` |
| Android emulator | Remote server | `http://<server-ip>:8000` |
| Physical Android | Remote server | `http://<server-lan-ip>:8000` |
| Flutter Web | Same machine | `http://localhost:8000` |
| Flutter Web | Remote server | `http://<server-ip>:8000` |
| iOS simulator (future) | Mac local backend | `http://localhost:8000` |
| iOS device (future) | Remote server | HTTPS preferred; ATS exception for local HTTP only |

For Android `10.0.2.2` is the **host machine running the emulator**, not this
server. Use `--dart-define=API_BASE_URL=...` or the Auth form to point at the
correct backend.

## Test status legend
- `[ ]` not run
- `[x]` passed
- `[!]` known failure or blocked

## Parked iOS notes
When the iOS track resumes:
- Install Xcode and CocoaPods; run `flutter doctor`.
- Simulator works without a paid Apple Developer account.
- Physical iPhone needs signing; free provisioning expires after 7 days.
- App Transport Security blocks plain HTTP by default; use a dev-only ATS
  exception or HTTPS for the backend.
