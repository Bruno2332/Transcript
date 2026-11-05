
use sqlx::types::chrono::{DateTime, NaiveDate, Utc};

#[derive(Debug)]
pub enum RecordingCommand {
    Pause,
    Resume,
    Stop,
}

#[derive(Debug, sqlx::FromRow)]
pub struct Transcript {
    pub id: Option<i64>,
    pub name: String,
    pub date_of_service: NaiveDate,
    pub transcription: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}