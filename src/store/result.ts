// 结算页状态

import { create } from "zustand";
import type { TestResult } from "@/types/result";
import { scoreFullTest } from "@/lib/tauri";

interface ResultState {
  result: TestResult | null;
  loading: boolean;
  error: string | null;

  load: () => Promise<void>;
  reset: () => void;
}

/**
 * 进行中的评分 Promise：用来在并发调用时复用同一次后端请求。
 *
 * 背景：Result.tsx 的 useEffect 在 React 18 StrictMode 下会在同一次挂载中
 * 执行两次（mount → fake unmount → re-mount）。两次 effect 都会读到的
 * `loading=false`（组件闭包里的旧值），guard 都会通过，从而触发两次
 * `score_full_test`，导致 LLM 被并发调用两次（15-18 + 19 各自重复一次）。
 *
 * 用模块级 Promise 跟踪在飞请求，让第二次及以后的调用直接复用同一 Promise，
 * 不再发出新的后端命令。
 */
let loadInFlight: Promise<void> | null = null;

export const useResultStore = create<ResultState>((set) => ({
  result: null,
  loading: false,
  error: null,

  load: async () => {
    // 并发互斥：如果已经在跑，直接返回同一 Promise
    if (loadInFlight) return loadInFlight;
    set({ loading: true, error: null });
    loadInFlight = (async () => {
      try {
        const r = await scoreFullTest();
        set({ result: r, loading: false });
      } catch (e) {
        set({ error: String(e), loading: false });
      } finally {
        loadInFlight = null;
      }
    })();
    return loadInFlight;
  },

  reset: () => {
    // 重置时丢弃在飞请求的引用，避免重置后残留状态导致下次 load 被错误复用
    loadInFlight = null;
    set({ result: null, loading: false, error: null });
  },
}));