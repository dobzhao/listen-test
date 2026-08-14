// 设置状态：保存当前内存中的 AppConfig，并提供加载/保存/重置 actions

import { create } from "zustand";
import {
  AppConfig,
  ConfigResponse,
  PromptKey,
  defaultAppConfig,
} from "@/types/config";
import {
  getConfig,
  saveConfig,
  resetConfig,
  restoreDefaultPrompt,
} from "@/lib/tauri";

interface SettingsState {
  config: AppConfig;
  configPath: string;
  loaded: boolean;
  loadError: string | null;
  saving: boolean;

  // actions
  load: () => Promise<void>;
  persist: () => Promise<void>;
  reset: () => Promise<void>;
  updateLlm: (patch: Partial<AppConfig["llm"]>) => void;
  updateTts: (patch: Partial<AppConfig["tts"]>) => void;
  updateStt: (patch: Partial<AppConfig["stt"]>) => void;
  updateLlmParams: (patch: Partial<AppConfig["llm_params"]>) => void;
  updateAudio: (patch: Partial<AppConfig["audio"]>) => void;
  updatePrompt: (key: PromptKey, value: string) => void;
  restoreOnePrompt: (key: PromptKey) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  config: defaultAppConfig(),
  configPath: "",
  loaded: false,
  loadError: null,
  saving: false,

  load: async () => {
    try {
      const resp: ConfigResponse = await getConfig();
      set({
        config: resp.config,
        configPath: resp.config_path,
        loaded: true,
        loadError: null,
      });
    } catch (e) {
      set({
        loadError: String(e),
        loaded: true,
      });
    }
  },

  persist: async () => {
    set({ saving: true });
    try {
      await saveConfig(get().config);
    } finally {
      set({ saving: false });
    }
  },

  reset: async () => {
    const fresh = await resetConfig();
    set({ config: fresh });
  },

  updateLlm: (patch) =>
    set((s) => ({ config: { ...s.config, llm: { ...s.config.llm, ...patch } } })),
  updateTts: (patch) =>
    set((s) => ({ config: { ...s.config, tts: { ...s.config.tts, ...patch } } })),
  updateStt: (patch) =>
    set((s) => ({ config: { ...s.config, stt: { ...s.config.stt, ...patch } } })),
  updateLlmParams: (patch) =>
    set((s) => ({
      config: { ...s.config, llm_params: { ...s.config.llm_params, ...patch } },
    })),
  updateAudio: (patch) =>
    set((s) => ({ config: { ...s.config, audio: { ...s.config.audio, ...patch } } })),
  updatePrompt: (key, value) =>
    set((s) => ({
      config: { ...s.config, prompts: { ...s.config.prompts, [key]: value } },
    })),

  restoreOnePrompt: async (key) => {
    const value = await restoreDefaultPrompt(key);
    get().updatePrompt(key, value);
  },
}));
