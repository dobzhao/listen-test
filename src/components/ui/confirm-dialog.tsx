// 全局确认对话框：基于原生 <dialog> + showModal()，
// 替代 window.confirm()，避免 macOS WKWebView 下 wry 未实现
// runJavaScriptConfirmPanelWithMessage 导致的「永远看不见、永远返回 false」问题。
//
// 视觉与事件处理参照 src/components/test/RecorderPanel.tsx:131-172（已在生产线验证可用）。

import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { resolveConfirm, useConfirmStore } from "@/store/confirm";

export function ConfirmDialog() {
  const open = useConfirmStore((s) => s.open);
  const request = useConfirmStore((s) => s.request);
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [busy, setBusy] = useState(false);

  // 跟踪 open 变化，调用 native <dialog> API
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) {
      dialog.showModal();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  }, [open]);

  // 监听 ESC 触发的 cancel 事件：把它当作"取消"语义
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    const handleCancel = (e: Event) => {
      e.preventDefault();
      resolveConfirm(false);
    };
    dialog.addEventListener("cancel", handleCancel);
    return () => dialog.removeEventListener("cancel", handleCancel);
  }, []);

  // 兜底：组件卸载时若仍有未决 promise，resolve(false) 避免悬空
  useEffect(() => {
    return () => resolveConfirm(false);
  }, []);

  const handleConfirm = () => {
    if (busy) return;
    setBusy(true);
    resolveConfirm(true);
    setBusy(false);
  };

  const handleCancel = () => {
    if (busy) return;
    resolveConfirm(false);
  };

  return (
    <dialog
      ref={dialogRef}
      className="p-0 m-0 w-full h-full max-w-none max-h-none bg-transparent backdrop:bg-black/40"
    >
      <div
        className="w-full h-full flex items-center justify-center p-4"
        onClick={(e) => {
          // 点击 flex 容器自身（遮罩区域）时关闭，点击子元素时不关闭
          if (e.target === e.currentTarget) {
            handleCancel();
          }
        }}
      >
        <div
          className="w-[360px] max-w-full bg-background border border-border rounded-lg shadow-lg p-6 space-y-4"
          onClick={(e) => e.stopPropagation()}
        >
          <h2 className="text-base font-semibold">
            {request?.title ?? "确认"}
          </h2>
          <p className="text-sm text-muted-foreground whitespace-pre-wrap">
            {request?.description}
          </p>
          <div className="flex justify-end gap-2 pt-2">
            <Button
              type="button"
              variant="outline"
              onClick={handleCancel}
              disabled={busy}
            >
              {request?.cancelText ?? "取消"}
            </Button>
            <Button
              type="button"
              variant={
                request?.variant === "destructive" ? "destructive" : "default"
              }
              onClick={handleConfirm}
              disabled={busy}
            >
              {request?.confirmText ?? "确认"}
            </Button>
          </div>
        </div>
      </div>
    </dialog>
  );
}
