//! 测试会话相关 Tauri commands

use crate::commands::config::ConfigState;
use crate::models::question::TestSession;
use crate::services::test_session::generate_full_session;
use std::sync::Mutex;
use tauri::{AppHandle, State};

/// 全局测试会话状态
pub struct SessionState {
    pub inner: Mutex<Option<TestSession>>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }
}

/// 启动测试会话预生成（完整 LLM + TTS 流程）
///
/// 返回完整 TestSession（含音频路径）。
/// 进度事件通过 `test-generation-progress` 推送。
#[tauri::command]
pub async fn generate_test_session(
    app: AppHandle,
    session_state: State<'_, SessionState>,
    config_state: State<'_, ConfigState>,
) -> Result<TestSession, String> {
    let config = {
        let guard = config_state
            .inner
            .read()
            .map_err(|e| format!("锁读取失败: {e}"))?;
        guard.clone()
    };

    let session = generate_full_session(&app, &config)
        .await
        .map_err(|e| format!("测试会话预生成失败: {e}"))?;

    {
        let mut guard = session_state
            .inner
            .lock()
            .map_err(|e| format!("锁写入失败: {e}"))?;
        *guard = Some(session.clone());
    }

    Ok(session)
}

/// 获取当前内存中的测试会话（前端进入 /test 页面时调用）
#[tauri::command]
pub fn get_test_session(
    session_state: State<'_, SessionState>,
) -> Result<Option<TestSession>, String> {
    let guard = session_state
        .inner
        .lock()
        .map_err(|e| format!("锁读取失败: {e}"))?;
    Ok(guard.clone())
}

/// 清除当前测试会话（用户主动退出或重新开始时）
#[tauri::command]
pub fn clear_test_session(
    session_state: State<'_, SessionState>,
) -> Result<(), String> {
    let mut guard = session_state
        .inner
        .lock()
        .map_err(|e| format!("锁写入失败: {e}"))?;
    *guard = None;
    Ok(())
}
