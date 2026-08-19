//! 音频生成流水线：为一次测试的所有对话/独白/转述材料合成 TTS 音频
//!
//! 输出 wav 文件保存到 `session_cache_dir/audio/`：
//! - `q1.wav` .. `q4.wav`：第 1-4 题短对话
//! - `d1.wav` .. `d4.wav`：第 5-12 题长对话（每段配 2 题，2 次播放复用同一文件）
//! - `m1.wav`：第 13-14 题独白
//! - `retell.wav`：第 15-19 题听力材料

use crate::models::config::ModelConfig;
use crate::models::question::{
    DialogueTurn, LongDialogue, Monologue, RetellMaterial, ShortDialogue,
};
use crate::services::tts_service::{synthesize_dialogue, synthesize_one, TtsError};
use std::path::Path;
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum AudioPipelineError {
    #[error("TTS 错误: {0}")]
    Tts(#[from] TtsError),
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
}

/// 把所有题目统一合成音频，返回 audio_paths 映射（key -> 绝对路径）
///
/// audio_paths 由调用方合并到 TestSession.audio_paths
pub async fn synthesize_all(
    client: &reqwest::Client,
    tts_cfg: &ModelConfig,
    short_dialogues: &[ShortDialogue],
    long_dialogues: &[LongDialogue],
    monologue: &Monologue,
    retell: &RetellMaterial,
    cache_dir: &Path,
    silence_ms: u32,
) -> Result<std::collections::HashMap<String, String>, AudioPipelineError> {
    let audio_dir = cache_dir.join("audio");
    std::fs::create_dir_all(&audio_dir)?;

    let tmp_dir = cache_dir.join("tmp");
    std::fs::create_dir_all(&tmp_dir)?;

    let mut paths = std::collections::HashMap::new();

    // 1-4 题短对话
    for (idx, d) in short_dialogues.iter().enumerate() {
        let out = audio_dir.join(format!("q{}.wav", idx + 1));
        let turns: Vec<DialogueTurn> = d.dialogue.clone();
        synthesize_dialogue(client, tts_cfg, &turns, &tmp_dir, &out, silence_ms).await?;
        info!(q = idx + 1, path = ?out, "短对话音频已生成");
        paths.insert(format!("q{}", idx + 1), out.to_string_lossy().to_string());
    }

    // 5-12 题长对话
    for (idx, d) in long_dialogues.iter().enumerate() {
        let out = audio_dir.join(format!("d{}.wav", idx + 1));
        let turns: Vec<DialogueTurn> = d.dialogue.clone();
        synthesize_dialogue(client, tts_cfg, &turns, &tmp_dir, &out, silence_ms).await?;
        info!(d = idx + 1, path = ?out, "长对话音频已生成");
        paths.insert(format!("d{}", idx + 1), out.to_string_lossy().to_string());
    }

    // 13-14 题独白
    let mono_out = audio_dir.join("m1.wav");
    let bytes = synthesize_one(client, tts_cfg, &monologue.text, "af_heart", 1.0).await?;
    std::fs::write(&mono_out, &bytes)?;
    info!(path = ?mono_out, "独白音频已生成");
    paths.insert("m1".to_string(), mono_out.to_string_lossy().to_string());

    // 15-19 题听力材料
    let retell_out = audio_dir.join("retell.wav");
    let bytes = synthesize_one(client, tts_cfg, &retell.passage, "af_heart", 1.0).await?;
    std::fs::write(&retell_out, &bytes)?;
    info!(path = ?retell_out, "转述材料音频已生成");
    paths.insert("retell".to_string(), retell_out.to_string_lossy().to_string());

    Ok(paths)
}

