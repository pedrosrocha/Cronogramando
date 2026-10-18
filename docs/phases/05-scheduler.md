# Phase 05 — Scheduler

## Goal
Greedy fit of steps into weekly availability with safe move/re-fit.

## Tasks
- [ ] `schedule_slots` table (step_id, date, time_range)
- [ ] Greedy allocator over availability JSON
- [ ] `GET /schedule?date=`, `GET /schedule?goal_id=&from=&to=`
- [ ] `PATCH /schedule/:slot_id` (move) re-fits source + destination days, never silent overflow
- [ ] Highest-value scheduler unit tests

## Exit criteria
- [ ] No day overflows availability after alloc or move
- [ ] Move re-fit covered by tests
