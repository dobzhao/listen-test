//! HTTP 客户端封装
//!
//! 集中创建 reqwest::Client 实例，统一超时与 User-Agent。
//! 由于 LLM 流式响应可能很长，read_timeout 单独配置。

use crate::models::config::ModelConfig;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use reqwest::{Client, ClientBuilder};
use std::time::Duration;
use thiserror::Error;
use tracing::warn;

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("HTTP 请求失败: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("HTTP 状态错误: {status}, body: {body}")]
    Status { status: u16, body: String },
    #[error("响应不是 UTF-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

/// 创建默认的 reqwest 客户端（共享给所有模型服务调用）
pub fn build_client() -> Result<Client, reqwest::Error> {
    ClientBuilder::new()
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(300)) // LLM 推理可能很慢
        .user_agent("peiyuan/0.1.0 (Tauri)")
        .build()
}

/// 根据 ModelConfig 构造带认证头的 HeaderMap
pub fn auth_headers(config: &ModelConfig) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if !config.api_key.is_empty() {
        let token = format!("Bearer {}", config.api_key);
        if let Ok(v) = HeaderValue::from_str(&token) {
            headers.insert(AUTHORIZATION, v);
        } else {
            warn!("api_key 含非法字符，无法构造 Authorization 头");
        }
    }
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(USER_AGENT, HeaderValue::from_static("peiyuan/0.1.0"));
    headers
}

/// 简化的 HTTP 状态检查：2xx 通过；其余抛出 HttpError::Status
pub async fn ensure_success(resp: reqwest::Response) -> Result<reqwest::Response, HttpError> {
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(HttpError::Status {
            status: status.as_u16(),
            body: body.chars().take(500).collect(),
        });
    }
    Ok(resp)
}
