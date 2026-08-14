// 倒计时进度条：作答阶段显示精确倒计时

import { Progress } from "@/components/ui/progress";
import { formatDuration } from "@/lib/utils";
import { useTestFlowStore } from "@/store/testFlow";

export function PhaseCountdown() {
  const remainingMs = useTestFlowStore((s) => s.remainingMs);
  const durationMs = useTestFlowStore((s) => s.durationMs);
  const phase = useTestFlowStore((s) => s.phase);

  if (!phase) return null;

  const progress =
    durationMs > 0 ? ((durationMs - remainingMs) / durationMs) * 100 : 0;

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between text-sm">
        <span className="text-muted-foreground">剩余时间</span>
        <span className="font-mono font-medium">
          {formatDuration(remainingMs)}
        </span>
      </div>
      <Progress
        value={progress}
        className="h-2"
        indicatorClassName={
          phase === "answering"
            ? "bg-amber-500 transition-all"
            : "bg-primary transition-all"
        }
      />
    </div>
  );
}
