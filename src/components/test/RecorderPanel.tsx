// 录音面板：倒计时 + 实时音量条 + 录音状态

import { useEffect, useRef, useState } from "react";
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

  const confirmDialogRef = useRef<HTMLDialogElement>(null);
  const [confirming, setConfirming] = useState(false);

  // 录音阶段倒计时是 90 秒；显示进度条
  const progress =
    durationMs > 0 ? ((durationMs - remainingMs) / durationMs) * 100 : 0;

  const openConfirm = () => {
    confirmDialogRef.current?.showModal();
  };

  const closeConfirm = () => {
    confirmDialogRef.current?.close();
  };

  const handleConfirmStop = async () => {
    if (confirming) return;
    setConfirming(true);
    closeConfirm();
    try {
      await onStop();
    } finally {
      setConfirming(false);
    }
  };

  // 对话框打开时屏蔽意外的回车触发（用户在录音中按回车可能误触发）
  useEffect(() => {
    const dialog = confirmDialogRef.current;
    if (!dialog) return;
    const handleCancel = (e: Event) => {
      e.preventDefault();
    };
    dialog.addEventListener("cancel", handleCancel);
    return () => dialog.removeEventListener("cancel", handleCancel);
  }, []);

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
              onClick={openConfirm}
              disabled={confirming}
            >
              <Square className="w-4 h-4 mr-2" />
              提前结束录音
            </Button>

            <dialog
              ref={confirmDialogRef}
              className="p-0 m-0 w-full h-full max-w-none max-h-none bg-transparent backdrop:bg-black/40"
            >
              <div
                className="w-full h-full flex items-center justify-center p-4"
                onClick={(e) => {
                  // 点击 flex 容器自身（遮罩区域）时关闭，点击子元素时不关闭
                  if (e.target === e.currentTarget) {
                    closeConfirm();
                  }
                }}
              >
                <div
                  className="w-[360px] max-w-full bg-background border border-border rounded-lg shadow-lg p-6 space-y-4"
                  onClick={(e) => e.stopPropagation()}
                >
                  <h2 className="text-base font-semibold">提前结束录音？</h2>
                  <p className="text-sm text-muted-foreground">
                    结束录音后将立即进入评分环节，无法重新录制。确定要提前结束吗？
                  </p>
                  <div className="flex justify-end gap-2 pt-2">
                    <Button
                      type="button"
                      variant="outline"
                      onClick={closeConfirm}
                      disabled={confirming}
                    >
                      取消
                    </Button>
                    <Button
                      type="button"
                      variant="destructive"
                      onClick={handleConfirmStop}
                      disabled={confirming}
                    >
                      确认结束
                    </Button>
                  </div>
                </div>
              </div>
            </dialog>
          </>
        )}

        {phase === "answering" && null}
      </CardContent>
    </Card>
  );
}
