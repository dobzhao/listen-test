// 订阅后端生成进度事件的 hook

import { useEffect } from "react";
import { onGenerationProgress } from "@/lib/tauri";
import { useTestStore } from "@/store/test";

/**
 * 在挂载期间订阅后端进度事件，自动写入 test store。
 * 卸载时自动取消订阅。
 */
export function useGenerationProgress() {
  const setProgress = useTestStore((s) => s.setProgress);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    onGenerationProgress((payload) => {
      if (cancelled) return;
      setProgress(payload);
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
      }
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [setProgress]);
}
