use std::{
    sync::{mpsc::Sender, Mutex},
    thread::JoinHandle,
};

use crate::models::transcript::RecordingCommand;

pub struct RecordingSession {
    pub command_sender: Sender<RecordingCommand>,
    pub task: JoinHandle<Result<(), String>>,
    pub output_path: String,
}

#[derive(Default)]
pub struct RecordingState {
    pub session: Option<RecordingSession>,
    pub stopping: bool,
}

#[derive(Default)]
pub struct RecordingManager {
    pub state: Mutex<RecordingState>,
}