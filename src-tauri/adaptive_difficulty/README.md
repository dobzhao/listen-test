# adaptive_difficulty

基于能力值的自适应难度算法库，带缓冲区滞后判定。为 peiyuan 英语听力练习程序设计。

## 简介

每次完整 19 题测试结束后，根据三段得分率 `a`（1–14 题）、`b`（15–18 题）、`c`（19 题）更新持久化状态：

- `ability_score: f64` ∈ [0, 600]
- `trend: f64` ∈ [-1, 1]（EMA 平滑）
- `current_level: Level`（junior_high / senior_high / undergraduate，必须独立持久化）

并按滞后规则决定下次测试使用的难度档位。

## 公共 API（速查）

```rust
use adaptive_difficulty::*;

// 构造初始状态
let mut state = AdaptiveState::junior_high();  // ability=100, trend=0, level=junior_high

// 应用一次完整测试结果
let trace = update(&mut state, &Params::default(), 0.85, 0.67, 0.90)?;
// 或：update_with_defaults(&mut state, 0.85, 0.67, 0.90)?

// 手动重置（用户在设置界面切换难度时调用）
reset_to(&mut state, Level::SeniorHigh);

// 加载外部 JSON 时校验
let parsed: AdaptiveState = serde_json::from_str(&text)?;
let state = validate_loaded(parsed);
```

## 集成到 peiyuan

作为独立 crate 通过 `path` 依赖集成到 `peiyuan/src-tauri/`：

```toml
# peiyuan/src-tauri/Cargo.toml
adaptive_difficulty = { path = "../../adaptive_difficulty" }
```

详细接入步骤、持久化格式、错误处理与算法细节见 [INTEGRATION.md](./INTEGRATION.md)。

`Level` 枚举的 `serde` 形式（snake_case 字符串）与 peiyuan 现有 `DifficultyConfig.level: String` 字面量完全一致，集成阶段无序列化迁移成本。

## 许可证

MIT OR Apache-2.0