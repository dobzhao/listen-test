// 结算页：展示 1-14 选择题对错、15-18 挖空得分、19 题转述得分

import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  CheckCircle2,
  XCircle,
  Loader2,
  RefreshCw,
  Home,
  AlertCircle,
} from "lucide-react";
import { useResultStore } from "@/store/result";
import { useTestStore } from "@/store/test";
import type { McqResult, BlankResult, RetellResult } from "@/types/result";

export default function ResultPage() {
  const navigate = useNavigate();
  const result = useResultStore((s) => s.result);
  const loading = useResultStore((s) => s.loading);
  const error = useResultStore((s) => s.error);
  const load = useResultStore((s) => s.load);
  const resetResult = useResultStore((s) => s.reset);

  const resetSession = useTestStore((s) => s.reset);

  // 进入页面时自动加载评分
  useEffect(() => {
    if (!result && !loading && !error) {
      load();
    }
  }, [result, loading, error, load]);

  const handleBackToMenu = () => {
    if (
      window.confirm(
        "确认返回主菜单？\n将清空当前测试结果。"
      )
    ) {
      resetResult();
      resetSession();
      navigate("/");
    }
  };

  const handleRetest = async () => {
    resetResult();
    resetSession();
    navigate("/");
    // 用户需要重新点"开始测试"生成新的题目
  };

  if (loading && !result) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-slate-50">
        <Card className="max-w-md w-full">
          <CardContent className="py-12 text-center space-y-3">
            <Loader2 className="w-10 h-10 mx-auto animate-spin text-primary" />
            <p className="text-sm">正在评分（1-14 本地 + 15-19 LLM + STT 转写）…</p>
            <p className="text-xs text-muted-foreground">
              首次调用 STT 与 LLM 可能需要数十秒
            </p>
          </CardContent>
        </Card>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-slate-50 p-8">
        <Card className="max-w-md w-full">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-destructive">
              <AlertCircle className="w-5 h-5" />
              评分失败
            </CardTitle>
            <CardDescription className="text-xs whitespace-pre-wrap">
              {error}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-2">
            <Button className="w-full" onClick={() => load()}>
              <RefreshCw className="w-4 h-4 mr-2" />
              重试评分
            </Button>
            <Button
              variant="outline"
              className="w-full"
              onClick={() => navigate("/")}
            >
              返回主菜单
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  if (!result) return null;

  const correctCount = result.mcq_results.filter((r) => r.is_correct).length;
  const totalPct = (result.total_score / result.max_score) * 100;

  return (
    <div className="min-h-screen bg-slate-50">
      <header className="border-b bg-white sticky top-0 z-10">
        <div className="container max-w-5xl mx-auto py-4 flex items-center justify-between">
          <div>
            <h1 className="text-xl font-bold">测试结果</h1>
            <p className="text-xs text-muted-foreground">
              Session: <span className="font-mono">{result.session_id}</span>
            </p>
          </div>
          <div className="flex gap-2">
            <Button variant="outline" onClick={handleRetest}>
              <RefreshCw className="w-4 h-4 mr-1" />
              重新测试
            </Button>
            <Button onClick={handleBackToMenu}>
              <Home className="w-4 h-4 mr-1" />
              返回主菜单
            </Button>
          </div>
        </div>
      </header>

      <main className="container max-w-5xl mx-auto py-6 space-y-6">
        {/* 总分卡片 */}
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">总分</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-baseline gap-2">
              <span className="text-5xl font-bold text-primary">
                {result.total_score.toFixed(1)}
              </span>
              <span className="text-xl text-muted-foreground">
                / {result.max_score}
              </span>
              <span className="ml-3 text-sm text-muted-foreground">
                ({totalPct.toFixed(1)}%)
              </span>
            </div>
            <Separator className="my-3" />
            <div className="grid grid-cols-3 gap-4 text-sm">
              <SummaryItem
                label="1-14 题正确数"
                value={`${correctCount} / 14`}
              />
              <SummaryItem
                label="15-18 题得分"
                value={`${result.blank_total_score.toFixed(1)} / 6`}
              />
              <SummaryItem
                label="19 题得分"
                value={`${result.retell_result?.score.toFixed(1) ?? "0.0"} / 10`}
              />
            </div>
          </CardContent>
        </Card>

        {/* 1-14 题逐题对错 */}
        <McqSection results={result.mcq_results} dialogueTexts={result.dialogue_texts} />

        {/* 15-18 题填空 */}
        <BlankSection results={result.blank_results} />

        {/* 19 题转述 */}
        <RetellSection retell={result.retell_result} passage={result.dialogue_texts.retell} />
      </main>
    </div>
  );
}

function SummaryItem({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="font-mono font-medium">{value}</p>
    </div>
  );
}

// === 1-14 题 ===

function McqSection({
  results,
  dialogueTexts,
}: {
  results: McqResult[];
  dialogueTexts: Record<string, string>;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">1-14 题（听后选择）</CardTitle>
        <CardDescription>
          点击展开对话原文与正确答案
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-2 md:grid-cols-7 gap-2">
          {results.map((r) => (
            <McqCard key={r.question_id} result={r} />
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

function McqCard({ result }: { result: McqResult }) {
  const [open, setOpen] = useState(false);
  const userLabel = result.user_answer ?? "未作答";
  return (
    <div>
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className={`w-full rounded-md border p-2.5 text-left transition-colors ${
          result.is_correct
            ? "border-emerald-200 bg-emerald-50 hover:bg-emerald-100"
            : "border-rose-200 bg-rose-50 hover:bg-rose-100"
        }`}
      >
        <div className="flex items-center justify-between">
          <span className="font-mono text-sm font-semibold">
            Q{result.question_id}
          </span>
          {result.is_correct ? (
            <CheckCircle2 className="w-4 h-4 text-emerald-600" />
          ) : (
            <XCircle className="w-4 h-4 text-rose-600" />
          )}
        </div>
        <p className="mt-1 text-xs">
          {result.is_correct ? (
            <span className="text-emerald-700">正确 ({userLabel})</span>
          ) : (
            <span className="text-rose-700">
              错（你：{userLabel} / 正：{result.correct_answer}）
            </span>
          )}
        </p>
      </button>
    </div>
  );
}

// === 15-18 题 ===

function BlankSection({ results }: { results: BlankResult[] }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">15-18 题（听后填词）</CardTitle>
        <CardDescription>
          由 LLM 按 9.4 节评分 Prompt 评分（大小写不敏感、单复数严格、英美拼写允许）
        </CardDescription>
      </CardHeader>
      <CardContent>
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b">
              <th className="text-left py-2 w-16">题号</th>
              <th className="text-left py-2">你的答案</th>
              <th className="text-left py-2">标准答案</th>
              <th className="text-right py-2 w-20">得分</th>
              <th className="text-right py-2 w-16">结果</th>
            </tr>
          </thead>
          <tbody>
            {results.map((r) => (
              <tr key={r.blank_id} className="border-b">
                <td className="py-2 font-mono font-semibold">{r.blank_id}</td>
                <td className="py-2 font-mono">
                  {r.user_answer || <span className="text-muted-foreground">空</span>}
                </td>
                <td className="py-2 font-mono">{r.correct_answer}</td>
                <td className="py-2 text-right font-mono">{r.score.toFixed(1)}</td>
                <td className="py-2 text-right">
                  {r.is_correct ? (
                    <Badge variant="success" className="text-xs">
                      对
                    </Badge>
                  ) : (
                    <Badge variant="destructive" className="text-xs">
                      错
                    </Badge>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
          <tfoot>
            <tr>
              <td colSpan={3} className="py-2 text-right font-semibold">
                小计
              </td>
              <td className="py-2 text-right font-mono font-bold">
                {results.reduce((sum, r) => sum + r.score, 0).toFixed(1)} / 6
              </td>
              <td />
            </tr>
          </tfoot>
        </table>
      </CardContent>
    </Card>
  );
}

// === 19 题转述 ===

function RetellSection({
  retell,
  passage,
}: {
  retell: RetellResult | null;
  passage?: string;
}) {
  if (!retell) return null;
  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">19 题（听后转述）</CardTitle>
        <CardDescription>
          STT 转写 → LLM 按 9.5 节评分 Prompt 评分
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex items-baseline gap-3">
          <span className="text-3xl font-bold text-primary">
            {retell.score.toFixed(1)}
          </span>
          <span className="text-muted-foreground">
            / {retell.max_score} 分
          </span>
        </div>

        {retell.comment && (
          <div>
            <p className="text-xs text-muted-foreground mb-1">LLM 评语</p>
            <p className="text-sm leading-relaxed p-3 bg-muted/30 rounded">
              {retell.comment}
            </p>
          </div>
        )}

        <details className="text-sm">
          <summary className="cursor-pointer text-muted-foreground hover:text-foreground">
            STT 转写文本（点击展开）
          </summary>
          <ScrollArea className="mt-2 max-h-60 rounded border bg-muted/30 p-3">
            <p className="whitespace-pre-wrap text-xs leading-relaxed">
              {retell.stt_text || "（空）"}
            </p>
          </ScrollArea>
        </details>

        {passage && (
          <details className="text-sm">
            <summary className="cursor-pointer text-muted-foreground hover:text-foreground">
              参考听力原文（点击展开）
            </summary>
            <p className="mt-2 text-xs leading-relaxed p-3 bg-muted/30 rounded whitespace-pre-wrap">
              {passage}
            </p>
          </details>
        )}
      </CardContent>
    </Card>
  );
}