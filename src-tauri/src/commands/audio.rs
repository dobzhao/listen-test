//! 音频播放 Tauri command（基于 rodio，跨平台一致）
//!
//! - `play_audio_file`：启动非阻塞播放，立即返回
//! - `play_audio_background`：非阻塞播放（设备测试回放录音等）
//! - `play_audio_blocking`：阻塞播放，调用方 await
//! - `stop_audio`：停止当前由测试流程发起的播放（下一题切歌用）

use crate::services::audio_player::{play_wav_blocking, play_wav_nonblocking};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::State;
use tracing::{error, info};

/// 全局音频播放状态
pub struct AudioPlaybackState {
    pub is_playing: Arc<Mutex<bool>>,
    /// 当前由测试流程启动的播放的停止信号，stop_audio 命令可借此切歌
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
pub async fn play_audio_blocking(path: String) -> Result<(), String> {
    let pb = PathBuf::from(&path);
    if !pb.exists() {
        return Err(format!("音频文件不存在: {path}"));
    }
    let stop_flag = Arc::new(AtomicBool::new(false));
    tokio::task::spawn_blocking(move || play_wav_blocking(&pb, stop_flag))
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

/// 停止当前由测试流程发起的播放（不影响设备测试的回放）
#[tauri::command]
pub fn stop_audio(state: State<'_, AudioPlaybackState>) -> Result<(), String> {
    let flag = state.active_stop_flag.lock().unwrap().clone();
    if let Some(f) = flag {
        f.store(true, Ordering::Relaxed);
    }
    Ok(())
}
