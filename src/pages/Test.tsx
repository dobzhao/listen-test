// 19 题测试主页面：根据 phase 切换子组件展示

import { useEffect, useMemo } from "react";
import { useNavigate } from "react-router-dom";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Loader2, Play, FileText } from "lucide-react";
import { useTestStore } from "@/store/test";
import {
  PHASE_LABELS,
  useTestFlowStore,
} from "@/store/testFlow";
import {
  useTestFlowEvents,
  useAudioPlayHandler,
} from "@/hooks/useTestFlowEvents";
import { useAudioPlayer } from "@/hooks/useAudioPlayer";
import { useRecorder } from "@/hooks/useRecorder";
import { GlobalHeader } from "@/components/test/GlobalHeader";
import { PhaseCountdown } from "@/components/test/PhaseCountdown";
import {
  ShortDialogueDisplay,
  GroupDialogueDisplay,
} from "@/components/test/QuestionDisplay";
import { FillBlankTable } from "@/components/test/FillBlankTable";
import { RecorderPanel } from "@/components/test/RecorderPanel";
import { startTestFlow } from "@/lib/tauri";

export default function TestPage() {
  const navigate = useNavigate();

  // 整体测试会话
  const session = useTestStore((s) => s.session);
  const reset = useTestStore((s) => s.reset);

  // 流程运行时状态
  const questionIndex = useTestFlowStore((s) => s.questionIndex);
  const phase = useTestFlowStore((s) => s.phase);
  const isGroup = useTestFlowStore((s) => s.isGroup);
  const finished = useTestFlowStore((s) => s.finished);
  const error = useTestFlowStore((s) => s.error);

  // 订阅所有后端事件
  useTestFlowEvents();

  // 音频播放
  const { play } = useAudioPlayer();
  useAudioPlayHandler((path, loop) => {
    play(path, loop);
  });

  // 录音
  const recorder = useRecorder();

  // 计算当前应展示的题目
  const currentDialogue = useMemo(() => {
    if (!session || questionIndex < 1) return null;

    if (questionIndex <= 4) {
      const idx = questionIndex - 1;
      const d = session.short_dialogues[idx];
      if (!d) return null;
      return { kind: "short" as const, data: d };
    }
    if (questionIndex <= 12) {
      const idx = (questionIndex - 5) / 2;
      const d = session.long_dialogues[idx];
      if (!d) return null;
      return { kind: "group" as const, data: d };
    }
    if (questionIndex <= 14) {
      return { kind: "monologue" as const, data: session.monologue };
    }
    return null; // 15-19 不走 dialogue 流
  }, [session, questionIndex]);

  // 完成后跳转结算页（Phase 5 实现）
  useEffect(() => {
    if (finished && !error) {
      const t = setTimeout(() => navigate("/result"), 1500);
      return () => clearTimeout(t);
    }
  }, [finished, error, navigate]);

  if (!session) {
    return (
      <div className="min-h-screen flex items-center justify-center p-8">
        <Card className="max-w-md w-full">
          <CardHeader>
            <CardTitle>尚未生成测试会话</CardTitle>
            <CardDescription>
              请先返回主菜单，点击「开始测试」完成题目预生成。
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Button onClick={() => navigate("/")} className="w-full">
              返回主菜单
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  if (!phase || questionIndex === 0) {
    return <TestStartCard onStart={startTestFlow} />;
  }

  // 15-19 题专用视图
  if (questionIndex >= 15) {
    return (
      <div className="min-h-screen flex flex-col bg-slate-50">
        <GlobalHeader />
        <main className="flex-1 container max-w-5xl mx-auto py-8 space-y-6">
          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-base flex items-center justify-between">
                <span>{PHASE_LABELS[phase]}</span>
                {phase === "fill_blank" && (
                  <span className="text-xs text-amber-600 font-normal">
                    请在 90 秒内完成 4 个挖空
                  </span>
                )}
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <PhaseCountdown />
              {phase === "prepare" && (
                <p className="text-sm text-muted-foreground">
                  请阅读下方总-分结构与挖空，音频即将播放（共 3 次）
                </p>
              )}
              {phase === "playing" && (
                <p className="text-sm text-muted-foreground">
                  正在播放听力材料…
                </p>
              )}
              {phase === "fill_blank" && (
                <p className="text-sm text-amber-600 font-medium">
                  ⏱ 请使用键盘在下方空格中输入答案
                </p>
              )}
              {phase === "recall_prep" && (
                <p className="text-sm text-amber-600 font-medium">
                  ⏱ 默读时间，整理转述思路（不可再听音频）
                </p>
              )}
              {phase === "recording" && (
                <p className="text-sm text-red-600 font-medium">
                  🎙 录音中，请用清晰的英语口头转述刚才听到的听力材料
                </p>
              )}
            </CardContent>
          </Card>

          {/* 挖空表格 - 始终可见 */}
          <FillBlankTable table={session.retell.table} enabled={phase === "fill_blank"} />

          {/* 录音面板 - 仅 recall_prep / recording 时 */}
          {(phase === "recall_prep" || phase === "recording") && (
            <RecorderPanel
              isRecording={recorder.isRecording}
              audioLevel={recorder.audioLevel}
              onStop={recorder.stopLocalRecording}
            />
          )}

          {error && (
            <Card>
              <CardContent className="py-4 text-destructive text-sm">
                {error}
              </CardContent>
            </Card>
          )}

          {finished && !error && (
            <Card>
              <CardContent className="py-6 text-center text-emerald-600">
                ✓ 全部 19 题已完成，正在跳转到结算页…
              </CardContent>
            </Card>
          )}
        </main>
        <footer className="border-t bg-white py-2 text-center">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              if (window.confirm("确认放弃本次测试？所有作答将被清空。")) {
                reset();
                navigate("/");
              }
            }}
          >
            放弃并返回主菜单
          </Button>
        </footer>
      </div>
    );
  }

  // 1-14 题主视图
  return (
    <div className="min-h-screen flex flex-col bg-slate-50">
      <GlobalHeader />
      <main className="flex-1 container max-w-5xl mx-auto py-8 space-y-6">
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-base flex items-center justify-between">
              <span>{PHASE_LABELS[phase]}</span>
              {phase === "answering" && (
                <span className="text-xs text-amber-600 font-normal">
                  点击下方选项作答（{isGroup ? "两题共享作答时间" : "本题独立作答"}）
                </span>
              )}
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <PhaseCountdown />
            {phase === "intro" && (
              <p className="text-sm text-muted-foreground">
                本部分共 4 道题（短对话），听完对话后选择正确答案。每题 10 秒作答时间。
              </p>
            )}
            {phase === "prepare" && (
              <p className="text-sm text-muted-foreground">
                {isGroup ? "请阅读两组题目，音频即将播放 2 次" : "请阅读题目，音频即将播放"}
              </p>
            )}
            {phase === "playing" && (
              <p className="text-sm text-muted-foreground">
                {isGroup ? "正在播放对话音频（将播放 2 次）…" : "正在播放对话音频…"}
              </p>
            )}
            {phase === "answering" && (
              <p className="text-sm text-amber-600 font-medium">
                ⏱ 请尽快作答，倒计时结束后自动进入下一题
              </p>
            )}
          </CardContent>
        </Card>

        {currentDialogue && (
          <div className="space-y-4">
            {currentDialogue.kind === "short" && (
              <ShortDialogueDisplay
                dialogue={currentDialogue.data}
                showQuestion={phase !== "intro"}
                showAnswer={phase === "answering"}
              />
            )}
            {(currentDialogue.kind === "group" ||
              currentDialogue.kind === "monologue") && (
              <GroupDialogueDisplay
                dialogue={currentDialogue.data}
                showQuestion={phase !== "intro"}
                showAnswer={phase === "answering"}
                groupStartId={questionIndex}
              />
            )}
          </div>
        )}

        {error && (
          <Card>
            <CardContent className="py-4 text-destructive text-sm">{error}</CardContent>
          </Card>
        )}

        {finished && !error && (
          <Card>
            <CardContent className="py-6 text-center text-emerald-600">
              ✓ 第 1-14 题已完成，正在跳转到结算页…
            </CardContent>
          </Card>
        )}
      </main>
      <footer className="border-t bg-white py-2 text-center">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            if (window.confirm("确认放弃本次测试？所有作答将被清空。")) {
              reset();
              navigate("/");
            }
          }}
        >
          放弃并返回主菜单
        </Button>
      </footer>
    </div>
  );
}

function TestStartCard({ onStart }: { onStart: () => Promise<unknown> }) {
  const [busy, setBusy] = useState(false);
  const navigate = useNavigate();
  return (
    <div className="min-h-screen flex items-center justify-center p-8 bg-slate-50">
      <Card className="max-w-md w-full">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Play className="w-5 h-5" />
            准备开始测试
          </CardTitle>
          <CardDescription>
            测试共 19 题，分两部分：1-14 题（听后选择）+ 15-19 题（听后转述）。
            开始后将自动播放音频与计时，请保持专注。
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <Button
            className="w-full"
            size="lg"
            disabled={busy}
            onClick={async () => {
              setBusy(true);
              try {
                await onStart();
              } finally {
                setBusy(false);
              }
            }}
          >
            {busy ? (
              <>
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                启动中…
              </>
            ) : (
              <>
                <Play className="w-4 h-4 mr-2" />
                开始测试
              </>
            )}
          </Button>
          <Button
            variant="ghost"
            className="w-full"
            onClick={() => navigate("/")}
          >
            返回主菜单
          </Button>
          <p className="text-xs text-muted-foreground text-center flex items-center justify-center gap-1">
            <FileText className="w-3 h-3" />
            15-18 题将使用键盘输入答案，第 19 题需要麦克风录音
          </p>
        </CardContent>
      </Card>
    </div>
  );
}

import { useState } from "react";
