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

    // Captura o áudio que está saindo pelo dispositivo padrão do Windows (Reunião/Browser/Teams).
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

    let output_event = output_client.set_get_eventhandle()?;
    let output_capture = output_client.get_audiocaptureclient()?;

    // Captura o microfone/dispositivo padrão de entrada do Windows (Sua voz).
    let microphone_device = enumerator.get_default_device(&Direction::Capture)?;
    let mut microphone_client = microphone_device.get_iaudioclient()?;

    microphone_client.initialize_client(
        &audio_format,
        &Direction::Capture,
        &stream_mode,
    )?;

    let microphone_event = microphone_client.set_get_eventhandle()?;
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
    let mut mixed_chunk = Vec::with_capacity(INTERLEAVED_SAMPLES_PER_FRAME);
    let mut paused = false;

    let recording_result = (|| -> Result<(), Box<dyn Error>> {
        loop {
            let mut stopping = false;

            // Timeout curto para responder rapidamente aos comandos do app
            match commands.recv_timeout(Duration::from_millis(1)) {
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
                // Processa blocos enquanto QUALQUER uma das partes tiver dados acumulados.
                // Evita que o silêncio de uma ponta congele a gravação da outra.
                while output_samples.len() >= SAMPLES_PER_FRAME || microphone_samples.len() >= SAMPLES_PER_FRAME {
                    mixed_chunk.clear();

                    for _ in 0..SAMPLES_PER_FRAME {
                        // Se um dos lados não tiver frames disponíveis, assume silêncio dinamicamente [0.0, 0.0]
                        let output_frame = output_samples.pop_front().unwrap_or([0.0, 0.0]);
                        let microphone_frame = microphone_samples.pop_front().unwrap_or([0.0, 0.0]);

                        for channel in 0..CHANNELS {
                            // Soma linear das vozes com um ganho compensatório leve para manter clareza na IA
                            let mixed = (output_frame[channel] + microphone_frame[channel]) * 0.75;
                            mixed_chunk.push(mixed.clamp(-1.0, 1.0));
                        }
                    }

                    recorder.write_interleaved(&mixed_chunk)?;
                }
            }

            if stopping {
                break;
            }

            // Sincroniza o loop usando os sinais nativos de hardware das placas de áudio.
            // Impede estouro de uso de CPU e garante que o processamento rode assim que houver dados.
            let _ = output_event.wait_for_event(10);
            let _ = microphone_event.wait_for_event(10);
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

