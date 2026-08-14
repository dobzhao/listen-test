// 与 Rust 后端 models/config.rs 保持字段一一对应

export interface ModelConfig {
  /** 协议："http" 或 "https" */
  protocol: "http" | "https";
  host: string;
  port: number;
  api_path: string;
  model: string;
  api_key: string;
}

export interface LlmParams {
  temperature: number;
  max_tokens: number;
  top_p: number;
  top_k: number;
}

export interface PromptConfig {
  q1_4: string;
  q5_14: string;
  q15_18: string;
  q15_18_scoring: string;
  q19_scoring: string;
}

export interface AudioConfig {
  playback_volume: number;
  mic_gain: number;
  tts_silence_ms: number;
}

/**
 * 测试流程各阶段时长（毫秒）。
 * 与 Rust `models::config::TimingConfig` 字段一一对应。
 * RECORDING 时长不在此配置，由后端固定为 90 秒。
 */
export interface TimingConfig {
  intro_ms: number;
  short_dialogue_prepare_ms: number;
  short_dialogue_answer_ms: number;
  group_prepare_ms: number;
  group_pause_ms: number;
  group_answer_ms: number;
  retell_prepare_ms: number;
  retell_pause_ms: number;
  retell_fill_blank_ms: number;
  retell_recall_prep_ms: number;
}

export interface AppConfig {
  llm: ModelConfig;
  tts: ModelConfig;
  stt: ModelConfig;
  llm_params: LlmParams;
  prompts: PromptConfig;
  audio: AudioConfig;
  timing: TimingConfig;
}

export interface ConfigResponse {
  config: AppConfig;
  config_path: string;
}

export type PromptKey =
  | "q1_4"
  | "q5_14"
  | "q15_18"
  | "q15_18_scoring"
  | "q19_scoring";

export const PROMPT_KEY_LABELS: Record<PromptKey, string> = {
  q1_4: "1-4 题出题 Prompt（短对话）",
  q5_14: "5-14 题出题 Prompt（长对话/独白）",
  q15_18: "15-18 题出题 Prompt（长文本+表格+挖空）",
  q15_18_scoring: "15-18 题填空判分 Prompt",
  q19_scoring: "19 题转述评分 Prompt",
};

export const PROMPT_PLACEHOLDERS: Record<PromptKey, Array<{ key: string; desc: string }>> = {
  q1_4: [{ key: "QUESTION_COUNT", desc: "生成题目数量（固定 4）" }],
  q5_14: [],
  q15_18: [],
  q15_18_scoring: [
    { key: "ORIGINAL_TEXT", desc: "15-19 题听力材料原文" },
    { key: "ANSWERS", desc: "用户填写的 4 个答案（JSON 字符串）" },
  ],
  q19_scoring: [
    { key: "ORIGINAL_TEXT", desc: "15-19 题听力材料原文" },
    { key: "STT_RESULT", desc: "用户第 19 题录音经 STT 转写后的文本" },
  ],
};

export const DEFAULT_LLM_API_PATH = "/v1/chat/completions";
export const DEFAULT_TTS_API_PATH = "/v1/audio/speech";
export const DEFAULT_STT_API_PATH = "/v1/audio/transcriptions";

export const DEFAULT_TTS_MODEL = "mlx-community/Kokoro-82M-bf16";
export const DEFAULT_STT_MODEL = "mlx-community/whisper-large-v3-turbo-asr-fp16";

export function defaultAppConfig(): AppConfig {
  return {
    llm: {
      protocol: "http",
      host: "127.0.0.1",
      port: 8000,
      api_path: DEFAULT_LLM_API_PATH,
      model: "default-llm",
      api_key: "",
    },
    tts: {
      protocol: "http",
      host: "127.0.0.1",
      port: 8000,
      api_path: DEFAULT_TTS_API_PATH,
      model: DEFAULT_TTS_MODEL,
      api_key: "",
    },
    stt: {
      protocol: "http",
      host: "127.0.0.1",
      port: 8000,
      api_path: DEFAULT_STT_API_PATH,
      model: DEFAULT_STT_MODEL,
      api_key: "",
    },
    llm_params: {
      temperature: 1.0,
      max_tokens: 81920,
      top_p: 0.95,
      top_k: 64,
    },
    prompts: {
      q1_4: "",
      q5_14: "",
      q15_18: "",
      q15_18_scoring: "",
      q19_scoring: "",
    },
    audio: {
      playback_volume: 1.0,
      mic_gain: 1.0,
      tts_silence_ms: 400,
    },
    timing: {
      intro_ms: 10000,
      short_dialogue_prepare_ms: 5000,
      short_dialogue_answer_ms: 10000,
      group_prepare_ms: 10000,
      group_pause_ms: 2000,
      group_answer_ms: 10000,
      retell_prepare_ms: 30000,
      retell_pause_ms: 3000,
      retell_fill_blank_ms: 90000,
      retell_recall_prep_ms: 120000,
    },
  };
}
