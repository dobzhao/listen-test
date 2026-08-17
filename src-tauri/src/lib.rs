//! 库入口：暴露 `run()` 给 `main.rs` 调用。
//!
//! 把所有 Tauri 注册逻辑放在这里便于跨平台 desktop 复用。

pub mod commands;
pub mod models;
pub mod services;
pub mod utils;

use commands::audio::AudioPlaybackState;
use commands::config::ConfigState;
use commands::recorder::RecorderGlobal;
use commands::test_flow::FlowGlobal;
use commands::test_session::SessionState;
use std::io;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// 初始化日志：stdout + 文件双输出
///
/// 文件路径：`<app_data_dir>/logs/peiyuan.log.YYYY-MM-DD`（按天滚动）
/// 级别：默认 `info`，可被 `RUST_LOG` 覆盖
/// 文件初始化失败时回退到 stdout-only，不阻塞应用启动
/// 返回的 `WorkerGuard` 必须保留到进程结束，drop 时自动 flush
fn init_logging() -> WorkerGuard {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // 文件输出（rolling::daily 自动加日期后缀）。失败时回退到 io::sink()（丢弃），
    // 避免在 WorkerGuard 返回值上让 stdout 与 file 两条路径出现类型不匹配
    let (file_writer, guard) = match utils::path::logs_dir() {
        Ok(logs_dir) => {
            let appender = tracing_appender::rolling::daily(&logs_dir, "peiyuan.log");
            tracing_appender::non_blocking(appender)
        }
        Err(e) => {
            eprintln!("[peiyuan] 无法初始化文件日志 ({e})，仅写入 stdout");
            tracing_appender::non_blocking(io::sink())
        }
    };

    let stdout_layer = fmt::layer()
        .with_writer(io::stdout)
        .with_target(false)
        .compact();

    let file_layer = fmt::layer()
        .with_writer(move || file_writer.clone())
        .with_ansi(false)
        .with_target(false)
        .compact();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();

    guard
}

/// Tauri 应用入口
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // _log_guard 必须活到进程结束；drop 时 non-blocking 自动 flush
    let _log_guard = init_logging();

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(ConfigState::default())
        .manage(SessionState::default())
        .manage(FlowGlobal::default())
        .manage(RecorderGlobal::default())
        .manage(AudioPlaybackState::default())
        .invoke_handler(tauri::generate_handler![
            // 配置
            commands::config::get_config,
            commands::config::save_config,
            commands::config::reset_config,
            commands::config::restore_default_prompt,
            commands::config::restore_default_timing,
            commands::config::restore_default_difficulty_demand,
            commands::config::restore_default_difficulty_level,
            commands::config::restore_default_difficulty,
            commands::config::open_config_dir,
            // 模型连接测试
            commands::llm::test_llm_connection,
            commands::llm::generate_with_llm,
            commands::tts::test_tts_connection,
            commands::stt::test_stt_connection,
            commands::stt::transcribe_audio,
            // 设备
            commands::device::list_input_devices,
            commands::device::list_output_devices,
            commands::device::test_input_device,
            commands::device::test_output_device,
            // 音频播放
            commands::audio::play_audio_file,
            commands::audio::play_audio_background,
            // 测试会话预生成
            commands::test_session::generate_test_session,
            commands::test_session::get_test_session,
            commands::test_session::clear_test_session,
            // 1-19 题测试流程
            commands::test_flow::start_test_flow,
            commands::test_flow::submit_answer,
            commands::test_flow::get_flow_state,
            commands::test_flow::get_answer_set,
            commands::test_flow::reset_test_flow,
            commands::test_flow::skip_to_next,
            commands::test_flow::notify_recording_completed,
            // 录音
            commands::recorder::start_recording,
            commands::recorder::stop_recording,
            commands::recorder::get_audio_level,
            // 评分
            commands::scoring::score_full_test,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
