use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize)]
pub struct SpeechDto {
    pub speaker: String,

    #[serde(rename = "fala")]
    pub text: String,
}



