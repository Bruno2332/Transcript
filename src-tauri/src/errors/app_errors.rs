use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppError {
    NotFound { message: String },
    Validation { errors: Vec<FieldError> },
    Database { message: String },
    Transcription { message: String },
}