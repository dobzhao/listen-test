//! 设备相关 Tauri commands
//!
//! - `list_input_devices` / `list_output_devices`：列出系统中可用的麦克风/扬声器
//! - `test_input_device`：使用指定设备录制 N 秒音频并保存为 wav，返回路径
//! - `test_output_device`：使用指定输出设备播放 440Hz 测试音（基于 rodio）

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat as CpalSampleFormat;
use hound::{SampleFormat as HoundSampleFormat, WavSpec, WavWriter};
use rodio::source::SineWave;
use rodio::{OutputStream, Sink, Source};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tracing::{error, info};

#[derive(Debug, Serialize, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub is_default: bool,
}

fn find_input_device(name: Option<&str>) -> Result<cpal::Device, String> {
    let host = cpal::default_host();
    if let Some(target_name) = name {
        if let Ok(devices) = host.input_devices() {
            for d in devices {
                if let Ok(n) = d.name() {
                    if n == target_name {
                        return Ok(d);
                    }
                }
            }
        }
        return Err(format!("找不到输入设备: {target_name}"));
    }
    host.default_input_device()
        .ok_or_else(|| "系统没有可用的默认输入设备".to_string())
}

fn find_output_device(name: Option<&str>) -> Result<cpal::Device, String> {
    let host = cpal::default_host();
    if let Some(target_name) = name {
        if let Ok(devices) = host.output_devices() {
            for d in devices {
                if let Ok(n) = d.name() {
                    if n == target_name {
                        return Ok(d);
                    }
                }
            }
        }
        return Err(format!("找不到输出设备: {target_name}"));
    }
    host.default_output_device()
        .ok_or_else(|| "系统没有可用的默认输出设备".to_string())
}

#[tauri::command]
pub fn list_input_devices() -> Result<Vec<DeviceInfo>, String> {
    let host = cpal::default_host();
    let mut devices = Vec::new();
    let default_name = host
        .default_input_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_default();
    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            let name = device.name().unwrap_or_else(|_| "<未知设备>".to_string());
            devices.push(DeviceInfo {
                is_default: name == default_name,
                name,
            });
        }
    }
    Ok(devices)
}

#[tauri::command]
pub fn list_output_devices() -> Result<Vec<DeviceInfo>, String> {
    let host = cpal::default_host();
    let mut devices = Vec::new();
    let default_name = host
        .default_output_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_default();
    if let Ok(output_devices) = host.output_devices() {
        for device in output_devices {
            let name = device.name().unwrap_or_else(|_| "<未知设备>".to_string());
            devices.push(DeviceInfo {
                is_default: name == default_name,
                name,
            });
        }
    }
    Ok(devices)
}

#[derive(Debug, serde::Deserialize)]
pub struct TestInputArgs {
    pub device_name: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, serde::Serialize)]
pub struct TestInputResponse {
    pub output_path: String,
    pub duration_ms: u64,
    pub sample_count: usize,
}

/// 使用指定输入设备录制 N 秒音频并保存为 wav
#[tauri::command]
pub fn test_input_device(
    app: AppHandle,
    args: TestInputArgs,
) -> Result<TestInputResponse, String> {
    let duration_ms = args.duration_ms.unwrap_or(3000);
    let device = find_input_device(args.device_name.as_deref())?;
    let device_name = device.name().unwrap_or_else(|_| "<未知>".to_string());
    info!(device = %device_name, duration_ms, "测试输入设备：开始录音");

    let config = device
        .default_input_config()
        .map_err(|e| format!("无法获取输入流配置: {e}"))?;
    let sample_rate = config.sample_rate().0;
    let channels = config.channels();

    let (tx, rx) = mpsc::channel::<Vec<f32>>();
    let stream_config: cpal::StreamConfig = config.clone().into();

    let stream = match config.sample_format() {
        CpalSampleFormat::F32 => device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let _ = tx.send(data.to_vec());
                },
                |err| error!(error = ?err, "测试输入设备：麦克风流错误"),
                None,
            )
            .map_err(|e| format!("无法构建输入流: {e}"))?,
        CpalSampleFormat::I16 => {
            let tx2 = tx.clone();
            device
                .build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let f: Vec<f32> =
                            data.iter().map(|&v| v as f32 / i16::MAX as f32).collect();
                        let _ = tx2.send(f);
                    },
                    |err| error!(error = ?err, "测试输入设备：麦克风流错误"),
                    None,
                )
                .map_err(|e| format!("无法构建输入流: {e}"))?
        }
        CpalSampleFormat::U16 => {
            let tx2 = tx.clone();
            device
                .build_input_stream(
                    &stream_config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        let f: Vec<f32> = data
                            .iter()
                            .map(|&v| (v as f32 - 32768.0) / 32768.0)
                            .collect();
                        let _ = tx2.send(f);
                    },
                    |err| error!(error = ?err, "测试输入设备：麦克风流错误"),
                    None,
                )
                .map_err(|e| format!("无法构建输入流: {e}"))?
        }
        other => return Err(format!("不支持的采样格式: {other:?}")),
    };

    stream.play().map_err(|e| format!("无法启动输入流: {e}"))?;

    // 在主线程上收集采样，到时长后跳出循环
    let mut samples = Vec::new();
    let collect_start = std::time::Instant::now();
    let target_samples = (sample_rate as u64 * duration_ms / 1000) as usize;
    while collect_start.elapsed() < Duration::from_millis(duration_ms) {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(chunk) => samples.extend(chunk),
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        // 保险：如果采集到的样本量已超过目标，提前结束
        if samples.len() >= target_samples {
            break;
        }
    }
    drop(stream); // 停止录音

    if samples.is_empty() {
        return Err("录音数据为空（请检查麦克风权限与设备是否被占用）".into());
    }

    // 转 mono
    let mono: Vec<i16> = if channels == 1 {
        samples
            .iter()
            .map(|&v| (v.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
            .collect()
    } else {
        let ch = channels as usize;
        samples
            .chunks(ch)
            .map(|c| {
                let avg = c.iter().copied().sum::<f32>() / ch as f32;
                (avg.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
            })
            .collect()
    };

    // 写入 wav 到应用缓存目录
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法解析应用数据目录: {e}"))?;
    let cache_dir = data_dir.join("device-tests");
    std::fs::create_dir_all(&cache_dir).ok();
    let output_path: PathBuf = cache_dir.join(format!(
        "input-{}.wav",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: HoundSampleFormat::Int,
    };
    let mut writer = WavWriter::create(&output_path, spec)
        .map_err(|e| format!("无法创建 wav 文件: {e}"))?;
    for s in &mono {
        writer
            .write_sample(*s)
            .map_err(|e| format!("写入 wav 失败: {e}"))?;
    }
    writer.finalize().map_err(|e| format!("finalize wav 失败: {e}"))?;

    info!(path = ?output_path, samples = mono.len(), "测试输入设备：录音完成");
    Ok(TestInputResponse {
        output_path: output_path.to_string_lossy().to_string(),
        duration_ms,
        sample_count: mono.len(),
    })
}

#[derive(Debug, serde::Deserialize)]
pub struct TestOutputArgs {
    pub device_name: Option<String>,
    pub duration_ms: Option<u64>,
}

/// 使用指定输出设备播放 440Hz 测试音（基于 rodio）
#[tauri::command]
pub fn test_output_device(args: TestOutputArgs) -> Result<String, String> {
    let duration_ms = args.duration_ms.unwrap_or(1500);
    let device = find_output_device(args.device_name.as_deref())?;
    let device_name = device.name().unwrap_or_else(|_| "<未知>".to_string());
    info!(device = %device_name, duration_ms, "测试输出设备：开始播放");

    // rodio 0.19：基于指定 cpal 设备创建 OutputStream
    let (_stream, handle) = OutputStream::try_from_device(&device)
        .map_err(|e| format!("无法打开输出设备 {device_name}: {e}"))?;
    let sink = Sink::try_new(&handle).map_err(|e| format!("无法创建音频 sink: {e}"))?;

    let source = SineWave::new(440.0)
        .take_duration(Duration::from_millis(duration_ms))
        .amplify(0.5);

    sink.append(source);
    sink.sleep_until_end();

    info!("测试输出设备：播放完成");
    Ok(format!(
        "已通过 {device_name} 播放 {duration_ms}ms 440Hz 测试音"
    ))
}
