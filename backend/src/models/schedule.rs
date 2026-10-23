use chrono::NaiveDate;
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct ScheduleSlotRow {
    pub id: Uuid,
    pub step_id: Uuid,
    pub step_title: String,
    pub book_id: Uuid,
    pub book_title: String,
    pub goal_id: Uuid,
    pub goal_title: String,
    pub slot_date: NaiveDate,
    pub start_minute: i32,
    pub end_minute: i32,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScheduleSlotResponse {
    pub id: Uuid,
    pub step_id: Uuid,
    pub step_title: String,
    pub book_id: Uuid,
    pub book_title: String,
    pub goal_id: Uuid,
    pub goal_title: String,
    pub date: NaiveDate,
    pub start_minute: i32,
    pub end_minute: i32,
    pub start_time: String,
    pub end_time: String,
    pub completed: bool,
}

impl From<ScheduleSlotRow> for ScheduleSlotResponse {
    fn from(row: ScheduleSlotRow) -> Self {
        Self {
            id: row.id,
            step_id: row.step_id,
            step_title: row.step_title,
            book_id: row.book_id,
            book_title: row.book_title,
            goal_id: row.goal_id,
            goal_title: row.goal_title,
            date: row.slot_date,
            start_minute: row.start_minute,
            end_minute: row.end_minute,
            start_time: format_minutes(row.start_minute),
            end_time: format_minutes(row.end_minute),
            completed: row.completed,
        }
    }
}

pub fn format_minutes(minutes: i32) -> String {
    let minutes = minutes.clamp(0, 1440);
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}
