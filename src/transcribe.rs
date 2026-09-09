//! Telegram audio download and local Whisper transcription.
//!
//! Set `WHISPER_MODEL_PATH` to a local GGML/GGUF Whisper model file. The model
//! is never downloaded by the bot, so its version remains under operator control.

use anyhow::{anyhow, Context};
use hound::{SampleFormat, WavReader};
use std::{env, path::{Path, PathBuf}};
use teloxide::{net::Download, prelude::*};
use tempfile::NamedTempFile;
use tokio::{io::AsyncWriteExt, process::Command};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

const SAMPLE_RATE: u32 = 16_000;

/// Transcribes a voice note or audio file in the message being replied to.
///
/// The reply must contain either a Telegram voice note or an audio document.
/// Audio is converted to 16 kHz mono PCM WAV before local Whisper inference.
pub async fn handle_voice_message(bot: Bot, msg: Message) -> anyhow::Result<String> {
    let replied_message = msg
        .reply_to_message()
        .ok_or_else(|| anyhow!("The message must reply to an audio message."))?;

    let file_id = replied_message
        .voice()
        .map(|voice| voice.file.id.clone())
        .or_else(|| replied_message.audio().map(|audio| audio.file.id.clone()))
        .ok_or_else(|| anyhow!("The replied message does not contain audio or a voice note."))?;

    let input_file = download_telegram_file(&bot, &file_id).await?;
    let output_file = NamedTempFile::new().context("creating temporary WAV file")?;

    convert_to_wav(input_file.path(), output_file.path()).await?;

    let model_path = env::var("WHISPER_MODEL_PATH")
        .context("WHISPER_MODEL_PATH must point to a local Whisper model file")?;
    transcribe_wav(output_file.path(), PathBuf::from(model_path)).await
}

/// Downloads a Telegram file into a temporary file using the authenticated bot.
///
/// The temporary-file owner remains alive until transcription has completed, so
/// its path is removed automatically after the request.
async fn download_telegram_file(bot: &Bot, file_id: &str) -> anyhow::Result<NamedTempFile> {
    let file = bot
        .get_file(file_id.to_owned())
        .await
        .context("getting file metadata from Telegram")?;

    let temp_file = NamedTempFile::new().context("creating temporary audio file")?;
    let mut destination = tokio::fs::File::from_std(
        temp_file
            .reopen()
            .context("opening temporary audio file for download")?,
    );

    bot.download_file(&file.path, &mut destination)
        .await
        .context("downloading audio from Telegram")?;
    destination
        .flush()
        .await
        .context("flushing temporary audio file")?;

    Ok(temp_file)
}

/// Converts input audio to the PCM format accepted by Whisper.
///
/// `ffmpeg` must be installed and available through `PATH` at runtime.
async fn convert_to_wav(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    let status = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(input_path)
        .args([
            "-ar",
            "16000",
            "-ac",
            "1",
            "-c:a",
            "pcm_s16le",
            "-f",
            "wav",
        ])
        .arg(output_path)
        .status()
        .await
        .context("could not run ffmpeg; ensure it is installed and available in PATH")?;

    if !status.success() {
        return Err(anyhow!("ffmpeg could not convert the audio to WAV"));
    }

    Ok(())
}

/// Runs CPU-bound Whisper inference outside Tokio's asynchronous worker threads.
async fn transcribe_wav(wav_path: &Path, model_path: PathBuf) -> anyhow::Result<String> {
    let wav_path = wav_path.to_owned();

    tokio::task::spawn_blocking(move || transcribe_wav_blocking(&wav_path, &model_path))
        .await
        .context("the Whisper transcription task stopped unexpectedly")?
}

/// Loads the model, validates PCM WAV input, and returns the joined segments.
///
/// A model is loaded per request to keep the state isolated and the code simple.
/// If traffic grows, this function can be changed to reuse a shared context.
fn transcribe_wav_blocking(wav_path: &Path, model_path: &Path) -> anyhow::Result<String> {
    if !model_path.is_file() {
        return Err(anyhow!(
            "Whisper model file does not exist: {}",
            model_path.display()
        ));
    }

    let samples = read_pcm_samples(wav_path)?;
    let context = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
        .context("loading the local Whisper model")?;
    let mut state = context.create_state().context("creating Whisper state")?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("es"));
    params.set_translate(false);
    params.set_no_timestamps(true);
    params.set_n_threads(whisper_thread_count());

    state.full(params, &samples).context("running local Whisper inference")?;

    state
        .as_iter()
        .map(|segment| segment.to_str().map(str::trim).map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()
        .context("reading Whisper transcription segments")
        .map(|segments| {
            segments
                .into_iter()
                .filter(|segment| !segment.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
}

/// Reads the 16-bit PCM WAV created by [`convert_to_wav`] as normalized samples.
fn read_pcm_samples(wav_path: &Path) -> anyhow::Result<Vec<f32>> {
    let mut reader = WavReader::open(wav_path)
        .with_context(|| format!("opening temporary WAV file: {}", wav_path.display()))?;
    let spec = reader.spec();

    if spec.channels != 1
        || spec.sample_rate != SAMPLE_RATE
        || spec.bits_per_sample != 16
        || spec.sample_format != SampleFormat::Int
    {
        return Err(anyhow!(
            "unexpected WAV format; expected 16 kHz, mono, signed 16-bit PCM"
        ));
    }

    reader
        .samples::<i16>()
        .map(|sample| sample.map(|value| f32::from(value) / f32::from(i16::MAX)))
        .collect::<Result<Vec<_>, _>>()
        .context("reading PCM samples from temporary WAV file")
}

/// Returns a conservative CPU thread count, configurable with `WHISPER_THREADS`.
fn whisper_thread_count() -> i32 {
    env::var("WHISPER_THREADS")
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .filter(|&threads| threads > 0)
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|count| count.get().min(4) as i32)
                .unwrap_or(1)
        })
}
