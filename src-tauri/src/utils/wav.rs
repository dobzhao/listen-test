//! WAV 工具：读写 wav 文件、生成静音、拼接多段 wav
//!
//! 假设所有 TTS 输出均为 PCM 16bit 单声道。拼接前会校验采样率/声道/位深，
//! 不一致则报错（避免无声或爆音）。

use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WavError {
    #[error("读取 wav 失败: {0}")]
    Read(#[from] hound::Error),
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("采样格式不一致: {actual} != {expected}")]
    SpecMismatch { actual: String, expected: String },
}

/// wav 规范摘要（用于日志与不匹配检测）
#[derive(Debug, Clone)]
pub struct WavInfo {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
}

impl std::fmt::Display for WavInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}Hz/{}ch/{}bit",
            self.sample_rate, self.channels, self.bits_per_sample
        )
    }
}

pub fn read_info(path: &Path) -> Result<WavInfo, WavError> {
    let reader = WavReader::open(path)?;
    let spec = reader.spec();
    Ok(WavInfo {
        sample_rate: spec.sample_rate,
        channels: spec.channels,
        bits_per_sample: spec.bits_per_sample,
    })
}

/// 计算 wav 文件的播放时长（毫秒）
///
/// 通过读取 WAV 头的总采样数与采样率得到。仅读取头部，不解码样本数据。
/// 用于在测试流程中预先计算 PLAYING 阶段的倒计时总长，让前端进度条
/// 在音频播放期间能真实反映剩余时间，而不是卡在 0 秒不动。
pub fn duration_ms(path: &Path) -> Result<u64, WavError> {
    let reader = WavReader::open(path)?;
    let spec = reader.spec();
    let total_samples = reader.duration() as u64;
    let sample_rate = spec.sample_rate as u64;
    if sample_rate == 0 {
        return Err(WavError::SpecMismatch {
            actual: "sample_rate=0".into(),
            expected: "sample_rate>0".into(),
        });
    }
    Ok((total_samples * 1000) / sample_rate)
}

/// 把任意 wav 文件读取为 i16 采样（自动展平多声道为单声道）。
///
/// - 单声道：直接取样
/// - 多声道：取所有声道平均值（简单但足够应付当前 TTS 输出）
pub fn read_to_mono_i16(path: &Path) -> Result<(WavInfo, Vec<i16>), WavError> {
    let mut reader = WavReader::open(path)?;
    let spec = reader.spec();
    let channels = spec.channels as usize;
    let mut samples = Vec::new();
    if spec.sample_format == SampleFormat::Int && spec.bits_per_sample == 16 {
        let mut frame = Vec::with_capacity(channels);
        for s in reader.samples::<i16>() {
            let s = s?;
            frame.push(s);
            if frame.len() == channels {
                let avg = frame.iter().map(|v| *v as i32).sum::<i32>() / channels as i32;
                samples.push(avg as i16);
                frame.clear();
            }
        }
    } else if spec.sample_format == SampleFormat::Float && spec.bits_per_sample == 32 {
        let mut frame = Vec::with_capacity(channels);
        for s in reader.samples::<f32>() {
            let s = s?;
            frame.push(s);
            if frame.len() == channels {
                let avg = frame.iter().copied().sum::<f32>() / channels as f32;
                samples.push((avg * i16::MAX as f32) as i16);
                frame.clear();
            }
        }
    } else {
        return Err(WavError::SpecMismatch {
            actual: format!("{:?}/{}bit", spec.sample_format, spec.bits_per_sample),
            expected: "PCM Int 16bit or Float 32bit".into(),
        });
    }
    let info = WavInfo {
        sample_rate: spec.sample_rate,
        channels: 1,
        bits_per_sample: 16,
    };
    Ok((info, samples))
}

/// 生成一段静音 wav（i16 单声道 0 值）
pub fn generate_silence(spec: &WavSpec, duration_ms: u32) -> Result<Vec<i16>, WavError> {
    let n_samples = (spec.sample_rate as u64 * duration_ms as u64 / 1000) as usize;
    Ok(vec![0i16; n_samples])
}

/// 把多段 wav 合并为一段：所有段必须采样率一致；每段之间插入指定毫秒的静音。
///
/// - input_paths：要拼接的 wav 文件路径列表
/// - output_path：拼接结果输出路径
/// - silence_ms：相邻 wav 之间插入的静音毫秒数
pub fn concatenate_with_silence(
    input_paths: &[&Path],
    output_path: &Path,
    silence_ms: u32,
) -> Result<WavInfo, WavError> {
    if input_paths.is_empty() {
        return Err(WavError::SpecMismatch {
            actual: "empty".into(),
            expected: "at least one wav".into(),
        });
    }

    // 1. 读取第一段，获取 spec
    let (first_info, mut all_samples) = read_to_mono_i16(input_paths[0])?;
    let spec = WavSpec {
        channels: first_info.channels,
        sample_rate: first_info.sample_rate,
        bits_per_sample: first_info.bits_per_sample,
        sample_format: SampleFormat::Int,
    };

    // 2. 拼接其余段（先静音再 wav）
    let silence = generate_silence(&spec, silence_ms)?;
    for path in &input_paths[1..] {
        let (info, samples) = read_to_mono_i16(path)?;
        if info.sample_rate != first_info.sample_rate {
            return Err(WavError::SpecMismatch {
                actual: format!("sample_rate={}", info.sample_rate),
                expected: format!("sample_rate={}", first_info.sample_rate),
            });
        }
        all_samples.extend_from_slice(&silence);
        all_samples.extend_from_slice(&samples);
    }

    // 3. 写入输出文件
    let parent = output_path
        .parent()
        .ok_or_else(|| WavError::SpecMismatch {
            actual: "no parent dir".into(),
            expected: "valid path".into(),
        })?;
    std::fs::create_dir_all(parent)?;
    let mut writer = WavWriter::create(output_path, spec)?;
    for s in &all_samples {
        writer.write_sample(*s)?;
    }
    writer.finalize()?;
    Ok(first_info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn write_test_wav(path: &Path, freq: f32, duration_ms: u32) {
        let spec = WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(path, spec).unwrap();
        let n = (spec.sample_rate as u32 * duration_ms / 1000) as usize;
        for i in 0..n {
            let t = i as f32 / spec.sample_rate as f32;
            let s = (2.0 * std::f32::consts::PI * freq * t).sin();
            writer.write_sample((s * i16::MAX as f32 * 0.3) as i16).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn test_concat_two_wavs() {
        let dir = env::temp_dir().join("peiyuan_wav_test");
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.wav");
        let b = dir.join("b.wav");
        let out = dir.join("out.wav");
        write_test_wav(&a, 440.0, 200);
        write_test_wav(&b, 880.0, 200);
        let info = concatenate_with_silence(&[&a, &b], &out, 100).unwrap();
        assert_eq!(info.sample_rate, 16000);
        assert!(out.exists());
    }
}
