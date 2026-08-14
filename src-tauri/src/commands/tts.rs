//! TTS 相关 Tauri commands

use crate::models::config::ModelConfig;
use crate::services::http_client::build_client;
use crate::services::tts_service::connection_test;

/// 测试 TTS 连接
#[tauri::command]
pub async fn test_tts_connection(config: ModelConfig) -> Result<String, String> {
    let client = build_client().map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    connection_test(&client, &config)
        .await
        .map_err(|e| format!("TTS 连接失败: {e}"))?;
    Ok("TTS 连接成功".to_string())
}
