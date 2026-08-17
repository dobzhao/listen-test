// Prompt 试生成：用当前 LLM 配置 + Prompt 模板调用一次 LLM，方便设置界面预览生成效果

import { useState } from "react";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Loader2, Sparkles } from "lucide-react";
import { useSettingsStore } from "@/store/settings";
import { generateWithLlm } from "@/lib/tauri";
import type { PromptKey } from "@/types/config";

interface Props {
  promptKey: PromptKey;
}

export function GenerationPreview({ promptKey }: Props) {
  const config = useSettingsStore((s) => s.config);
  const [varsText, setVarsText] = useState("");
  const [output, setOutput] = useState("");
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleRun = async () => {
    setRunning(true);
    setError(null);
    setOutput("");
    try {
      const vars: Record<string, string> = {};
      varsText
        .split("\n")
        .map((line) => line.trim())
        .filter((line) => line && !line.startsWith("#"))
        .forEach((line) => {
          const [k, ...rest] = line.split("=");
          if (k) vars[k.trim()] = rest.join("=").trim();
        });

      const text = await generateWithLlm(
        config.llm,
        config.llm_params,
        config.prompts[promptKey],
        vars
      );
      setOutput(text);
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base flex items-center gap-2">
          <Sparkles className="w-4 h-4" />
          试生成（用当前 LLM 配置调用一次）
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="space-y-1">
          <Label htmlFor={`vars-${promptKey}`}>
            占位符变量（每行 KEY=VALUE；空 KEY 由后端自动注入）
          </Label>
          <Input
            id={`vars-${promptKey}`}
            value={varsText}
            onChange={(e) => setVarsText(e.target.value)}
            placeholder="KEY=VALUE\nKEY=VALUE"
            className="font-mono text-xs"
          />
        </div>
        <Button onClick={handleRun} disabled={running} size="sm">
          {running ? (
            <>
              <Loader2 className="w-4 h-4 mr-2 animate-spin" />
              调用中…
            </>
          ) : (
            "调用 LLM"
          )}
        </Button>
        {error && (
          <p className="text-sm text-destructive whitespace-pre-wrap">
            {error}
          </p>
        )}
        {output && (
          <Textarea
            readOnly
            value={output}
            rows={12}
            className="font-mono text-xs"
          />
        )}
      </CardContent>
    </Card>
  );
}
