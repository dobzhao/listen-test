//! STT 服务：调用 `/v1/audio/transcriptions` 转写 wav 为文本

use crate::models::config::ModelConfig;
use crate::services::http_client::{auth_headers, ensure_success, HttpError};
use reqwest::multipart::{Form, Part};
use std::path::Path;
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Error)]
pub enum SttError {
    #[error("HTTP 错误: {0}")]
    Http(#[from] HttpError),
    #[error("reqwest 错误: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("multipart 构建失败: {0}")]
    Multipart(String),
    #[error("STT 返回空文本")]
    Empty,
}

/// 将 wav 文件转写为文本
pub async fn transcribe(
    client: &reqwest::Client,
    config: &ModelConfig,
    wav_path: &Path,
    language: &str,
) -> Result<String, SttError> {
    let url = config.endpoint_url();
    let bytes = std::fs::read(wav_path)?;
    let filename = wav_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "audio.wav".to_string());
    debug!(url, filename, "调用 STT");

    let file_part = Part::bytes(bytes)
        .file_name(filename)
        .mime_str("audio/wav")
        .map_err(|e| SttError::Multipart(e.to_string()))?;

    let form = Form::new()
        .text("model", config.model.clone())
        .text("language", language.to_string())
        .part("file", file_part);

    let mut headers = auth_headers(config);
    // multipart/form-data 由 reqwest 自动处理 Content-Type（带 boundary），
    // 移除我们手动设置的 application/json 避免冲突
    headers.remove(reqwest::header::CONTENT_TYPE);

    let resp = client.post(&url).headers(headers).multipart(form).send().await?;
    let resp = ensure_success(resp).await?;
    let text = resp.text().await?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(SttError::Empty);
    }

    // 兼容两种 STT 响应：
    //   - verbose_json（含 text / language / segments / tokens 等元数据）→ 仅取 text
    //   - 纯文本 → 原样返回
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(t) = json.get("text").and_then(|v| v.as_str()) {
            debug!("STT 返回 verbose_json 格式，已提取 text 字段");
            return Ok(t.trim().to_string());
        }
    }

    Ok(trimmed.to_string())
}

/// 连接测试：发送一段最小 wav（静音）检验接口可用性
pub async fn connection_test(
    client: &reqwest::Client,
    config: &ModelConfig,
) -> Result<(), SttError> {
    // 构造一段 100ms 静音 wav，发送到 STT 接口
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut buf = std::io::Cursor::new(Vec::<u8>::new());
    {
        let mut writer = hound::WavWriter::new(&mut buf, spec).map_err(|e| {
            SttError::Multipart(format!("构造静音 wav 失败: {e}"))
        })?;
        for _ in 0..(spec.sample_rate / 10) {
            writer.write_sample(0i16).map_err(|e| {
                SttError::Multipart(format!("写入静音样本失败: {e}"))
            })?;
        }
        writer.finalize().map_err(|e| {
            SttError::Multipart(format!("finalize 静音 wav 失败: {e}"))
        })?;
    }
    let bytes = buf.into_inner();

    let file_part = Part::bytes(bytes)
        .file_name("silence.wav".to_string())
        .mime_str("audio/wav")
        .map_err(|e| SttError::Multipart(e.to_string()))?;

    let form = Form::new()
        .text("model", config.model.clone())
        .text("language", "en".to_string())
        .part("file", file_part);

    let mut headers = auth_headers(config);
    headers.remove(reqwest::header::CONTENT_TYPE);

    let url = config.endpoint_url();
    let resp = client
        .post(&url)
        .headers(headers)
        .multipart(form)
        .send()
        .await?;
    let _ = ensure_success(resp).await?;
    Ok(())
}
