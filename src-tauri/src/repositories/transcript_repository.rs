use async_trait::async_trait;
use crate::models::transcript::Transcript;
use sqlx::SqlitePool;
use crate::utils::search::name_search_pattern;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TranscriptRepository: Send + Sync {
    async fn find_all_paginated(
        &self,
        page: i64,
        size: i64,
        name: Option<String>,
    ) -> Result<Vec<Transcript>, sqlx::Error>;

    async fn count_all(&self, name: Option<String>) -> Result<i64, sqlx::Error>;

    async fn insert(&self, entity: Transcript) -> Result<Transcript, sqlx::Error>;

    async fn update(&self, entity: Transcript, id: i64) -> Result<Transcript, sqlx::Error>;

    async fn delete_by_id(&self, id: i64) -> Result<u64, sqlx::Error>;
}

#[async_trait]
impl TranscriptRepository for SqlitePool {
    async fn find_all_paginated(
        &self,
        page: i64,
        size: i64,
        name: Option<String>,
    ) -> Result<Vec<Transcript>, sqlx::Error> {
        let offset = page * size;

        let pattern = name
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(name_search_pattern);

        sqlx::query_as::<_, Transcript>(
        r#"
            SELECT *
            FROM transcripts
            WHERE ($1 IS NULL OR name REGEXP $1)
            ORDER BY name, id
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(pattern)
        .bind(size)
        .bind(offset)
        .fetch_all(self)
        .await
}

async fn count_all(&self, name: Option<String>) -> Result<i64, sqlx::Error> {
    let pattern = name
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(name_search_pattern);

    sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM transcripts
        WHERE ($1 IS NULL OR name REGEXP $1)
        "#,
    )
        .bind(pattern)
        .fetch_one(self)
        .await
}

    async fn insert(&self, entity: Transcript) -> Result<Transcript, sqlx::Error> {
        sqlx::query_as::<_, Transcript>(
            r#"
                    INSERT INTO transcripts (
                        name,
                        date_of_service,
                        transcription,
                        created_at,
                        updated_at
                    )
                    VALUES ($1, $2, $3, $4, $5)
                    RETURNING *
                   "#,
        )
            .bind(entity.name)
            .bind(entity.date_of_service)
            .bind(entity.transcription)
            .bind(entity.created_at)
            .bind(entity.updated_at)
            .fetch_one(self)
            .await
    }

    async fn update(&self, entity: Transcript, id: i64) -> Result<Transcript, sqlx::Error> {
        sqlx::query_as::<_, Transcript>(
            r#"
                    UPDATE transcripts
                    SET
                        name = $1,
                        date_of_service = $2,
                        transcription = $3,
                        updated_at = $4
                    WHERE id = $5
                    RETURNING *
                "#,
        )
            .bind(entity.name)
            .bind(entity.date_of_service)
            .bind(entity.transcription)
            .bind(entity.updated_at)
            .bind(id)
            .fetch_one(self)
            .await
    }

    async fn delete_by_id(&self, id: i64) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
                    DELETE FROM transcripts
                    WHERE id = $1
                "#,
        )
            .bind(id)
            .execute(self)
            .await?;

        Ok(result.rows_affected())
    }
}