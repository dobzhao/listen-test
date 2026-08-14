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

    // 解析评分结果
    #[derive(Debug, Deserialize)]
    struct BlankScoreItem {
        #[serde(default)]
        blank_id: String,
        #[serde(default)]
        user_answer: Option<String>,
        #[serde(default)]
        correct_answer: Option<String>,
        #[serde(default)]
        is_correct: Option<bool>,
        #[serde(default)]
        score: Option<f32>,
    }
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum BlankScoreResp {
        Detailed {
            #[serde(default)]
            blanks: Vec<BlankScoreItem>,
            #[serde(default)]
            total_score: Option<f32>,
            #[serde(default)]
            comment: Option<String>,
        },
        Simple {
            #[serde(default)]
            total_score: Option<f32>,
            #[serde(default)]
            results: Vec<BlankScoreItem>,
        },
    }

    let parsed: BlankScoreResp = try_parse(&text)?;
    let (items, total) = match parsed {
        BlankScoreResp::Detailed { blanks, total_score, .. } => (
            blanks,
            total_score.unwrap_or(0.0),
        ),
        BlankScoreResp::Simple { results, total_score } => (
            results,
            total_score.unwrap_or(0.0),
        ),
    };

    // 兜底：用本地规则生成明细
    let mut results = Vec::new();
    for key in ["15", "16", "17", "18"] {
        let qid: u32 = key.parse().unwrap_or(0);
        let user = user_answers
            .get(&qid)
            .cloned()
            .flatten()
            .unwrap_or_default();
        let correct = retell
            .blanks
            .get(key)
            .cloned()
            .unwrap_or_default();
        // 大小写不敏感、单复数严格、英美拼写允许
        let is_correct_simple = !user.is_empty()
            && user.to_lowercase() == correct.to_lowercase();
        let per_score = if is_correct_simple { 1.5 } else { 0.0 };

        // 优先使用 LLM 返回的明细
        let item_score = items
            .iter()
            .find(|it| it.blank_id == key)
            .and_then(|it| it.score)
            .unwrap_or(per_score);

        results.push(BlankResult {
            blank_id: key.to_string(),
            user_answer: user,
            correct_answer: correct,
            is_correct: item_score > 0.0,
            score: item_score,
        });
    }

    // 总分优先用 LLM 返回，否则累加
    let final_total = if total > 0.0 {
        total
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
    for (idx, d) in session.short_dialogues.iter().enumerate() {
        let qid = d.question.id;
        let text = d
            .dialogue
            .iter()
            .map(|t| format!("{}: {}", t.speaker, t.text))
            .collect::<Vec<_>>()
            .join("\n");
        dialogue_texts.insert(format!("q{qid}"), text);
    }
    for (idx, d) in session.long_dialogues.iter().enumerate() {
        for q in &d.questions {
            let text = d
                .dialogue
                .iter()
                .map(|t| format!("{}: {}", t.speaker, t.text))
                .collect::<Vec<_>>()
                .join("\n");
            dialogue_texts.insert(format!("q{}", q.id), text);
        }
        let _ = idx;
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