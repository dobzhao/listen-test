// 订阅后端测试流程事件的 hook 集合
//
// - useTestFlowEvents: 同时订阅 timer-tick / flow-state / flow-finished / phase-finished
//   写入 useTestFlowStore

import { useEffect } from "react";
import {
  onAudioPlay,
  onFlowFinished,
  onFlowState,
  onPhaseFinished,
  onTimerTick,
} from "@/lib/tauri";
import { useTestFlowStore } from "@/store/testFlow";

/**
 * 在挂载期间订阅所有测试流程事件。卸载时自动取消订阅。
 */
export function useTestFlowEvents() {
  const applyTick = useTestFlowStore((s) => s.applyTick);
  const applyFlowState = useTestFlowStore((s) => s.applyFlowState);
  const applyFinished = useTestFlowStore((s) => s.applyFinished);

  useEffect(() => {
    let unsub: Array<() => void> = [];
    let cancelled = false;

    const setup = async () => {
      const u1 = await onTimerTick((p) => !cancelled && applyTick(p));
      const u2 = await onFlowState((p) => !cancelled && applyFlowState(p));
      const u3 = await onFlowFinished((p) => !cancelled && applyFinished(p));
      const u4 = await onPhaseFinished(() => {
        // 阶段结束不需要更新 store（由下一个 flow-state 触发）
      });
      if (cancelled) {
        [u1, u2, u3, u4].forEach((fn) => fn());
      } else {
        unsub = [u1, u2, u3, u4];
      }
    };
    setup();

    return () => {
      cancelled = true;
      unsub.forEach((fn) => fn());
      unsub = [];
    };
  }, [applyTick, applyFlowState, applyFinished]);
}

/**
 * 订阅 audio-play 事件，自动调用 handler(path, loop)
 */
export function useAudioPlayHandler(
  handler: (path: string | null, loop: boolean) => void
) {
  useEffect(() => {
    let unsub: (() => void) | undefined;
    let cancelled = false;
    onAudioPlay((p) => {
      if (cancelled) return;
      handler(p.path, p.loop);
    }).then((fn) => {
      if (cancelled) fn();
      else unsub = fn;
    });
    return () => {
      cancelled = true;
      unsub?.();
    };
  }, [handler]);
}
