// 录音 hook：响应后端的 record-start/stop 事件，自动启停 cpal 录音
//
// 设计：
// - 后端 test_flow 在 Recording 阶段开始时 emit `test-record-start` 事件
// - 前端收到后计算输出路径（基于应用数据目录），调用 `startRecording` 命令
// - 录音时长由前端 setTimeout 控制（90 秒），到时调用 stopRecording
// - 实时拉取 `get_audio_level` 更新 UI 音量条
// - 后端发送 `test-record-stop` 时，前端强制停止并保存
// - 录音路径作为 q19 的"答案"提交到后端

import { useEffect, useRef, useState, useCallback } from "react";
import {
  onRecordStart,
  onRecordStop,
  startRecording,
  stopRecording,
  getAudioLevel,
  submitAnswer,
} from "@/lib/tauri";
import { appDataDir, join } from "@tauri-apps/api/path";
import { useTestStore } from "@/store/test";

interface RecorderHookResult {
  isRecording: boolean;
  audioLevel: number;
  recordedPath: string | null;
  startLocalRecording: (outputPath: string, durationMs: number) => Promise<void>;
  stopLocalRecording: () => Promise<void>;
}

/**
 * 计算录音输出路径：{app_data}/cache/{session_id}/recording.wav
 */
async function computeOutputPath(sessionId: string): Promise<string> {
  const dir = await appDataDir();
  return await join(dir, "cache", sessionId, "recording.wav");
}

export function useRecorder(): RecorderHookResult {
  const session = useTestStore((s) => s.session);
  const sessionId = session?.session_id ?? "default";

  const [isRecording, setIsRecording] = useState(false);
  const [audioLevel, setAudioLevel] = useState(0);
  const [recordedPath, setRecordedPath] = useState<string | null>(null);

  const timerRef = useRef<number | null>(null);
  const pollRef = useRef<number | null>(null);
  const outputPathRef = useRef<string | null>(null);

  const stopLocalRecording = useCallback(async () => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    if (pollRef.current !== null) {
      window.clearInterval(pollRef.current);
      pollRef.current = null;
    }
    if (!isRecording || !outputPathRef.current) return;
    try {
      const resp = await stopRecording(outputPathRef.current);
      setRecordedPath(resp.outputPath);
      setIsRecording(false);
      setAudioLevel(0);
      outputPathRef.current = null;
      // 把录音路径作为 q19 的答案
      submitAnswer(19, resp.outputPath).catch(() => {});
    } catch (e) {
      console.error("停止录音失败", e);
      setIsRecording(false);
    }
  }, [isRecording]);

  const startLocalRecording = useCallback(
    async (outputPath: string, durationMs: number) => {
      outputPathRef.current = outputPath;
      try {
        await startRecording();
        setIsRecording(true);

        pollRef.current = window.setInterval(async () => {
          try {
            const { level } = await getAudioLevel();
            setAudioLevel(level);
          } catch {
            // ignore
          }
        }, 100);

        timerRef.current = window.setTimeout(() => {
          stopLocalRecording();
        }, durationMs);
      } catch (e) {
        console.error("启动录音失败", e);
        setIsRecording(false);
      }
    },
    [stopLocalRecording]
  );

  // 监听后端 record-start/stop 事件
  useEffect(() => {
    let unsub1: (() => void) | undefined;
    let unsub2: (() => void) | undefined;
    let cancelled = false;

    onRecordStart(async (p) => {
      if (cancelled) return;
      try {
        const outPath = await computeOutputPath(sessionId);
        await startLocalRecording(outPath, p.durationMs);
      } catch (e) {
        console.error("录音启动失败", e);
      }
    }).then((fn) => {
      if (cancelled) fn();
      else unsub1 = fn;
    });

    onRecordStop(() => {
      if (cancelled) return;
      stopLocalRecording();
    }).then((fn) => {
      if (cancelled) fn();
      else unsub2 = fn;
    });

    return () => {
      cancelled = true;
      unsub1?.();
      unsub2?.();
    };
  }, [sessionId, startLocalRecording, stopLocalRecording]);

  // 卸载时清理
  useEffect(() => {
    return () => {
      if (timerRef.current !== null) window.clearTimeout(timerRef.current);
      if (pollRef.current !== null) window.clearInterval(pollRef.current);
    };
  }, []);

  return {
    isRecording,
    audioLevel,
    recordedPath,
    startLocalRecording,
    stopLocalRecording,
  };
}
