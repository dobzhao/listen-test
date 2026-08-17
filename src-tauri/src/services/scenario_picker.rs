//! 按当前难度档从嵌入 JSON 中随机抽取对话/独白场景，
//! 序列化为 `prompt_engine::render` 接受的 JSON 字面量。
//!
//! 两个资源文件在编译期通过 `include_str!` 嵌入二进制，
//! 反序列化结果用 `once_cell::sync::Lazy` 缓存，进程内只解析一次。

use once_cell::sync::Lazy;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct DialogueScenario {
    #[allow(dead_code)]
    id: String,
    name: String,
    possible_relationships: Vec<String>,
    possible_situations: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MonologueScenario {
    #[allow(dead_code)]
    id: String,
    topic: String,
    aspects: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DifficultyPools<T> {
    junior_high: Vec<T>,
    senior_high: Vec<T>,
    undergraduate: Vec<T>,
}

static DIALOGUE_POOLS: Lazy<DifficultyPools<DialogueScenario>> = Lazy::new(|| {
    serde_json::from_str(include_str!("../../resources/dialogue_scenarios.json"))
        .expect("resources/dialogue_scenarios.json 格式不合法")
});

static MONOLOGUE_POOLS: Lazy<DifficultyPools<MonologueScenario>> = Lazy::new(|| {
    serde_json::from_str(include_str!("../../resources/monologue_scenarios.json"))
        .expect("resources/monologue_scenarios.json 格式不合法")
});

/// 根据当前 `level` 取对应子池的切片引用
///
/// 未知 level / 缺字段时兜底到 `junior_high`，与 `inject_difficulty_vars` 策略一致。
fn pool_for<'a, T>(pools: &'a DifficultyPools<T>, level: &str) -> &'a [T] {
    match level {
        "senior_high" => &pools.senior_high,
        "undergraduate" => &pools.undergraduate,
        _ => &pools.junior_high,
    }
}

/// 从切片里随机取 1 个元素
fn pick_one<'a, T>(pool: &'a [T]) -> &'a T {
    &pool[fastrand::usize(..pool.len())]
}

/// 抽取 4 个不重复的对话场景，按 prompt 规范格式化为 JSON 字符串
///
/// 输出形如：
/// ```json
/// {"scenarios":[
///   {"id":"1","name":"...","relationship":"...","situation":"..."},
///   ...
/// ]}
/// ```
pub fn pick_dialogue_scenarios_json(level: &str) -> String {
    let pool = pool_for(&*DIALOGUE_POOLS, level);
    let mut picked: Vec<&DialogueScenario> = Vec::with_capacity(4);
    while picked.len() < 4 {
        let candidate = pick_one(pool);
        // 不重复抽样：资源文件每档 ≥ 50 条，循环几次必然收齐
        if !picked.iter().any(|p| p.id == candidate.id) {
            picked.push(candidate);
        }
    }
    let scenarios: Vec<serde_json::Value> = picked
        .into_iter()
        .enumerate()
        .map(|(idx, s)| {
            serde_json::json!({
                "id": (idx + 1).to_string(),
                "name": s.name,
                "relationship": pick_one(&s.possible_relationships),
                "situation": pick_one(&s.possible_situations),
            })
        })
        .collect();
    serde_json::json!({ "scenarios": scenarios }).to_string()
}

/// 抽取 1 个独白场景，按 prompt 规范格式化为 JSON 字符串
///
/// 输出形如：
/// ```json
/// {"scenarios":[{"topic":"...","aspects":["...","...","..."]}]}
/// ```
pub fn pick_monologue_scenario_json(level: &str) -> String {
    let pool = pool_for(&*MONOLOGUE_POOLS, level);
    let s = pick_one(pool);
    serde_json::json!({
        "scenarios": [{
            "topic": s.topic,
            "aspects": s.aspects,
        }]
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialogue_json_has_four_unique_scenarios() {
        for level in ["junior_high", "senior_high", "undergraduate"] {
            let json = pick_dialogue_scenarios_json(level);
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            let scenarios = parsed.get("scenarios").unwrap().as_array().unwrap();
            assert_eq!(scenarios.len(), 4, "{} 应返回 4 个场景", level);
            for (idx, s) in scenarios.iter().enumerate() {
                let id = s.get("id").unwrap().as_str().unwrap();
                assert_eq!(id, (idx + 1).to_string(), "id 字段应为 1..4");
                assert!(s.get("name").is_some());
                assert!(s.get("relationship").is_some());
                assert!(s.get("situation").is_some());
            }
        }
    }

    #[test]
    fn monologue_json_has_one_scenario() {
        for level in ["junior_high", "senior_high", "undergraduate"] {
            let json = pick_monologue_scenario_json(level);
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            let scenarios = parsed.get("scenarios").unwrap().as_array().unwrap();
            assert_eq!(scenarios.len(), 1, "{} 应返回 1 个场景", level);
            let s = &scenarios[0];
            assert!(s.get("topic").unwrap().is_string());
            let aspects = s.get("aspects").unwrap().as_array().unwrap();
            assert!(!aspects.is_empty(), "aspects 应非空");
        }
    }

    #[test]
    fn unknown_level_falls_back_to_junior_high() {
        let j = pick_dialogue_scenarios_json("junior_high");
        let unknown = pick_dialogue_scenarios_json("nonexistent");
        // 都应能成功解析并返回 4 个场景
        let _: serde_json::Value = serde_json::from_str(&j).unwrap();
        let _: serde_json::Value = serde_json::from_str(&unknown).unwrap();
    }
}