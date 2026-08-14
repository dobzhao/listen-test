# CLAUDE.md

> 给 Claude Code / AI 助手的项目说明文件
> 详细需求请参考 [Spec.md](./Spec.md)

## 一、项目简介

**peiyuan（英语听力练习）** 是一个跨平台桌面英语听力练习程序，基于 **Tauri 2.0 + React 18 + TypeScript + Tailwind CSS + shadcn/ui**。

完整模拟 19 道英语听说考试题：
- **第 1-14 题**：听后选择（短对话 / 长对话 / 独白）
- **第 15-19 题**：听后转述（挖空填空 + 口头转述录音）

所有题目内容由本地/自建的 OpenAI 兼容 LLM 实时生成，语音由 TTS 实时合成，第 19 题用户录音经 STT 转写后由 LLM 判分。

**所有 Prompt 模板与判分规则均可在设置界面编辑，无需修改代码。**

---

## 二、技术栈

### 2.1 前端

| 技术 | 版本 | 用途 |
|------|------|------|
| Tauri | 2.x | 桌面运行时（含 protocol-asset feature） |
| React | 18 | UI 框架 |
| TypeScript | 5.6+ | strict 模式 |
| Vite | 5 | 构建工具 |
| Tailwind CSS | 3.4 | 样式 |
| shadcn/ui | 最新 | UI 组件（手动引入源码） |
| Zustand | 5 | 状态管理 |
| React Router | 6 | 路由 |
| @tauri-apps/api | 2 | 与 Rust 后端通信 |

### 2.2 后端 Rust

| 技术 | 版本 | 用途 |
|------|------|------|
| Rust | 2021 ≥ 1.75 | 系统语言 |
| tokio | 1（full | 异步运行时 |
| reqwest | 0.12 | HTTP 客户端（含 stream + multipart + rustls-tls） |
| eventsource-stream | 0.2 | SSE 流式响应解析 |
| cpal | 0.15 | 跨平台麦克风采集 |
| rodio | 0.19 | 跨平台音频播放 |
| hound | 3.5 | WAV 编码/解码 |
| serde / serde_json | 1 | 序列化 |
| tracing | 0.1 | 日志 |
| uuid | 1（v4 | 会话标识 |
| thiserror / anyhow | - | 错误处理 |

### 2.3 数据存储

- 配置文件：`{app_data_dir}/config.json`
- 测试缓存：`{app_data_dir}/cache/{session_uuid}/`
- 设备测试录音：`{app_data_dir}/device-tests/`

平台路径：
- Windows: `%APPDATA%\com.peiyuan.app\`
- macOS: `~/Library/Application Support/com.peiyuan.app/`
- Linux: `~/.local/share/com.peiyuan.app/`

---

## 三、目录结构

```
peiyuan/
├── src/                              # 前端 React + TS
│   ├── components/
│   │   ├── ui/                       # shadcn/ui 基础组件（button, card, input, tabs, ...）
│   │   ├── settings/                 # ModelConfigForm / PromptEditor / MicTest / KeyboardTest
│   │   ├── test/                     # GlobalHeader / PhaseCountdown / QuestionDisplay / FillBlankTable / RecorderPanel
│   │   ├── ErrorBoundary.tsx
│   │   └── Toast.tsx
│   ├── pages/                         # MainMenu / Settings / Test / Result
│   ├── store/                         # Zustand: settings / test / testFlow / result
│   ├── hooks/                         # useTimerEvent / useAudioPlayer / useRecorder / useGenerationProgress / useTestFlowEvents
│   ├── types/                         # TypeScript 类型（与 Rust models/ 对齐）
│   ├── lib/                           # tauri invoke 封装 + utils
│   ├── App.tsx                        # 路由根
│   └── main.tsx                       # React 入口
│
├── src-tauri/                         # 后端 Rust
│   ├── src/
│   │   ├── commands/                  # Tauri commands
│   │   │   ├── config.rs              # get_config / save_config / reset_config / restore_default_prompt
│   │   │   ├── llm.rs                 # test_llm_connection / generate_with_llm
│   │   │   ├── tts.rs                 # test_tts_connection
│   │   │   ├── stt.rs                 # test_stt_connection / transcribe_audio
│   │   │   ├── audio.rs               # play_audio_file / play_audio_background / play_audio_blocking
│   │   │   ├── recorder.rs            # start_recording / stop_recording / get_audio_level
│   │   │   ├── device.rs              # list_input/output_devices / test_input/output_device
│   │   │   ├── test_session.rs        # generate_test_session / get_test_session / clear_test_session
│   │   │   ├── test_flow.rs           # start_test_flow / submit_answer / get_flow_state / get_answer_set / reset_test_flow
│   │   │   └── scoring.rs             # score_full_test
│   │   ├── services/                  # 内部服务层（与 commands 一一对应或多个合并）
│   │   │   ├── http_client.rs
│   │   │   ├── llm_service.rs         # 流式 LLM + SSE 拼接
│   │   │   ├── tts_service.rs         # 多 voice 拼接
│   │   │   ├── stt_service.rs
│   │   │   ├── audio_pipeline.rs      # 音频生成流水线
│   │   │   ├── audio_player.rs        # rodio 后端播放
│   │   │   ├── prompt_engine_service.rs # 占位符替换
│   │   │   ├── question_generator.rs  # LLM 出题
│   │   │   ├── recorder.rs            # cpal 录音（worker 线程 + 共享 Arc）
│   │   │   ├── scoring.rs             # 1-14 本地 / 15-18 LLM / 19 STT+LLM
│   │   │   ├── test_flow.rs           # 状态机编排
│   │   │   ├── test_session.rs        # 测试会话生成
│   │   │   ├── timer.rs               # 精确计时 + 事件推送
│   │   │   └── tts_service.rs
│   │   ├── models/                    # 数据结构
│   │   │   ├── config.rs              # AppConfig / ModelConfig / LlmParams / PromptConfig / AudioConfig
│   │   │   ├── question.rs            # ShortDialogue / LongDialogue / Monologue / RetellMaterial / TestSession
│   │   │   └── result.rs              # McqResult / BlankResult / RetellResult / TestResult
│   │   ├── utils/
│   │   │   ├── json_extract.rs       # 剥离 ```json 围栏 + 容错解析
│   │   │   ├── path.rs                # 应用数据目录
│   │   │   ├── prompt_engine.rs       # {{KEY}} 占位符替换
│   │   │   ├── retry.rs               # 指数退避重试
│   │   │   └── wav.rs                 # wav 读写 / 拼接 / 静音生成
│   │   ├── lib.rs                     # Tauri Builder（注册 commands 与 State）
│   │   └── main.rs                    # 主入口
│   ├── prompts/                       # 默认 Prompt 模板（编译期 include_str! 嵌入）
│   │   ├── q1_4.txt
│   │   ├── q5_14.txt
│   │   ├── q15_18.txt
│   │   ├── q15_18_scoring.txt
│   │   └── q19_scoring.txt
│   ├── Cargo.toml
│   ├── tauri.conf.json                # 含 assetProtocol.scope: ["**"]
│   ├── capabilities/default.json
│   └── build.rs
│
├── package.json
├── tsconfig.json
├── vite.config.ts
├── tailwind.config.js
├── postcss.config.js
├── components.json
├── Spec.md                            # 详细需求规格
├── CLAUDE.md                          # 本文件
└── README.md                          # 启动 / 跨平台打包指南
```

---

## 四、核心架构约定

### 4.1 前后端数据流

- **命令式调用**：前端通过 `invoke('command_name', args)` 主动调用后端命令
- **事件式通知**：后端通过 `app.emit('event_name', payload)` 主动推送，前端 `listen` 订阅
- **共享类型**：TS 类型在 `src/types/`，Rust 类型在 `src-tauri/src/models/`，字段一一对应

### 4.2 后端状态管理（Tauri State）

| 状态 | 类型 | 生命周期 | 用途 |
|------|------|----------|------|
| `ConfigState` | `RwLock<AppConfig>` | 整个 App | 内存中的配置缓存 |
| `SessionState` | `Mutex<Option<TestSession>>` | 整个 App | 当前测试会话 |
| `FlowGlobal` | 含 `FlowStateContainer` | 整个 App | 1-19 题流程运行时状态 |
| `RecorderGlobal` | 含 `Arc<RecorderState>` | 整个 App | 录音状态（worker 线程） |
| `AudioPlaybackState` | 含 `Arc<Mutex<bool>>` | 整个 App | 音频播放状态标记 |

### 4.3 后端事件命名

| 事件名 | Payload | 触发时机 |
|--------|---------|----------|
| `test-generation-progress` | `{stage, message, progress}` | 题目预生成各阶段 |
| `test-timer-tick` | `{phase, elapsedMs, durationMs, remainingMs, progress}` | 每 100ms |
| `test-phase-finished` | `phase` | 阶段倒计时结束 |
| `test-flow-state` | `FlowState` | 阶段切换 |
| `test-flow-finished` | `{ok, completed?, error?}` | 1-19 题全部完成 |
| `test-audio-play` | `{path, loop}` | 通知前端播放（实际播放由后端 rodio 完成） |
| `test-record-start` | `{durationMs}` | 进入 19 题录音阶段 |
| `test-record-stop` | - | 录音结束 |

### 4.4 关键设计决策

#### 4.4.1 后端驱动计时
所有倒计时由 Rust `tokio::time::Instant` 驱动，每 100ms emit `test-timer-tick` 事件。前端订阅事件渲染进度条，避免 `setTimeout` 在 webview 失焦时漂移。

#### 4.4.2 后端驱动音频播放
测试阶段音频由后端 **rodio** 播放，而非前端 HTML5 audio。原因：
- 避免 WebKitGTK / Chromium webview 的 autoplay 限制
- 避免 asset protocol 跨平台兼容问题
- 跨平台一致

#### 4.4.3 cpal::Stream 非 Send/Sync 的处理
cpal::Stream 标记为 `!Send + !Sync`，无法在 Tauri State 中直接保存。解决方案：
- `RecorderState` 仅保存 Send + Sync 字段（Sender、共享 Arc<Mutex<Vec<f32>>>）
- 独立 worker 线程通过 `mpsc::channel` 接收命令，持有实际的 cpal::Stream

#### 4.4.4 LLM 流式响应拼接
- reqwest `bytes_stream()` + `eventsource-stream` 解析 SSE
- 拼接 `choices[0].delta.content` 得到完整文本
- 用 `utils::json_extract::try_parse` 剥离 ```json 围栏后解析

#### 4.4.5 JSON 解析容错
- `try_parse(text)` 自动剥离 ```json 围栏与首尾杂文
- 解析失败 → `retry_async` 重试（最多 2-3 次）
- 重试全部失败 → 返回错误给前端，用户可手动重试

#### 4.4.6 配置向后兼容
新增字段（如 `protocol`）使用 `#[serde(default = "default_xxx")]`，旧配置文件自动回退默认值。

#### 4.4.7 UTF-8 字符串安全截断
日志截断必须按字符而非字节切割（避免多字节字符中间 panic）。提供 `truncate_chars(s, n)` 工具。

### 4.5 模块分层

```
UI (React 组件)
  ↓ invoke / listen
Tauri commands (commands/*.rs)        ← 入参校验、状态读写、事件发射
  ↓
Services (services/*.rs)              ← 业务逻辑、HTTP / 音频处理、状态机
  ↓
Models (models/*.rs)                  ← 数据结构（与前端 types/ 对齐）
  ↓
Utils (utils/*.rs)                    ← 通用工具：JSON 解析、重试、占位符、wav
```

### 4.6 测试流程状态机

```
1-4 题：INTRO → PREPARE → PLAYING(1x) → ANSWERING → 下一题
5-12 题（4 段）：PREPARE → PLAYING(2x 间隔 2s) → ANSWERING(共享 10s)
13-14 题（独白）：PREPARE → PLAYING(1x) → ANSWERING
15-19 题：
  PREPARE(30s) → PLAYING(2x 间隔 3s) → FILL_BLANK(90s) →
  PLAYING(第 3 次) → RECALL_PREP(120s) → RECORDING(90s) → DONE
```

---

## 五、开发与构建

### 5.1 开发模式

```bash
npm install
npm run tauri:dev
```

启用 debug 日志：
```bash
RUST_LOG=peiyuan=debug npm run tauri:dev
RUST_LOG=trace npm run tauri:dev
```

### 5.2 生产打包

```bash
npm run tauri:build                                    # 当前平台
npm run tauri:build -- --target x86_64-pc-windows-msvc # Windows
npm run tauri:build -- --target aarch64-apple-darwin  # macOS ARM
npm run tauri:build -- --target x86_64-unknown-linux-gnu # Linux
```

详见 [README.md](./README.md)。

---

## 七、详细需求

所有功能需求、UI 规范、评分规则、非功能性需求等请参考 [Spec.md](./Spec.md)。

**核心要点速查**：
- 19 道题分两段：1-14 选择 / 15-19 转述
- 全部使用本地/自建 OpenAI 兼容模型服务
- 5 个 Prompt 模板可在 UI 编辑 + 恢复默认
- 后端驱动计时与音频播放
- 总分 30（14 + 6 + 10），自动评分
- 跨平台 Windows / macOS / Linux