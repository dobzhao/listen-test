//! 测试流程 Tauri commands
//!
//! - `start_test_flow`：从内存中的 TestSession 启动 1-14 题流程
//! - `submit_answer`：前端用户点击选项时调用，记录作答
//! - `get_flow_state`：前端可拉取当前阶段状态（也可走事件订阅）
//! - `get_answer_set`：返回 1-14 题作答结果（用于结算页）

use crate::commands::test_session::SessionState;
use crate::services::test_flow::{spawn_test_flow, FlowState, FlowStateContainer};

/// 全局测试流程状态（独立于 SessionState，方便 reset）
pub struct FlowGlobal {
    pub container: FlowStateContainer,
}

impl Default for FlowGlobal {
    fn default() -> Self {
        Self {
            container: FlowStateContainer::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StartFlowResponse {
    pub ok: bool,
}

/// 启动测试流程（异步）
///
/// 流程推进会通过 `test-flow-state` / `test-flow-finished` 事件通知前端。
#[tauri::command]
pub async fn start_test_flow(
    app: tauri::AppHandle,
    flow: tauri::State<'_, FlowGlobal>,
    session_state: tauri::State<'_, SessionState>,
) -> Result<StartFlowResponse, String> {
    let session = {
        let guard = session_state
            .inner
            .lock()
            .map_err(|e| format!("锁读取失败: {e}"))?;
        guard.clone().ok_or_else(|| "尚未生成测试会话，请先预生成".to_string())?
    };

    spawn_test_flow(app, flow.container.clone(), session);
    Ok(StartFlowResponse { ok: true })
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SubmitAnswerArgs {
    pub question_id: u32,
    /// "A" / "B" / "C" 或 None 表示清除作答
    pub answer: Option<String>,
}

/// 提交/修改用户作答
#[tauri::command]
pub fn submit_answer(
    flow: tauri::State<'_, FlowGlobal>,
    args: SubmitAnswerArgs,
) -> Result<(), String> {
    let mut guard = flow
        .container
        .inner
        .lock()
        .map_err(|e| format!("锁写入失败: {e}"))?;
    guard.answers.insert(args.question_id, args.answer);
    Ok(())
}

/// 获取当前测试流程状态
#[tauri::command]
pub fn get_flow_state(
    flow: tauri::State<'_, FlowGlobal>,
) -> Result<FlowStateSnapshot, String> {
    let guard = flow
        .container
        .inner
        .lock()
        .map_err(|e| format!("锁读取失败: {e}"))?;
    Ok(FlowStateSnapshot {
        state: guard.state.clone(),
        finished: guard.finished,
    })
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowStateSnapshot {
    pub state: Option<FlowState>,
    pub finished: bool,
}

/// 获取 1-14 题作答结果（用于结算）
#[tauri::command]
pub fn get_answer_set(
    flow: tauri::State<'_, FlowGlobal>,
) -> Result<AnswerSetDto, String> {
    let guard = flow
        .container
        .inner
        .lock()
        .map_err(|e| format!("锁读取失败: {e}"))?;
    Ok(AnswerSetDto {
        answers: guard.answers.clone(),
        correct_answers: guard.correct_answers.clone(),
    })
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerSetDto {
    pub answers: std::collections::HashMap<u32, Option<String>>,
    pub correct_answers: std::collections::HashMap<u32, String>,
}

/// 重置测试流程状态（用户重新开始时调用）
#[tauri::command]
pub fn reset_test_flow(
    flow: tauri::State<'_, FlowGlobal>,
) -> Result<(), String> {
    let mut guard = flow
        .container
        .inner
        .lock()
        .map_err(|e| format!("锁写入失败: {e}"))?;
    guard.answers.clear();
    guard.state = None;
    guard.finished = false;
    // correct_answers / audio_paths 由 init_container_from_session 重新填充
    Ok(())
}
