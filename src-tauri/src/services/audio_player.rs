//! 音频播放服务：基于 rodio 的同步播放
//!
//! 用于在测试流程中代替 webview HTML5 audio，避免：
//! - WebKitGTK / Chromium webview 的 autoplay 限制
//! - asset protocol 跨平台兼容问题
//!
//! 所有播放统一通过系统默认输出设备，跨平台一致。

use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tracing::{error, info};

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("无法解码音频: {0}")]
    Decode(String),
    #[error("无法打开输出设备: {0}")]
    Output(String),
}

/// 同步播放 wav 文件（阻塞直到播放完成或收到停止信号）
///
/// `stop_flag`：当被设为 `true` 时，停止播放并提前返回 Ok(())。
pub fn play_wav_blocking(path: &Path, stop_flag: Arc<AtomicBool>) -> Result<(), AudioError> {
    info!(path = ?path, "开始播放音频");
    let (_stream, handle) = OutputStream::try_default()
        .map_err(|e| AudioError::Output(e.to_string()))?;
    let file = File::open(path)?;
    let source = Decoder::new(BufReader::new(file))
        .map_err(|e| AudioError::Decode(e.to_string()))?;
    let sink = Sink::try_new(&handle)
        .map_err(|e| AudioError::Output(e.to_string()))?;
    sink.append(source);

    // 轮询停止信号；rodio 0.19 的 sleep_until_end 没有可中断 API，
    // 通过定期检查 stop_flag 来实现"切歌"功能。
    while !sink.empty() {
        if stop_flag.load(Ordering::Relaxed) {
            info!(path = ?path, "收到停止信号，停止播放");
            sink.stop();
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    info!(path = ?path, "音频播放结束");
    Ok(())
}

/// 非阻塞播放：调用者不应 await 此函数，由后台线程播放
pub fn play_wav_nonblocking(path: std::path::PathBuf) {
    std::thread::spawn(move || {
        let stop_flag = Arc::new(AtomicBool::new(false));
        if let Err(e) = play_wav_blocking(&path, stop_flag) {
            error!(error = %e, "非阻塞播放失败");
        }
    });
}
