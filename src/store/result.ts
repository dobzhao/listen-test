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

export const useResultStore = create<ResultState>((set) => ({
  result: null,
  loading: false,
  error: null,

  load: async () => {
    set({ loading: true, error: null });
    try {
      const r = await scoreFullTest();
      set({ result: r, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  reset: () => set({ result: null, loading: false, error: null }),
}));