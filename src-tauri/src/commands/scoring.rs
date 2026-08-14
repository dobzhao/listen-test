//! 判分相关 Tauri commands
//!
//! - `score_full_test`：完整评分入口（1-14 本地 + 15-18 LLM + 19 STT+LLM）

use crate::commands::config::ConfigState;
use crate::commands::test_flow::FlowGlobal;
use crate::commands::test_session::SessionState;
use crate::models::result::TestResult;
use crate::services::scoring::score_full_test as run_scoring;

#[tauri::command]
pub async fn score_full_test(
    config_state: tauri::State<'_, ConfigState>,
    session_state: tauri::State<'_, SessionState>,
    flow: tauri::State<'_, FlowGlobal>,
) -> Result<TestResult, String> {
    // 1. 取配置
    let config = {
        let guard = config_state
            .inner
            .read()
            .map_err(|e| format!("锁读取失败: {e}"))?;
        guard.clone()
    };

    // 2. 取会话
    let session = {
        let guard = session_state
            .inner
            .lock()
            .map_err(|e| format!("锁读取失败: {e}"))?;
        guard.clone().ok_or_else(|| "尚未生成测试会话".to_string())?
    };

    // 3. 取用户作答与正确答案
    let (user_answers, correct_answers, recording_path) = {
        let guard = flow
            .container
            .inner
            .lock()
            .map_err(|e| format!("锁读取失败: {e}"))?;
        let rec = guard.answers.get(&19).cloned().flatten();
        (guard.answers.clone(), guard.correct_answers.clone(), rec)
    };

    // 4. 计算 1-14 题 ID 列表（按升序）
    let mut mcq_ids: Vec<u32> = correct_answers
        .keys()
        .copied()
        .filter(|k| *k <= 14)
        .collect();
    mcq_ids.sort();

    // 5. 执行完整评分
    let result = run_scoring(
        &config,
        &session,
        &user_answers,
        &correct_answers,
        &mcq_ids,
        recording_path.as_deref(),
    )
    .await
    .map_err(|e| format!("评分失败: {e}"))?;

    Ok(result)
}