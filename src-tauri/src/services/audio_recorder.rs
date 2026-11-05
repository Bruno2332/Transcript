use std::{
    collections::VecDeque,
    error::Error,
    fs::OpenOptions,
    io::{self, BufWriter, Write},
    path::PathBuf,
    sync::mpsc::{Receiver, RecvTimeoutError},
    time::Duration,
};

use mp3lame_encoder::{Builder, FlushGap, InterleavedPcm};

use wasapi::{AudioClient, Direction, SampleType, StreamMode, WaveFormat, initialize_mta};

use crate::models::transcript::RecordingCommand;

pub fn record_process(
    process_id: u32,
    output_path: PathBuf,
    commands: Receiver<RecordingCommand>,
) -> Result<(), Box<dyn Error>> {
    initialize_mta().ok()?;

    // PCM signed 16-bit, stereo, 48 kHz.
    let format = WaveFormat::new(16, 16, &SampleType::Int, 48_000, 2, None);

    let mut audio_client = AudioClient::new_application_loopback_client(process_id, true)?;

    let stream_mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: 200_000,
    };

    audio_client.initialize_client(&format, &Direction::Capture, &stream_mode)?;

    let _event_handle = audio_client.set_get_eventhandle()?;
    let capture_client = audio_client.get_audiocaptureclient()?;

    // Builder defaults to constant bitrate MP3 at 128 kbps.
    let mut encoder_builder =
        Builder::new().ok_or_else(|| io::Error::other("Unable to create MP3 encoder."))?;

    encoder_builder.set_num_channels(2).map_err(encoder_error)?;
    encoder_builder
        .set_sample_rate(48_000)
        .map_err(encoder_error)?;

    // Avoid reserving a tag that would require rewriting the file header.
    encoder_builder
        .set_to_write_vbr_tag(false)
        .map_err(encoder_error)?;

    let mut encoder = encoder_builder.build().map_err(encoder_error)?;

    let output_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_path)?;

    let mut writer = BufWriter::with_capacity(64 * 1024, output_file);

    // Buffers are reused; they do not grow with recording duration.
    let mut sample_queue = VecDeque::<u8>::new();
    let mut pcm_buffer = Vec::<i16>::with_capacity(8_192);
    let mut mp3_buffer =
        Vec::<u8>::with_capacity(mp3lame_encoder::max_required_buffer_size(4_096).max(7_200));

    let mut paused = false;

    audio_client.start_stream()?;

    let recording_result = (|| -> Result<(), Box<dyn Error>> {
        loop {
            let mut stopping = false;

            match commands.recv_timeout(Duration::from_millis(5)) {
                Ok(RecordingCommand::Pause) => {
                    paused = true;
                    sample_queue.clear();
                }

                Ok(RecordingCommand::Resume) => {
                    // Discard packets captured during the pause before resuming.
                    while capture_client.get_next_packet_size()?.unwrap_or(0) > 0 {
                        capture_client.read_from_device_to_deque(&mut sample_queue)?;
                        sample_queue.clear();
                    }

                    paused = false;
                }

                Ok(RecordingCommand::Stop) => {
                    stopping = true;
                }

                Err(RecvTimeoutError::Disconnected) => {
                    // Finalize if the owner drops the command sender.
                    stopping = true;
                }

                Err(RecvTimeoutError::Timeout) => {}
            }

            if stopping {
                audio_client.stop_stream()?;
            }

            // While paused, keep draining WASAPI without encoding.
            // This prevents paused audio from accumulating in the buffer.
            while capture_client.get_next_packet_size()?.unwrap_or(0) > 0 {
                capture_client.read_from_device_to_deque(&mut sample_queue)?;

                if paused {
                    sample_queue.clear();
                    continue;
                }

                // One stereo frame contains two i16 samples: four bytes.
                while sample_queue.len() >= 4 {
                    pcm_buffer.clear();

                    let frame_count = (sample_queue.len() / 4).min(4_096);

                    for _ in 0..frame_count {
                        let left = [
                            sample_queue.pop_front().unwrap(),
                            sample_queue.pop_front().unwrap(),
                        ];

                        let right = [
                            sample_queue.pop_front().unwrap(),
                            sample_queue.pop_front().unwrap(),
                        ];

                        pcm_buffer.push(i16::from_le_bytes(left));
                        pcm_buffer.push(i16::from_le_bytes(right));
                    }

                    mp3_buffer.clear();

                    encoder
                        .encode_to_vec(InterleavedPcm(pcm_buffer.as_slice()), &mut mp3_buffer)
                        .map_err(encoder_error)?;

                    writer.write_all(&mp3_buffer)?;
                }
            }

            if stopping {
                break;
            }
        }

        Ok(())
    })();

    // Best effort cleanup if capturing or writing failed.
    if recording_result.is_err() {
        let _ = audio_client.stop_stream();
    }

    // Flush encoder frames once, when the session ends.
    // Pausing never finalizes the MP3 encoder.
    let finalize_result = (|| -> Result<(), Box<dyn Error>> {
        mp3_buffer.clear();

        encoder
            .flush_to_vec::<FlushGap>(&mut mp3_buffer)
            .map_err(encoder_error)?;
        writer.write_all(&mp3_buffer)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;

        Ok(())
    })();

    recording_result?;
    finalize_result?;

    Ok(())
}

fn encoder_error(error: impl std::fmt::Debug) -> io::Error {
    io::Error::other(format!("MP3 encoder error: {error:?}"))
}
