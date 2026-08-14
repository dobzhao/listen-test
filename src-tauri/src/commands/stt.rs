//! STT 相关 Tauri commands

use crate::models::config::ModelConfig;
use crate::services::http_client::build_client;
use crate::services::stt_service::{connection_test, transcribe};

/// 测试 STT 连接
#[tauri::command]
pub async fn test_stt_connection(config: ModelConfig) -> Result<String, String> {
    let client = build_client().map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    connection_test(&client, &config)
        .await
        .map_err(|e| format!("STT 连接失败: {e}"))?;
    Ok("STT 连接成功".to_string())
}

/// 转写本地 wav 文件
#[tauri::command]
pub async fn transcribe_audio(
    config: ModelConfig,
    wav_path: String,
    language: Option<String>,
) -> Result<String, String> {
    let client = build_client().map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    let lang = language.unwrap_or_else(|| "en".to_string());
    let text = transcribe(&client, &config, std::path::Path::new(&wav_path), &lang)
        .await
        .map_err(|e| format!("STT 转写失败: {e}"))?;
    Ok(text)
}
