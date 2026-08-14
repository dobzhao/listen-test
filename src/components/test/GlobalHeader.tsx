// 顶部全局进度条："第 N/14 题"

import { Card } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { Badge } from "@/components/ui/badge";
import { PHASE_LABELS } from "@/store/testFlow";
import { useTestFlowStore } from "@/store/testFlow";

export function GlobalHeader() {
  const questionIndex = useTestFlowStore((s) => s.questionIndex);
  const phase = useTestFlowStore((s) => s.phase);

  const total = 14;
  const progress = questionIndex > 0 ? ((questionIndex - 1) / total) * 100 : 0;

  return (
    <Card className="rounded-none border-x-0 border-t-0">
      <div className="container max-w-5xl mx-auto py-3 space-y-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <span className="text-sm font-semibold">
              {questionIndex > 0
                ? `第 ${questionIndex} / ${total} 题`
                : "等待开始…"}
            </span>
            {phase && (
              <Badge variant="secondary">{PHASE_LABELS[phase]}</Badge>
            )}
          </div>
          <span className="text-xs text-muted-foreground font-mono">
            1-14 题 · 听后选择
          </span>
        </div>
        <Progress value={progress} className="h-1.5" />
      </div>
    </Card>
  );
}
