//! 应用配置数据结构
//!
//! 这些结构会被序列化为 JSON 保存到本地配置文件，并通过 Tauri command
//! 暴露给前端。字段顺序与默认值必须与前端 TypeScript 类型保持一致。

use serde::{Deserialize, Serialize};

/// 单个模型服务（LLM / TTS / STT）的连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// 协议："http" 或 "https"
    #[serde(default = "default_protocol")]
    pub protocol: String,
    /// 主机地址，例如 "127.0.0.1"
    pub host: String,
    /// 端口，例如 8000
    pub port: u16,
    /// API 路径，例如 "/v1/chat/completions"
    pub api_path: String,
    /// 模型名称
    pub model: String,
    /// Authorization Bearer Token
    pub api_key: String,
}

fn default_protocol() -> String {
    "http".to_string()
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            protocol: default_protocol(),
            host: "127.0.0.1".to_string(),
            port: 8000,
            api_path: String::new(),
            model: String::new(),
            api_key: String::new(),
        }
    }
}

impl ModelConfig {
    /// 拼接完整的 endpoint URL
    pub fn endpoint_url(&self) -> String {
        let proto = if self.protocol.is_empty() {
            "http"
        } else {
            &self.protocol
        };
        format!("{}://{}:{}{}", proto, self.host, self.port, self.api_path)
    }
}

/// LLM 调用参数（不同本地模型对参数最优取值不同，开放给用户配置）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmParams {
    pub temperature: f32,
    pub max_tokens: u32,
    pub top_p: f32,
    pub top_k: u32,
}

impl Default for LlmParams {
    fn default() -> Self {
        Self {
            temperature: 1.0,
            max_tokens: 81920,
            top_p: 0.95,
            top_k: 64,
        }
    }
}

/// 5 个 Prompt 模板（全部可在设置界面编辑）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    /// 1-4 题（短对话，每段配 1 题）
    pub q1_4: String,
    /// 5-14 题（长对话/独白，每段配 2 题）
    pub q5_14: String,
    /// 15-18 题（长文本 + 总分表格 + 挖空）
    pub q15_18: String,
    /// 第 15-18 题填空判分 Prompt
    pub q15_18_scoring: String,
    /// 第 19 题转述评分 Prompt
    pub q19_scoring: String,
}

/// 音频相关设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    /// 播放音量 0.0 ~ 1.0
    pub playback_volume: f32,
    /// 麦克风录音音量增益 0.0 ~ 2.0
    pub mic_gain: f32,
    /// TTS 拼接时轮次间隔静音（毫秒）
    pub tts_silence_ms: u32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            playback_volume: 1.0,
            mic_gain: 1.0,
            tts_silence_ms: 400,
        }
    }
}

/// 整个应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub llm: ModelConfig,
    pub tts: ModelConfig,
    pub stt: ModelConfig,
    pub llm_params: LlmParams,
    pub prompts: PromptConfig,
    pub audio: AudioConfig,
}

impl AppConfig {
    /// 默认 LLM api_path
    pub fn default_llm_path() -> &'static str {
        "/v1/chat/completions"
    }

    /// 默认 TTS api_path
    pub fn default_tts_path() -> &'static str {
        "/v1/audio/speech"
    }

    /// 默认 STT api_path
    pub fn default_stt_path() -> &'static str {
        "/v1/audio/transcriptions"
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut llm = ModelConfig::default();
        llm.api_path = AppConfig::default_llm_path().to_string();
        llm.model = "default-llm".to_string();

        let mut tts = ModelConfig::default();
        tts.api_path = AppConfig::default_tts_path().to_string();
        tts.model = "mlx-community/Kokoro-82M-bf16".to_string();

        let mut stt = ModelConfig::default();
        stt.api_path = AppConfig::default_stt_path().to_string();
        stt.model = "mlx-community/whisper-large-v3-turbo-asr-fp16".to_string();

        Self {
            llm,
            tts,
            stt,
            llm_params: LlmParams::default(),
            prompts: default_prompts(),
            audio: AudioConfig::default(),
        }
    }
}

/// 第九节「LLM Prompt 设计（默认模板）」的默认值
///
/// 这些 Prompt 都可以在设置界面编辑，运行时通过 `prompt_engine::render`
/// 替换 `{{PLACEHOLDER}}`。
pub fn default_prompts() -> PromptConfig {
    PromptConfig {
        q1_4: include_str!("../../prompts/q1_4.txt").to_string(),
        q5_14: include_str!("../../prompts/q5_14.txt").to_string(),
        q15_18: include_str!("../../prompts/q15_18.txt").to_string(),
        q15_18_scoring: include_str!("../../prompts/q15_18_scoring.txt").to_string(),
        q19_scoring: include_str!("../../prompts/q19_scoring.txt").to_string(),
    }
}
