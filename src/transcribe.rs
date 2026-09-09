//! Telegram audio download and Google Speech-to-Text integration.
//!
//! The public entry point is [`handle_voice_message`]. All files are created
//! as temporary files and are removed automatically when the request ends.

use anyhow::{anyhow, Context};
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;
use teloxide::{net::Download, prelude::*};
use tempfile::NamedTempFile;
use tokio::{fs, io::AsyncWriteExt, process::Command};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
/// Google Speech-to-Text settings for the generated WAV file.
struct RecognitionConfig {
    encoding: String,
    sample_rate_hertz: i32,
    language_code: String,
}

#[derive(Serialize)]
/// Base64-encoded audio payload expected by Google Speech-to-Text.
struct RecognitionAudio {
    content: String,
}

#[derive(Serialize)]
/// JSON body sent to the synchronous Google Speech-to-Text endpoint.
struct RecognizeRequest {
    config: RecognitionConfig,
    audio: RecognitionAudio,
}

#[derive(Deserialize)]
/// A candidate transcript returned by Google.
struct SpeechRecognitionAlternative {
    transcript: Option<String>,
}

#[derive(Deserialize)]
/// A section of speech recognition output.
struct SpeechRecognitionResult {
    alternatives: Option<Vec<SpeechRecognitionAlternative>>,
}

#[derive(Deserialize)]
/// Top-level response returned by Google Speech-to-Text.
struct RecognizeResponse {
    results: Option<Vec<SpeechRecognitionResult>>,
}

/// Transcribes a voice note or audio file in the message being replied to.
///
/// The reply must contain either a Telegram voice note or an audio document.
/// The input is converted to 16 kHz mono WAV before it is sent to Google.
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
    transcribe_file(output_file.path()).await
}

/// Downloads a Telegram file into a temporary file using the authenticated bot.
///
/// This intentionally uses `Bot::download_file` instead of constructing a URL
/// from an environment token, so it works whether the token was passed through
/// the command line or `TELOXIDE_TOKEN`.
async fn download_telegram_file(bot: &Bot, file_id: &str) -> anyhow::Result<NamedTempFile> {
    let file = bot
        .get_file(file_id.to_owned())
        .await
        .context("getting file metadata from Telegram")?;

    // `reopen` gives Tokio an independent file handle while `temp_file` keeps
    // ownership of the path and removes it automatically on drop.
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

/// Converts Telegram's OGG/MP3 audio to the WAV format required by Google.
///
/// `ffmpeg` must be installed and available through `PATH` at runtime.
async fn convert_to_wav(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    let status = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(input_path)
        .args(["-ar", "16000", "-ac", "1", "-f", "wav"])
        .arg(output_path)
        .status()
        .await
        .context("could not run ffmpeg; ensure it is installed and available in PATH")?;

    if !status.success() {
        return Err(anyhow!("ffmpeg could not convert the audio to WAV"));
    }

    Ok(())
}

/// Sends a WAV file to Google Speech-to-Text and returns its first transcript.
///
/// Google requires an OAuth access token in `GOOGLE_OAUTH_TOKEN`. The function
/// returns an empty string when Google accepts the audio but finds no speech.
async fn transcribe_file(wav_path: &Path) -> anyhow::Result<String> {
    let bytes = fs::read(wav_path)
        .await
        .with_context(|| format!("reading temporary WAV file: {}", wav_path.display()))?;

    let request = RecognizeRequest {
        config: RecognitionConfig {
            encoding: "LINEAR16".to_owned(),
            sample_rate_hertz: 16_000,
            language_code: "es-ES".to_owned(),
        },
        audio: RecognitionAudio {
            content: general_purpose::STANDARD.encode(bytes),
        },
    };

    let oauth_token = env::var("GOOGLE_OAUTH_TOKEN")
        .context("GOOGLE_OAUTH_TOKEN environment variable is not configured")?;

    let response = Client::new()
        .post("https://speech.googleapis.com/v1/speech:recognize")
        .bearer_auth(oauth_token)
        .json(&request)
        .send()
        .await
        .context("sending audio to Google Speech-to-Text")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Google Speech-to-Text returned {status}: {body}"));
    }

    let response: RecognizeResponse = response
        .json()
        .await
        .context("parsing Google Speech-to-Text response")?;

    let transcript = response
        .results
        .unwrap_or_default()
        .into_iter()
        .filter_map(|result| result.alternatives)
        .filter_map(|alternatives| alternatives.into_iter().next())
        .filter_map(|alternative| alternative.transcript)
        .collect::<Vec<_>>()
        .join(" ");

    Ok(transcript.trim().to_owned())
}
