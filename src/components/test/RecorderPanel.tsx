// 录音面板：倒计时 + 实时音量条 + 录音状态

import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { Mic, MicOff, Square } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useTestFlowStore } from "@/store/testFlow";
import { formatDuration } from "@/lib/utils";

interface Props {
  isRecording: boolean;
  audioLevel: number; // 0.0 ~ 1.0
  onStop: () => Promise<void>;
}

export function RecorderPanel({ isRecording, audioLevel, onStop }: Props) {
  const remainingMs = useTestFlowStore((s) => s.remainingMs);
  const durationMs = useTestFlowStore((s) => s.durationMs);
  const phase = useTestFlowStore((s) => s.phase);

  // 录音阶段倒计时是 90 秒；显示进度条
  const progress =
    durationMs > 0 ? ((durationMs - remainingMs) / durationMs) * 100 : 0;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg flex items-center justify-between">
          <span className="flex items-center gap-2">
            {isRecording ? (
              <Mic className="w-5 h-5 text-red-500 animate-pulse" />
            ) : (
              <MicOff className="w-5 h-5 text-muted-foreground" />
            )}
            第 19 题：口头转述
          </span>
          {phase === "recall_prep" && (
            <span className="text-xs text-amber-600 font-normal">
              默读准备中
            </span>
          )}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        {phase === "recall_prep" && (
          <p className="text-sm text-muted-foreground">
            请根据 15-18 题填写的总-分结构，整理自己的转述思路。
            倒计时结束后将自动开始录音。
          </p>
        )}

        {(phase === "recording" || isRecording) && (
          <>
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
                indicatorClassName="bg-red-500 transition-all"
              />
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between text-sm">
                <span className="text-muted-foreground">麦克风音量</span>
                <span className="font-mono text-xs">
                  {(audioLevel * 100).toFixed(0)}%
                </span>
              </div>
              <Progress
                value={audioLevel * 100}
                className="h-2"
                indicatorClassName="bg-emerald-500 transition-all"
              />
            </div>

            <Button
              variant="outline"
              className="w-full"
              onClick={onStop}
            >
              <Square className="w-4 h-4 mr-2" />
              提前结束录音
            </Button>
          </>
        )}

        {phase === "answering" && null}
      </CardContent>
    </Card>
  );
}
