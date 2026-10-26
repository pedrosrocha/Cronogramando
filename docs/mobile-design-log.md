# Mobile Design Log

Living record for the Flutter app design. The design is **provisional**: the
owner does not have a final visual direction yet, so the goal is to keep the app
cheap to restyle and to record decisions as they emerge.

## Current direction (from Phase 08a)
- Three-tab shell: **Hoje**, **Calendário**, **Roadmap**.
- Activity detail is a full page pushed from the list.
- Persistent “why” reminder and a compact streak chip on Today.
- Mixed activity types in one day, starting with a quick win.
- PDF reflowed/original toggle as the human fallback.
- Module roadmap with locked/unlocked progression and a celebration placeholder.

## Open questions
- Visual identity: colours, typography, illustration/photography, brand voice.
- Is onboarding one screen or a short multi-step flow?
- Calendar: week strip first, or month grid first?
- How much of the “why” and streak should be visible on small phones?
- Native media: YouTube plugin vs IFrame/WebView; native PDF viewer vs reflowed
  text only.
- Accessibility target: WCAG AA plus TalkBack/VoiceOver pass in Phase 10b.

## Decision log
| Date | Decision | Why | Revisit |
|---|---|---|---|
| 2026-10-26 | Start from 08a wireframe, no final-design gate | Owner still exploring; blocking would stall learning | 09b / 10b |
| 2026-10-26 | Keep state/theme replaceable; screens composable | Design is expected to change | 09b |
| 2026-10-26 | Three-tab shell + pushed activity detail | Keeps “today” primary and detail focused | 09b |
| 2026-10-26 | PT-BR default, English fallback | Primary audience is Brazilian | 09b |
| 2026-10-26 | Android-first target; iOS parked | Windows PC is the primary Flutter machine; Mac available later | 09d / 10a |

## How to use this log
- Add an entry when a design decision affects more than one screen.
- Link the phase and the reason, not just the visual result.
- Keep visual polish tasks in Phase 10b; keep structural decisions in 09b.
