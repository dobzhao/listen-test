//! 后端计时器：驱动测试流程的精确倒计时
//!
//! 设计要点：
//! - 用 `tokio::time::Instant` 计算真实剩余时间，避免 interval 漂移
//! - 每 100ms 通过 Tauri Event 向前端推送 `timer-tick` 事件
//! - 计时结束时推送 `phase-finished` 事件
//! - 使用 RAII（Drop）确保 panic / cancel 时也能停止计时任务

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::task::JoinHandle;
use tracing::{debug, warn};

pub const TIMER_TICK_EVENT: &str = "test-timer-tick";
pub const PHASE_FINISHED_EVENT: &str = "test-phase-finished";

/// 当前阶段标识（前端据此切 UI 子组件）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// 题目介绍（仅 1-4 题首次显示，10 秒）
    Intro,
    /// 准备阶段（1-4 题 5 秒，5-14 题 10 秒，15-19 题 30 秒）
    Prepare,
    /// 播放阶段（1-4 题 1 次播放，5-14 题 2 次播放含 2 秒间隔，15-19 题 3 次播放含 3 秒间隔）
    Playing,
    /// 作答阶段（10 秒，1-14 题）
    Answering,
    /// 挖空填写阶段（15-18 题，90 秒）
    FillBlank,
    /// 默读准备阶段（19 题录音前，120 秒）
    RecallPrep,
    /// 录音阶段（19 题，90 秒）
    Recording,
}

impl Phase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Phase::Intro => "intro",
            Phase::Prepare => "prepare",
            Phase::Playing => "playing",
            Phase::Answering => "answering",
            Phase::FillBlank => "fill_blank",
            Phase::RecallPrep => "recall_prep",
            Phase::Recording => "recording",
        }
    }
}

/// 单次 tick payload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerTickPayload {
    pub phase: Phase,
    /// 当前阶段已过毫秒
    pub elapsed_ms: u64,
    /// 当前阶段总毫秒
    pub duration_ms: u64,
    /// 剩余毫秒
    pub remaining_ms: u64,
    /// 进度 0.0 ~ 1.0
    pub progress: f32,
}

/// 计时器句柄（持有 JoinHandle，可在外部取消）
pub struct PhaseTimer {
    handle: Option<JoinHandle<()>>,
    /// 用于 cancel：把 cancelled 设为 true
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl PhaseTimer {
    /// 启动一个 phase 计时器。返回的 PhaseTimer 在被 drop 或调用 cancel() 时会停止。
    ///
    /// - `phase`：要发送的阶段标识
    /// - `duration_ms`：阶段总时长（毫秒）
    /// - `tick_interval_ms`：tick 推送间隔（默认 100ms）
    pub fn start(
        app: AppHandle,
        phase: Phase,
        duration_ms: u64,
        tick_interval_ms: u64,
    ) -> Self {
        let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancel_clone = cancelled.clone();

        let handle = tokio::spawn(async move {
            let start = Instant::now();
            let total = duration_ms;
            let interval = Duration::from_millis(tick_interval_ms.max(20));

            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                ticker.tick().await;
                if cancel_clone.load(std::sync::atomic::Ordering::Relaxed) {
                    debug!(phase = phase.as_str(), "计时器已取消");
                    return;
                }

                let elapsed = start.elapsed().as_millis() as u64;
                if elapsed >= total {
                    // 阶段结束，发送最后一次 tick（remaining=0）
                    let payload = TimerTickPayload {
                        phase: phase.clone(),
                        elapsed_ms: total,
                        duration_ms: total,
                        remaining_ms: 0,
                        progress: 1.0,
                    };
                    if let Err(e) = app.emit(TIMER_TICK_EVENT, &payload) {
                        warn!(error = %e, "发送最终 tick 失败");
                    }
                    // 发送阶段结束事件
                    if let Err(e) = app.emit(PHASE_FINISHED_EVENT, &phase) {
                        warn!(error = %e, "发送 phase-finished 失败");
                    }
                    debug!(phase = phase.as_str(), "phase 完成");
                    return;
                }

                let remaining = total - elapsed;
                let progress = elapsed as f32 / total as f32;
                let payload = TimerTickPayload {
                    phase: phase.clone(),
                    elapsed_ms: elapsed,
                    duration_ms: total,
                    remaining_ms: remaining,
                    progress,
                };
                if let Err(e) = app.emit(TIMER_TICK_EVENT, &payload) {
                    warn!(error = %e, "发送 tick 失败");
                }
            }
        });

        Self {
            handle: Some(handle),
            cancelled,
        }
    }

    /// 取消计时器（不等待 join）
    pub fn cancel(&mut self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Drop for PhaseTimer {
    fn drop(&mut self) {
        self.cancel();
        // 不等待 handle（避免长时间阻塞）
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}
