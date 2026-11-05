use tauri::Manager;

mod dtos;
mod errors;
mod handlers;
mod models;
mod repositories;
mod services;
mod db;
mod utils;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let pool = tauri::async_runtime::block_on(db::init(&data_dir))?;
            app.manage(pool);
            Ok(())
        })
        .manage(models::recording_state::RecordingManager::default())
        .invoke_handler(tauri::generate_handler![
            handlers::transcription_handler::transcribe_audio,
            handlers::recording_handler::start_recording,
            handlers::recording_handler::pause_recording,
            handlers::recording_handler::resume_recording,
            handlers::recording_handler::stop_recording,
            handlers::transcript_handler::find_all_transcripts_paginated,
            handlers::transcript_handler::count_all_transcripts,
            handlers::transcript_handler::insert_transcript,
            handlers::transcript_handler::update_transcript,
            handlers::transcript_handler::delete_transcript,
                ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
