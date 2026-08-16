//! 音频播放 Tauri command（基于 rodio，跨平台一致）
//!
//! - `play_audio_file`：启动非阻塞播放，立即返回
//! - `play_audio_background`：非阻塞播放（设备测试回放录音等）
//! - 「下一题」切歌：`skip_to_next` 通过 `AudioPlaybackState.active_stop_flag` 设置停止信号

use crate::services::audio_player::{play_wav_blocking, play_wav_nonblocking};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tauri::State;
use tracing::{error, info};

/// 全局音频播放状态
pub struct AudioPlaybackState {
    pub is_playing: Arc<Mutex<bool>>,
    /// 当前由测试流程启动的播放的停止信号，`skip_to_next` 通过此字段切歌
    pub active_stop_flag: Arc<Mutex<Option<Arc<AtomicBool>>>>,
}

impl Default for AudioPlaybackState {
    fn default() -> Self {
        Self {
            is_playing: Arc::new(Mutex::new(false)),
            active_stop_flag: Arc::new(Mutex::new(None)),
        }
    }
}

#[tauri::command]
pub async fn play_audio_file(
    state: State<'_, AudioPlaybackState>,
    path: String,
) -> Result<String, String> {
    info!(path = %path, "play_audio_file: 启动后台播放");
    let pb = PathBuf::from(&path);
    if !pb.exists() {
        return Err(format!("音频文件不存在: {path}"));
    }

    {
        let mut guard = state.is_playing.lock().unwrap();
        *guard = true;
    }

    let is_playing = state.is_playing.clone();
    let stop_flag = Arc::new(AtomicBool::new(false));
    {
        let mut slot = state.active_stop_flag.lock().unwrap();
        *slot = Some(stop_flag.clone());
    }
    let stop_for_cleanup = state.active_stop_flag.clone();

    std::thread::spawn(move || {
        if let Err(e) = play_wav_blocking(&pb, stop_flag) {
            error!(error = %e, "播放失败");
        }
        if let Ok(mut guard) = is_playing.lock() {
            *guard = false;
        }
        if let Ok(mut slot) = stop_for_cleanup.lock() {
            *slot = None;
        }
    });

    Ok(format!("开始播放: {path}"))
}

#[tauri::command]
pub fn play_audio_background(path: String) -> Result<(), String> {
    let pb = PathBuf::from(&path);
    if !pb.exists() {
        return Err(format!("音频文件不存在: {path}"));
    }
    play_wav_nonblocking(pb);
    Ok(())
}
