# Phase 06 — Deadline Reconciliation

## Goal
Detect drift on book completion; enforce LLM compression floor.

## Tasks
- [ ] `POST /books/:id/complete` → mark completed, unlock next book
- [ ] Compare planned vs actual duration for finished book
- [ ] Behind → prompt stay-on-deadline (compress) vs push-deadline
- [ ] `POST /books/:id/reconcile-deadline` with `min_duration` floor check
- [ ] Floor exceeded → force push + warning explaining why

## Exit criteria
- [ ] On-time book unlocks next silently
- [ ] Over-compression is overridden with clear warning
