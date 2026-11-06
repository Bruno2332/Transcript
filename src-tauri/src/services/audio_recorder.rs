use std::{
    collections::VecDeque,
    error::Error,
    fs::OpenOptions,
    io::{self, Write},
    path::PathBuf,
    sync::mpsc::{Receiver, RecvTimeoutError},
    thread,
    time::Duration,
};

use ogg::{PacketWriteEndInfo, PacketWriter};
use opus::{Application, Bitrate, Channels, Encoder};
use wasapi::{
    initialize_mta, AudioCaptureClient, DeviceEnumerator, Direction, SampleType,
    StreamMode, WaveFormat,
};

use crate::models::transcript::RecordingCommand;

const SAMPLE_RATE: usize = 48_000;
const CHANNELS: usize = 2;
const SAMPLES_PER_FRAME: usize = 960;
const INTERLEAVED_SAMPLES_PER_FRAME: usize = SAMPLES_PER_FRAME * CHANNELS;
const BYTES_PER_AUDIO_FRAME: usize = CHANNELS * std::mem::size_of::<f32>();
const MAX_QUEUE_SKEW_FRAMES: usize = 480;

pub fn record_process(
    output_path: PathBuf,
    commands: Receiver<RecordingCommand>,
) -> Result<(), Box<dyn Error>> {
    initialize_mta().ok()?;

    let enumerator = DeviceEnumerator::new()?;
    let audio_format = WaveFormat::new(
        32,
        32,
        &SampleType::Float,
        SAMPLE_RATE,
        CHANNELS,
        None,
    );

    // Captura o áudio que está saindo pelo dispositivo padrão do Windows.
    let output_device = enumerator.get_default_device(&Direction::Render)?;
    let mut output_client = output_device.get_iaudioclient()?;

    let stream_mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: 200_000,
    };

    // Loopback do dispositivo de saída é inicializado como captura.
    output_client.initialize_client(
        &audio_format,
        &Direction::Capture,
        &stream_mode,
    )?;

    let _output_event = output_client.set_get_eventhandle()?;
    let output_capture = output_client.get_audiocaptureclient()?;

    // Captura o microfone/dispositivo padrão de entrada do Windows.
    let microphone_device = enumerator.get_default_device(&Direction::Capture)?;
    let mut microphone_client = microphone_device.get_iaudioclient()?;

    microphone_client.initialize_client(
        &audio_format,
        &Direction::Capture,
        &stream_mode,
    )?;

    let _microphone_event = microphone_client.set_get_eventhandle()?;
    let microphone_capture = microphone_client.get_audiocaptureclient()?;

    output_client.start_stream()?;

    if let Err(error) = microphone_client.start_stream() {
        let _ = output_client.stop_stream();
        return Err(error.into());
    }

    let output_file = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_path)
    {
        Ok(file) => file,
        Err(error) => {
            let _ = output_client.stop_stream();
            let _ = microphone_client.stop_stream();
            return Err(error.into());
        }
    };

    let mut recorder = match OggOpusRecorder::new(output_file) {
        Ok(recorder) => recorder,
        Err(error) => {
            let _ = output_client.stop_stream();
            let _ = microphone_client.stop_stream();
            return Err(error);
        }
    };

    let mut output_samples = VecDeque::<[f32; CHANNELS]>::new();
    let mut microphone_samples = VecDeque::<[f32; CHANNELS]>::new();
    let mut paused = false;

    let recording_result = (|| -> Result<(), Box<dyn Error>> {
        loop {
            let mut stopping = false;

            match commands.recv_timeout(Duration::from_millis(5)) {
                Ok(RecordingCommand::Pause) => {
                    paused = true;
                }
                Ok(RecordingCommand::Resume) => {
                    paused = false;
                }
                Ok(RecordingCommand::Stop) => {
                    stopping = true;
                }
                Err(RecvTimeoutError::Disconnected) => {
                    stopping = true;
                }
                Err(RecvTimeoutError::Timeout) => {}
            }

            read_available_audio(&output_capture, &mut output_samples)?;
            read_available_audio(&microphone_capture, &mut microphone_samples)?;

            if paused {
                output_samples.clear();
                microphone_samples.clear();
            } else {
                correct_queue_skew(&mut output_samples, &mut microphone_samples);

                let mut mixed_samples = Vec::with_capacity(
                    output_samples
                        .len()
                        .min(microphone_samples.len())
                        * CHANNELS,
                );

                while let (Some(output_frame), Some(microphone_frame)) =
                    (output_samples.pop_front(), microphone_samples.pop_front())
                {
                    for channel in 0..CHANNELS {
                        let mixed =
                            (output_frame[channel] + microphone_frame[channel]) * 0.5;
                        mixed_samples.push(mixed.clamp(-1.0, 1.0));
                    }
                }

                recorder.write_interleaved(&mixed_samples)?;
            }

            if stopping {
                break;
            }

            thread::sleep(Duration::from_millis(2));
        }

        Ok(())
    })();

    let output_stop_result = output_client.stop_stream();
    let microphone_stop_result = microphone_client.stop_stream();
    let finalize_result = recorder.finish();

    recording_result?;
    output_stop_result?;
    microphone_stop_result?;
    finalize_result?;

    Ok(())
}

fn read_available_audio(
    capture: &AudioCaptureClient,
    samples: &mut VecDeque<[f32; CHANNELS]>,
) -> Result<(), Box<dyn Error>> {
    loop {
        let frame_count = capture.get_next_packet_size()?.unwrap_or(0) as usize;

        if frame_count == 0 {
            break;
        }

        let mut bytes = vec![0_u8; frame_count * BYTES_PER_AUDIO_FRAME];
        let (frames_read, buffer_info) = capture.read_from_device(&mut bytes)?;
        let frames_read = frames_read as usize;

        if buffer_info.flags.silent {
            samples.extend(std::iter::repeat_n([0.0, 0.0], frames_read));
            continue;
        }

        for frame_bytes in bytes[..frames_read * BYTES_PER_AUDIO_FRAME]
            .chunks_exact(BYTES_PER_AUDIO_FRAME)
        {
            let left = f32::from_le_bytes(frame_bytes[0..4].try_into()?);
            let right = f32::from_le_bytes(frame_bytes[4..8].try_into()?);
            samples.push_back([left, right]);
        }
    }

    Ok(())
}

fn correct_queue_skew(
    output_samples: &mut VecDeque<[f32; CHANNELS]>,
    microphone_samples: &mut VecDeque<[f32; CHANNELS]>,
) {
    let difference = output_samples
        .len()
        .abs_diff(microphone_samples.len());

    if difference <= MAX_QUEUE_SKEW_FRAMES {
        return;
    }

    let frames_to_discard = difference - MAX_QUEUE_SKEW_FRAMES;

    if output_samples.len() > microphone_samples.len() {
        output_samples.drain(..frames_to_discard);
    } else {
        microphone_samples.drain(..frames_to_discard);
    }
}

struct OggOpusRecorder<W: Write> {
    encoder: Encoder,
    writer: PacketWriter<'static, W>,
    serial: u32,
    pre_skip: u64,
    input_frames: u64,
    encoded_frames: u64,
    pending_samples: Vec<f32>,
    pending_packet: Option<Vec<u8>>,
}

impl<W: Write> OggOpusRecorder<W> {
    fn new(output: W) -> Result<Self, Box<dyn Error>> {
        let mut encoder = Encoder::new(
            SAMPLE_RATE as u32,
            Channels::Stereo,
            Application::Audio,
        )?;

        encoder.set_bitrate(Bitrate::Bits(128_000))?;
        let pre_skip = u16::try_from(encoder.get_lookahead()?)?;

        let serial = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos() as u32)
            ^ std::process::id();

        let mut recorder = Self {
            encoder,
            writer: PacketWriter::new(output),
            serial,
            pre_skip: u64::from(pre_skip),
            input_frames: 0,
            encoded_frames: 0,
            pending_samples: Vec::with_capacity(INTERLEAVED_SAMPLES_PER_FRAME),
            pending_packet: None,
        };

        recorder.write_headers(pre_skip)?;
        Ok(recorder)
    }

    fn write_interleaved(
        &mut self,
        samples: &[f32],
    ) -> Result<(), Box<dyn Error>> {
        if samples.len() % CHANNELS != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "O bloco de áudio não contém amostras estéreo completas.",
            )
                .into());
        }

        self.input_frames += (samples.len() / CHANNELS) as u64;
        self.pending_samples
            .extend(samples.iter().map(|sample| sample.clamp(-1.0, 1.0)));

        while self.pending_samples.len() >= INTERLEAVED_SAMPLES_PER_FRAME {
            let frame: Vec<f32> = self
                .pending_samples
                .drain(..INTERLEAVED_SAMPLES_PER_FRAME)
                .collect();

            self.encode_frame(&frame)?;
        }

        Ok(())
    }

    fn finish(mut self) -> Result<(), Box<dyn Error>> {
        if !self.pending_samples.is_empty() {
            self.pending_samples
                .resize(INTERLEAVED_SAMPLES_PER_FRAME, 0.0);

            let final_frame = std::mem::take(&mut self.pending_samples);
            self.encode_frame(&final_frame)?;
        }

        let final_packet = self.pending_packet.take().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Nenhum áudio foi capturado.",
            )
        })?;

        let final_granule_position = self.pre_skip + self.input_frames;

        self.writer.write_packet(
            final_packet,
            self.serial,
            PacketWriteEndInfo::EndStream,
            final_granule_position,
        )?;

        let mut output = self.writer.into_inner();
        output.flush()?;

        Ok(())
    }

    fn write_headers(&mut self, pre_skip: u16) -> Result<(), Box<dyn Error>> {
        let mut opus_head = Vec::with_capacity(19);
        opus_head.extend_from_slice(b"OpusHead");
        opus_head.push(1);
        opus_head.push(CHANNELS as u8);
        opus_head.extend_from_slice(&pre_skip.to_le_bytes());
        opus_head.extend_from_slice(&(SAMPLE_RATE as u32).to_le_bytes());
        opus_head.extend_from_slice(&0_i16.to_le_bytes());
        opus_head.push(0);

        self.writer.write_packet(
            opus_head,
            self.serial,
            PacketWriteEndInfo::EndPage,
            0,
        )?;

        let vendor = b"transcript";
        let mut opus_tags = Vec::new();
        opus_tags.extend_from_slice(b"OpusTags");
        opus_tags.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
        opus_tags.extend_from_slice(vendor);
        opus_tags.extend_from_slice(&0_u32.to_le_bytes());

        self.writer.write_packet(
            opus_tags,
            self.serial,
            PacketWriteEndInfo::EndPage,
            0,
        )?;

        Ok(())
    }

    fn encode_frame(&mut self, samples: &[f32]) -> Result<(), Box<dyn Error>> {
        let mut packet = vec![0_u8; 4_000];
        let packet_size = self.encoder.encode_float(samples, &mut packet)?;
        packet.truncate(packet_size);

        self.encoded_frames += SAMPLES_PER_FRAME as u64;

        if let Some(previous_packet) = self.pending_packet.replace(packet) {
            let granule_position =
                self.pre_skip + self.encoded_frames - SAMPLES_PER_FRAME as u64;

            self.writer.write_packet(
                previous_packet,
                self.serial,
                PacketWriteEndInfo::NormalPacket,
                granule_position,
            )?;
        }

        Ok(())
    }
}