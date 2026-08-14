// 键盘测试：捕获所有按键事件，方便用户确认键盘输入正常

import { useEffect, useState, useRef } from "react";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Trash2, Keyboard } from "lucide-react";

interface KeyEvent {
  id: number;
  key: string;
  code: string;
  shift: boolean;
  ctrl: boolean;
  alt: boolean;
  meta: boolean;
  timestamp: number;
}

export function KeyboardTest() {
  const [events, setEvents] = useState<KeyEvent[]>([]);
  const [focused, setFocused] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const counterRef = useRef(0);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // 只在组件聚焦时响应（避免影响其他输入）
      if (!inputRef.current?.contains(document.activeElement)) return;
      counterRef.current += 1;
      const evt: KeyEvent = {
        id: counterRef.current,
        key: e.key,
        code: e.code,
        shift: e.shiftKey,
        ctrl: e.ctrlKey,
        alt: e.altKey,
        meta: e.metaKey,
        timestamp: Date.now(),
      };
      setEvents((prev) => [evt, ...prev].slice(0, 50));
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const handleClear = () => setEvents([]);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg flex items-center gap-2">
          <Keyboard className="w-5 h-5" />
          键盘测试
        </CardTitle>
        <p className="text-sm text-muted-foreground">
          点击下方输入框后按键，所有按键事件会显示在下方。
          用于确认键盘响应正常、输入法状态正确等。
        </p>
      </CardHeader>
      <CardContent className="space-y-3">
        <Input
          ref={inputRef}
          placeholder="在此输入测试键盘…"
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
        />
        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <span>
            状态：
            {focused ? (
              <Badge variant="success" className="ml-1.5">已聚焦</Badge>
            ) : (
              <Badge variant="secondary" className="ml-1.5">未聚焦</Badge>
            )}
          </span>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleClear}
            disabled={events.length === 0}
          >
            <Trash2 className="w-4 h-4 mr-1" />
            清空
          </Button>
        </div>

        {events.length > 0 && (
          <div className="border rounded-md bg-muted/30 max-h-64 overflow-y-auto">
            <table className="w-full text-xs">
              <thead className="sticky top-0 bg-muted">
                <tr>
                  <th className="text-left p-2">key</th>
                  <th className="text-left p-2">code</th>
                  <th className="text-left p-2">修饰键</th>
                </tr>
              </thead>
              <tbody>
                {events.map((e) => (
                  <tr key={e.id} className="border-t">
                    <td className="p-2 font-mono">{e.key}</td>
                    <td className="p-2 font-mono">{e.code}</td>
                    <td className="p-2 font-mono">
                      {[
                        e.ctrl && "Ctrl",
                        e.shift && "Shift",
                        e.alt && "Alt",
                        e.meta && "Meta",
                      ]
                        .filter(Boolean)
                        .join("+") || "—"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
