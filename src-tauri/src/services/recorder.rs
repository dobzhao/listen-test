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
    /// 启动录音：samples 是共享 buffer，config 是主线程已经选定的设备配置
    /// （不能再调一次 default_input_config，否则可能返回不同的 sample rate）
    Start {
        samples: Arc<Mutex<Vec<f32>>>,
        config: cpal::SupportedStreamConfig,
    },
    Stop {
        output_path: PathBuf,
        /// 麦克风原始采样率，用于把 samples 重采样到 16kHz 再写 wav
        sample_rate: u32,
        /// 声道数（1=mono；>1 时需要先降混到 mono 再写 wav）
        channels: u16,
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
            WorkerCommand::Start { samples, config } => {
                // 若已有录音，先丢弃
                active = None;
                match start_stream(samples.clone(), config) {
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
                sample_rate,
                channels,
                response,
            } => {
                let result = if let Some(a) = active.take() {
                    drop(a._stream); // 停止流
                    let samples = a.samples.lock().unwrap().clone();
                    write_wav(&samples, sample_rate, channels, &output_path)
                } else {
                    Err(RecorderError::NotStarted)
                };
                let _ = response.send(result);
            }
        }
    }
    info!("录音 worker 线程退出");
}

fn start_stream(
    samples: Arc<Mutex<Vec<f32>>>,
    config: cpal::SupportedStreamConfig,
) -> Result<cpal::Stream, RecorderError> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or(RecorderError::NoInputDevice)?;
    let device_name = device.name().unwrap_or_else(|_| "<未知>".to_string());
    info!(
        device = %device_name,
        sample_rate = config.sample_rate().0,
        channels = config.channels(),
        sample_format = ?config.sample_format(),
        "使用麦克风设备"
    );

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

fn write_wav(
    samples: &[f32],
    from_rate: u32,
    channels: u16,
    output_path: &Path,
) -> Result<(), RecorderError> {
    if samples.is_empty() {
        return Err(RecorderError::UnsupportedConfig("录音数据为空".into()));
    }
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // cpal 把多声道数据交错写入 samples（每帧 channels 个样本）；
    // 写 wav 前先降混到 mono，否则 stereo 数据塞进 mono WAV 会把时长翻倍
    let mono = downmix_to_mono(samples, channels);

    // 麦克风原始采样率通常为 44.1k/48k，统一重采样到 16kHz 后写入 wav
    // （header 与 samples 速率一致，回放和 STT 都能正确处理）
    const TARGET_RATE: u32 = 16_000;
    let pcm = if from_rate == TARGET_RATE {
        mono
    } else {
        linear_resample(&mono, from_rate, TARGET_RATE)
    };
    let spec = WavSpec {
        channels: 1,
        sample_rate: TARGET_RATE,
        bits_per_sample: 16,
        sample_format: HoundSampleFormat::Int,
    };
    let mut writer = WavWriter::create(output_path, spec)
        .map_err(|e| RecorderError::Hound(e.to_string()))?;
    for &v in &pcm {
        let s = (v.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer
            .write_sample(s)
            .map_err(|e| RecorderError::Hound(e.to_string()))?;
    }
    writer
        .finalize()
        .map_err(|e| RecorderError::Hound(e.to_string()))?;
    let secs_in = samples.len() as f64 / (from_rate as f64 * channels.max(1) as f64);
    let secs_out = pcm.len() as f64 / TARGET_RATE as f64;
    info!(
        from_rate,
        to_rate = TARGET_RATE,
        channels,
        samples_in = samples.len(),
        samples_out = pcm.len(),
        seconds_in = format!("{secs_in:.2}"),
        seconds_out = format!("{secs_out:.2}"),
        path = ?output_path,
        "录音已保存"
    );
    Ok(())
}

/// 把 cpal 输出的交错多声道样本降混成 mono：
/// - channels == 1 → 原样返回
/// - channels == 2 → (L + R) / 2
/// - channels > 2 → 取所有声道算术平均
fn downmix_to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    let ch = channels.max(1) as usize;
    if ch <= 1 || samples.len() < ch {
        return samples.to_vec();
    }
    let frames = samples.len() / ch;
    let mut out = Vec::with_capacity(frames);
    for i in 0..frames {
        let mut sum = 0.0f32;
        for c in 0..ch {
            sum += samples[i * ch + c];
        }
        out.push(sum / ch as f32);
    }
    out
}

/// 简单的线性插值重采样（from_rate → to_rate）。
/// 对语音足够；不做低通滤波，仅供录音场景使用。
fn linear_resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if samples.len() < 2 || from_rate == 0 || to_rate == 0 {
        return samples.to_vec();
    }
    let ratio = to_rate as f64 / from_rate as f64;
    let new_len = ((samples.len() as f64) * ratio).round() as usize;
    let mut out = Vec::with_capacity(new_len.max(1));
    for i in 0..new_len {
        let src_pos = i as f64 / ratio;
        let idx = src_pos as usize;
        let frac = (src_pos - idx as f64) as f32;
        let v = if idx + 1 < samples.len() {
            let s0 = samples[idx];
            let s1 = samples[idx + 1];
            s0 + (s1 - s0) * frac
        } else {
            samples[idx]
        };
        out.push(v);
    }
    out
}

impl RecorderState {
    pub fn start_recording(&self) -> Result<(), RecorderError> {
        spawn_worker_if_needed(self)?;

        let samples = Arc::new(Mutex::new(Vec::<f32>::new()));

        // 选定设备配置（只调一次 default_input_config，并把结果传给 worker，
        // 避免主线程拿到的 sample_rate 和 worker 实际开流时拿到的 rate 不一致）
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
        tx.send(WorkerCommand::Start {
            samples: samples.clone(),
            config,
        })
        .map_err(|_| RecorderError::WorkerStopped)?;

        *self.samples.lock().unwrap() = Some(samples);
        self.is_recording.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn stop_recording(&self, output_path: &Path) -> Result<(), RecorderError> {
        // 取一份共享 samples（让 worker 释放它持有的 Arc）
        let _shared = self.samples.lock().unwrap().take();

        // 取出录音时记录的麦克风采样率 + 声道数
        let sample_rate = self.sample_rate.load(Ordering::Relaxed);
        let channels = self.channels.load(Ordering::Relaxed);
        if sample_rate == 0 {
            return Err(RecorderError::UnsupportedConfig(
                "采样率未初始化（start_recording 未成功？）".into(),
            ));
        }

        let (resp_tx, resp_rx) = channel::<Result<(), RecorderError>>();
        let tx = self
            .command_tx
            .lock()
            .unwrap()
            .clone()
            .ok_or(RecorderError::NoWorker)?;
        tx.send(WorkerCommand::Stop {
            output_path: output_path.to_path_buf(),
            sample_rate,
            channels,
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
