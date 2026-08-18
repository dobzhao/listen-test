// 顶部全局进度条 + 当前题目/段落标签

import { Card } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { Badge } from "@/components/ui/badge";
import { PHASE_LABELS } from "@/store/testFlow";
import { useTestFlowStore } from "@/store/testFlow";

/**
 * 5-12 题每两题一组（5-6、7-8、9-10、11-12），由当前题号反推组首题号，
 * 用于显示「第 X-(X+1) 题」。
 */
function getLongDialogueGroupStart(questionIndex: number): number {
  // 5,6 -> 5；7,8 -> 7；9,10 -> 9；11,12 -> 11
  return 5 + 2 * Math.floor((questionIndex - 5) / 2);
}

/**
 * 左上角标签：
 * - 1-4 题：第 N 题
 * - 5-12 题：第 X-(X+1) 题（同一长对话两题共享标签）
 * - 13-14 题：第 13-14 题
 * - 15-18 题：第 15-18 题
 * - 19 题：第 19 题
 */
function getQuestionLabel(questionIndex: number): string {
  if (questionIndex <= 0) return "等待开始…";
  if (questionIndex <= 4) return `第 ${questionIndex} 题`;
  if (questionIndex <= 12) {
    const start = getLongDialogueGroupStart(questionIndex);
    return `第 ${start}-${start + 1} 题`;
  }
  if (questionIndex <= 14) return "第 13-14 题";
  if (questionIndex <= 18) return "第 15-18 题";
  return "第 19 题";
}

/**
 * 右上角段落标签：
 * - 1-4 题：第 1-4 题 · 短对话听后选择
 * - 5-14 题：第 5-14 题 · 长对话听后选择
 * - 15-18 题：第 15-18 题 · 听后记录
 * - 19 题：第 19 题 · 听后转述
 */
function getSectionLabel(questionIndex: number): string {
  if (questionIndex <= 0) return "1-19 题 · 英语听力练习";
  if (questionIndex <= 4) return "第 1-4 题 · 短对话听后选择";
  if (questionIndex <= 14) return "第 5-14 题 · 长对话听后选择";
  if (questionIndex <= 18) return "第 15-18 题 · 听后记录";
  return "第 19 题 · 听后转述";
}

export function GlobalHeader() {
  const questionIndex = useTestFlowStore((s) => s.questionIndex);
  const phase = useTestFlowStore((s) => s.phase);

  const total = 19;
  const progress = questionIndex > 0 ? ((questionIndex - 1) / total) * 100 : 0;

  return (
    <Card className="rounded-none border-x-0 border-t-0">
      <div className="container max-w-5xl mx-auto py-3 space-y-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <span className="text-sm font-semibold">
              {getQuestionLabel(questionIndex)}
            </span>
            {phase && (
              <Badge variant="secondary">{PHASE_LABELS[phase]}</Badge>
            )}
          </div>
          <span className="text-xs text-muted-foreground font-mono">
            {getSectionLabel(questionIndex)}
          </span>
        </div>
        <Progress value={progress} className="h-1.5" />
      </div>
    </Card>
  );
}