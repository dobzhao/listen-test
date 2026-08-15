//! 判分服务
//!
//! - 1-14 题（MCQ）：本地比对 user_answer vs correct_answer
//! - 15-18 题（填空）：调用 LLM，按 `q15_18_scoring` Prompt 评分
//! - 19 题（口头转述）：先调 STT 转写录音，再调 LLM 按 `q19_scoring` Prompt 评分

use crate::models::config::{AppConfig, ModelConfig, PromptConfig};
use crate::models::question::TestSession;
use crate::models::result::{BlankResult, McqResult, RetellResult, TestResult};
use crate::services::http_client::build_client;
use crate::services::llm_service::{call_llm, LlmError};
use crate::services::prompt_engine_service::render;
use crate::services::stt_service::{transcribe, SttError};
use crate::utils::json_extract::try_parse;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{info, warn};

/// 安全截断字符串到 N 个字符（不是字节），避免在多字节字符中间切割
fn truncate_chars(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s, // 不足 max_chars 个字符
    }
}

/// 15-18 题 LLM 评分的单项结构：LLM 返回的每空得分明细
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BlankScoreItem {
    /// 接受多种字段名（blank_id 是规范名，其他作为 LLM 漂移时的容错）
    #[serde(
        default,
        alias = "id",
        alias = "question_id",
        alias = "blank",
        alias = "blankId"
    )]
    blank_id: Option<String>,
    #[serde(default)]
    user_answer: Option<String>,
    #[serde(default)]
    correct_answer: Option<String>,
    #[serde(default)]
    is_correct: Option<bool>,
    #[serde(default)]
    score: Option<f32>,
}

/// 15-18 题 LLM 评分的整体响应：明细数组 + 总分 + 评语
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BlankScoreResp {
    #[serde(default)]
    blanks: Vec<BlankScoreItem>,
    #[serde(default)]
    total_score: Option<f32>,
    #[serde(default)]
    comment: Option<String>,
}

/// 在 LLM 返回的明细数组中查找对应空号。
///
/// 轨道 A：按 blank_id 字符串匹配（首选，兼容 alias）
/// 轨道 B：按数组索引匹配（兜底，应对 LLM 漏写 blank_id 但顺序对的情况）
fn find_blank_item<'a>(
    items: &'a [BlankScoreItem],
    keys: &[&'static str; 4],
    i: usize,
) -> Option<&'a BlankScoreItem> {
    let key = keys[i];
    items
        .iter()
        .find(|it| it.blank_id.as_deref() == Some(key))
        .or_else(|| items.get(i))
}

/// 三级 fallback 决定每空最终得分：
///   1) LLM 的 score（吸附到 0/1.5）
///   2) LLM 的 is_correct（映射到 1.5/0）
///   3) 本地字符串匹配（兜底兜底）
fn resolve_blank_outcome(item: Option<&BlankScoreItem>, local_score: f32) -> f32 {
    item.and_then(|it| it.score)
        .map(|s| if s >= 0.75 { 1.5 } else { 0.0 }) // 吸附到 0/1.5
        .or_else(|| {
            item.and_then(|it| it.is_correct)
                .map(|ok| if ok { 1.5 } else { 0.0 })
        })
        .unwrap_or(local_score)
}

#[derive(Debug, Error)]
pub enum ScoringError {
    #[error("LLM 调用失败: {0}")]
    Llm(#[from] LlmError),
    #[error("STT 调用失败: {0}")]
    Stt(#[from] SttError),
    #[error("Prompt 渲染失败: {0}")]
    Prompt(String),
    #[error("评分结果解析失败: {0}")]
    Json(#[from] serde_json::Error),
    #[error("HTTP 客户端构建失败: {0}")]
    HttpClient(String),
}

/// 1-14 题本地评分（不调用 LLM）
pub fn score_mcq(
    answers: &HashMap<u32, Option<String>>,
    correct_answers: &HashMap<u32, String>,
    question_ids: &[u32],
) -> Vec<McqResult> {
    question_ids
        .iter()
        .map(|&qid| {
            let user_answer = answers.get(&qid).cloned().flatten();
            let correct_answer = correct_answers
                .get(&qid)
                .cloned()
                .unwrap_or_else(|| "A".to_string());
            let is_correct = user_answer.as_deref() == Some(correct_answer.as_str());
            McqResult {
                question_id: qid,
                user_answer,
                correct_answer,
                is_correct,
            }
        })
        .collect()
}

/// 15-18 题填空评分（调用 LLM）
///
/// 返回每空的得分明细与总分（满分 6，每空 1.5）。
pub async fn score_blanks_15_18(
    llm_cfg: &ModelConfig,
    llm_params: &crate::models::config::LlmParams,
    prompts: &PromptConfig,
    retell: &crate::models::question::RetellMaterial,
    user_answers: &HashMap<u32, Option<String>>,
) -> Result<(Vec<BlankResult>, f32), ScoringError> {
    // 构造 ORIGINAL_TEXT 占位：含原文 + 标准答案
    let mut original_text = format!("【听力原文】\n{}\n\n【标准答案】\n", retell.passage);
    for key in ["15", "16", "17", "18"] {
        if let Some(ans) = retell.blanks.get(key) {
            original_text.push_str(&format!("第 {key} 空：{ans}\n"));
        }
    }

    // 构造 ANSWERS 占位
    let answers_json = serde_json::json!({
        "15": user_answers.get(&15).cloned().flatten().unwrap_or_default(),
        "16": user_answers.get(&16).cloned().flatten().unwrap_or_default(),
        "17": user_answers.get(&17).cloned().flatten().unwrap_or_default(),
        "18": user_answers.get(&18).cloned().flatten().unwrap_or_default(),
    });
    let answers_text = serde_json::to_string_pretty(&answers_json)
        .map_err(|e| ScoringError::Prompt(format!("序列化答案失败: {e}")))?;

    let mut vars = HashMap::new();
    vars.insert("ORIGINAL_TEXT", original_text.as_str());
    vars.insert("ANSWERS", answers_text.as_str());

    let prompt = render(&prompts.q15_18_scoring, &vars)
        .map_err(|e| ScoringError::Prompt(e.to_string()))?;

    let client = build_client().map_err(|e| ScoringError::HttpClient(e.to_string()))?;
    let text = call_llm(&client, llm_cfg, llm_params, prompt).await?;

    info!("15-18 题评分 LLM 返回：{}", truncate_chars(&text, 200));

    let parsed: BlankScoreResp = try_parse(&text)?;
    let items = parsed.blanks;
    let llm_total = parsed.total_score.unwrap_or(0.0);

    // 三级 fallback 装配每空得分（详见 module-level `resolve_blank_outcome`）：
    //   1) LLM 的 score（吸附到 0/1.5）
    //   2) LLM 的 is_correct（映射到 1.5/0）
    //   3) 本地字符串匹配（兜底兜底）
    // 匹配策略双轨（详见 module-level `find_blank_item`）：
    //   轨道 A：按 blank_id 字符串匹配（首选）
    //   轨道 B：按数组索引匹配（兜底，应对 LLM 漏写 blank_id 但顺序对的情况）
    let keys = ["15", "16", "17", "18"];
    let mut results = Vec::new();
    for (i, key) in keys.iter().enumerate() {
        let qid: u32 = key.parse().unwrap_or(0);
        let user = user_answers
            .get(&qid)
            .cloned()
            .flatten()
            .unwrap_or_default();
        let correct = retell
            .blanks
            .get(*key)
            .cloned()
            .unwrap_or_default();
        // 本地兜底：仅大小写归一化
        let local_score = if !user.is_empty()
            && user.to_lowercase() == correct.to_lowercase()
        {
            1.5
        } else {
            0.0
        };

        let item = find_blank_item(&items, &keys, i);
        let final_score = resolve_blank_outcome(item, local_score);

        results.push(BlankResult {
            blank_id: key.to_string(),
            user_answer: user,
            correct_answer: correct,
            is_correct: final_score > 0.0,
            score: final_score,
        });
    }

    // 总分优先用 LLM 返回（夹紧到 [0, 6]），否则累加
    let final_total = if llm_total > 0.0 {
        llm_total.clamp(0.0, 6.0)
    } else {
        results.iter().map(|r| r.score).sum()
    };

    Ok((results, final_total))
}

/// 19 题转述评分：先 STT 转写录音，再 LLM 评分
pub async fn score_retell_19(
    stt_cfg: &ModelConfig,
    llm_cfg: &ModelConfig,
    llm_params: &crate::models::config::LlmParams,
    prompts: &PromptConfig,
    original_text: &str,
    recording_path: Option<&str>,
) -> RetellResult {
    // 1. STT 转写
    let stt_text = match recording_path {
        Some(p) if !p.is_empty() && std::path::Path::new(p).exists() => {
            let client = match build_client() {
                Ok(c) => c,
                Err(e) => {
                    warn!(error = %e, "构建 STT HTTP 客户端失败");
                    return RetellResult {
                        score: 0.0,
                        max_score: 10.0,
                        comment: format!("STT 客户端初始化失败: {e}"),
                        stt_text: String::new(),
                    };
                }
            };
            match transcribe(&client, stt_cfg, std::path::Path::new(p), "en").await {
                Ok(t) => t,
                Err(e) => {
                    warn!(error = %e, "STT 转写失败");
                    return RetellResult {
                        score: 0.0,
                        max_score: 10.0,
                        comment: format!("录音转写失败: {e}"),
                        stt_text: String::new(),
                    };
                }
            }
        }
        _ => {
            return RetellResult {
                score: 0.0,
                max_score: 10.0,
                comment: "未找到录音文件".to_string(),
                stt_text: String::new(),
            };
        }
    };

    info!("STT 转写结果（前 200 字）：{}", truncate_chars(&stt_text, 200));

    // 2. LLM 评分
    let mut vars = HashMap::new();
    vars.insert("ORIGINAL_TEXT", original_text);
    vars.insert("STT_RESULT", stt_text.as_str());

    let prompt = match render(&prompts.q19_scoring, &vars) {
        Ok(p) => p,
        Err(e) => {
            return RetellResult {
                score: 0.0,
                max_score: 10.0,
                comment: format!("Prompt 渲染失败: {e}"),
                stt_text,
            };
        }
    };

    let client = match build_client() {
        Ok(c) => c,
        Err(e) => {
            return RetellResult {
                score: 0.0,
                max_score: 10.0,
                comment: format!("LLM 客户端初始化失败: {e}"),
                stt_text,
            };
        }
    };

    let text = match call_llm(&client, llm_cfg, llm_params, prompt).await {
        Ok(t) => t,
        Err(e) => {
            return RetellResult {
                score: 0.0,
                max_score: 10.0,
                comment: format!("LLM 评分调用失败: {e}"),
                stt_text,
            };
        }
    };

    info!("19 题评分 LLM 返回：{}", truncate_chars(&text, 200));

    // 解析评分
    #[derive(Debug, Deserialize)]
    struct RetellScore {
        score: f32,
        #[serde(default)]
        max_score: f32,
        #[serde(default)]
        comment: String,
    }

    match try_parse::<RetellScore>(&text) {
        Ok(p) => RetellResult {
            score: p.score,
            max_score: if p.max_score > 0.0 { p.max_score } else { 10.0 },
            comment: p.comment,
            stt_text,
        },
        Err(e) => RetellResult {
            score: 0.0,
            max_score: 10.0,
            comment: format!("评分解析失败: {e}\n原始返回: {}", truncate_chars(&text, 300)),
            stt_text,
        },
    }
}

/// 完整评分入口
pub async fn score_full_test(
    config: &AppConfig,
    session: &TestSession,
    user_answers: &HashMap<u32, Option<String>>,
    correct_answers: &HashMap<u32, String>,
    mcq_question_ids: &[u32],
    recording_path: Option<&str>,
) -> Result<TestResult, ScoringError> {
    // 1. 1-14 题本地评分
    let mcq_results = score_mcq(user_answers, correct_answers, mcq_question_ids);

    // 2. 15-18 题 LLM 评分
    let (blank_results, blank_total) = score_blanks_15_18(
        &config.llm,
        &config.llm_params,
        &config.prompts,
        &session.retell,
        user_answers,
    )
    .await?;

    // 3. 19 题 STT + LLM 评分
    let retell_result = score_retell_19(
        &config.stt,
        &config.llm,
        &config.llm_params,
        &config.prompts,
        &session.retell.passage,
        recording_path,
    )
    .await;

    // 4. 构造对话原文映射（结算页展示用）
    let mut dialogue_texts = HashMap::new();
    for d in &session.short_dialogues {
        let qid = d.question.id;
        let text = d
            .dialogue
            .iter()
            .map(|t| format!("{}: {}", t.speaker, t.text))
            .collect::<Vec<_>>()
            .join("\n");
        dialogue_texts.insert(format!("q{qid}"), text);
    }
    for d in &session.long_dialogues {
        for q in &d.questions {
            let text = d
                .dialogue
                .iter()
                .map(|t| format!("{}: {}", t.speaker, t.text))
                .collect::<Vec<_>>()
                .join("\n");
            dialogue_texts.insert(format!("q{}", q.id), text);
        }
    }
    dialogue_texts.insert(
        "m13".to_string(),
        session.monologue.text.clone(),
    );
    dialogue_texts.insert(
        "retell".to_string(),
        session.retell.passage.clone(),
    );

    // 5. 总分：1-14 正确数 + 15-18 得分 + 19 得分
    let correct_count = mcq_results.iter().filter(|r| r.is_correct).count();
    let total_score = correct_count as f32 + blank_total + retell_result.score;

    Ok(TestResult {
        session_id: session.session_id.clone(),
        mcq_results,
        blank_results,
        retell_result: Some(retell_result),
        blank_total_score: blank_total,
        total_score,
        max_score: 14.0 + 6.0 + 10.0,
        dialogue_texts,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreProgress {
    pub stage: &'static str, // "mcq" | "blanks" | "retell" | "done"
    pub message: String,
}

pub const SCORE_PROGRESS_EVENT: &str = "test-score-progress";

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(blank_id: Option<&str>, score: Option<f32>, is_correct: Option<bool>) -> BlankScoreItem {
        BlankScoreItem {
            blank_id: blank_id.map(String::from),
            user_answer: None,
            correct_answer: None,
            is_correct,
            score,
        }
    }

    #[test]
    fn deserialize_accepts_blank_id_alias_id() {
        let json = r#"{"id": "15", "score": 1.5, "is_correct": true}"#;
        let parsed: BlankScoreItem = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.blank_id.as_deref(), Some("15"));
    }

    #[test]
    fn deserialize_accepts_question_id_alias() {
        let json = r#"{"question_id": "17", "score": 0.0, "is_correct": false}"#;
        let parsed: BlankScoreItem = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.blank_id.as_deref(), Some("17"));
    }

    #[test]
    fn deserialize_blank_id_field_missing_yields_none() {
        let json = r#"{"score": 1.5, "is_correct": true}"#;
        let parsed: BlankScoreItem = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.blank_id, None);
    }

    #[test]
    fn find_blank_item_matches_by_id_first() {
        let items = vec![
            make_item(Some("15"), Some(1.5), Some(true)),
            make_item(Some("16"), Some(0.0), Some(false)),
        ];
        let keys = ["15", "16", "17", "18"];
        let it = find_blank_item(&items, &keys, 1).unwrap();
        assert_eq!(it.score, Some(0.0));
    }

    #[test]
    fn find_blank_item_falls_back_to_index_when_no_blank_id() {
        let items = vec![
            make_item(None, Some(1.5), Some(true)),
            make_item(None, Some(0.0), Some(false)),
            make_item(None, Some(1.5), Some(true)),
            make_item(None, Some(0.0), Some(false)),
        ];
        let keys = ["15", "16", "17", "18"];
        assert_eq!(find_blank_item(&items, &keys, 0).unwrap().score, Some(1.5));
        assert_eq!(find_blank_item(&items, &keys, 1).unwrap().score, Some(0.0));
        assert_eq!(find_blank_item(&items, &keys, 2).unwrap().score, Some(1.5));
        assert_eq!(find_blank_item(&items, &keys, 3).unwrap().score, Some(0.0));
    }

    #[test]
    fn resolve_prefers_score_over_is_correct_on_conflict() {
        // score=0.0 + is_correct=true 同时存在矛盾时，score 优先
        let it = make_item(Some("15"), Some(0.0), Some(true));
        let s = resolve_blank_outcome(Some(&it), 1.5);
        assert_eq!(s, 0.0);
    }

    #[test]
    fn resolve_uses_is_correct_when_score_missing() {
        let it = make_item(Some("16"), None, Some(true));
        let s = resolve_blank_outcome(Some(&it), 0.0);
        assert_eq!(s, 1.5);
    }

    #[test]
    fn resolve_falls_back_to_local_when_item_missing() {
        let s = resolve_blank_outcome(None, 1.5);
        assert_eq!(s, 1.5);
        let s = resolve_blank_outcome(None, 0.0);
        assert_eq!(s, 0.0);
    }

    #[test]
    fn resolve_clamps_midpoint_score_to_nearest_zero() {
        // 0.5 应该被吸附到 0
        let it = make_item(Some("17"), Some(0.5), None);
        assert_eq!(resolve_blank_outcome(Some(&it), 0.0), 0.0);
        // 0.9 应该被吸附到 1.5
        let it = make_item(Some("18"), Some(0.9), None);
        assert_eq!(resolve_blank_outcome(Some(&it), 0.0), 1.5);
    }

    #[test]
    fn deserialize_full_response() {
        let json = r#"{
            "blanks": [
                {"blank_id": "15", "is_correct": true,  "score": 1.5, "user_answer": "london", "correct_answer": "London"},
                {"blank_id": "16", "is_correct": false, "score": 0.0, "user_answer": "x",       "correct_answer": "y"},
                {"blank_id": "17", "is_correct": true,  "score": 1.5, "user_answer": "color",   "correct_answer": "colour"},
                {"blank_id": "18", "is_correct": false, "score": 0.0, "user_answer": "",       "correct_answer": "z"}
            ],
            "total_score": 3.0,
            "comment": ""
        }"#;
        let parsed: BlankScoreResp = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.blanks.len(), 4);
        assert_eq!(parsed.total_score, Some(3.0));
        assert_eq!(parsed.blanks[2].correct_answer.as_deref(), Some("colour"));
    }

    #[test]
    fn deserialize_response_with_alias_keys() {
        let json = r#"{
            "blanks": [
                {"id": "15", "score": 1.5, "is_correct": true},
                {"question_id": "16", "score": 0.0, "is_correct": false},
                {"blank": "17", "score": 1.5, "is_correct": true},
                {"blankId": "18", "score": 0.0, "is_correct": false}
            ],
            "total_score": 3.0
        }"#;
        let parsed: BlankScoreResp = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.blanks[0].blank_id.as_deref(), Some("15"));
        assert_eq!(parsed.blanks[1].blank_id.as_deref(), Some("16"));
        assert_eq!(parsed.blanks[2].blank_id.as_deref(), Some("17"));
        assert_eq!(parsed.blanks[3].blank_id.as_deref(), Some("18"));
    }

    #[test]
    fn try_parse_handles_markdown_fenced_json() {
        let raw = "思考过程...\n```json\n{\"blanks\":[{\"blank_id\":\"15\",\"score\":1.5,\"is_correct\":true}],\"total_score\":1.5}\n```\n结束";
        let parsed: BlankScoreResp = try_parse(raw).unwrap();
        assert_eq!(parsed.blanks.len(), 1);
        assert_eq!(parsed.total_score, Some(1.5));
    }

    #[test]
    fn try_parse_handles_raw_json_with_chatter() {
        let raw = "好的，我开始评分：\n{\"blanks\":[{\"blank_id\":\"15\",\"score\":1.5,\"is_correct\":true}],\"total_score\":1.5,\"comment\":\"good\"}";
        let parsed: BlankScoreResp = try_parse(raw).unwrap();
        assert_eq!(parsed.blanks.len(), 1);
        assert_eq!(parsed.comment.as_deref(), Some("good"));
    }
}