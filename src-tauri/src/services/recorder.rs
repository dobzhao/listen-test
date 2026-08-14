//! 麦克风录音模块：cpal 跨平台采集 + hound 编码为 wav
//!
//! 设计：
//! - cpal::Stream 不是 Send + Sync，因此不能在 Tauri State 里直接保存
//! - 采用 worker 线程 + channel 模式：
//!   - `RecorderState`（Send + Sync）持有 Sender 与共享 samples Arc
//!   - worker 线程拥有 cpal::Stream，负责启动/停止录音
//! - 前端通过 `start_recording` / `stop_recording` 命令发送指令
//! - 实时音量通过共享 Arc<Mutex<Vec<f32>>> 计算

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat as CpalSampleFormat;
use hound::WavSpec;
use hound::{SampleFormat as HoundSampleFormat, WavWriter};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use thiserror::Error;
use tracing::{error, info};

/// Worker 线程命令
enum WorkerCommand {
    Start(Arc<Mutex<Vec<f32>>>),
    Stop {
        output_path: PathBuf,
        response: std::sync::mpsc::Sender<Result<(), RecorderError>>,
    },
}

/// 全局录音器：仅保存 Send + Sync 字段
pub struct RecorderState {
    /// 给 worker 线程的命令 sender
    command_tx: Mutex<Option<Sender<WorkerCommand>>>,
    /// 共享采样 buffer（与 worker 共享同一 Arc）
    samples: Mutex<Option<Arc<Mutex<Vec<f32>>>>>,
    /// 当前录音采样率（用于 wav 编码）
    sample_rate: AtomicU32,
    /// 当前录音声道数
    channels: AtomicU16,
    /// 是否正在录音
    is_recording: AtomicBool,
}

impl Default for RecorderState {
    fn default() -> Self {
        Self {
            command_tx: Mutex::new(None),
            samples: Mutex::new(None),
            sample_rate: AtomicU32::new(0),
            channels: AtomicU16::new(0),
            is_recording: AtomicBool::new(false),
        }
    }
}

#[derive(Debug, Error)]
pub enum RecorderError {
    #[error("没有可用的输入设备")]
    NoInputDevice,
    #[error("不支持的流配置: {0}")]
    UnsupportedConfig(String),
    #[error("cpal 错误: {0}")]
    Cpal(String),
    #[error("音频流错误: {0}")]
    PlayStream(String),
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("wav 写入失败: {0}")]
    Hound(String),
    #[error("尚未开始录音")]
    NotStarted,
    #[error("worker 线程未启动")]
    NoWorker,
    #[error("worker 线程已停止")]
    WorkerStopped,
}

/// 启动 worker 线程（首次 start_recording 时自动调用）
fn spawn_worker_if_needed(state: &RecorderState) -> Result<(), RecorderError> {
    let mut guard = state.command_tx.lock().unwrap();
    if guard.is_some() {
        return Ok(());
    }
    let (tx, rx) = channel::<WorkerCommand>();
    *guard = Some(tx.clone());
    drop(guard);

    thread::Builder::new()
        .name("peiyuan-recorder".into())
        .spawn(move || worker_loop(rx))
        .map_err(|e| RecorderError::Cpal(format!("无法启动 worker 线程: {e}")))?;

    info!("录音 worker 线程已启动");
    Ok(())
}

fn worker_loop(rx: std::sync::mpsc::Receiver<WorkerCommand>) {
    // worker 线程持有 cpal::Stream（非 Send + Sync）
    struct Active {
        _stream: cpal::Stream,
        samples: Arc<Mutex<Vec<f32>>>,
    }
    let mut active: Option<Active> = None;

    while let Ok(cmd) = rx.recv() {
        match cmd {
            WorkerCommand::Start(samples) => {
                // 若已有录音，先丢弃
                active = None;
                match start_stream(samples.clone()) {
                    Ok(stream) => {
                        active = Some(Active {
                            _stream: stream,
                            samples,
                        });
                    }
                    Err(e) => {
                        error!(error = ?e, "worker 启动录音失败");
                    }
                }
            }
            WorkerCommand::Stop {
                output_path,
                response,
            } => {
                let result = if let Some(a) = active.take() {
                    drop(a._stream); // 停止流
                    let samples = a.samples.lock().unwrap().clone();
                    write_wav(&samples, &output_path)
                } else {
                    Err(RecorderError::NotStarted)
                };
                let _ = response.send(result);
            }
        }
    }
    info!("录音 worker 线程退出");
}

fn start_stream(samples: Arc<Mutex<Vec<f32>>>) -> Result<cpal::Stream, RecorderError> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or(RecorderError::NoInputDevice)?;
    let device_name = device.name().unwrap_or_else(|_| "<未知>".to_string());
    info!(device = %device_name, "使用麦克风设备");

    let config = device
        .default_input_config()
        .map_err(|e| RecorderError::UnsupportedConfig(format!("default_input_config: {e}")))?;

    let stream = match config.sample_format() {
        CpalSampleFormat::F32 => {
            let samples_clone = samples.clone();
            device
                .build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let mut s = samples_clone.lock().unwrap();
                        s.extend_from_slice(data);
                    },
                    |err| error!(error = ?err, "麦克风流错误"),
                    None,
                )
                .map_err(|e| RecorderError::Cpal(e.to_string()))?
        }
        CpalSampleFormat::I16 => {
            let samples_clone = samples.clone();
            device
                .build_input_stream(
                    &config.into(),
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let mut s = samples_clone.lock().unwrap();
                        s.extend(data.iter().map(|&v| v as f32 / i16::MAX as f32));
                    },
                    |err| error!(error = ?err, "麦克风流错误"),
                    None,
                )
                .map_err(|e| RecorderError::Cpal(e.to_string()))?
        }
        CpalSampleFormat::U16 => {
            let samples_clone = samples.clone();
            device
                .build_input_stream(
                    &config.into(),
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        let mut s = samples_clone.lock().unwrap();
                        s.extend(data.iter().map(|&v| (v as f32 - 32768.0) / 32768.0));
                    },
                    |err| error!(error = ?err, "麦克风流错误"),
                    None,
                )
                .map_err(|e| RecorderError::Cpal(e.to_string()))?
        }
        other => {
            return Err(RecorderError::UnsupportedConfig(format!("{other:?}")))
        }
    };

    stream.play().map_err(|e| RecorderError::PlayStream(e.to_string()))?;
    Ok(stream)
}

fn write_wav(samples: &[f32], output_path: &Path) -> Result<(), RecorderError> {
    if samples.is_empty() {
        return Err(RecorderError::UnsupportedConfig("录音数据为空".into()));
    }
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000, // 录音时统一重采样到 16kHz 简化 STT
        bits_per_sample: 16,
        sample_format: HoundSampleFormat::Int,
    };
    let mut writer = WavWriter::create(output_path, spec)
        .map_err(|e| RecorderError::Hound(e.to_string()))?;
    for &v in samples {
        let s = (v.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer
            .write_sample(s)
            .map_err(|e| RecorderError::Hound(e.to_string()))?;
    }
    writer
        .finalize()
        .map_err(|e| RecorderError::Hound(e.to_string()))?;
    info!(
        samples = samples.len(),
        path = ?output_path,
        "录音已保存"
    );
    Ok(())
}

impl RecorderState {
    pub fn start_recording(&self) -> Result<(), RecorderError> {
        spawn_worker_if_needed(self)?;

        let samples = Arc::new(Mutex::new(Vec::<f32>::new()));

        // 先确定 sample_rate / channels（用于 stop 时编码 wav）
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or(RecorderError::NoInputDevice)?;
        let config = device
            .default_input_config()
            .map_err(|e| RecorderError::UnsupportedConfig(format!("default_input_config: {e}")))?;
        self.sample_rate.store(config.sample_rate().0, Ordering::Relaxed);
        self.channels.store(config.channels(), Ordering::Relaxed);

        let tx = self
            .command_tx
            .lock()
            .unwrap()
            .clone()
            .ok_or(RecorderError::NoWorker)?;
        tx.send(WorkerCommand::Start(samples.clone()))
            .map_err(|_| RecorderError::WorkerStopped)?;

        *self.samples.lock().unwrap() = Some(samples);
        self.is_recording.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn stop_recording(&self, output_path: &Path) -> Result<(), RecorderError> {
        // 取一份共享 samples（让 worker 释放它持有的 Arc）
        let _shared = self.samples.lock().unwrap().take();

        let (resp_tx, resp_rx) = channel::<Result<(), RecorderError>>();
        let tx = self
            .command_tx
            .lock()
            .unwrap()
            .clone()
            .ok_or(RecorderError::NoWorker)?;
        tx.send(WorkerCommand::Stop {
            output_path: output_path.to_path_buf(),
            response: resp_tx,
        })
        .map_err(|_| RecorderError::WorkerStopped)?;

        let result = resp_rx
            .recv()
            .map_err(|_| RecorderError::WorkerStopped)?;
        self.is_recording.store(false, Ordering::Relaxed);
        result
    }

    /// 拉取当前最近 0.5 秒采样的 RMS（0.0 ~ 1.0）
    pub fn current_level(&self) -> f32 {
        let guard = self.samples.lock().unwrap();
        let Some(samples_arc) = guard.as_ref() else {
            return 0.0;
        };
        let samples = samples_arc.lock().unwrap();
        if samples.is_empty() {
            return 0.0;
        }
        let sample_rate = self.sample_rate.load(Ordering::Relaxed) as usize;
        let window = sample_rate / 2;
        let start = samples.len().saturating_sub(window);
        let slice = &samples[start..];
        let sum_sq: f32 = slice.iter().map(|v| v * v).sum();
        let rms = (sum_sq / slice.len() as f32).sqrt();
        (rms * 4.0).clamp(0.0, 1.0)
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }
}
