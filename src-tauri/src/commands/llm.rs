//! LLM 相关 Tauri commands
//!
//! - test_llm_connection: 设置界面的"测试连接"按钮

use crate::models::config::ModelConfig;
use crate::services::http_client::build_client;
use crate::services::llm_service::connection_test;

/// 测试 LLM 连接
#[tauri::command]
pub async fn test_llm_connection(config: ModelConfig) -> Result<String, String> {
    let client = build_client().map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    connection_test(&client, &config)
        .await
        .map_err(|e| format!("LLM 连接失败: {e}"))?;
    Ok("LLM 连接成功".to_string())
}
