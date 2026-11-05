
use tauri::{Manager, State};

use crate::models::{
    recording_state::RecordingManager,
};
use crate::models::transcript::RecordingCommand;

#[tauri::command]
pub async fn start_recording(
    app: tauri::AppHandle,
    manager: State<'_, RecordingManager>,
    process_id: u32,
) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::{
            sync::mpsc,
            thread,
            time::{SystemTime, UNIX_EPOCH},
        };

        use crate::{
            models::recording_state::RecordingSession,
            services::audio_recorder,
        };

        if process_id == 0 {
            return Err("Invalid process ID.".into());
        }

        let mut state = manager
            .state
            .lock()
            .map_err(|_| "Unable to access recording state.".to_string())?;

        if state.stopping {
            return Err("The previous recording is still being finalized.".into());
        }

        if state.session.is_some() {
            return Err("A recording session already exists.".into());
        }

        let recordings_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| error.to_string())?
            .join("recordings");

        std::fs::create_dir_all(&recordings_dir)
            .map_err(|error| error.to_string())?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();

        let output_path =
            recordings_dir.join(format!("session-{timestamp}.mp3"));

        let output_path_string =
            output_path.to_string_lossy().into_owned();

        let (command_sender, command_receiver) = mpsc::channel();

        let task = thread::Builder::new()
            .name("audio-recorder".to_string())
            .spawn(move || {
                audio_recorder::record_process(
                    process_id,
                    output_path,
                    command_receiver,
                )
                    .map_err(|error| error.to_string())
            })
            .map_err(|error| error.to_string())?;

        state.session = Some(RecordingSession {
            command_sender,
            task,
            output_path: output_path_string.clone(),
        });

        Ok(output_path_string)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, manager, process_id);
        Err("Process audio capture requires Windows.".into())
    }
}

#[tauri::command]
pub async fn pause_recording(
    manager: State<'_, RecordingManager>,
) -> Result<(), String> {
    send_command(manager.inner(), RecordingCommand::Pause)
}

#[tauri::command]
pub async fn resume_recording(
    manager: State<'_, RecordingManager>,
) -> Result<(), String> {
    send_command(manager.inner(), RecordingCommand::Resume)
}

#[tauri::command]
pub async fn stop_recording(
    app: tauri::AppHandle,
    manager: State<'_, RecordingManager>,
) -> Result<String, String> {
    let session = {
        let mut state = manager
            .state
            .lock()
            .map_err(|_| "Unable to access recording state.".to_string())?;

        if state.stopping {
            return Err("The recording is already being finalized.".into());
        }

        let session = state
            .session
            .take()
            .ok_or_else(|| "There is no recording session.".to_string())?;

        state.stopping = true;

        session
    };

    // If the worker already failed, joining it retrieves the actual error.
    let _ = session.command_sender.send(RecordingCommand::Stop);

    // Join and cleanup run together even if the caller stops awaiting.
    tauri::async_runtime::spawn_blocking(move || {
        let recording_result = match session.task.join() {
            Ok(result) => result,
            Err(_) => Err("The recording thread terminated unexpectedly.".into()),
        };

        let manager = app.state::<RecordingManager>();

        let mut state = manager
            .state
            .lock()
            .map_err(|_| "Unable to access recording state.".to_string())?;

        state.stopping = false;

        recording_result?;

        // The service has finished flushing and closing the MP3.
        Ok(session.output_path)
    })
        .await
        .map_err(|error| error.to_string())?
}

fn send_command(
    manager: &RecordingManager,
    command: RecordingCommand,
) -> Result<(), String> {
    let state = manager
        .state
        .lock()
        .map_err(|_| "Unable to access recording state.".to_string())?;

    if state.stopping {
        return Err("The recording is being finalized.".into());
    }

    let session = state
        .session
        .as_ref()
        .ok_or_else(|| "There is no recording session.".to_string())?;

    if session.task.is_finished() {
        return Err(
            "The recording has ended. Call stop_recording to retrieve its result."
                .into(),
        );
    }

    session
        .command_sender
        .send(command)
        .map_err(|_| {
            "The recorder stopped responding. Call stop_recording to retrieve its result."
                .to_string()
        })
}