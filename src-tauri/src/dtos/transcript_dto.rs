use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, NaiveDate, Utc};
use crate::models::transcript::Transcript;

#[derive(Debug, Serialize, Deserialize)]
pub struct TranscriptDto {
    pub id: Option<i64>,
    pub name: String,
    pub date_of_service: NaiveDate,
    pub transcription: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Transcript> for TranscriptDto {
    fn from(t: Transcript) -> Self {
        Self {
            id: t.id,
            name: t.name,
            date_of_service: t.date_of_service,
            transcription: t.transcription,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }
}

impl From<TranscriptDto> for Transcript {
    fn from(dto: TranscriptDto) -> Self {
        Self {
            id: dto.id,
            name: dto.name,
            date_of_service: dto.date_of_service,
            transcription: dto.transcription,
            created_at: dto.created_at,
            updated_at: dto.updated_at,
        }
    }
}