use chrono::Utc;

use crate::{
    dtos::transcript_dto::TranscriptDto,
    models::transcript::Transcript,
    repositories::transcript_repository::TranscriptRepository,
};
use crate::dtos::pagination::PageResponse;
use crate::errors::app_errors::AppError;

pub async fn find_all_paginated<R: TranscriptRepository>(
    repository: &R,
    page: i64,
    size: i64,
    name: Option<String>,
) -> Result<PageResponse<TranscriptDto>, AppError> {
    let transcripts = repository
        .find_all_paginated(page, size, name.clone())
        .await
        .map_err(|_| AppError::Database {
            message: "Erro ao buscar transcrições".to_string(),
        })?;

    let total = repository.count_all(name).await.map_err(|error| {
        eprintln!("Erro em count_all: {error:?}");
        AppError::Database {
            message: "Erro ao contar alunos".to_string(),
        }
    })?;

    let items = transcripts.into_iter().map(TranscriptDto::from).collect();

    Ok(PageResponse {
        items,
        page,
        size,
        total,
    })
}


pub async fn insert<R: TranscriptRepository>(
    repository: &R,
    dto: TranscriptDto,
) -> Result<TranscriptDto, AppError> {
    let mut result = Transcript::from(dto);
    let now = Utc::now();
    result.created_at = now;
    result.updated_at = now;

    let created = repository
        .insert(result)
        .await
        .map_err(|_| AppError::Database {
            message: "Erro ao criar transcrição".to_string(),
        })?;

    Ok(TranscriptDto::from(created))
}

pub async fn update<R: TranscriptRepository>(
    repository: &R,
    id: i64,
    dto: TranscriptDto,
) -> Result<TranscriptDto, AppError> {
    let mut result = Transcript::from(dto);
    result.updated_at = Utc::now();

    let updated = repository
        .update(result, id)
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => AppError::NotFound {
                message: "Transcrição não encontrada".to_string(),
            },
            _ => AppError::Database {
                message: "Erro ao atualizar transcrição".to_string(),
            },
        })?;

    Ok(TranscriptDto::from(updated))
}

pub async fn delete<R: TranscriptRepository>(
    repository: &R,
    id: i64,
) -> Result<(), AppError> {
    let rows_affected = repository
        .delete_by_id(id)
        .await
        .map_err(|_| AppError::Database {
            message: "Erro ao deletar transcrição".to_string(),
        })?;

    if rows_affected == 0 {
        return Err(AppError::NotFound {
            message: "Transcrição não encontrada".to_string(),
        });
    }

    Ok(())
}