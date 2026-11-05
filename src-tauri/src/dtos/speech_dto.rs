use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize)]
pub struct SpeechDto {
    pub speaker: String,

    #[serde(rename = "fala")]
    pub text: String,
}

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    results: TranscriptionResults,
}

#[derive(Debug, Deserialize)]
struct TranscriptionResults {
    utterances: Vec<Utterance>,
}

#[derive(Debug, Deserialize)]
struct Utterance {
    words: Vec<Word>,
}

#[derive(Debug, Deserialize)]
struct Word {
    speaker: u32,
    word: String,
    punctuated_word: Option<String>,
}