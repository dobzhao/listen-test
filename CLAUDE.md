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
| tracing | 0.1 | 日志门面 |
| tracing-subscriber | 0.3 | 日志格式化 + EnvFilter |
| tracing-appender | 0.2 | 按天滚动日志文件（详见 §4.4.13） |
| dirs | 5 | 跨平台路径解析（应用数据目录） |
| chrono | 0.4 | 时间戳（评分 prompt 用） |
| once_cell | 1 | 懒加载静态变量 |
| base64 | 0.22 | 音频内嵌备用编码 |
| futures | 0.3 | StreamExt（SSE 流解析） |
| uuid | 1（v4 | 会话标识 |
| thiserror / anyhow | - | 错误处理 |

### 2.3 数据存储

- 配置文件：`{app_data_dir}/config.json`
- 测试缓存：`{app_data_dir}/cache/{session_uuid}/`
  - 题目内容、生成的音频片段
  - `recording.wav` — Q19 用户录音（落盘前下采样到 mono 16kHz PCM，详见 §4.4.10）
- 设备测试录音：`{app_data_dir}/device-tests/` （由 `commands/device.rs::test_input_device` 按需创建）
- 日志：`{app_data_dir}/logs/peiyuan.log.YYYY-MM-DD` — 按天滚动，详见 §4.4.13

平台路径（应用 identifier 为 `com.peiyuan.desktop`，与 `tauri.conf.json:5` 一致）：
- Windows: `%APPDATA%\com.peiyuan.desktop\`
- macOS: `~/Library/Application Support/com.peiyuan.desktop/`
- Linux: `~/.local/share/com.peiyuan.desktop/`

---

## 三、目录结构

```
peiyuan/
├── src/                              # 前端 React + TS
│   ├── components/
│   │   ├── ui/                       # shadcn/ui 基础组件（button, card, input, tabs, ...）+ ConfirmDialog
│   │   ├── settings/                 # ModelConfigForm / PromptEditor / MicTest / KeyboardTest / AudioSettingsPanel / GenerationPreview / LlmParamsPanel / TimingPanel / DifficultyPanel
│   │   ├── test/                     # GlobalHeader / PhaseCountdown / QuestionDisplay / FillBlankTable / RecorderPanel
│   │   ├── ErrorBoundary.tsx
│   │   └── Toast.tsx
│   ├── pages/                         # MainMenu / Settings / Test / Result
│   ├── store/                         # Zustand: settings / test / testFlow / result / confirm
│   ├── hooks/                         # useTimerEvent / useAudioPlayer / useRecorder / useGenerationProgress / useTestFlowEvents
│   ├── types/                         # TypeScript 类型（与 Rust models/ 对齐）
│   ├── lib/                           # tauri invoke 封装 + utils
│   ├── App.tsx                        # 路由根
│   └── main.tsx                       # React 入口
│
├── src-tauri/                         # 后端 Rust
│   ├── src/
│   │   ├── commands/                  # Tauri commands
│   │   │   ├── config.rs              # get_config / save_config / reset_config / restore_default_prompt / restore_default_timing / open_config_dir
│   │   │   ├── llm.rs                 # test_llm_connection / generate_with_llm
│   │   │   ├── tts.rs                 # test_tts_connection
│   │   │   ├── stt.rs                 # test_stt_connection / transcribe_audio
│   │   │   ├── audio.rs               # play_audio_file / play_audio_background（切歌由 skip_to_next 通过 AudioPlaybackState.active_stop_flag 实现）
│   │   │   ├── recorder.rs            # start_recording / stop_recording / get_audio_level
│   │   │   ├── device.rs              # list_input/output_devices / test_input/output_device
│   │   │   ├── test_session.rs        # generate_test_session / get_test_session / clear_test_session
│   │   │   ├── test_flow.rs           # start_test_flow / submit_answer / get_flow_state / get_answer_set / reset_test_flow / skip_to_next / notify_recording_completed
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
│   │   │   ├── recorder.rs            # cpal 录音（worker 线程 + 共享 Arc；支持 downmix_to_mono + linear_resample，见 §4.4.10）
│   │   │   ├── scoring.rs             # 1-14 本地 / 15-18 LLM JSON 每空评分（见 §4.4.12） / 19 STT+LLM
│   │   │   ├── test_flow.rs           # 状态机编排
│   │   │   ├── test_session.rs        # 测试会话生成
│   │   │   ├── timer.rs               # 精确计时 + 事件推送
│   │   │   └── tts_service.rs
│   │   ├── models/                    # 数据结构
│   │   │   ├── config.rs              # AppConfig / ModelConfig / LlmParams / PromptConfig / AudioConfig / TimingConfig（见 §4.4.8）
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
| `AudioPlaybackState.active_stop_flag` | `Mutex<Option<Arc<AtomicBool>>>` | 整个 App | `skip_to_next` 中断当前 rodio 播放（§4.4.9） |
| `FlowStateInner.skip_requested` | `Arc<AtomicBool>` | 单次 run_* | 「下一题」信号位，interruptible_sleep 轮询 |
| `FlowStateInner.recording_completed` | `Arc<AtomicBool>` | 单次 run_* | 「提前结束录音」信号位，interruptible_sleep 轮询 |
| `FlowStateInner.group_membership` | `HashMap<u32, u32>` | 单次 run_* | 内部：题号 → 组号（用于 5-12 题共享 ANSWERING 时段） |

### 4.3 后端事件命名

| 事件名 | Payload | 触发时机 |
|--------|---------|----------|
| `test-generation-progress` | `{stage, message, progress}` | 题目预生成各阶段 |
| `test-timer-tick` | `{phase, elapsedMs, durationMs, remainingMs, progress}` | 每 100ms |
| `test-phase-finished` | `phase` | 阶段倒计时结束 |
| `test-flow-state` | `FlowState`（含 `play_count: Option<u32>`，15-19 题 PLAYING 时取 1/2/3） | 阶段切换 |
| `test-flow-finished` | `{ok, completed?, error?}` | 1-19 题全部完成 |
| `test-audio-play` | `{path, loop}` | 通知前端播放（实际播放由后端 rodio 完成）。**两次 PLAYING 之间的静音间隔会以 `{ path: null }` 发射一次**，前端需据此重置进度 |
| `test-record-start` | `{durationMs}` | 进入 19 题录音阶段 |
| `test-record-stop` | - | 录音结束 |
| `test-score-progress` | `{stage, message}`（`stage ∈ {"mcq","blanks","retell","done"}`） | **已定义但当前未发射**（详见 §六） |

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
日志/预览中涉及字符串截断时，必须按字符而非字节切割（避免多字节字符中间 panic）。
- 前端使用 `src/lib/utils.ts::truncate(s, max)`。
- 后端日志直接打印完整字符串，不再做截断（日志已落盘，原 `truncate_chars` 工具已删除）。


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
1-4 题：INTRO(intro_ms) → PREPARE(short_dialogue_prepare_ms) → PLAYING(1x) → ANSWERING(short_dialogue_answer_ms) → 下一题
5-12 题（4 段共享一个 ANSWERING 时段）：PREPARE(group_prepare_ms) → PLAYING #1 → 间隔(group_pause_ms) → PLAYING #2 → ANSWERING(共享 group_answer_ms)
13-14 题（独白）：PREPARE(group_prepare_ms) → PLAYING(1x) → ANSWERING(group_answer_ms)
15-19 题：
  PREPARE(retell_prepare_ms) → PLAYING #1 → 间隔(retell_pause_ms) → PLAYING #2 →
  FILL_BLANK(retell_fill_blank_ms) → PLAYING #3 → RECALL_PREP(retell_recall_prep_ms) →
  RECORDING(固定 90s) → DONE
```

关键约定：
- **所有时长（除 RECORDING 固定 90s 外）均由 `TimingConfig` 控制**，详见 §4.4.8
- 15-19 题 PLAYING 阶段在 `test-flow-state` 事件中带 `play_count: Some(1|2|3)` 字段；前端在 `play_count = Some(3)` 时必须禁用填空白编辑
- `skip_to_next` 可用阶段：PREPARE / PLAYING #1 / pause / PLAYING #2 / FILL_BLANK（直接跳到 PLAYING #3）
- `skip_to_next` **不可用**阶段：PLAYING #3 / RECALL_PREP / RECORDING（保证最后两段答题时间不被压缩）
- `notify_recording_completed` 可在 RECORDING 任意时刻触发，提前结束录音
- 所有跨阶段睡眠均走 `interruptible_sleep()`，每 100ms 轮询对应信号位（详见 §4.4.9 / §4.4.11）

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

查看落盘日志文件（按天滚动，详见 §4.4.13）：
```bash
# Linux
tail -F ~/.local/share/com.peiyuan.desktop/logs/peiyuan.log.*
# macOS
tail -F "$HOME/Library/Application Support/com.peiyuan.desktop/logs/peiyuan.log.*"
# Windows (PowerShell)
Get-Content "$env:APPDATA\com.peiyuan.desktop\logs\peiyuan.log.*" -Wait
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

## 六、已知问题 / 死代码

本次文档审计发现但**未在代码层面处理**的项，留作未来清理 TODO：

| 项目 | 文件 | 说明 |
|------|------|------|
| `test-score-progress` 事件未发射 | `src-tauri/src/services/scoring.rs` | `ScoreProgress` 结构与 `SCORE_PROGRESS_EVENT` 常量已定义，但 `score_full_test` 全程未调用 `emit()`。前端无 listener。死代码。 |

---

## 七、详细需求

所有功能需求、UI 规范、评分规则、非功能性需求等请参考 [Spec.md](./Spec.md)。

**核心要点速查**：
- 19 道题分两段：1-14 选择 / 15-19 转述
- 全部使用本地/自建 OpenAI 兼容模型服务
- 5 个 Prompt 模板可在 UI 编辑 + 恢复默认（`PromptEditor`）
- 阶段时长可在 UI 编辑 + 恢复默认（`TimingPanel`，对应 `TimingConfig`，详见 §4.4.8）
- 后端驱动计时与音频播放
- 15-18 题评分：每空 LLM JSON 评分（0 或 1.5 分），详见 §4.4.12
- 总分 30（14 + 6 + 10），自动评分
- 跨平台 Windows / macOS / Linux