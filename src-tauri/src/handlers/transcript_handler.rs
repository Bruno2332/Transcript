use sqlx::SqlitePool;
use tauri::State;

use crate::{
    dtos::transcript_dto::TranscriptDto,
    services::transcript_service,
};
use crate::errors::app_errors::AppError;


#[tauri::command]
pub async fn find_all_transcripts_paginated(
    pool: State<'_, SqlitePool>,
    page: i64,
    size: i64,
    name: Option<String>,
) -> Result<Vec<TranscriptDto>, AppError> {
    transcript_service::find_all_paginated(pool.inner(), page, size, name).await
}

#[tauri::command]
pub async fn count_all_transcripts(
    pool: State<'_, SqlitePool>,
    name: Option<String>,
) -> Result<i64, AppError> {
    transcript_service::count_all(pool.inner(), name).await
}

#[tauri::command]
pub async fn insert_transcript(
    pool: State<'_, SqlitePool>,
    dto: TranscriptDto,
) -> Result<TranscriptDto, AppError> {
    transcript_service::insert(pool.inner(), dto).await
}

#[tauri::command]
pub async fn update_transcript(
    pool: State<'_, SqlitePool>,
    id: i64,
    dto: TranscriptDto,
) -> Result<TranscriptDto, AppError> {
    transcript_service::update(pool.inner(), id, dto).await
}

#[tauri::command]
pub async fn delete_transcript(
    pool: State<'_, SqlitePool>,
    id: i64,
) -> Result<(), AppError> {
    transcript_service::delete(pool.inner(), id).await
}