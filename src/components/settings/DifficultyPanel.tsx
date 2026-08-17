// 题目难度设置：当前激活档下拉框 + 三档文字编辑（每档独立编辑，独立恢复默认）
//
// 顶层 Card 放激活档下拉框；下方三个 Card 分别对应三档难度（始终全部展示，
// 便于跨档对照）。每段文字旁有「恢复默认」按钮，每档标题旁有「恢复整档」按钮。
// 文本编辑直接绑 store，与 PromptEditor 行为一致。

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { RotateCcw } from "lucide-react";
import {
  DIFFICULTY_DEMAND_KEYS,
  DIFFICULTY_DEMAND_LABELS,
  DIFFICULTY_LEVELS,
  DIFFICULTY_LEVEL_LABELS,
  type DifficultyDemandKey,
  type DifficultyLevel,
} from "@/types/config";
import { useSettingsStore } from "@/store/settings";
import { confirm } from "@/store/confirm";

const SELECT_CLASS =
  "flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2";

export function DifficultyPanel() {
  const difficulty = useSettingsStore((s) => s.config.difficulty);
  const setDifficultyLevel = useSettingsStore((s) => s.setDifficultyLevel);
  const updateDifficultyDemand = useSettingsStore((s) => s.updateDifficultyDemand);
  const restoreOneDifficultyDemand = useSettingsStore(
    (s) => s.restoreOneDifficultyDemand
  );
  const restoreOneDifficultyLevel = useSettingsStore(
    (s) => s.restoreOneDifficultyLevel
  );

  const handleRestoreDemand = async (level: DifficultyLevel, key: DifficultyDemandKey) => {
    if (
      !(await confirm(
        `确认将「${DIFFICULTY_LEVEL_LABELS[level]} / ${DIFFICULTY_DEMAND_LABELS[key]}」恢复为默认文字？\n当前编辑内容将丢失。`
      ))
    ) {
      return;
    }
    await restoreOneDifficultyDemand(level, key);
  };

  const handleRestoreLevel = async (level: DifficultyLevel) => {
    if (
      !(await confirm(
        `确认将「${DIFFICULTY_LEVEL_LABELS[level]}」整档恢复为默认文字？\n该档 3 段文字均会还原，当前编辑内容将丢失。`
      ))
    ) {
      return;
    }
    await restoreOneDifficultyLevel(level);
  };

  return (
    <div className="space-y-4">
      {/* 顶部：当前激活档下拉框 */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">当前激活档</CardTitle>
          <p className="text-sm text-muted-foreground">
            选择本次出题使用的难度档位。下方三个卡片始终展示全部档位的文字，
            修改后请点击右上角「保存配置」生效。当前档位与所选文字共同决定出题
            prompt 中的{" "}
            <code className="px-1.5 py-0.5 rounded bg-muted font-mono text-xs">
              {"{{DIFFICULTY_DEMAND_*}}"}
            </code>{" "}
            占位符取值。
          </p>
        </CardHeader>
        <CardContent className="space-y-2">
          <Label htmlFor="difficulty-level">难度档位</Label>
          <select
            id="difficulty-level"
            className={SELECT_CLASS}
            value={difficulty.level}
            onChange={(e) => setDifficultyLevel(e.target.value as DifficultyLevel)}
          >
            {DIFFICULTY_LEVELS.map((lv) => (
              <option key={lv} value={lv}>
                {DIFFICULTY_LEVEL_LABELS[lv]}
              </option>
            ))}
          </select>
        </CardContent>
      </Card>

      {/* 三档文字编辑 */}
      {DIFFICULTY_LEVELS.map((lv) => (
        <Card key={lv}>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle className="text-lg">
                {DIFFICULTY_LEVEL_LABELS[lv]}
                {difficulty.level === lv && (
                  <span className="ml-2 text-xs font-normal text-muted-foreground">
                    （当前激活）
                  </span>
                )}
              </CardTitle>
              <Button variant="ghost" size="sm" onClick={() => handleRestoreLevel(lv)}>
                <RotateCcw className="w-4 h-4 mr-1" />
                恢复整档
              </Button>
            </div>
          </CardHeader>
          <CardContent className="space-y-4">
            {DIFFICULTY_DEMAND_KEYS.map((k, idx) => (
              <div key={k} className="space-y-1">
                {idx > 0 && <Separator className="my-3" />}
                <div className="flex items-center justify-between">
                  <Label htmlFor={`${lv}-${k}`}>{DIFFICULTY_DEMAND_LABELS[k]}</Label>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handleRestoreDemand(lv, k)}
                  >
                    <RotateCcw className="w-3 h-3 mr-1" />
                    恢复默认
                  </Button>
                </div>
                <Textarea
                  id={`${lv}-${k}`}
                  value={difficulty[lv][k]}
                  onChange={(e) => updateDifficultyDemand(lv, k, e.target.value)}
                  rows={Math.min(6, Math.max(2, Math.ceil(difficulty[lv][k].length / 60)))}
                  className="font-mono text-xs leading-relaxed"
                />
                <p className="text-xs text-muted-foreground">
                  占位符：{" "}
                  <code className="px-1 py-0.5 rounded bg-muted font-mono text-[10px]">
                    {`{{DIFFICULTY_DEMAND_${k.replace(/^demand_/, "").toUpperCase()}}}`}
                  </code>
                </p>
              </div>
            ))}
          </CardContent>
        </Card>
      ))}

      <Card>
        <CardContent className="py-4 text-xs text-muted-foreground">
          修改难度档位或文字后请点击右上角「保存配置」按钮。
        </CardContent>
      </Card>
    </div>
  );
}