//! 测试流程 Tauri commands
//!
//! - `start_test_flow`：从内存中的 TestSession 启动 1-14 题流程；时长从 ConfigState 读取
//! - `submit_answer`：前端用户点击选项时调用，记录作答
//! - `get_flow_state`：前端可拉取当前阶段状态（也可走事件订阅）
//! - `get_answer_set`：返回 1-14 题作答结果（用于结算页）
//! - `skip_to_next`：前端点击"下一题"时调用，停止当前播放并快速进入下一阶段

use crate::commands::audio::AudioPlaybackState;
use crate::commands::config::ConfigState;
use crate::commands::test_session::SessionState;
use crate::services::test_flow::{spawn_test_flow, FlowState, FlowStateContainer};
use std::sync::atomic::Ordering;
use tauri::State;
use tracing::{debug, info, warn};

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
/// 各阶段时长从 ConfigState 读取后冻结传入异步任务，避免测试中途修改配置导致
/// 计时跳变。
#[tauri::command]
pub async fn start_test_flow(
    app: tauri::AppHandle,
    flow: tauri::State<'_, FlowGlobal>,
    session_state: tauri::State<'_, SessionState>,
    config_state: tauri::State<'_, ConfigState>,
) -> Result<StartFlowResponse, String> {
    let session = {
        let guard = session_state
            .inner
            .lock()
            .map_err(|e| format!("锁读取失败: {e}"))?;
        guard.clone().ok_or_else(|| "尚未生成测试会话，请先预生成".to_string())?
    };

    // 防止重复启动：flow 已经处于 running 状态时直接拒绝。
    // 背景：ResultPage "重新测试" 流程会先 reset_test_flow 再 start_test_flow。
    // 如果前端在尚未收到事件时再次点 "开始测试"，第二次调用会再 spawn 一份 run_flow，
    // 导致两条并发 run_flow 互相覆盖 state / 答案。
    {
        let guard = flow
            .container
            .inner
            .lock()
            .map_err(|e| format!("锁读取失败: {e}"))?;
        if guard.state.is_some() && !guard.finished {
            warn!(
                "start_test_flow: 流程已在运行中，session_id={}，拒绝重复启动",
                session.session_id
            );
            return Err("测试流程已在运行中".to_string());
        }
    }

    info!(
        "start_test_flow: 启动测试流程 session_id={}, 短对话={}, 长对话={}, 独白题数={}",
        session.session_id,
        session.short_dialogues.len(),
        session.long_dialogues.len(),
        session.monologue.questions.len(),
    );

    // 读取流程时长配置（用快照，不用读锁常驻）
    let timing = {
        let guard = config_state
            .inner
            .read()
            .map_err(|e| format!("配置锁读取失败: {e}"))?;
        guard.timing.clone()
    };

    debug!(
        "start_test_flow: timing={:?}",
        timing
    );

    spawn_test_flow(app, flow.container.clone(), session, timing);
    info!("start_test_flow: 已 spawn 测试流程异步任务");
    Ok(StartFlowResponse { ok: true })
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
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
    let qid = args.question_id;
    let answer_preview = args
        .answer
        .as_ref()
        .map(|s| {
            if s.len() > 40 {
                format!("{}…(len={})", &s[..40], s.len())
            } else {
                s.clone()
            }
        })
        .unwrap_or_else(|| "<None>".to_string());
    guard.answers.insert(qid, args.answer);
    info!(
        "submit_answer: qid={}, answer={}",
        qid, answer_preview
    );
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
    let cleared_answers = guard.answers.len();
    guard.answers.clear();
    guard.state = None;
    guard.finished = false;
    guard.skip_requested.store(false, Ordering::Relaxed);
    guard.recording_completed.store(false, Ordering::Relaxed);
    // correct_answers / audio_paths 由 init_container_from_session 重新填充
    info!(
        "reset_test_flow: 已清空 {} 个答案记录，状态已重置（包含 finished / skip / recording 标志）",
        cleared_answers
    );
    Ok(())
}

/// 用户点击"下一题"按钮：
/// 1) 设置 skip_requested 让 run_* 函数跳出当前 sleep
/// 2) 设置 active_stop_flag 让当前 rodio 播放立即停止（切歌）
/// 3) submit_answer 时已经保存作答，无需再处理
#[tauri::command]
pub fn skip_to_next(
    flow: State<'_, FlowGlobal>,
    audio: State<'_, AudioPlaybackState>,
) -> Result<(), String> {
    let guard = flow
        .container
        .inner
        .lock()
        .map_err(|e| format!("锁读取失败: {e}"))?;

    // 触发跳过
    guard.skip_requested.store(true, Ordering::Relaxed);
    // 切歌：取出当前 stop_flag（若存在）置 true
    let stop_flag = audio.active_stop_flag.lock().unwrap().clone();
    drop(guard);
    if let Some(f) = stop_flag {
        f.store(true, Ordering::Relaxed);
    }
    Ok(())
}

/// Q19 用户点击"提前结束录音"按钮：
/// 通知后端 `finish_recording_phases` 立即跳出剩余录音等待，进入评分阶段。
/// 由前端在 `stopRecording` 写入 wav 并 `submit_answer(19, path)` 之后调用。
#[tauri::command]
pub fn notify_recording_completed(flow: State<'_, FlowGlobal>) -> Result<(), String> {
    flow.container.notify_recording_completed();
    Ok(())
}
