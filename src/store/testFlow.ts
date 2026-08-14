// 测试流程运行时状态：当前题号、阶段、剩余时间、用户作答

import { create } from "zustand";
import type {
  FlowStatePayload,
  TestPhase,
  TimerTickPayload,
} from "@/lib/tauri";

interface AnswerMap {
  [questionId: number]: string | null;
}

interface TestFlowState {
  // 当前题号（1-14，未开始时为 0）
  questionIndex: number;
  // 当前阶段
  phase: TestPhase | null;
  // 进度 0.0 ~ 1.0
  progress: number;
  // 剩余毫秒
  remainingMs: number;
  // 总毫秒
  durationMs: number;
  // 当前播放路径
  audioPath: string | null;
  // 是否在材料组内
  isGroup: boolean;
  // 组内索引（0/1）
  questionInGroup: number;
  // 用户作答
  answers: AnswerMap;
  // 是否已完成
  finished: boolean;
  // 错误信息
  error: string | null;

  // actions
  applyTick: (p: TimerTickPayload) => void;
  applyFlowState: (p: FlowStatePayload) => void;
  applyFinished: (p: { ok: boolean; error?: string }) => void;
  setAnswer: (questionId: number, answer: string | null) => void;
  reset: () => void;
}

export const useTestFlowStore = create<TestFlowState>((set) => ({
  questionIndex: 0,
  phase: null,
  progress: 0,
  remainingMs: 0,
  durationMs: 0,
  audioPath: null,
  isGroup: false,
  questionInGroup: 0,
  answers: {},
  finished: false,
  error: null,

  applyTick: (p) =>
    set({
      remainingMs: p.remainingMs,
      durationMs: p.durationMs,
      progress: p.progress,
    }),

  applyFlowState: (p) =>
    set({
      questionIndex: p.questionIndex,
      phase: p.phase,
      progress: p.progress,
      audioPath: p.audioPath,
      isGroup: p.isGroup,
      questionInGroup: p.questionInGroup,
    }),

  applyFinished: (p) =>
    set({
      finished: p.ok,
      error: p.error ?? null,
    }),

  setAnswer: (questionId, answer) =>
    set((s) => ({
      answers: { ...s.answers, [questionId]: answer },
    })),

  reset: () =>
    set({
      questionIndex: 0,
      phase: null,
      progress: 0,
      remainingMs: 0,
      durationMs: 0,
      audioPath: null,
      isGroup: false,
      questionInGroup: 0,
      answers: {},
      finished: false,
      error: null,
    }),
}));

export const PHASE_LABELS: Record<TestPhase, string> = {
  intro: "题目介绍",
  prepare: "准备中",
  playing: "音频播放中",
  answering: "作答中",
  fill_blank: "填写挖空",
  recall_prep: "默读准备",
  recording: "录音中",
};
