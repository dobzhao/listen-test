// 全局确认对话框 store：替代 window.confirm()，解决 macOS WKWebView 下
// window.confirm 不可用（Tauri/wry 未实现 WKUIDelegate 的 confirm panel）的问题。

import { create } from "zustand";

export type ConfirmOptions = {
  title?: string;
  description: string;
  confirmText?: string;
  cancelText?: string;
  variant?: "default" | "destructive";
};

type State = {
  open: boolean;
  request: ConfirmOptions | null;
  resolver: ((value: boolean) => void) | null;
};

export const useConfirmStore = create<State>(() => ({
  open: false,
  request: null,
  resolver: null,
}));

/**
 * 打开全局确认对话框，返回 Promise<boolean>：
 * - 用户点击「确认」 → resolve(true)
 * - 用户点击「取消」/ESC/遮罩 → resolve(false)
 *
 * 用法：
 *   if (await confirm("确认放弃本次测试？")) { ... }
 */
export function confirm(opts: string | ConfirmOptions): Promise<boolean> {
  const state = useConfirmStore.getState();
  if (state.open) {
    throw new Error("ConfirmDialog: another request is already pending");
  }
  const options: ConfirmOptions =
    typeof opts === "string" ? { description: opts } : opts;
  return new Promise<boolean>((resolve) => {
    useConfirmStore.setState({
      open: true,
      request: options,
      resolver: resolve,
    });
  });
}

/** 关闭当前对话框并 resolve 用户的决定。组件内部使用。 */
export function resolveConfirm(value: boolean): void {
  const { resolver } = useConfirmStore.getState();
  if (resolver) {
    resolver(value);
  }
  useConfirmStore.setState({ open: false, request: null, resolver: null });
}
