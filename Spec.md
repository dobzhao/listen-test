# 英语听力练习程序 - 详细需求规格（Spec）

> 版本：v1.0
> 最后更新：2026-08-14
> 适用代码版本：v0.1.0（已交付全部 6 个 Phase）

## 一、项目概述

### 1.1 目标

本项目开发一个**跨平台桌面英语听力练习程序**，用于模拟"英语听说考试"完整流程。系统通过本地/自建的 OpenAI 兼容 LLM 服务实时生成题目内容，TTS 服务合成语音，STT 服务转写用户口头作答，由 LLM 自动评分。

### 1.2 核心场景

用户在桌面应用中依次完成 **19 道题目**，其中：
- **第 1-14 题**：听后选择（听对话/独白，从 A/B/C 三个选项中选择正确答案）
- **第 15-19 题**：听后转述（听长材料 → 填写 4 个挖空单词 → 口头转述录音 → LLM 评分）

### 1.3 目标用户

- 需要练习英语听说的学生


### 1.4 平台支持

| 平台 | 最低版本 | 状态 |
|------|----------|------|
| Windows | Windows 10 | ✅ |
| macOS | 11.0 Big Sur | ✅ |
| Linux | Ubuntu 22.04 LTS | ✅ |

---

## 二、技术栈

### 2.1 前端

- **Tauri 2.0** 桌面运行时
- **React 18** + **TypeScript**（strict 模式）
- **Vite 5** 构建工具
- **Tailwind CSS 3.4** + **shadcn/ui** 组件
- **Zustand 5** 状态管理
- **React Router v6** 路由
- **@tauri-apps/api** 与后端通信

### 2.2 后端

- **Rust 2021 edition**（≥ 1.75）
- **tokio** 异步运行时（full features）
- **reqwest** HTTP 客户端（含流式 + multipart）
- **eventsource-stream** SSE 解析
- **cpal** 跨平台音频采集
- **rodio** 跨平台音频播放
- **hound** WAV 编码/解码
- **serde** + **serde_json** 序列化
- **tracing** 日志
- **uuid** 会话标识
- **thiserror** + **anyhow** 错误处理

### 2.3 数据存储

- 配置文件：`{app_data_dir}/config.json`
- 测试缓存：`{app_data_dir}/cache/{session_uuid}/`
- 设备测试录音：`{app_data_dir}/device-tests/`

---

## 三、19 题详细流程

### 3.1 第 1-4 题：短对话听后选择（每题独立流程）

**题目结构**：4 段短对话，每段配 1 道选择题。

**生成 Prompt 模板**：默认见 `src-tauri/prompts/q1_4.txt`；占位符详见第五章 5.2 占位符总表。

**每题流程**：

| 阶段 | 默认时长 | UI 显示 |
|------|---------|---------|
| INTRO | 10s | 仅第 1 题前显示一次 |
| PREPARE | 5s | 显示题干与 3 个选项（不可点） |
| PLAYING | ~8s | 后端播放对话音频 1 次（用户可提前选答案） |
| ANSWERING | 10s | 选项可点击，倒计时进度条 |

### 3.2 第 5-12 题：长对话听后选择（4 段材料，每段配 2 题）

**题目结构**：4 段长对话，每段配 2 道选择题（5+6 / 7+8 / 9+10 / 11+12）。

**生成 Prompt 模板**：默认见 `src-tauri/prompts/q5_14.txt`，无占位符。

**每段材料流程**（两题共用）：

| 阶段 | 默认时长 | UI 显示 |
|------|---------|---------|
| PREPARE | 10s | 同时显示两道题的题干与 3 个选项 |
| PLAYING #1 | ~10s | 播放第 1 次 |
| 静音间隔 | 2s | 停止播放 |
| PLAYING #2 | ~10s | 播放第 2 次 |
| ANSWERING | 10s | 两题共享作答时间（10s） |

### 3.3 第 13-14 题：独白听后选择

**题目结构**：1 段独白（150-200 词）配 2 道选择题。

**生成流程**：同 5-12 题（独白单次播放）。

### 3.4 第 15-19 题：听后转述

**题目结构**：
- 1 段较长听力材料（200-220 词）
- 总-分结构 3 行表格（Overview + Details 两列）
- Details 中挖去 4 个单词（对应第 15-18 题）
- 第 19 题：口头转述

**生成 Prompt 模板**：默认见 `src-tauri/prompts/q15_18.txt`，无占位符。

**完整流程**：

| 阶段 | 默认时长 | UI 显示 |
|------|---------|---------|
| PREPARE | 30s | 显示挖空表格（不可填） |
| PLAYING #1 | ~12s | 播放第 1 次 （此时开始挖空可填） |
| 静音间隔 | 3s | 停止播放 |
| PLAYING #2 | ~12s | 播放第 2 次 |
| FILL_BLANK | 90s | 额外给用户的填空时间（挖空可填） |
| PLAYING #3 | ~12s | 第 3 次播放（不可继续填写） |
| RECALL_PREP | 120s | 显示已填表格，默读准备 |
| RECORDING | 90s（**固定） | 自动开始录音，实时音量条 |

**第 19 题录音要求**：
- 自动开始，**固定 90 秒**（受 STT/LLM 判分稳定性约束，不可在设置中调整）
- 可提前结束但不可延长
- 麦克风权限需正确配置（macOS/Linux）
- 录音保存到 `{app_data_dir}/cache/{session_id}/recording.wav`

### 3.5 流程时长可配置说明

除 **第 19 题录音 90 秒**外，上述 10 个时长（INTRO / PREPARE / 静音间隔 / ANSWERING / FILL_BLANK / RECALL_PREP）均可在 **设置 → 流程时长** Tab 中调整，单位为秒，内部以毫秒存储：

- 适用场景：自测/教学场景下缩短等待，或为听障考生延长准备时间
- 默认值与本节表格一致
- 设置修改后立即生效，下次测试流程启动时读取
- 提供「恢复默认」按钮一键还原
- 修改后必须点击「保存时长」按钮才会写入 `config.json`

---

## 四、模型服务集成

### 4.1 三组独立可配置服务

| 服务 | 默认端点 | 默认模型 |
|------|----------|----------|
| LLM | `http://127.0.0.1:8000/v1/chat/completions` | 用户配置 |
| TTS | `http://127.0.0.1:8000/v1/audio/speech` | Kokoro-82M-bf16 |
| STT | `http://127.0.0.1:8000/v1/audio/transcriptions` | whisper-large-v3-turbo-asr-fp16 |

每组配置字段：
- **protocol**：`http` 或 `https`（默认 http）
- **host**：主机地址
- **port**：端口
- **api_path**：API 路径
- **model**：模型名称
- **api_key**：Authorization Bearer Token

### 4.2 LLM 调用细节

```json
POST /v1/chat/completions
{
  "model": "{model}",
  "messages": [{"role": "user", "content": "..."}],
  "stream": true,
  "stream_options": { "include_usage": true },
  "temperature": 1.0,
  "max_tokens": 81920,
  "top_p": 0.95,
  "top_k": 64,
  "chat_template_kwargs": { "enable_thinking": false }
}
```

**响应处理**：解析 SSE 流，拼接 `choices[0].delta.content` 得到完整文本。调用方再解析 JSON。

### 4.3 TTS 调用细节

```json
POST /v1/audio/speech
{
  "model": "{model}",
  "input": "对话文本",
  "voice": "af_heart" | "am_michael",
  "speed": 1.0,
  "lang_code": "a"
}
```

**Voice 约定**（可由用户在 Prompt 中调整）：
- 说话人 M → `am_michael`
- 说话人 W → `af_heart`

### 4.4 STT 调用细节

```bash
POST /v1/audio/transcriptions
Form-data: file=audio.wav, model=..., language=en
```

---

## 五、Prompt 模板

19 道题的题目生成与评分全部由 LLM 完成，共 5 个 Prompt 模板，均可在设置界面编辑并支持"恢复默认"。

### 5.1 模板清单与位置

5 个模板以纯文本形式存放于 `src-tauri/prompts/`，编译期由 `include_str!` 嵌入二进制，作为 `PromptConfig` 的默认值（`models::config::default_prompts()`）：

| 模板 Key | 文件 | 行数 | 适用阶段 |
|----------|------|------|----------|
| `q1_4` | `q1_4.txt` | 40 | 1-4 题出题（短对话）|
| `q5_14` | `q5_14.txt` | 86 | 5-14 题出题（长对话 + 独白）|
| `q15_18` | `q15_18.txt` | 48 | 15-19 题出题（长文本 + 表格 + 挖空）|
| `q15_18_scoring` | `q15_18_scoring.txt` | 35 | 15-18 题填空判分 |
| `q19_scoring` | `q19_scoring.txt` | 21 | 19 题口头转述评分 |

### 5.2 占位符总表

10 个占位符全部采用 `{{KEY}}` 语法。下表按"模板 → KEY"列出每条占位符的用途、值来源与是否可被用户在设置界面编辑。

| 模板 | 占位符 KEY | 用途 | 值来源 | 用户可编辑 |
|------|------------|------|--------|------------|
| `q1_4` | `{{DIALOGUE_SCENARIOS}}` | 4 段短对话的场景清单 | 按当前难度档从内置对话场景库随机抽 4 个不重复 | 否 |
| `q1_4` | `{{DIFFICULTY_DEMAND_1_4}}` | 当前档位 1-4 题难度文字要求 | `DifficultyConfig.{level}.demand_1_4` | 是（难度 Tab）|
| `q5_14` | `{{DIALOGUE_SCENARIOS}}` | 4 段长对话的场景清单 | 同 `q1_4` | 否 |
| `q5_14` | `{{MONOLOGUE_SCENARIO}}` | 1 段独白的话题与展开方向 | 按当前难度档从独白场景库随机抽 1 个 | 否 |
| `q5_14` | `{{DIFFICULTY_DEMAND_5_14}}` | 当前档位 5-14 题难度文字要求 | `DifficultyConfig.{level}.demand_5_14` | 是（难度 Tab）|
| `q15_18` | `{{MONOLOGUE_SCENARIO}}` | 较长听力材料的话题与 3 个展开方向 | 同 `q5_14` | 否 |
| `q15_18` | `{{DIFFICULTY_DEMAND_15_18}}` | 当前档位 15-18 题难度文字要求 | `DifficultyConfig.{level}.demand_15_18` | 是（难度 Tab）|
| `q15_18_scoring` | `{{ORIGINAL_TEXT}}` | 15-19 题听力原文 + 4 空标准答案 | 运行时拼接：原文 + 标准答案 | 否 |
| `q15_18_scoring` | `{{ANSWERS}}` | 用户填写的 4 空答案 | `{"15":..,"16":..,"17":..,"18":..}` pretty JSON | 否 |
| `q19_scoring` | `{{ORIGINAL_TEXT}}` | 15-19 题听力原文 | `session.retell.passage` | 否 |
| `q19_scoring` | `{{STT_RESULT}}` | 第 19 题录音经 STT 转写后的文本 | STT 服务返回的转写文本 | 否 |


### 5.3 与场景/难度的协同

- 难度档位（`junior_high` / `senior_high` / `undergraduate`）由用户在"难度"Tab 顶部切换
- 场景库（`src-tauri/resources/dialogue_scenarios.json`、`monologue_scenarios.json`）按 3 档分组；调用方传 `level` 即可拿到对应子池的随机条目
- `inject_difficulty_vars`（`services/question_generator.rs`）按当前 `level` 取出对应的 3 段 `demand_*` 文字，一次性注入 3 个 KEY，未知 level 兜底为 `junior_high`
- 由于引擎忽略未引用 KEY，3 个 `DIFFICULTY_DEMAND_*` 同时注入是安全的


### 5.4 "恢复默认"按钮

设置界面每个 Prompt 模板都有"恢复默认"按钮，点击后将对应模板重置为 `src-tauri/prompts/<key>.txt` 编译期默认值，**仅影响该模板，不影响其他模板或难度/场景配置**。

---

## 六、评分规则

### 6.1 第 1-14 题（MCQ）

- 本地比对 `user_answer` 与 `correct_answer`
- 每题 1 分，总分 14

### 6.2 第 15-18 题（填空）

由 LLM 按 `q15_18_scoring` Prompt 评分：

- 大小写不敏感
- 单复数必须完全正确
- 允许英式/美式拼写差异（colour/color 等）
- 允许原文中出现过的同义词
- **每空 1.5 分，总分 6 分**

### 6.3 第 19 题（口头转述）

1. 先调用 STT 将录音转写为文本
2. 由 LLM 按 `q19_scoring` Prompt 评分：
   - 内容完整度（4 分）
   - 关键信息准确性（3 分）
   - 语言表达与逻辑连贯性（3 分）
3. **总分 10 分**

### 6.4 总分

`总分 = 1-14 正确数 + 15-18 得分 + 19 得分`，满分 30。

---

## 七、UI 需求

### 7.1 设置界面

6 个 Tab：
1. **LLM**：host / port / api_path / model / api_key / protocol + 调用参数
2. **TTS**：同上
3. **STT**：同上
4. **提示词**：5 个 Prompt 模板可编辑 + 恢复默认
5. **音频**：播放音量、麦克风增益、TTS 拼接静音时长
6. **流程时长**：答题过程每个阶段等待时长 + 恢复默认
7. **设备测试**：可选输入/输出设备 + 录音测试 + 测试音播放

每个模型配置都有"测试连接"按钮（最小请求验证）。

### 7.2 主菜单

- 显示 LLM / TTS / STT 三组配置状态（已就绪 / 未完成）
- 「开始测试」按钮（配置完成才可点）
- 「设置」入口

### 7.3 测试页面

- 顶部全局进度条（第 N/19 题）
- 当前阶段说明 + 倒计时进度条
- 题目展示（题干 + 3 个选项）
- 挖空表格（15-18 题，含可填 input）
- 录音面板（19 题，倒计时 + 实时音量条）
- 「放弃并返回主菜单」按钮（带确认）

### 7.4 结算页

- 总分卡片
- 1-14 题：14 个可点击小卡片，展示对错
- 15-18 题：表格展示用户答案 vs 标准答案
- 19 题：分数 + LLM 评语 + STT 转写文本 + 听力原文
- 「重新测试」/「返回主菜单」按钮

---

## 八、非功能性需求

### 8.1 计时精度

- 后端 Rust `tokio::time::Instant` 计算真实剩余时间
- 每 100ms 通过 Tauri Event 推送 `test-timer-tick`
- 阶段结束推送 `test-phase-finished`

### 8.2 容错与重试

- LLM/TTS/STT 调用失败自动重试（指数退避，最多 3 次）
- 关键路径失败提示用户重试/跳过
- React ErrorBoundary 兜底前端崩溃
- 题目生成失败可一键清除并重新生成

### 8.3 预加载机制

- 测试开始前预生成全部 19 题内容（含音频）
- 单次测试内容缓存到 `cache/{session_id}/`

### 8.4 跨平台兼容

- 音频播放统一用后端 rodio（无 webview autoplay 限制）
- assetProtocol 已启用 + `protocol-asset` feature
- cpal 在 macOS / Linux / Windows 自动适配

### 8.5 异常处理

- 用户关闭窗口释放音频流
- 网络中断有清晰错误提示
- 配置缺失有占位默认

---


## 九、配置兼容性

`config.json` 中新增字段（如 `protocol`）通过 `#[serde(default = ...)]` 自动回退到默认值，向后兼容旧配置文件，无需用户手动迁移。

---

## 十、已知约束

1. **单次测试题目内容不可恢复**：缓存清理后无法重新查看
2. **TTS 输出采样率依赖服务**：当前代码假定所有 TTS 输出采样率一致，拼接 wav 时统一
3. **录音固定采样率 16kHz**：简化 STT 处理，重采样在保存时完成
4. **同时只能运行一个测试会话**：未实现多会话隔离
5. **19 题录音时长固定 90 秒**：受 STT/LLM 判分稳定性约束，不可在 UI 中调整；其余 10 个流程时长均可在「设置 → 流程时长」中调整