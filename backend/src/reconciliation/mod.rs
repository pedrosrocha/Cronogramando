use chrono::NaiveDate;
use uuid::Uuid;

use crate::{models::goal::Availability, scheduler};

const MAX_HORIZON_DAYS: i64 = 365 * 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileChoice {
    StayOnDeadline,
    PushDeadline,
}

impl ReconcileChoice {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "stay_on_deadline" => Some(Self::StayOnDeadline),
            "push_deadline" => Some(Self::PushDeadline),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::StayOnDeadline => "stay_on_deadline",
            Self::PushDeadline => "push_deadline",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemainingBook {
    pub id: Uuid,
    pub title: String,
    pub planned_duration_mins: i32,
    pub min_duration_mins: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressedBook {
    pub book_id: Uuid,
    pub planned_duration_mins: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompressionOutcome {
    /// The remaining books already fit in the time left before the deadline.
    Fits,
    Compressed(Vec<CompressedBook>),
    FloorExceeded {
        book_id: Uuid,
        book_title: String,
        compressed_mins: i32,
        floor_mins: i32,
    },
}

pub fn remaining_demand(books: &[RemainingBook]) -> i32 {
    books
        .iter()
        .map(|book| book.planned_duration_mins.max(0) as i64)
        .sum::<i64>()
        .clamp(0, i32::MAX as i64) as i32
}

/// Study minutes available between `from` and `to`, inclusive, for the goal's
/// weekly availability. Empty days means every day, matching the scheduler.
pub fn available_minutes(availability: &Availability, from: NaiveDate, to: NaiveDate) -> i32 {
    if to < from {
        return 0;
    }

    let per_day = availability.minutes_per_day as i64;
    let mut total: i64 = 0;
    let mut day = from;
    for _ in 0..MAX_HORIZON_DAYS {
        if scheduler::is_available_day(availability, day) {
            total = total.saturating_add(per_day);
        }
        if day == to {
            break;
        }
        match day.succ_opt() {
            Some(next) => day = next,
            None => break,
        }
    }
    total.clamp(0, i32::MAX as i64) as i32
}

/// Earliest date on or after `from` whose accumulated availability covers
/// `demand` minutes. `from` itself counts when it is an available day.
pub fn deadline_for(availability: &Availability, from: NaiveDate, demand: i32) -> NaiveDate {
    if demand <= 0 {
        return from;
    }

    let per_day = availability.minutes_per_day.max(1) as i64;
    let mut total: i64 = 0;
    let mut day = from;
    for _ in 0..MAX_HORIZON_DAYS {
        if scheduler::is_available_day(availability, day) {
            total += per_day;
            if total >= demand as i64 {
                return day;
            }
        }
        match day.succ_opt() {
            Some(next) => day = next,
            None => return day,
        }
    }
    day
}

/// Proportionally shrink the remaining books to fit `available` minutes.
/// Any book that would fall below its compression floor aborts the whole
/// compression so the caller can force a deadline push instead.
pub fn compress(books: &[RemainingBook], available: i32) -> CompressionOutcome {
    let demand = remaining_demand(books);
    if demand == 0 || available as i64 >= demand as i64 {
        return CompressionOutcome::Fits;
    }

    let ratio = available.max(0) as f64 / demand as f64;
    let mut compressed = Vec::with_capacity(books.len());

    for book in books {
        let planned = book.planned_duration_mins.max(1);
        let scaled = ((planned as f64) * ratio).round() as i32;
        let scaled = scaled.max(1);

        if scaled < book.min_duration_mins {
            return CompressionOutcome::FloorExceeded {
                book_id: book.id,
                book_title: book.title.clone(),
                compressed_mins: scaled,
                floor_mins: book.min_duration_mins,
            };
        }

        compressed.push(CompressedBook {
            book_id: book.id,
            planned_duration_mins: scaled,
        });
    }

    CompressionOutcome::Compressed(compressed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn availability(days: &[&str], minutes: u32) -> Availability {
        Availability {
            days: days.iter().map(|day| day.to_string()).collect(),
            minutes_per_day: minutes,
        }
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn book(planned: i32, min: i32) -> RemainingBook {
        RemainingBook {
            id: Uuid::from_u128(planned as u128),
            title: format!("Book {planned}"),
            planned_duration_mins: planned,
            min_duration_mins: min,
        }
    }

    #[test]
    fn counts_only_available_weekdays() {
        let avail = availability(&["mon", "wed", "fri"], 60);
        // Mon 2026-01-05 through Sun 2026-01-11 has 3 available days.
        assert_eq!(
            available_minutes(&avail, date(2026, 1, 5), date(2026, 1, 11)),
            180
        );
        // Reversed ranges are empty, not negative.
        assert_eq!(
            available_minutes(&avail, date(2026, 1, 11), date(2026, 1, 5)),
            0
        );
    }

    #[test]
    fn empty_days_means_every_day() {
        let avail = availability(&[], 30);
        assert_eq!(
            available_minutes(&avail, date(2026, 1, 5), date(2026, 1, 7)),
            90
        );
    }

    #[test]
    fn finds_earliest_deadline_covering_demand() {
        let avail = availability(&["mon", "wed", "fri"], 60);
        // 120 minutes fits on Monday + Wednesday.
        assert_eq!(
            deadline_for(&avail, date(2026, 1, 5), 120),
            date(2026, 1, 7)
        );
        // 180 needs the Friday too.
        assert_eq!(
            deadline_for(&avail, date(2026, 1, 5), 180),
            date(2026, 1, 9)
        );
        assert_eq!(deadline_for(&avail, date(2026, 1, 5), 0), date(2026, 1, 5));
    }

    #[test]
    fn no_compression_when_remaining_books_fit() {
        let books = vec![book(120, 60), book(60, 30)];
        assert_eq!(compress(&books, 180), CompressionOutcome::Fits);
        assert_eq!(compress(&books, 500), CompressionOutcome::Fits);
    }

    #[test]
    fn compresses_proportionally_above_the_floor() {
        let books = vec![book(300, 100), book(200, 100)];
        match compress(&books, 250) {
            CompressionOutcome::Compressed(list) => {
                assert_eq!(
                    list,
                    vec![
                        CompressedBook {
                            book_id: Uuid::from_u128(300),
                            planned_duration_mins: 150,
                        },
                        CompressedBook {
                            book_id: Uuid::from_u128(200),
                            planned_duration_mins: 100,
                        },
                    ]
                );
            }
            other => panic!("expected compression, got {other:?}"),
        }
    }

    #[test]
    fn reports_the_first_book_below_its_floor() {
        let books = vec![book(400, 100), book(100, 90)];
        match compress(&books, 100) {
            CompressionOutcome::FloorExceeded {
                book_id,
                compressed_mins,
                floor_mins,
                ..
            } => {
                assert_eq!(book_id, Uuid::from_u128(400));
                assert!(compressed_mins < floor_mins);
                assert_eq!(floor_mins, 100);
            }
            other => panic!("expected a floor breach, got {other:?}"),
        }
    }
}
