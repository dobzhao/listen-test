// 音频设置：音量、TTS 静音时长

import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useSettingsStore } from "@/store/settings";

export function AudioSettingsPanel() {
  const audio = useSettingsStore((s) => s.config.audio);
  const updateAudio = useSettingsStore((s) => s.updateAudio);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">音频设置</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-2 gap-3">
          <div className="space-y-1">
            <Label htmlFor="playback-volume">
              播放音量（0.0 - 1.0）
            </Label>
            <Input
              id="playback-volume"
              type="number"
              min={0}
              max={1}
              step={0.05}
              value={audio.playback_volume}
              onChange={(e) =>
                updateAudio({ playback_volume: Number(e.target.value) })
              }
            />
          </div>
          <div className="space-y-1">
            <Label htmlFor="mic-gain">
              麦克风增益（0.0 - 2.0）
            </Label>
            <Input
              id="mic-gain"
              type="number"
              min={0}
              max={2}
              step={0.05}
              value={audio.mic_gain}
              onChange={(e) =>
                updateAudio({ mic_gain: Number(e.target.value) })
              }
            />
          </div>
        </div>
        <div className="space-y-1">
          <Label htmlFor="tts-silence">
            TTS 拼接静音时长（毫秒，推荐 300-500）
          </Label>
          <Input
            id="tts-silence"
            type="number"
            min={0}
            max={2000}
            step={50}
            value={audio.tts_silence_ms}
            onChange={(e) =>
              updateAudio({ tts_silence_ms: Number(e.target.value) })
            }
          />
        </div>
      </CardContent>
    </Card>
  );
}
