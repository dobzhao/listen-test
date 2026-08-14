// 题目展示组件：根据题号与组索引显示题干与选项

import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { Check, Circle } from "lucide-react";
import type {
  MultipleChoiceQuestion,
  ShortDialogue,
  LongDialogue,
  Monologue,
} from "@/types/question";
import { useTestFlowStore } from "@/store/testFlow";
import { submitAnswer } from "@/lib/tauri";

interface BaseProps {
  question: MultipleChoiceQuestion;
  show: boolean;
}

function SingleQuestion({ question, show }: BaseProps) {
  const answer = useTestFlowStore(
    (s) => s.answers[question.id] ?? null
  );
  const setAnswer = useTestFlowStore((s) => s.setAnswer);

  const handleClick = async (key: "A" | "B" | "C") => {
    // 切换选择：再点一次取消
    const next = answer === key ? null : key;
    setAnswer(question.id, next);
    try {
      await submitAnswer(question.id, next);
    } catch (e) {
      // 不要静默吞错 — 否则后端落库失败时前端看起来"已选"但判分全 0
      console.error("submit_answer 失败", e);
    }
  };

  return (
    <Card>
      <CardContent className="py-5 space-y-4">
        <div className="flex items-start gap-3">
          <Badge variant="outline" className="font-mono">
            Q{question.id}
          </Badge>
          <p className="text-base font-medium leading-relaxed">
            {question.question}
          </p>
        </div>
        <div className="space-y-2">
          {(["A", "B", "C"] as const).map((key) => {
            const text = question.options[key] ?? "";
            const selected = answer === key;
            return (
              <Button
                key={key}
                variant={selected ? "default" : "outline"}
                className={cn(
                  "w-full justify-start text-left h-auto py-3",
                  !show && "pointer-events-none opacity-60"
                )}
                onClick={() => handleClick(key)}
              >
                <span className="flex items-center gap-3 w-full">
                  <span className="font-mono font-semibold">{key}.</span>
                  <span className="flex-1">{text}</span>
                  {selected ? (
                    <Check className="w-4 h-4 ml-auto" />
                  ) : (
                    <Circle className="w-4 h-4 ml-auto opacity-30" />
                  )}
                </span>
              </Button>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}

interface ShortProps {
  dialogue: ShortDialogue;
  showQuestion: boolean;
  showAnswer: boolean;
}

export function ShortDialogueDisplay({
  dialogue,
  showQuestion,
  showAnswer,
}: ShortProps) {
  return (
    <SingleQuestion
      question={dialogue.question}
      show={showAnswer}
      // showQuestion 用于控制是否显示题干（始终显示题干，但仅作答时可点击）
      // 单题模式直接用 showAnswer 决定是否可点击
    />
  );
}

interface GroupProps {
  dialogue: LongDialogue | Monologue;
  showQuestion: boolean;
  showAnswer: boolean;
  groupStartId: number;
}

export function GroupDialogueDisplay({
  dialogue,
  showAnswer,
}: GroupProps) {
  return (
    <div className="grid gap-4 md:grid-cols-2">
      {dialogue.questions.map((q) => (
        <SingleQuestion key={q.id} question={q} show={showAnswer} />
      ))}
    </div>
  );
}
