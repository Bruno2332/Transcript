use std::path::Path;
use crate::{dtos::speech_dto::SpeechDto, errors::app_errors::AppError, services::transcription_service};

#[tauri::command]
pub async fn transcribe_audio(audio_path: String) -> Result<Vec<SpeechDto>, AppError> {
    transcription_service::transcribe(Path::new(&audio_path)).await
}
