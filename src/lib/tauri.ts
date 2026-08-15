// Tauri invoke 封装与命令常量集中管理

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppConfig, ConfigResponse, TimingConfig } from "@/types/config";
import type { TestSession } from "@/types/question";

// ===== 配置相关 =====

export async function getConfig(): Promise<ConfigResponse> {
  return invoke<ConfigResponse>("get_config");
}

export async function saveConfig(config: AppConfig): Promise<void> {
  await invoke("save_config", { config });
}

export async function resetConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("reset_config");
}

export async function restoreDefaultPrompt(key: string): Promise<string> {
  return invoke<string>("restore_default_prompt", { args: { key } });
}

export async function restoreDefaultTiming(): Promise<TimingConfig> {
  return invoke<TimingConfig>("restore_default_timing");
}

export async function openConfigDir(): Promise<string> {
  return invoke<string>("open_config_dir");
}

// ===== 模型连接测试 =====

export async function testLlmConnection(llm: AppConfig["llm"]): Promise<string> {
  return invoke<string>("test_llm_connection", { config: llm });
}

export async function testTtsConnection(tts: AppConfig["tts"]): Promise<string> {
  return invoke<string>("test_tts_connection", { config: tts });
}

export async function testSttConnection(stt: AppConfig["stt"]): Promise<string> {
  return invoke<string>("test_stt_connection", { config: stt });
}

export async function generateWithLlm(
  config: AppConfig["llm"],
  params: AppConfig["llm_params"],
  template: string,
  vars: Record<string, string>
): Promise<string> {
  return invoke<string>("generate_with_llm", {
    config,
    params,
    template,
    vars,
  });
}

export async function transcribeAudio(
  config: AppConfig["stt"],
  wavPath: string,
  language?: string
): Promise<string> {
  return invoke<string>("transcribe_audio", {
    config,
    wavPath,
    language: language ?? null,
  });
}

// ===== 设备 =====

export interface DeviceInfo {
  name: string;
  is_default: boolean;
}

export async function listInputDevices(): Promise<DeviceInfo[]> {
  return invoke<DeviceInfo[]>("list_input_devices");
}

export async function listOutputDevices(): Promise<DeviceInfo[]> {
  return invoke<DeviceInfo[]>("list_output_devices");
}

export async function testInputDevice(args: {
  deviceName: string | null;
  durationMs?: number;
}): Promise<{ outputPath: string; durationMs: number; sampleCount: number }> {
  return invoke("test_input_device", {
    args: {
      device_name: args.deviceName,
      duration_ms: args.durationMs ?? null,
    },
  });
}

export async function testOutputDevice(args: {
  deviceName: string | null;
  durationMs?: number;
}): Promise<string> {
  return invoke("test_output_device", {
    args: {
      device_name: args.deviceName,
      duration_ms: args.durationMs ?? null,
    },
  });
}

// ===== 测试会话 =====

export interface ProgressPayload {
  stage: "llm_q1_4" | "llm_q5_14" | "llm_q15_18" | "tts" | "done" | string;
  message: string;
  /** 0.0 ~ 1.0 */
  progress: number;
}

export async function generateTestSession(): Promise<TestSession> {
  return invoke<TestSession>("generate_test_session");
}

export async function getTestSession(): Promise<TestSession | null> {
  return invoke<TestSession | null>("get_test_session");
}

export async function clearTestSession(): Promise<void> {
  await invoke("clear_test_session");
}

/**
 * 订阅测试会话预生成进度事件。
 * 返回 unlisten 函数用于取消订阅。
 */
export async function onGenerationProgress(
  handler: (payload: ProgressPayload) => void
): Promise<UnlistenFn> {
  return listen<ProgressPayload>("test-generation-progress", (e) =>
    handler(e.payload)
  );
}

// ===== 测试流程（1-14 题） =====

export type TestPhase =
  | "intro"
  | "prepare"
  | "playing"
  | "answering"
  | "fill_blank"
  | "recall_prep"
  | "recording";

export interface TimerTickPayload {
  phase: TestPhase;
  elapsedMs: number;
  durationMs: number;
  remainingMs: number;
  progress: number;
}

export interface FlowStatePayload {
  questionIndex: number;
  phase: TestPhase;
  progress: number;
  audioPath: string | null;
  isGroup: boolean;
  questionInGroup: number;
}

export interface AudioPlayPayload {
  path: string | null;
  loop: boolean;
}

export interface FlowFinishedPayload {
  ok: boolean;
  completed?: number;
  error?: string;
}

export async function startTestFlow(): Promise<{ ok: boolean }> {
  return invoke("start_test_flow");
}

export async function submitAnswer(
  questionId: number,
  answer: string | null
): Promise<void> {
  await invoke("submit_answer", { args: { questionId, answer } });
}

export async function getFlowState(): Promise<{
  state: FlowStatePayload | null;
  finished: boolean;
}> {
  return invoke("get_flow_state");
}

export async function getAnswerSet(): Promise<{
  answers: Record<number, string | null>;
  correctAnswers: Record<number, string>;
}> {
  return invoke("get_answer_set");
}

export async function resetTestFlow(): Promise<void> {
  await invoke("reset_test_flow");
}

/**
 * 用户在测试过程中点击"下一题"按钮：
 * - 立即停止当前 rodio 播放（切歌）
 * - 让后端 run_* 流程跳出当前 sleep 并进入下一段
 */
export async function skipToNext(): Promise<void> {
  await invoke("skip_to_next");
}

/**
 * Q19 用户点击"提前结束录音"按钮：
 * - 通知后端 finish_recording_phases 立即跳出剩余录音等待
 * - 进入评分阶段，无需再等满 90 秒
 */
export async function notifyRecordingCompleted(): Promise<void> {
  await invoke("notify_recording_completed");
}

export async function onTimerTick(
  handler: (payload: TimerTickPayload) => void
): Promise<UnlistenFn> {
  return listen<TimerTickPayload>("test-timer-tick", (e) =>
    handler(e.payload)
  );
}

export async function onPhaseFinished(
  handler: (phase: TestPhase) => void
): Promise<UnlistenFn> {
  return listen<TestPhase>("test-phase-finished", (e) => handler(e.payload));
}

export async function onFlowState(
  handler: (payload: FlowStatePayload) => void
): Promise<UnlistenFn> {
  return listen<FlowStatePayload>("test-flow-state", (e) =>
    handler(e.payload)
  );
}

export async function onAudioPlay(
  handler: (payload: AudioPlayPayload) => void
): Promise<UnlistenFn> {
  return listen<AudioPlayPayload>("test-audio-play", (e) =>
    handler(e.payload)
  );
}

export async function onFlowFinished(
  handler: (payload: FlowFinishedPayload) => void
): Promise<UnlistenFn> {
  return listen<FlowFinishedPayload>("test-flow-finished", (e) =>
    handler(e.payload)
  );
}

// ===== 音频播放 =====

export async function playAudioBackground(path: string): Promise<string> {
  return invoke<string>("play_audio_background", { path });
}

export async function playAudioFile(path: string): Promise<string> {
  return invoke<string>("play_audio_file", { path });
}

// ===== 录音 =====

export async function startRecording(): Promise<void> {
  await invoke("start_recording");
}

// ===== 评分 =====

import type { TestResult } from "@/types/result";

export async function scoreFullTest(): Promise<TestResult> {
  return invoke<TestResult>("score_full_test");
}

export async function stopRecording(outputPath: string): Promise<{ outputPath: string }> {
  return invoke("stop_recording", { args: { outputPath } });
}

export async function getAudioLevel(): Promise<{ level: number; isRecording: boolean }> {
  return invoke("get_audio_level");
}

export interface RecordStartPayload {
  durationMs: number;
}

export async function onRecordStart(
  handler: (payload: RecordStartPayload) => void
): Promise<UnlistenFn> {
  return listen<RecordStartPayload>("test-record-start", (e) =>
    handler(e.payload)
  );
}

export async function onRecordStop(
  handler: () => void
): Promise<UnlistenFn> {
  return listen("test-record-stop", () => handler());
}
