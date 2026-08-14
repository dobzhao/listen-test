# 英语听力练习 (peiyuan)

跨平台桌面英语听力练习程序，基于 **Tauri 2.0 + React 18 + TypeScript + Tailwind CSS + shadcn/ui**。

支持 19 道完整题目：
- **第 1-14 题**：听后选择（短对话 / 长对话 / 独白）
- **第 15-19 题**：听后转述（挖空填空 + 口头转述录音）

所有题目内容由本地/自建 OpenAI 兼容 LLM 实时生成，语音由 TTS 实时合成，第 19 题用户录音经 STT 转写后由 LLM 判分。

---

## 当前阶段：全部 6 个 Phase 已完成 ✅

| Phase | 内容 | 状态 |
|-------|------|------|
| 1 | 项目骨架 + 设置界面（6 个 Tab） | ✅ |
| 2 | LLM/TTS 客户端 + 题目预生成流水线 | ✅ |
| 3 | 1-14 题完整测试流程（计时 / 播放 / 状态切换） | ✅ |
| 4 | 15-19 题挖空表格 + 麦克风录音 | ✅ |
| 5 | 判分 + 结算页（1-14 本地 / 15-18 LLM / 19 STT+LLM） | ✅ |
| 6 | 错误处理（ErrorBoundary + 重试 UI）+ 跨平台打包 | ✅ |

### 已实现能力

- **设置**：6 个 Tab（LLM / TTS / STT / 提示词 / 音频 / 设备测试），3 组模型服务独立配置（http/https 可选），5 个 Prompt 模板可编辑 + 恢复默认，LLM 调用参数可调
- **设备测试**：可选输入/输出设备，录音 3 秒并保存 wav + 回放，播放 440Hz 测试音
- **测试会话**：自动预生成 14 题对话音频 + 15-19 题听力材料，全程后端 rodio 播放（无 webview autoplay 限制）
- **测试流程**：后端驱动精确计时 + 状态机编排，前端实时倒计时进度条 + 阶段切换
- **判分**：1-14 本地比对，15-18 LLM 按 `q15_18_scoring` Prompt（满分 6 分），19 STT 转写 + LLM 按 `q19_scoring` Prompt（满分 10 分）
- **结算页**：1-14 逐题对错 + 答案 + 对话原文，15-18 用户答案 vs 标准答案 + 听力原文，19 得分 + LLM 评语 + STT 转写文本 + 原文，总分 30
- **错误处理**：React ErrorBoundary 兜底，生成失败可一键清除重试

---

## 启动开发环境

### 前置条件

| 工具 | 版本要求 | 说明 |
|------|----------|------|
| Node.js | ≥ 18 | 前端构建 |
| pnpm / npm / yarn | 任意 | 推荐 pnpm |
| Rust | ≥ 1.75 | Tauri 后端 |
| Tauri CLI | 2.x | `cargo install tauri-cli --version "^2"` |

平台特定依赖（参考 [Tauri 官方文档](https://tauri.app/start/prerequisites/)）：
- **macOS**：Xcode Command Line Tools
- **Windows**：Microsoft Visual Studio C++ Build Tools + WebView2
- **Linux**：
  ```bash
  sudo apt update
  sudo apt install -y \
    libwebkit2gtk-4.1-dev \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    pkg-config \
    build-essential
  ```

  音频相关依赖：
  ```bash
  sudo apt install -y libasound2-dev libpulse-dev
  ```
  并将当前用户加入 `audio` 组：
  ```bash
  sudo usermod -aG audio $USER
  ```

### 安装依赖

```bash
# 前端依赖
npm install

# Rust 依赖（首次 cargo build 会自动下载，可能耗时较长）
```

### 开发模式

```bash
npm run tauri:dev
```

应用窗口会自动打开，修改前端代码会自动热更新；修改 Rust 代码会自动重新编译。

### 启用调试日志

```bash
RUST_LOG=info npm run tauri:dev        # 默认级别
RUST_LOG=peiyuan=debug npm run tauri:dev  # 仅本项目 debug
RUST_LOG=trace npm run tauri:dev       # 全部 trace
```

---

## 生产打包（跨平台）

### 当前平台打包

```bash
npm run tauri:build
```

产物位于 `src-tauri/target/release/bundle/`：

| 平台 | 产物格式 | 路径 |
|------|----------|------|
| Windows | `.msi` / `.exe` | `bundle/msi/` 与 `bundle/nsis/` |
| macOS | `.app` / `.dmg` | `bundle/macos/` 与 `bundle/dmg/` |
| Linux | `.deb` / `.AppImage` / `.rpm` | `bundle/deb/` 与 `bundle/appimage/` |

### 跨平台打包

每种目标都需要先安装对应的 cross 工具链。

#### macOS (Apple Silicon / Intel)

```bash
# 安装目标
rustup target add aarch64-apple-darwin
rustup target add x86_64-apple-darwin

# 构建通用二进制需要分别构建后 lipo 合并（详见 Tauri 跨编译文档）
npm run tauri:build -- --target aarch64-apple-darwin
# 或
npm run tauri:build -- --target x86_64-apple-darwin
```

需要在 macOS 上交叉构建；Linux 上交叉构建到 macOS 需要 osxcross 工具链（复杂）。

#### Windows

```bash
# 在 macOS / Linux 上交叉编译到 Windows：
rustup target add x86_64-pc-windows-msvc
npm run tauri:build -- --target x86_64-pc-windows-msvc
```

Linux 交叉到 Windows 需要安装 `mingw-w64`：
```bash
sudo apt install -y mingw-w64
```

#### Linux

```bash
# 当前架构
rustup target add x86_64-unknown-linux-gnu
npm run tauri:build -- --target x86_64-unknown-linux-gnu

# ARM64（如树莓派 / Linux 服务器）
rustup target add aarch64-unknown-linux-gnu
npm run tauri:build -- --target aarch64-unknown-linux-gnu
```

#### 通用构建（macOS Universal Binary）

```bash
rustup target add x86_64-apple-darwin aarch64-apple-darwin
npm run tauri:build -- --target universal-apple-darwin
```

### 打包配置说明

打包相关的 Tauri 配置位于 [src-tauri/tauri.conf.json](src-tauri/tauri.conf.json)：

- `bundle.targets`: 打包目标（默认 `"all"`，可指定 `"deb"` / `"appimage"` / `"msi"` / `"nsis"` 等）
- `bundle.icon`: 各平台图标路径（需准备 `icons/32x32.png`、`icons/128x128.png`、`icons/icon.icns`、`icons/icon.ico` 等）

生成图标：
```bash
# 使用 tauri-cli 内置命令生成图标（需要一个 1024×1024 的源 png）
npx tauri icon path/to/source-icon.png
```

---

## 配置说明

### 配置文件位置

| 平台 | 路径 |
|------|------|
| Windows | `%APPDATA%\com.peiyuan.app\config.json` |
| macOS | `~/Library/Application Support/com.peiyuan.app/config.json` |
| Linux | `~/.local/share/com.peiyuan.app/config.json` |

### 缓存目录

每次测试生成的题目与音频位于：

```
{应用数据}/cache/{session_uuid}/
├── session.json             # 完整会话（含题目结构）
├── audio/
│   ├── q1.wav .. q4.wav     # 1-4 题短对话
│   ├── d1.wav .. d4.wav     # 5-12 题长对话
│   ├── m1.wav               # 13-14 题独白
│   └── retell.wav           # 15-19 题听力材料
└── recording.wav            # 第 19 题用户录音
```

设备测试录音位于 `{应用数据}/device-tests/`。

### 默认端点（OpenAI 兼容）

LLM：
```
POST {protocol}://{host}:{port}/v1/chat/completions
Headers: Authorization: Bearer {api_key}
Body: {
  "model": "{model}",
  "messages": [{"role": "user", "content": "..."}],
  "stream": true,
  "stream_options": { "include_usage": true }
}
```

TTS（Kokoro 等）：
```
POST {protocol}://{host}:{port}/v1/audio/speech
Body: { "model": "...", "input": "...", "voice": "af_heart", "speed": 1.0, "lang_code": "a" }
```

STT（Whisper 等）：
```
POST {protocol}://{host}:{port}/v1/audio/transcriptions
Form-data: file=audio.wav, model=..., language=en
```

### 双人对话 voice 约定

- 说话人 M → `am_michael`
- 说话人 W → `af_heart`

---

## 目录结构

```
peiyuan/
├── src/                              # 前端 React + TS
│   ├── components/
│   │   ├── ui/                       # shadcn/ui 基础组件
│   │   ├── settings/                 # 设置相关（ModelConfigForm / PromptEditor / MicTest / ...）
│   │   ├── test/                     # 测试组件（QuestionDisplay / FillBlankTable / RecorderPanel / ...）
│   │   ├── ErrorBoundary.tsx
│   │   └── Toast.tsx
│   ├── pages/                         # MainMenu / Settings / Test / Result
│   ├── store/                         # Zustand: settings / test / testFlow / result
│   ├── hooks/                         # useTimerEvent / useAudioPlayer / useRecorder / ...
│   ├── types/                         # TypeScript 类型（与 Rust 一致）
│   ├── lib/                           # tauri invoke 封装 + 工具
│   ├── App.tsx                        # 路由根
│   └── main.tsx                       # React 入口
│
├── src-tauri/                         # 后端 Rust
│   ├── src/
│   │   ├── commands/                  # Tauri commands
│   │   │   ├── config.rs              # 配置读写
│   │   │   ├── llm.rs                 # LLM 调用
│   │   │   ├── tts.rs                 # TTS 调用
│   │   │   ├── stt.rs                 # STT 调用
│   │   │   ├── audio.rs               # 音频播放 command
│   │   │   ├── recorder.rs            # 录音 command
│   │   │   ├── device.rs              # 设备测试 command
│   │   │   ├── test_session.rs        # 测试会话生成
│   │   │   ├── test_flow.rs           # 1-19 题流程编排
│   │   │   └── scoring.rs             # 判分
│   │   ├── services/                  # 内部服务层
│   │   │   ├── http_client.rs
│   │   │   ├── llm_service.rs         # 流式 LLM
│   │   │   ├── tts_service.rs         # 多 voice 拼接
│   │   │   ├── stt_service.rs
│   │   │   ├── audio_pipeline.rs      # 音频生成流水线
│   │   │   ├── audio_player.rs       # rodio 播放
│   │   │   ├── question_generator.rs  # LLM 出题
│   │   │   ├── test_session.rs        # 编排预生成
│   │   │   ├── test_flow.rs           # 状态机
│   │   │   ├── timer.rs               # 精确计时
│   │   │   ├── recorder.rs           # cpal 录音
│   │   │   └── scoring.rs             # 评分逻辑
│   │   ├── models/                    # 数据结构
│   │   ├── utils/                     # JSON 提取 / 重试 / 占位符 / wav / 路径
│   │   ├── lib.rs                     # Tauri Builder
│   │   └── main.rs
│   ├── prompts/                       # 默认 Prompt 模板（编译期嵌入）
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── build.rs
│
├── package.json
├── tsconfig.json
├── vite.config.ts
├── tailwind.config.js
└── README.md
```

---

## 模型服务部署示例

```bash
# vllm / mlx-lm / ollama 部署 LLM
vllm serve meta-llama/Llama-3.1-8B-Instruct --port 8000

# Kokoro TTS（OpenAI 兼容）
python -m kokoro_tts_server --port 8000

# Whisper（OpenAI 兼容）
python -m whisper_server --model large-v3 --port 8000
```

实际部署因模型/硬件不同，请参考各项目官方文档。

---

## 错误排查

| 现象 | 排查 |
|------|------|
| 应用启动失败 | 查看 `RUST_LOG=info` 日志，确认依赖与平台 SDK 完整 |
| LLM 连接失败 | 在设置页点「测试连接」，检查 host / port / api_path / api_key |
| 题目生成超时 | LLM 服务慢或模型过大；调整 `temperature` / `max_tokens` |
| TTS 合成失败 | 检查 TTS 服务是否支持 `voice` 字段 |
| 录音无声音 | 设备权限未授予（macOS 系统偏好设置 → 麦克风；Linux 检查 audio 组） |
| 测试阶段音频无声 | 后端 rodio 直连系统默认输出设备，检查 PulseAudio/ALSA |
| 结算页评分失败 | 通常是 LLM 解析失败，查看 Rust 端日志的 "评分 LLM 返回" 字段 |

---

## 许可

本项目按需求文档开发，所有 Prompt 模板与判分逻辑均可通过 UI 配置调整，无需修改代码。