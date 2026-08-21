# 集成指南：把 `adaptive_difficulty` 接入主程序

> 面向主程序（peiyuan 英语听力练习 app）开发者的接入文档。读完本文档应该能写出调用代码，不必回头翻本仓库源码。

---

## 1. 概述

`adaptive_difficulty` 是 11 步自适应更新算法的纯 Rust 实现，源码在 `src/algorithm.rs`。主程序只需在每次 19 题测试结束后调一次 `update`，再把返回的 `AdaptiveState` 落盘。

库的运行时依赖只有 `serde` / `serde_json` / `thiserror`，**不依赖**任何 GUI / 文件 IO，序列化已通过派生实现内置。

## 2. Cargo 依赖

```toml
[dependencies]
adaptive_difficulty = { path = "../adaptive_difficulty" }   # 本地路径
# 将来发到 crates.io 之后改为：
# adaptive_difficulty = "0.1"
```

## 3. 需要持久化的字段

主程序只需要持久化**一个结构**——`AdaptiveState`，它在 JSON 里长这样：

```json
{
  "ability_score": 150.0,
  "trend": 0.1,
  "current_level": "senior_high",
  "update_count": 5
}
```

字段说明（来自 `src/state.rs`）：

| 字段 | 类型 | 说明 |
|---|---|---|
| `ability_score` | `f64` | 能力分，受 `params.ability_min/max` 钳制（默认 `[0.0, 600.0]`） |
| `trend` | `f64` | 表现趋势 EMA，归一化到约 `[-1, 1]` |
| `current_level` | `"junior_high"` \| `"senior_high"` \| `"undergraduate"` | **必须和 `ability_score` 一起存**，不能从能力分推导 |
| `update_count` | `u64` | 累计更新次数；带 `#[serde(default)]`，旧存档缺该字段时按 0 处理 |

**几条容易踩的坑**：

- **`current_level` 是独立状态**，不是 `ability_score` 的派生量。缓冲带（`params.buffer`）的判定需要"上一次等级"和"新能力分"一起算（参 `src/hysteresis.rs` 第 9–13 行注释）。如果只存能力分，每次启动重新算等级，会让用户在缓冲带边界来回跳级。
- **`Params` 不要进用户存档**。`Params` 是算法参数，由主程序配置而不是用户数据。
- **`Level` 序列化形式是 snake_case 字符串**（`"junior_high"` 等），与主程序现有 `DifficultyConfig.level: String` 字节级一致，无需迁移。
- 推荐直接 `serde_json::to_string(&state)` / `from_str`；字段名要照上面 JSON 示例写，错了反序列化会报错。

## 4. 如何调用

```rust
use adaptive_difficulty::{
    AdaptiveState, Level, Params, UpdateTrace,
    update, update_with_defaults, reset_to, validate_loaded,
};
```

`update` 的签名：

```rust
pub fn update(
    state: &mut AdaptiveState,
    params: &Params,
    a: f64, b: f64, c: f64,
) -> Result<UpdateTrace, AdaptiveError>
```

### 4.1 首次使用（全新用户）

```rust
let mut state = AdaptiveState::new(Level::JuniorHigh);
// 三个等级对应的初始能力分：100 / 300 / 500
// 也可以用便捷构造：
// let mut state = AdaptiveState::junior_high();
// let mut state = AdaptiveState::senior_high();
// let mut state = AdaptiveState::undergraduate();
```

### 4.2 一次测试结束后的标准流程

```rust
let a: f64 = correct_1_to_14 as f64 / 14.0;   // 1–14 题得分率，必须 ∈ [0, 1]
let b: f64 = correct_15_to_18 as f64 / 4.0;  // 15–18 题得分率
let c: f64 = q19_score;                      // Q19 复述题得分率（已是 0–1）

let params = Params::default();  // 用规范的默认值；如有调参需求自己 clone 一份

match update(&mut state, &params, a, b, c) {
    Ok(trace) => {
        // trace.level_after 就是下一份题目应该用的难度
        log::info!(
            "ability {:.2} → {:.2}, level {} → {}",
            trace.ability_before, trace.ability_after,
            trace.level_before.as_str(), trace.level_after.as_str(),
        );
    }
    Err(e) => {
        // 任何 Err 都会保持 state 完全不变 — 不会半更新
        log::error!("adaptive update failed: {}", e);
    }
}
// 立即落盘 state
save_state(&state);
```

### 4.3 不想管参数时的简写

```rust
update_with_defaults(&mut state, a, b, c)
// 等价于 update(state, &Params::default(), a, b, c)
```

### 4.4 主动重置到某个等级（"换难度包"）

```rust
reset_to(&mut state, Level::SeniorHigh);
// state.ability_score = 300.0
// state.trend        = 0.0
// state.current_level = SeniorHigh
// state.update_count 不变（这是计数器，不是状态）
```

### 4.5 加载存档（防御性校验）

```rust
let loaded: AdaptiveState = serde_json::from_str(&persisted_json)?;
let state = validate_loaded(loaded);  // 钳到合法范围，防手改存档
```

`validate_loaded` 只是把 `ability_score` / `trend` 钳到合法范围。**字段迁移**（旧版本字段缺失）由 `#[serde(default)]` 自动处理。

### 4.6 调参（可选）

```rust
let mut params = Params::default();
params.score_floor = 0.7;        // 例：放宽下限
// 其它字段含义见 src/params.rs 的文档注释
```

`Params` 是 `Copy + Clone`，单实例、`Rc<Params>`、`Arc<Params>` 都行，主程序通常启动加载一次全局只读。

## 5. 模块返回什么

### 5.1 成功：`UpdateTrace`

`update` 在 `Ok` 分支返回 `UpdateTrace`，三组字段：

**中间量**（每次 update 算出的步骤值，便于调试）：

| 字段 | 含义 |
|---|---|
| `combined` | 步骤 1：原始加权得分率（`w_A*a + w_B*b + w_C*c`） |
| `combined_clamped` | 步骤 2：`combined.max(score_floor)`，实际下游用这个 |
| `x` | 步骤 3：`combined_clamped - 0.8`（以 0.8 为中性线） |
| `x_norm` | 步骤 4：`x / (0.8 - score_floor)`，归一化到约 `[-1, 1]` |
| `magnitude` | 步骤 6：`min(max_magnitude, base + k * |trend|)` |
| `delta` | 步骤 7：`magnitude * x_norm`（受 step 8 钳制） |

**状态快照**：`ability_before` / `ability_after`、`trend_before` / `trend_after`、`level_before` / `level_after`。

**参数快照**：`params_used: Params`，当时用的参数副本。

**业务侧只需要 `trace.level_after`**（和可选的 `trace.ability_after`）来决定下一份题目。`trace` 整体不进用户主存档——它是衍生数据。

### 5.2 失败：`AdaptiveError`

```rust
pub enum AdaptiveError {
    InvalidScoreRate(String),  // a/b/c 越界 [0, 1] 或非有限
    NonFinite(String),         // Params 算出 NaN，或步骤 4 分母 (0.8 - score_floor) 为零
}
```

**关键不变量**：`Err` 返回时 `state` **保持完全不变**（见 `src/algorithm.rs` 第 38–43 行注释）——可以放心重试。

## 6. 输入约束速查

| 输入 | 约束 | 越界后果 |
|---|---|---|
| `a`, `b`, `c` | `[0.0, 1.0]` 且有限 | `Err(InvalidScoreRate)` |
| `Params::score_floor` | 不能等于 0.8 | `Err(NonFinite)` |
| `Params` 任意字段 | 有限数 | 塞了 NaN 时 `Err(NonFinite)` |
| `Params::alpha` | `(0, 1)` | 由调用方自己保证，库不校验语义 |

## 7. 端到端集成步骤

主程序接入按这份清单走：

1. **加 Cargo 依赖**（见第 2 节）。
2. **选定 `Params` 存放位置**：推荐启动加载一次，全局只读。
3. **启动时构造 state**：
   - 用户无存档 → `AdaptiveState::junior_high()`（或对应业务选择的等级）
   - 用户有存档 → `serde_json::from_str` + `validate_loaded`
4. **每次 19 题阅卷完成后**算 `a` / `b` / `c`，调 `update`，**立刻**把 `state` 落盘（防止崩溃丢档）。
5. **用 `trace.level_after` 决定下一份题目难度**。
6. **"重置到高一档 / 低一档"按钮**调用 `reset_to`。

## 8. 算法详解（11 步）

主程序调用 `update` 时，库内部会按以下 11 步顺序执行。理解这些步骤有助于：
- 调参时知道每个字段作用在第几步
- trace 里看不懂的字段可以回这里查
- 调试时定位是哪一步出问题

记号约定：每次 `update(state, params, a, b, c)` 调用前先记录快照 `ability_before = state.ability_score`、`trend_before = state.trend`、`level_before = state.current_level`。

### 步骤 1：加权综合得分率

```
combined = weight_a * a + weight_b * b + weight_c * c
```

把三部分得分率按权重线性组合。默认值 `weight_a=0.5, weight_b=0.2, weight_c=0.3`，意味着选择题（1–14 题）权重最大，复述题（Q19）次之，填空题（15–18 题）最低。

### 步骤 2：得分率下限截断

```
combined_clamped = max(combined, score_floor)   // 默认 score_floor = 0.6
```

无论用户考得多差，综合得分率都不会低于 `score_floor`。这一步是局部变量，**不会改写 a / b / c**。

### 步骤 3：相对中性线的偏差

```
x = combined_clamped - 0.8
```

0.8 是规范的"中性线"：综合得分率 = 0.8 时认为表现刚好，不需要调整能力分。x ∈ `[-0.8, 0.2]`（当 score_floor = 0.6）。

### 步骤 4：归一化

```
denom = 0.8 - score_floor        // 默认 = 0.2
x_norm = x / denom               // 归一化到约 [-1, 1]
```

**分母为零时返回 `Err(NonFinite)`**——这是为什么 `score_floor` 不能等于 0.8。

### 步骤 5：EMA 更新趋势

```
trend_after_unclamped = alpha * x_norm + (1 - alpha) * state.trend   // 默认 alpha = 0.4
trend_after = clamp(trend_after_unclamped, trend_min, trend_max)     // 默认 [-1, 1]
```

EMA（指数移动平均）：`alpha` 越大越看重最近一次表现，越小越平滑。钳制是为了保证后续 `magnitude` 的计算不会爆。

### 步骤 6：调整幅度上限

```
magnitude_unclamped = base_magnitude + k * |trend_after|   // 默认 3 + 10 * |trend|
magnitude = min(magnitude_unclamped, max_magnitude)        // 默认上限 8
```

`base_magnitude` 是最小调整幅度，`k * |trend|` 是趋势带来的额外幅度，`max_magnitude` 封顶。

### 步骤 7：带符号的调整量

```
delta = magnitude * x_norm
```

x_norm > 0（表现好）→ delta > 0（涨能力分）；x_norm < 0 → delta < 0（降）。`x_norm ∈ [-1, 1]` 且 `magnitude ≤ max_magnitude`，所以 `|delta| ≤ max_magnitude`。

### 步骤 8：钳到合法范围

```
ability_after = clamp(state.ability_score + delta, ability_min, ability_max)   // 默认 [0, 600]
```

到这一步才更新能力分。

### 步骤 9：缓冲带判定新等级

调用 `apply_hysteresis(params, ability_after, state.current_level)`：

```
upper_senior   = b1 + buffer    // 默认 200 + 20 = 220
lower_senior   = b1 - buffer    // 默认 180
upper_undergrad = b2 + buffer    // 默认 400 + 20 = 420
lower_undergrad = b2 - buffer    // 默认 380
```

按当前等级分别判定：

| 当前等级 | 升档条件 | 降档条件 | 否则 |
|---|---|---|---|
| `JuniorHigh` | `ability > 220` → `SeniorHigh` | — | 维持 `JuniorHigh` |
| `SeniorHigh` | `ability > 420` → `Undergraduate` | `ability <= 180` → `JuniorHigh` | 维持 `SeniorHigh` |
| `Undergraduate` | — | `ability <= 380` → `SeniorHigh` | 维持 `Undergraduate` |

**关键**：判定用的是**新能力分**和**旧等级**。这就是为什么 `current_level` 必须独立持久化，不能从 `ability_score` 推导。

**不允许跨级跳变**：`JuniorHigh` 永远要先升到 `SeniorHigh`，不能直接跳 `Undergraduate`（算法上不提供这条路径）。

### 步骤 10：原子写入 + 计数器自增

```
state.ability_score = ability_after
state.trend         = trend_after
state.current_level = level_after
state.update_count  = state.update_count + 1     // 饱和加，不会溢出
```

四个字段要么全更新要么全不更新（步骤 0 的输入校验失败时整个调用回滚）。

### 步骤 11：返回 trace

把 13 个中间量和快照打包成 `UpdateTrace` 返回（详见第 5.1 节）。

### 一图流

```
a, b, c ──①加权──> combined ──②下限截断──> combined_clamped ──③减0.8──> x
                                                                        │
                                                                        ▼
                                                          ④除以(0.8-score_floor)
                                                                        │
                                                                        ▼
                                                                     x_norm
                                                                        │
                                  ┌───⑥基础+k*|trend|──> magnitude <───┤
                                  │                                     │
                                  └──> ⑤EMA ──> trend_after             │
                                                          │             │
                                                          ▼             ▼
                                          state.trend 更新        ⑦magnitude*x_norm
                                                                        │
                                                                        ▼
                                                                      delta
                                                                        │
state.ability_score ──────────────────────────────────────────────⑧加+钳──> ability_after
                                                                              │
                                                                              ▼
                                                              ⑨hysteresis(old_level, ability_after)
                                                                              │
                                                                              ▼
                                                                          level_after
                                                                              │
   ┌──────────────────────────────────────────────────────────────────────────┘
   ▼
⑩原子写回 state (ability_score, trend, current_level, update_count+1)
                                                                              │
                                                                              ▼
                                                                  ⑪返回 UpdateTrace
```

## 9. 进一步阅读

- [README.md](./README.md) — 项目总览
- `src/lib.rs` — 模块导出列表
- `src/params.rs` — `Params` 每个字段的详细语义
- `src/hysteresis.rs` — 缓冲带规则的逐字注释（解释了为什么 `current_level` 必须独立存）
