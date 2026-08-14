// 全局 React 错误边界
// 捕获 React 组件树中任何位置抛出的错误，渲染友好的错误页
// 而不是白屏崩溃。

import { Component, type ErrorInfo, type ReactNode } from "react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { AlertCircle, RefreshCw, Home } from "lucide-react";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("ErrorBoundary 捕获到错误", error, info);
  }

  handleReset = () => {
    this.setState({ error: null });
  };

  handleReload = () => {
    window.location.reload();
  };

  handleHome = () => {
    window.location.href = "/";
  };

  render() {
    if (this.state.error) {
      return (
        <div className="min-h-screen flex items-center justify-center p-8 bg-slate-50">
          <Card className="max-w-lg w-full">
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-destructive">
                <AlertCircle className="w-5 h-5" />
                应用遇到错误
              </CardTitle>
              <CardDescription>
                界面渲染异常。可以尝试刷新页面或返回主菜单。
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="rounded-md border border-destructive/30 bg-destructive/5 p-3">
                <p className="text-xs font-mono text-destructive break-all whitespace-pre-wrap">
                  {this.state.error.message}
                </p>
              </div>
              <div className="flex gap-2">
                <Button onClick={this.handleReset} variant="outline">
                  <RefreshCw className="w-4 h-4 mr-1" />
                  重试
                </Button>
                <Button onClick={this.handleReload}>
                  <RefreshCw className="w-4 h-4 mr-1" />
                  刷新页面
                </Button>
                <Button onClick={this.handleHome} variant="ghost">
                  <Home className="w-4 h-4 mr-1" />
                  返回主菜单
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">
                如错误持续出现，请查看应用日志（Rust 端由 RUST_LOG 环境变量控制）。
              </p>
            </CardContent>
          </Card>
        </div>
      );
    }

    return this.props.children;
  }
}