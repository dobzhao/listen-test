// Prompt 编辑器：多行文本框 + 恢复默认值按钮 + 占位符说明

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import { RotateCcw } from "lucide-react";
import { PROMPT_PLACEHOLDERS } from "@/types/config";
import type { PromptKey } from "@/types/config";
import { useSettingsStore } from "@/store/settings";

interface Props {
  promptKey: PromptKey;
  title: string;
  description: string;
}

export function PromptEditor({ promptKey, title, description }: Props) {
  const value = useSettingsStore((s) => s.config.prompts[promptKey]);
  const updatePrompt = useSettingsStore((s) => s.updatePrompt);
  const restoreOnePrompt = useSettingsStore((s) => s.restoreOnePrompt);
  const placeholders = PROMPT_PLACEHOLDERS[promptKey];

  const handleRestore = async () => {
    if (
      !window.confirm(
        `确认将「${title}」恢复为默认模板？\n当前编辑内容将丢失。`
      )
    ) {
      return;
    }
    await restoreOnePrompt(promptKey);
  };

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg">{title}</CardTitle>
          <Button variant="ghost" size="sm" onClick={handleRestore}>
            <RotateCcw className="w-4 h-4 mr-1" />
            恢复默认
          </Button>
        </div>
        <p className="text-sm text-muted-foreground">{description}</p>
      </CardHeader>
      <CardContent className="space-y-3">
        <Textarea
          value={value}
          onChange={(e) => updatePrompt(promptKey, e.target.value)}
          rows={Math.min(16, Math.max(8, Math.ceil(value.length / 80)))}
          className="font-mono text-xs leading-relaxed"
          placeholder="Prompt 模板尚未加载，点击「恢复默认」获取默认值…"
        />
        <div className="text-xs space-y-1">
          <p className="font-medium text-muted-foreground">可用占位符：</p>
          {placeholders.length === 0 ? (
            <p className="text-muted-foreground">无（此 Prompt 不需要替换变量）</p>
          ) : (
            <div className="flex flex-wrap gap-2">
              {placeholders.map((p) => (
                <Badge key={p.key} variant="secondary" className="font-mono">
                  {`{{${p.key}}}`}
                  <span className="ml-1.5 font-sans font-normal text-muted-foreground">
                    {p.desc}
                  </span>
                </Badge>
              ))}
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
