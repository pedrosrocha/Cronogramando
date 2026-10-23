use std::collections::HashMap;

use chrono::{Datelike, NaiveDate, Weekday};
use uuid::Uuid;

use crate::models::goal::Availability;

pub const DAY_START_MINUTE: i32 = 9 * 60;
const MAX_LOOKAHEAD_DAYS: u64 = 365 * 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepAllocation {
    pub step_id: Uuid,
    pub duration_mins: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotPlacement {
    pub step_id: Uuid,
    pub date: NaiveDate,
    pub start_minute: i32,
    pub end_minute: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledStep {
    pub step_id: Uuid,
    pub duration_mins: i32,
    pub date: NaiveDate,
    pub start_minute: i32,
}

#[cfg(test)]
pub fn allocate(
    steps: &[StepAllocation],
    availability: &Availability,
    start: NaiveDate,
) -> Vec<SlotPlacement> {
    allocate_with_busy(steps, availability, start, &HashMap::new())
}

/// Greedy pack of ordered steps into available weekdays.
///
/// `busy` maps a date to minutes already occupied (e.g. completed slots) and is
/// subtracted from that day's capacity. A step is never split when it can fit
/// whole on the next available day; only a step larger than a full day is split
/// across days. No day is ever allocated beyond `minutes_per_day`.
pub fn allocate_with_busy(
    steps: &[StepAllocation],
    availability: &Availability,
    start: NaiveDate,
    busy: &HashMap<NaiveDate, i32>,
) -> Vec<SlotPlacement> {
    let allowed = allowed_days(availability);
    let capacity = (availability.minutes_per_day as i32).clamp(1, 1440 - DAY_START_MINUTE);
    let mut placements = Vec::new();
    let mut day = start;
    let mut day_used = used_minutes(busy, day, capacity);
    let mut guard: u64 = 0;

    for step in steps {
        let mut remaining = step.duration_mins.max(1);
        while remaining > 0 {
            guard += 1;
            if guard > MAX_LOOKAHEAD_DAYS * 64 {
                return placements;
            }

            if !is_allowed(day, &allowed) {
                match next_allowed_day(day, &allowed, busy, capacity) {
                    Some(next) => {
                        day = next;
                        day_used = used_minutes(busy, day, capacity);
                    }
                    None => return placements,
                }
                continue;
            }

            let available = capacity - day_used;
            if available <= 0 {
                match next_allowed_day(day, &allowed, busy, capacity) {
                    Some(next) => {
                        day = next;
                        day_used = used_minutes(busy, day, capacity);
                    }
                    None => return placements,
                }
                continue;
            }

            // Prefer a whole placement on the next day; only split when the
            // step cannot fit in a single day at all.
            if remaining > available && remaining <= capacity {
                match next_allowed_day(day, &allowed, busy, capacity) {
                    Some(next) => {
                        day = next;
                        day_used = used_minutes(busy, day, capacity);
                    }
                    None => return placements,
                }
                continue;
            }

            let take = remaining.min(available);
            let start_minute = DAY_START_MINUTE + day_used;
            let end_minute = start_minute + take;
            placements.push(SlotPlacement {
                step_id: step.step_id,
                date: day,
                start_minute,
                end_minute,
            });
            day_used += take;
            remaining -= take;

            if remaining > 0 {
                match next_allowed_day(day, &allowed, busy, capacity) {
                    Some(next) => {
                        day = next;
                        day_used = used_minutes(busy, day, capacity);
                    }
                    None => return placements,
                }
            }
        }
    }

    placements
}

/// Stable ordering by scheduled time, preserving relative order for ties.
pub fn sort_by_time(mut steps: Vec<ScheduledStep>) -> Vec<ScheduledStep> {
    steps.sort_by_key(|step| (step.date, step.start_minute));
    steps
}

pub fn is_available_day(availability: &Availability, date: NaiveDate) -> bool {
    is_allowed(date, &allowed_days(availability))
}

#[cfg(test)]
pub fn total_minutes_by_date(placements: &[SlotPlacement]) -> HashMap<NaiveDate, i32> {
    let mut totals = HashMap::new();
    for placement in placements {
        *totals.entry(placement.date).or_insert(0) += placement.end_minute - placement.start_minute;
    }
    totals
}

fn is_allowed(day: NaiveDate, allowed: &[Weekday]) -> bool {
    allowed.is_empty() || allowed.contains(&day.weekday())
}

fn used_minutes(busy: &HashMap<NaiveDate, i32>, day: NaiveDate, capacity: i32) -> i32 {
    busy.get(&day).copied().unwrap_or(0).clamp(0, capacity)
}

fn next_allowed_day(
    from: NaiveDate,
    allowed: &[Weekday],
    busy: &HashMap<NaiveDate, i32>,
    capacity: i32,
) -> Option<NaiveDate> {
    let mut candidate = from;
    for _ in 0..MAX_LOOKAHEAD_DAYS {
        candidate = candidate.succ_opt()?;
        if is_allowed(candidate, allowed) {
            let used = used_minutes(busy, candidate, capacity);
            if used < capacity {
                return Some(candidate);
            }
        }
    }
    None
}

fn allowed_days(availability: &Availability) -> Vec<Weekday> {
    availability
        .days
        .iter()
        .filter_map(|day| parse_day(day))
        .collect()
}

fn parse_day(value: &str) -> Option<Weekday> {
    match value.trim().to_lowercase().as_str() {
        "mon" | "monday" | "seg" | "segunda" => Some(Weekday::Mon),
        "tue" | "tues" | "tuesday" | "ter" | "terca" => Some(Weekday::Tue),
        "wed" | "wednesday" | "qua" | "quarta" => Some(Weekday::Wed),
        "thu" | "thur" | "thurs" | "thursday" | "qui" | "quinta" => Some(Weekday::Thu),
        "fri" | "friday" | "sex" | "sexta" => Some(Weekday::Fri),
        "sat" | "saturday" | "sab" | "sabado" => Some(Weekday::Sat),
        "sun" | "sunday" | "dom" | "domingo" => Some(Weekday::Sun),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dates(placements: &[SlotPlacement]) -> Vec<NaiveDate> {
        placements.iter().map(|slot| slot.date).collect()
    }

    fn availability(days: &[&str], minutes: u32) -> Availability {
        Availability {
            days: days.iter().map(|day| day.to_string()).collect(),
            minutes_per_day: minutes,
        }
    }

    fn step(day: u32) -> StepAllocation {
        StepAllocation {
            step_id: Uuid::from_u128(day as u128),
            duration_mins: 60,
        }
    }

    #[test]
    fn fills_only_configured_weekdays() {
        let start = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(); // Monday
        let slots = allocate(
            &[step(1), step(2), step(3)],
            &availability(&["mon", "wed", "fri"], 60),
            start,
        );
        assert_eq!(
            dates(&slots),
            vec![
                NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
                NaiveDate::from_ymd_opt(2026, 1, 7).unwrap(),
                NaiveDate::from_ymd_opt(2026, 1, 9).unwrap(),
            ]
        );
        assert!(slots
            .iter()
            .all(|slot| slot.start_minute == DAY_START_MINUTE
                && slot.end_minute == DAY_START_MINUTE + 60));
    }

    #[test]
    fn never_exceeds_daily_capacity() {
        let start = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        let steps = vec![
            StepAllocation {
                step_id: Uuid::from_u128(1),
                duration_mins: 40,
            },
            StepAllocation {
                step_id: Uuid::from_u128(2),
                duration_mins: 40,
            },
            StepAllocation {
                step_id: Uuid::from_u128(3),
                duration_mins: 40,
            },
        ];
        let slots = allocate(&steps, &availability(&["mon", "tue", "wed"], 60), start);
        for (date, minutes) in total_minutes_by_date(&slots) {
            assert!(minutes <= 60, "{date} overflowed with {minutes} minutes");
        }
    }

    #[test]
    fn splits_only_steps_larger_than_a_day() {
        let start = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        let steps = vec![StepAllocation {
            step_id: Uuid::from_u128(1),
            duration_mins: 90,
        }];
        let slots = allocate(&steps, &availability(&["mon", "tue"], 60), start);
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].date, start);
        assert_eq!(slots[0].end_minute - slots[0].start_minute, 60);
        assert_eq!(slots[1].date, NaiveDate::from_ymd_opt(2026, 1, 6).unwrap());
        assert_eq!(slots[1].end_minute - slots[1].start_minute, 30);
    }

    #[test]
    fn avoids_partial_day_when_next_day_fits_whole_step() {
        let start = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        let steps = vec![
            StepAllocation {
                step_id: Uuid::from_u128(1),
                duration_mins: 50,
            },
            StepAllocation {
                step_id: Uuid::from_u128(2),
                duration_mins: 30,
            },
        ];
        let slots = allocate(&steps, &availability(&["mon", "tue"], 60), start);
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[1].date, NaiveDate::from_ymd_opt(2026, 1, 6).unwrap());
        assert_eq!(slots[1].end_minute - slots[1].start_minute, 30);
    }

    #[test]
    fn accounts_for_busy_minutes() {
        let start = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        let mut busy = HashMap::new();
        busy.insert(start, 30);
        let steps = vec![StepAllocation {
            step_id: Uuid::from_u128(1),
            duration_mins: 40,
        }];
        let slots = allocate_with_busy(&steps, &availability(&["mon", "tue"], 60), start, &busy);
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].date, NaiveDate::from_ymd_opt(2026, 1, 6).unwrap());
    }

    #[test]
    fn empty_availability_means_every_day() {
        let start = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        let slots = allocate(&[step(1), step(2)], &availability(&[], 60), start);
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].date, start);
        assert_eq!(slots[1].date, NaiveDate::from_ymd_opt(2026, 1, 6).unwrap());
    }

    #[test]
    fn move_pins_step_to_target_day_even_when_source_day_came_first() {
        let target = NaiveDate::from_ymd_opt(2026, 1, 9).unwrap(); // Friday
        let pinned = StepAllocation {
            step_id: Uuid::from_u128(2),
            duration_mins: 60,
        };
        let slots = allocate(&[pinned], &availability(&["mon", "wed", "fri"], 60), target);
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].date, target);
        assert_eq!(slots[0].start_minute, DAY_START_MINUTE);
    }

    #[test]
    fn detects_available_days() {
        let monday = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        let tuesday = NaiveDate::from_ymd_opt(2026, 1, 6).unwrap();
        assert!(is_available_day(
            &availability(&["mon", "wed", "fri"], 60),
            monday
        ));
        assert!(!is_available_day(
            &availability(&["mon", "wed", "fri"], 60),
            tuesday
        ));
        assert!(is_available_day(&availability(&[], 60), tuesday));
    }
}
