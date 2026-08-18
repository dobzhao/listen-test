//! LLM 服务：调用 `/v1/chat/completions`，支持流式响应
//!
//! 主要流程：
//! 1. 构造请求体（含 stream=true、stream_options.include_usage=true）
//! 2. 通过 reqwest 发起请求，解析 SSE 流
//! 3. 拼接 `choices[0].delta.content`，返回完整文本
//! 4. 调用方拿到完整文本后再用 `utils::json_extract::try_parse` 解析为强类型

use crate::models::config::{LlmParams, ModelConfig};
use crate::services::http_client::{auth_headers, ensure_success, HttpError};
use crate::utils::retry::{retry_async, RetryConfig};
use eventsource_stream as ess;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tracing::{debug, warn};

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("HTTP 错误: {0}")]
    Http(#[from] HttpError),
    #[error("reqwest 错误: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("SSE 解析失败: {0}")]
    Sse(String),
    #[error("流式响应被截断或缺少 choices[0].delta.content")]
    EmptyContent,
}

/// 对话消息（OpenAI Chat Completions 兼容格式）
///
/// 角色支持 `"user" | "assistant" | "system"`，用于支持多轮对话：
/// 出题时若 LLM 返回不合规 JSON 或题目数量错误，调用方可在 messages
/// 末尾追加 assistant(原始输出) + user(具体错误反馈) 实现多轮修正。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: "user".into(), content: content.into() }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: "assistant".into(), content: content.into() }
    }
    #[allow(dead_code)]
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: "system".into(), content: content.into() }
    }
}

/// LLM 调用返回的完整文本
///
/// `messages` 需包含至少一条 user 消息；重试时调用方可在末尾追加
/// assistant + 用户反馈消息以实现多轮修正。
pub async fn call_llm(
    client: &reqwest::Client,
    config: &ModelConfig,
    params: &LlmParams,
    messages: Vec<ChatMessage>,
) -> Result<String, LlmError> {
    let url = config.endpoint_url();

    let body = json!({
        "model": config.model,
        "messages": messages,
        "stream": true,
        "stream_options": { "include_usage": true },
        "temperature": params.temperature,
        "max_tokens": params.max_tokens,
        "top_p": params.top_p,
        "top_k": params.top_k,
        // 关闭模型思维链输出；不支持该字段的服务会忽略未知字段
        "chat_template_kwargs": { "enable_thinking": false },
    });

    let url_clone = url.clone();
    let config_clone = config.clone();
    let body_clone = body.clone();
    let client_clone = client.clone();

    let retry_config = RetryConfig::default();

    let result = retry_async(
        &retry_config,
        "llm_call",
        move || {
            let url = url_clone.clone();
            let config = config_clone.clone();
            let body = body_clone.clone();
            let client = client_clone.clone();
            async move { call_llm_once(&client, &url, &config, body).await }
        },
        |err| matches!(err, LlmError::Http(_)),
    )
    .await?;

    if result.trim().is_empty() {
        return Err(LlmError::EmptyContent);
    }
    Ok(result)
}

/// 单次 LLM 调用（不含重试）
async fn call_llm_once(
    client: &reqwest::Client,
    url: &str,
    config: &ModelConfig,
    body: serde_json::Value,
) -> Result<String, LlmError> {
    debug!(url, prompt = %body, "调用 LLM（请求体）");
    let resp = client
        .post(url)
        .headers(auth_headers(config))
        .json(&body)
        .send()
        .await?;
    let resp = ensure_success(resp).await?;
    let stream = resp.bytes_stream();

    // 解析 SSE
    let mut event_stream = ess::EventStream::new(stream.map(|r| {
        r.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }));

    let mut full_text = String::new();
    while let Some(event) = event_stream.next().await {
        let event = match event {
            Ok(e) => e,
            Err(e) => return Err(LlmError::Sse(format!("{e:?}"))),
        };
        match event.event.as_str() {
            "error" => {
                return Err(LlmError::Sse(format!(
                    "SSE error event: {}",
                    event.data
                )));
            }
            _ => {
                // 解析 data 行的 JSON，提取 delta.content
                let line = event.data.trim();
                if line.is_empty() || line == "[DONE]" {
                    continue;
                }
                let parsed: serde_json::Value = match serde_json::from_str(line) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!(line, error = %e, "无法解析 SSE data 行");
                        continue;
                    }
                };
                if let Some(delta) = parsed
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("delta"))
                    .and_then(|d| d.get("content"))
                    .and_then(|c| c.as_str())
                {
                    full_text.push_str(delta);
                }
            }
        }
    }

    debug!(response = %full_text, "调用 LLM（响应全文）");
    Ok(full_text)
}

/// 连接测试：发送最小 prompt 检查连通性
pub async fn connection_test(
    client: &reqwest::Client,
    config: &ModelConfig,
) -> Result<(), LlmError> {
    let body = json!({
        "model": config.model,
        "messages": vec![ChatMessage::user("ping")],
        "max_tokens": 8,
    });
    let url = config.endpoint_url();
    let resp = client
        .post(&url)
        .headers(auth_headers(config))
        .json(&body)
        .send()
        .await?;
    let _ = ensure_success(resp).await?;
    Ok(())
}
