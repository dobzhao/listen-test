// 麦克风与扬声器设备测试：
// - 列出所有输入/输出设备，可单选
// - 输入设备测试：录制 3 秒并保存 wav，前端可播放回放
// - 输出设备测试：通过后端播放 440Hz 测试音

import { useEffect, useState, useRef } from "react";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import {
  Loader2,
  Mic,
  Volume2,
  Play,
  Square,
  RefreshCw,
} from "lucide-react";
import {
  listInputDevices,
  listOutputDevices,
  testInputDevice,
  testOutputDevice,
  playAudioBackground,
  type DeviceInfo,
} from "@/lib/tauri";

export function MicTest() {
  const [inputs, setInputs] = useState<DeviceInfo[]>([]);
  const [outputs, setOutputs] = useState<DeviceInfo[]>([]);
  const [selectedInput, setSelectedInput] = useState<string>("");
  const [selectedOutput, setSelectedOutput] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 录音测试状态
  const [recording, setRecording] = useState(false);
  const [recordProgress, setRecordProgress] = useState(0);
  const [recordedPath, setRecordedPath] = useState<string | null>(null);
  const recordTimerRef = useRef<number | null>(null);

  // 播放测试音状态
  const [testingOutput, setTestingOutput] = useState(false);
  const [testOutputResult, setTestOutputResult] = useState<string | null>(null);

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      const [inp, out] = await Promise.all([
        listInputDevices(),
        listOutputDevices(),
      ]);
      setInputs(inp);
      setOutputs(out);
      // 默认选中第一个（系统默认）设备
      const defIn = inp.find((d) => d.is_default) ?? inp[0];
      const defOut = out.find((d) => d.is_default) ?? out[0];
      if (defIn) setSelectedInput(defIn.name);
      if (defOut) setSelectedOutput(defOut.name);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  // 卸载时清理
  useEffect(() => {
    return () => {
      if (recordTimerRef.current !== null) {
        window.clearInterval(recordTimerRef.current);
      }
    };
  }, []);

  const handleTestInput = async () => {
    if (!selectedInput) {
      setError("请先选择一个输入设备");
      return;
    }
    setRecording(true);
    setRecordProgress(0);
    setRecordedPath(null);
    setError(null);

    // 进度条模拟（实际时长由后端决定）
    const start = Date.now();
    recordTimerRef.current = window.setInterval(() => {
      const elapsed = Date.now() - start;
      const pct = Math.min(100, (elapsed / 3000) * 100);
      setRecordProgress(pct);
    }, 50);

    try {
      const resp = await testInputDevice({
        deviceName: selectedInput,
        durationMs: 3000,
      });
      setRecordedPath(resp.outputPath);
    } catch (e) {
      setError(`录音失败: ${e}`);
    } finally {
      if (recordTimerRef.current !== null) {
        window.clearInterval(recordTimerRef.current);
        recordTimerRef.current = null;
      }
      setRecording(false);
      setRecordProgress(100);
    }
  };

  const handlePlayRecording = () => {
    if (!recordedPath) return;
    playAudioBackground(recordedPath).catch((e) => {
      console.error("回放录音失败", e);
    });
  };

  const handleTestOutput = async () => {
    if (!selectedOutput) {
      setError("请先选择一个输出设备");
      return;
    }
    setTestingOutput(true);
    setTestOutputResult(null);
    setError(null);
    try {
      const msg = await testOutputDevice({
        deviceName: selectedOutput,
        durationMs: 1500,
      });
      setTestOutputResult(msg);
    } catch (e) {
      setError(`播放失败: ${e}`);
    } finally {
      setTestingOutput(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg flex items-center gap-2">
            <Mic className="w-5 h-5" />
            麦克风与扬声器设备测试
          </CardTitle>
          <Button variant="ghost" size="sm" onClick={refresh}>
            {loading ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <>
                <RefreshCw className="w-4 h-4 mr-1" />
                刷新
              </>
            )}
          </Button>
        </div>
        <p className="text-sm text-muted-foreground">
          选择一个输入设备测试录音，选择一个输出设备测试播放。
        </p>
      </CardHeader>
      <CardContent className="space-y-6">
        {error && (
          <p className="text-sm text-destructive bg-destructive/10 rounded p-2">
            {error}
          </p>
        )}

        {/* 输入设备 */}
        <div className="space-y-3">
          <p className="text-sm font-medium flex items-center gap-1.5">
            <Mic className="w-4 h-4" /> 输入设备（麦克风）
          </p>
          {inputs.length === 0 ? (
            <p className="text-sm text-muted-foreground">未检测到输入设备</p>
          ) : (
            <div className="space-y-1.5">
              {inputs.map((d, i) => {
                const checked = selectedInput === d.name;
                return (
                  <label
                    key={i}
                    className={`flex items-center justify-between p-2.5 rounded border cursor-pointer transition-colors ${
                      checked
                        ? "border-primary bg-primary/5"
                        : "hover:bg-muted/30"
                    }`}
                  >
                    <div className="flex items-center gap-2">
                      <input
                        type="radio"
                        name="input-device"
                        value={d.name}
                        checked={checked}
                        onChange={() => setSelectedInput(d.name)}
                        className="accent-primary"
                      />
                      <span className="text-sm font-mono">{d.name}</span>
                    </div>
                    {d.is_default && <Badge variant="success">默认</Badge>}
                  </label>
                );
              })}
            </div>
          )}

          <div className="flex flex-wrap items-center gap-2 pt-1">
            <Button
              onClick={handleTestInput}
              disabled={recording || !selectedInput}
              size="sm"
            >
              {recording ? (
                <>
                  <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                  录音中…
                </>
              ) : (
                <>
                  <Mic className="w-4 h-4 mr-2" />
                  录音 3 秒
                </>
              )}
            </Button>
            {recordedPath && (
              <Button
                onClick={handlePlayRecording}
                variant="outline"
                size="sm"
              >
                <Play className="w-4 h-4 mr-2" />
                回放录音
              </Button>
            )}
          </div>

          {recording && (
            <Progress value={recordProgress} className="h-1.5" />
          )}

          {recordedPath && !recording && (
            <p className="text-xs text-muted-foreground font-mono break-all">
              录音已保存: {recordedPath}
            </p>
          )}
        </div>

        <div className="border-t" />

        {/* 输出设备 */}
        <div className="space-y-3">
          <p className="text-sm font-medium flex items-center gap-1.5">
            <Volume2 className="w-4 h-4" /> 输出设备（扬声器）
          </p>
          {outputs.length === 0 ? (
            <p className="text-sm text-muted-foreground">未检测到输出设备</p>
          ) : (
            <div className="space-y-1.5">
              {outputs.map((d, i) => {
                const checked = selectedOutput === d.name;
                return (
                  <label
                    key={i}
                    className={`flex items-center justify-between p-2.5 rounded border cursor-pointer transition-colors ${
                      checked
                        ? "border-primary bg-primary/5"
                        : "hover:bg-muted/30"
                    }`}
                  >
                    <div className="flex items-center gap-2">
                      <input
                        type="radio"
                        name="output-device"
                        value={d.name}
                        checked={checked}
                        onChange={() => setSelectedOutput(d.name)}
                        className="accent-primary"
                      />
                      <span className="text-sm font-mono">{d.name}</span>
                    </div>
                    {d.is_default && <Badge variant="success">默认</Badge>}
                  </label>
                );
              })}
            </div>
          )}

          <div className="flex flex-wrap items-center gap-2 pt-1">
            <Button
              onClick={handleTestOutput}
              disabled={testingOutput || !selectedOutput}
              size="sm"
            >
              {testingOutput ? (
                <>
                  <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                  播放中…
                </>
              ) : (
                <>
                  <Volume2 className="w-4 h-4 mr-2" />
                  播放测试音
                </>
              )}
            </Button>
          </div>

          {testOutputResult && (
            <p className="text-xs text-muted-foreground">{testOutputResult}</p>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
