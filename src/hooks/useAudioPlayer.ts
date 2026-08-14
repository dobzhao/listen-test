// 音频播放 hook：通过 Tauri command 调用后端 rodio 播放
//
// 之前用 HTMLAudioElement + convertFileSrc，但 WebKitGTK / Chromium webview
// 存在 autoplay 限制与 asset protocol 跨平台兼容问题。
// 改用后端 rodio 播放后，跨平台一致且无 webview 限制。

import { useCallback, useRef } from "react";
import { playAudioBackground, playAudioFile } from "@/lib/tauri";

/**
 * 播放控制接口
 *
 * - `play(path)`：非阻塞后台播放（用于测试阶段 UI 立即响应）
 * - `playBlock(path)`：阻塞播放，调用方需要 await（用于设备测试回放）
 * - `stop()`：停止（简化：仅清空内部状态）
 */
export function useAudioPlayer() {
  const currentPathRef = useRef<string | null>(null);

  const play = useCallback((path: string | null, _loop = false) => {
    if (!path) {
      currentPathRef.current = null;
      return;
    }
    currentPathRef.current = path;
    playAudioBackground(path).catch((e) => {
      console.error("后台播放失败", e);
    });
  }, []);

  const playBlock = useCallback(async (path: string) => {
    currentPathRef.current = path;
    return playAudioFile(path);
  }, []);

  const stop = useCallback(() => {
    currentPathRef.current = null;
  }, []);

  return { play, playBlock, stop };
}
