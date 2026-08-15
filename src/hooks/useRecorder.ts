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
  notifyRecordingCompleted,
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
  // 用 ref 镜像 isRecording，避免 stopLocalRecording 闭包里读到旧值
  // （setTimeout 抓的是录音开始那一刻的闭包，那时 isRecording=false）
  const isRecordingRef = useRef(false);

  const stopLocalRecording = useCallback(async () => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    if (pollRef.current !== null) {
      window.clearInterval(pollRef.current);
      pollRef.current = null;
    }
    // 用 ref 判断而不是 state 闭包，避免 stale closure 导致自动停止时静默 no-op
    if (!isRecordingRef.current || !outputPathRef.current) {
      console.log("[useRecorder] stopLocalRecording: 跳过（未在录音或无输出路径）");
      return;
    }
    const pathToSave = outputPathRef.current;
    console.log(`[useRecorder] stopLocalRecording: 准备写入 ${pathToSave}`);
    // 先清空 ref，避免并发触发时重复 stop
    isRecordingRef.current = false;
    outputPathRef.current = null;
    try {
      const resp = await stopRecording(pathToSave);
      console.log(
        `[useRecorder] stopLocalRecording: 录音已保存 outputPath=${resp.outputPath}`
      );
      setRecordedPath(resp.outputPath);
      setIsRecording(false);
      setAudioLevel(0);
      // 把录音路径作为 q19 的答案（静默提交即可）
      try {
        await submitAnswer(19, resp.outputPath);
        console.log(
          `[useRecorder] submit_answer(19, ${resp.outputPath}) 成功`
        );
      } catch (e) {
        console.error("submit_answer(q19) 失败", e);
      }
      // 通知后端：录音已结束，跳过剩余的 90 秒等待，立即进入评分阶段
      try {
        await notifyRecordingCompleted();
        console.log("[useRecorder] notify_recording_completed 成功");
      } catch (e) {
        console.error("notify_recording_completed 失败", e);
      }
    } catch (e) {
      console.error("停止录音失败", e);
      setIsRecording(false);
      // 停止失败时把 ref 还原回去，避免下次又无法触发
      isRecordingRef.current = true;
      outputPathRef.current = pathToSave;
    }
  }, []);

  const startLocalRecording = useCallback(
    async (outputPath: string, durationMs: number) => {
      console.log(
        `[useRecorder] startLocalRecording: outputPath=${outputPath}, durationMs=${durationMs}`
      );
      outputPathRef.current = outputPath;
      isRecordingRef.current = false; // 先置 false，startRecording 成功后再置 true
      try {
        await startRecording();
        // 进入"已开始"状态后再让 setTimeout / record-stop 能触发 stop
        isRecordingRef.current = true;
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
          console.log(
            `[useRecorder] ${durationMs}ms 计时到，自动停止录音`
          );
          stopLocalRecording();
        }, durationMs);
      } catch (e) {
        console.error("启动录音失败", e);
        setIsRecording(false);
        outputPathRef.current = null;
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
      // 立即调用（stopLocalRecording 现在用 ref 判断 isRecording，不再受闭包 stale 影响）
      void stopLocalRecording();
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
