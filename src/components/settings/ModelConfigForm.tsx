// 模型服务配置表单（LLM / TTS / STT 共用）

import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Loader2, Check, X } from "lucide-react";
import type { ModelConfig as ModelConfigType } from "@/types/config";
import { testLlmConnection, testTtsConnection, testSttConnection } from "@/lib/tauri";
import { cn } from "@/lib/utils";

export type ModelKind = "llm" | "tts" | "stt";

interface Props {
  kind: ModelKind;
  title: string;
  description: string;
  config: ModelConfigType;
  defaultPath: string;
  defaultModel: string;
  onChange: (patch: Partial<ModelConfigType>) => void;
}

export function ModelConfigForm({
  kind,
  title,
  description,
  config,
  defaultPath,
  defaultModel,
  onChange,
}: Props) {
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{
    ok: boolean;
    message: string;
  } | null>(null);

  const handleTest = async () => {
    setTesting(true);
    setTestResult(null);
    try {
      let msg: string;
      if (kind === "llm") msg = await testLlmConnection(config);
      else if (kind === "tts") msg = await testTtsConnection(config);
      else msg = await testSttConnection(config);
      setTestResult({ ok: true, message: msg });
    } catch (e) {
      setTestResult({ ok: false, message: String(e) });
    } finally {
      setTesting(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center justify-between text-lg">
          {title}
          {testResult && (
            <Badge variant={testResult.ok ? "success" : "destructive"}>
              {testResult.ok ? (
                <>
                  <Check className="w-3 h-3 mr-1" /> 已连接
                </>
              ) : (
                <>
                  <X className="w-3 h-3 mr-1" /> 失败
                </>
              )}
            </Badge>
          )}
        </CardTitle>
        <p className="text-sm text-muted-foreground">{description}</p>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-12 gap-3">
          <div className="col-span-3 space-y-1">
            <Label htmlFor={`${kind}-protocol`}>协议</Label>
            <select
              id={`${kind}-protocol`}
              className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
              value={config.protocol || "http"}
              onChange={(e) =>
                onChange({
                  protocol: e.target.value as "http" | "https",
                })
              }
            >
              <option value="http">http</option>
              <option value="https">https</option>
            </select>
          </div>
          <div className="col-span-6 space-y-1">
            <Label htmlFor={`${kind}-host`}>主机地址</Label>
            <Input
              id={`${kind}-host`}
              value={config.host}
              onChange={(e) => onChange({ host: e.target.value })}
              placeholder="127.0.0.1"
            />
          </div>
          <div className="col-span-3 space-y-1">
            <Label htmlFor={`${kind}-port`}>端口</Label>
            <Input
              id={`${kind}-port`}
              type="number"
              min={1}
              max={65535}
              value={config.port}
              onChange={(e) =>
                onChange({ port: Number(e.target.value) || 0 })
              }
            />
          </div>
        </div>

        <div className="space-y-1">
          <Label htmlFor={`${kind}-api-path`}>
            API 路径
            <span className="text-xs text-muted-foreground ml-2">
              默认：{defaultPath}
            </span>
          </Label>
          <Input
            id={`${kind}-api-path`}
            value={config.api_path}
            onChange={(e) => onChange({ api_path: e.target.value })}
            placeholder={defaultPath}
          />
        </div>

        <div className="space-y-1">
          <Label htmlFor={`${kind}-model`}>
            模型名称
            <span className="text-xs text-muted-foreground ml-2">
              默认：{defaultModel}
            </span>
          </Label>
          <Input
            id={`${kind}-model`}
            value={config.model}
            onChange={(e) => onChange({ model: e.target.value })}
            placeholder={defaultModel}
          />
        </div>

        <div className="space-y-1">
          <Label htmlFor={`${kind}-api-key`}>
            API Key（Authorization Bearer Token）
          </Label>
          <Input
            id={`${kind}-api-key`}
            type="password"
            value={config.api_key}
            onChange={(e) => onChange({ api_key: e.target.value })}
            placeholder="sk-..."
          />
        </div>

        <div className="flex items-center gap-2 pt-2">
          <Button onClick={handleTest} disabled={testing} variant="outline">
            {testing ? (
              <>
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                测试中…
              </>
            ) : (
              "测试连接"
            )}
          </Button>
          {testResult && (
            <p
              className={cn(
                "text-sm",
                testResult.ok ? "text-emerald-600" : "text-destructive"
              )}
            >
              {testResult.message}
            </p>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
