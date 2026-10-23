# Phase 05 — Scheduler

## Goal
Greedy fit of steps into weekly availability with safe move/re-fit.

## Tasks
- [x] `schedule_slots` table (step_id, date, time_range)
- [x] Greedy allocator over availability JSON
- [x] `GET /schedule?date=`, `GET /schedule?goal_id=&from=&to=`
- [x] `PATCH /schedule/:slot_id` (move) re-fits source + destination days, never silent overflow
- [x] Highest-value scheduler unit tests

## Exit criteria
- [x] No day overflows availability after alloc or move
- [x] Move re-fit covered by tests

## Notes
- Migration `backend/migrations/0005_schedule_slots.sql`: `schedule_slots`
  (`step_id`, `slot_date`, `start_minute`, `end_minute`, timestamps,
  `UNIQUE(step_id, slot_date)`). Times are stored as minutes since midnight so
  the allocator and tests stay timezone-free; responses also expose `start_time`
  / `end_time` as `HH:MM`.
- `src/scheduler/` is pure logic. `allocate_with_busy` greedily packs ordered
  steps into the goal's `availability.days` (accepts `mon`/`monday`/`seg` etc.),
  starting at 09:00 and never using more than `minutes_per_day` on a date.
  `busy` subtracts minutes already used by completed/fixed slots.
- Allocation rules:
  - A step that fits whole on the next available day is moved there instead of
    being split across a day boundary.
  - Only a step larger than a full day is split across consecutive available
    days.
  - Empty/invalid `days` means every day (defensive default).
- `POST /books/:id/generate-schedule` now builds the plan and then allocates
  every pending step of the goal that has no slot yet. `POST /books/:id/steps`
  auto-schedules a newly created step into the remaining capacity. Completed
  steps are never auto-scheduled.
- `GET /schedule?date=` returns one day; `GET /schedule?goal_id=&from=&to=`
  returns a range (defaults: today → +30 days, max 366). Without `goal_id`,
  users see only their own goals; admins see all.
- `PATCH /schedule/:id` moves the slot's step to `{"date": "YYYY-MM-DD"}`.
  The target must be a future date and one of the goal's available weekdays,
  and cannot be filled by completed slots (409 otherwise); moving a completed
  step is 409. The step is pinned first on the target day, then every other
  pending slot from `min(source date, target date)` is re-packed, so both the
  source and destination days are re-fitted. The endpoint returns the moved
  step's new slots (a step can legitimately have more than one if it exceeds a
  day).
- Safety: the move re-fit runs in one transaction and verifies afterwards that
  no date exceeds `minutes_per_day`; if it would, the transaction rolls back and
  returns 500 instead of silently overflowing.
- Unit tests cover weekday-only placement, daily capacity, oversized-step
  splitting, whole-step preference over partial splits, busy minutes, empty
  availability, target-day pinning and available-day detection.
- Verified end-to-end against Postgres: plan generation produced 12 slots on
  Mon/Wed/Fri with no day above 60 min; moves to an unavailable weekday and to
  the past were rejected (400), a full completed day was rejected (409),
  moving a completed step was rejected (409), and a successful move re-fitted
  both days with all daily totals still ≤ 60.
