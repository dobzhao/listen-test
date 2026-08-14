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
use tracing_subscriber::EnvFilter;

/// Tauri 应用入口
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志（默认 INFO 级别，RUST_LOG 可覆盖）
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .compact()
        .init();

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
            commands::audio::play_audio_blocking,
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
