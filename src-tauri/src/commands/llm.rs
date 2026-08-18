//! LLM 相关 Tauri commands
//!
//! - test_llm_connection: 设置界面的"测试连接"按钮
//! - generate_question_text: 设置界面/测试页面的"试生成"按钮

use crate::models::config::{LlmParams, ModelConfig};
use crate::services::http_client::build_client;
use crate::services::llm_service::{call_llm, connection_test, ChatMessage};
use crate::services::prompt_engine_service::render;
use std::collections::HashMap;

/// 测试 LLM 连接
#[tauri::command]
pub async fn test_llm_connection(config: ModelConfig) -> Result<String, String> {
    let client = build_client().map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    connection_test(&client, &config)
        .await
        .map_err(|e| format!("LLM 连接失败: {e}"))?;
    Ok("LLM 连接成功".to_string())
}

/// 用指定 Prompt 调用 LLM（设置界面可用来预览生成效果）
#[tauri::command]
pub async fn generate_with_llm(
    config: ModelConfig,
    params: LlmParams,
    template: String,
    vars: HashMap<String, String>,
) -> Result<String, String> {
    let client = build_client().map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    let borrowed: HashMap<&str, &str> =
        vars.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let prompt = render(&template, &borrowed)
        .map_err(|e| format!("Prompt 渲染失败: {e}"))?;
    let text = call_llm(&client, &config, &params, vec![ChatMessage::user(prompt)])
        .await
        .map_err(|e| format!("LLM 调用失败: {e}"))?;
    Ok(text)
}
