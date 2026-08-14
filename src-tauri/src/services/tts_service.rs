//! TTS 服务：调用 `/v1/audio/speech`，支持多 voice 拼接
//!
//! 主要场景：
//! - 单人朗读：直接返回 wav bytes
//! - 双人对话：按轮次生成多段 wav，按顺序拼接（中间插入静音）
//!
//! voice 约定：M → am_michael，W → af_heart（可在 settings 扩展为可配置）

use crate::models::config::ModelConfig;
use crate::models::question::DialogueTurn;
use crate::services::http_client::{auth_headers, ensure_success, HttpError};
use crate::utils::wav::concatenate_with_silence;
use serde_json::json;
use std::path::Path;
use thiserror::Error;
use tracing::{debug, warn};

#[derive(Debug, Error)]
pub enum TtsError {
    #[error("HTTP 错误: {0}")]
    Http(#[from] HttpError),
    #[error("reqwest 错误: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("wav 处理错误: {0}")]
    Wav(#[from] crate::utils::wav::WavError),
    #[error("TTS 返回空响应")]
    Empty,
}

/// 说话人到 voice 名的映射
fn speaker_to_voice(speaker: &str) -> &'static str {
    match speaker.to_uppercase().as_str() {
        "M" => "am_michael",
        "W" => "af_heart",
        _ => "af_heart",
    }
}

/// 单段 TTS：返回 wav 字节流
pub async fn synthesize_one(
    client: &reqwest::Client,
    config: &ModelConfig,
    text: &str,
    voice: &str,
    speed: f32,
) -> Result<Vec<u8>, TtsError> {
    let url = config.endpoint_url();
    let body = json!({
        "model": config.model,
        "input": text,
        "voice": voice,
        "speed": speed,
        "lang_code": "a",
    });
    debug!(url, voice, "调用 TTS");
    let resp = client
        .post(&url)
        .headers(auth_headers(config))
        .json(&body)
        .send()
        .await?;
    let resp = ensure_success(resp).await?;
    let bytes = resp.bytes().await?;
    if bytes.is_empty() {
        warn!("TTS 返回空响应");
        return Err(TtsError::Empty);
    }
    Ok(bytes.to_vec())
}

/// 双人对话 TTS：把每个 turn 单独合成，最后拼接为一段 wav
///
/// - `turns`：对话轮次
/// - `output_dir`：临时 wav 文件目录（每 turn 一个中间文件）
/// - `output_path`：拼接后的最终文件路径
/// - `silence_ms`：turn 之间插入的静音毫秒数
pub async fn synthesize_dialogue(
    client: &reqwest::Client,
    config: &ModelConfig,
    turns: &[DialogueTurn],
    output_dir: &Path,
    output_path: &Path,
    silence_ms: u32,
) -> Result<(), TtsError> {
    std::fs::create_dir_all(output_dir)?;

    let mut turn_paths: Vec<std::path::PathBuf> = Vec::with_capacity(turns.len());
    for (idx, turn) in turns.iter().enumerate() {
        let voice = speaker_to_voice(&turn.speaker);
        let wav_bytes =
            synthesize_one(client, config, &turn.text, voice, 1.0).await?;
        let turn_path = output_dir.join(format!("turn_{idx:03}.wav"));
        std::fs::write(&turn_path, &wav_bytes)?;
        turn_paths.push(turn_path);
    }

    let turn_path_refs: Vec<&Path> = turn_paths.iter().map(|p| p.as_path()).collect();
    concatenate_with_silence(&turn_path_refs, output_path, silence_ms)?;

    Ok(())
}

/// 连接测试：合成一段最短文本
pub async fn connection_test(client: &reqwest::Client, config: &ModelConfig) -> Result<(), TtsError> {
    let bytes = synthesize_one(client, config, "test", "af_heart", 1.0).await?;
    if bytes.len() < 100 {
        // wav 文件头就有 44 字节，100 字节说明可能没生成音频数据
        return Err(TtsError::Empty);
    }
    Ok(())
}
