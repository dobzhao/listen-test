//! 音频播放 Tauri command（基于 rodio，跨平台一致）
//!
//! - `play_audio_file`：启动非阻塞播放，立即返回
//! - `play_audio_background`：非阻塞播放（设备测试回放录音等）
//! - `play_audio_blocking`：阻塞播放，调用方 await

use crate::services::audio_player::{play_wav_blocking, play_wav_nonblocking};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;
use tracing::{error, info};

/// 全局音频播放状态
pub struct AudioPlaybackState {
    pub is_playing: Arc<Mutex<bool>>,
}

impl Default for AudioPlaybackState {
    fn default() -> Self {
        Self {
            is_playing: Arc::new(Mutex::new(false)),
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
    std::thread::spawn(move || {
        if let Err(e) = play_wav_blocking(&pb) {
            error!(error = %e, "播放失败");
        }
        if let Ok(mut guard) = is_playing.lock() {
            *guard = false;
        }
    });

    Ok(format!("开始播放: {path}"))
}

#[tauri::command]
pub async fn play_audio_blocking(path: String) -> Result<(), String> {
    let pb = PathBuf::from(&path);
    if !pb.exists() {
        return Err(format!("音频文件不存在: {path}"));
    }
    tokio::task::spawn_blocking(move || play_wav_blocking(&pb))
        .await
        .map_err(|e| format!("spawn_blocking 失败: {e}"))?
        .map_err(|e| format!("播放失败: {e}"))?;
    Ok(())
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
