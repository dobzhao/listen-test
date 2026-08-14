// 简易 toast 组件：错误提示 + 自动消失

import { useEffect } from "react";
import { AlertCircle, X } from "lucide-react";
import { cn } from "@/lib/utils";

interface Props {
  message: string;
  kind?: "error" | "info" | "success";
  durationMs?: number;
  onClose: () => void;
}

export function Toast({
  message,
  kind = "error",
  durationMs = 4000,
  onClose,
}: Props) {
  useEffect(() => {
    if (durationMs > 0) {
      const t = window.setTimeout(onClose, durationMs);
      return () => window.clearTimeout(t);
    }
  }, [durationMs, onClose]);

  return (
    <div
      className={cn(
        "fixed top-4 right-4 z-50 flex items-center gap-2 px-4 py-3 rounded-md shadow-lg border max-w-md",
        kind === "error" && "bg-destructive/10 border-destructive/40 text-destructive",
        kind === "info" && "bg-blue-50 border-blue-300 text-blue-700",
        kind === "success" && "bg-emerald-50 border-emerald-300 text-emerald-700"
      )}
    >
      {kind === "error" && <AlertCircle className="w-4 h-4 flex-shrink-0" />}
      <p className="text-sm flex-1">{message}</p>
      <button
        type="button"
        onClick={onClose}
        className="text-current opacity-70 hover:opacity-100"
      >
        <X className="w-3 h-3" />
      </button>
    </div>
  );
}