use std::{path::Path, time::Duration};

use reqwest::{
    header::{HeaderValue, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE},
    Body, Client,
};
use serde_json::Value;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::{
    dtos::speech_dto::SpeechDto,
};
use crate::errors::app_errors::AppError;

pub async fn transcribe(audio_path: &Path) -> Result<Vec<SpeechDto>, AppError> {
    let api_key = std::env::var("DEEPGRAM_API_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| service_error("DEEPGRAM_API_KEY não configurada."))?;

    let mut authorization =
        HeaderValue::from_str(&format!("Token {}", api_key.trim()))
            .map_err(|_| service_error("Chave da API inválida."))?;

    authorization.set_sensitive(true);

    let file = File::open(audio_path)
        .await
        .map_err(|_| service_error("Erro ao abrir o arquivo de áudio."))?;

    let metadata = file
        .metadata()
        .await
        .map_err(|_| service_error("Erro ao consultar o arquivo de áudio."))?;

    if !metadata.is_file() || metadata.len() == 0 {
        return Err(service_error(
            "O arquivo de áudio está vazio ou é inválido.",
        ));
    }

    let client = Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(900))
        .build()
        .map_err(|_| service_error("Erro ao configurar o cliente HTTP."))?;

    let response = client
        .post("https://api.deepgram.com/v1/listen")
        .query(&[
            ("model", "nova-3"),
            ("language", "pt-BR"),
            ("utterances", "true"),
            ("diarize", "true"),
            ("punctuate", "true"),
        ])
        .header(AUTHORIZATION, authorization)
        .header(CONTENT_TYPE, "application/octet-stream")
        .header(CONTENT_LENGTH, metadata.len().to_string())
        .body(Body::wrap_stream(ReaderStream::new(file)))
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                service_error("Tempo limite excedido ao transcrever o áudio.")
            } else {
                service_error("Erro ao conectar à API de transcrição.")
            }
        })?;

    let status = response.status();

    if !status.is_success() {
        return Err(service_error(format!(
            "A API de transcrição retornou HTTP {}.",
            status.as_u16()
        )));
    }

    let data = response
        .json::<Value>()
        .await
        .map_err(|_| service_error("Resposta inválida da API de transcrição."))?;

    group_speeches(&data)
}

fn group_speeches(data: &Value) -> Result<Vec<SpeechDto>, AppError> {
    let utterances = data
        .pointer("/results/utterances")
        .and_then(Value::as_array)
        .ok_or_else(|| service_error("Resposta sem a lista de falas."))?;

    let mut speeches = Vec::new();
    let mut current_speaker: Option<u64> = None;
    let mut current_words: Vec<String> = Vec::new();

    for utterance in utterances {
        let words = utterance
            .get("words")
            .and_then(Value::as_array)
            .ok_or_else(|| service_error("Fala sem a lista de palavras."))?;

        for word in words {
            let speaker = word
                .get("speaker")
                .and_then(Value::as_u64)
                .ok_or_else(|| service_error("Palavra sem speaker válido."))?;

            let token = word
                .get("punctuated_word")
                .and_then(Value::as_str)
                .or_else(|| word.get("word").and_then(Value::as_str))
                .ok_or_else(|| service_error("Palavra sem texto válido."))?;

            if token.trim().is_empty() {
                continue;
            }

            if let Some(previous_speaker) = current_speaker {
                if previous_speaker != speaker {
                    speeches.push(build_speech(
                        previous_speaker,
                        &current_words,
                    ));

                    current_words.clear();
                }
            }

            current_speaker = Some(speaker);
            current_words.push(token.to_owned());
        }
    }

    if let Some(speaker) = current_speaker {
        if !current_words.is_empty() {
            speeches.push(build_speech(speaker, &current_words));
        }
    }

    Ok(speeches)
}

fn build_speech(speaker: u64, words: &[String]) -> SpeechDto {
    SpeechDto {
        speaker: if speaker == 0 {
            "Entrevistador".to_string()
        } else {
            "Entrevistado".to_string()
        },
        text: words.join(" "),
    }
}

fn service_error(message: impl Into<String>) -> AppError {
    AppError::Transcription {
        message: message.into(),
    }
}