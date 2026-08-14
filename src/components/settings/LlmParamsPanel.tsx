// LLM 调用参数（temperature / max_tokens / top_p / top_k）

import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useSettingsStore } from "@/store/settings";

export function LlmParamsPanel() {
  const params = useSettingsStore((s) => s.config.llm_params);
  const updateLlmParams = useSettingsStore((s) => s.updateLlmParams);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">LLM 调用参数</CardTitle>
        <p className="text-sm text-muted-foreground">
          不同本地模型对这些参数的最优取值不同，请按实际部署调整。
        </p>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-2 gap-3">
          <div className="space-y-1">
            <Label htmlFor="temperature">Temperature（默认 1.0）</Label>
            <Input
              id="temperature"
              type="number"
              step={0.05}
              min={0}
              max={2}
              value={params.temperature}
              onChange={(e) =>
                updateLlmParams({ temperature: Number(e.target.value) })
              }
            />
          </div>
          <div className="space-y-1">
            <Label htmlFor="top-p">Top P（默认 0.95）</Label>
            <Input
              id="top-p"
              type="number"
              step={0.05}
              min={0}
              max={1}
              value={params.top_p}
              onChange={(e) =>
                updateLlmParams({ top_p: Number(e.target.value) })
              }
            />
          </div>
          <div className="space-y-1">
            <Label htmlFor="top-k">Top K（默认 64）</Label>
            <Input
              id="top-k"
              type="number"
              step={1}
              min={0}
              max={200}
              value={params.top_k}
              onChange={(e) =>
                updateLlmParams({ top_k: Number(e.target.value) })
              }
            />
          </div>
          <div className="space-y-1">
            <Label htmlFor="max-tokens">Max Tokens（默认 81920）</Label>
            <Input
              id="max-tokens"
              type="number"
              step={1024}
              min={256}
              value={params.max_tokens}
              onChange={(e) =>
                updateLlmParams({ max_tokens: Number(e.target.value) })
              }
            />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
