//! 题目生成服务：根据 Prompt 模板调用 LLM 并解析为强类型
//!
//! 三次 LLM 调用：
//! - `q1_4`：4 段短对话，每段配 1 题
//! - `q5_14`：4 段长对话 + 1 段独白，每段/每独白配 2 题
//! - `q15_18`：1 段较长听力材料 + 总-分表格 + 4 个挖空

use crate::models::config::{AppConfig, LlmParams, ModelConfig, PromptConfig};
use crate::models::question::{
    LongDialogue, Monologue, MultipleChoiceQuestion, RetellMaterial, ShortDialogue,
};
use crate::services::llm_service::{call_llm, LlmError};
use crate::services::prompt_engine_service::render;
use crate::utils::json_extract::try_parse;
use crate::utils::retry::{retry_async, RetryConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum GenError {
    #[error("LLM 调用失败: {0}")]
    Llm(#[from] LlmError),
    #[error("Prompt 渲染失败: {0}")]
    Prompt(String),
    #[error("JSON 解析失败: {0}")]
    Json(#[from] serde_json::Error),
    #[error("文本合规检查未通过: {0}")]
    Compliance(String),
    #[error("重试多次后仍无法生成有效结果")]
    Exhausted,
}

/// 1-4 题的 LLM 输出（数组）
type Q1To4Raw = Vec<ShortDialogueRaw>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortDialogueRaw {
    pub id: u32,
    pub question: String,
    pub dialogue: Vec<DialogueTurnRaw>,
    pub options: HashMap<String, String>,
    pub answer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueTurnRaw {
    pub speaker: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Q5To14Raw {
    pub dialogues: Vec<LongDialogueRaw>,
    pub monologue: MonologueRaw,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongDialogueRaw {
    pub id: u32,
    pub dialogue: Vec<DialogueTurnRaw>,
    pub questions: Vec<MultipleChoiceQuestionRaw>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonologueRaw {
    pub text: String,
    pub questions: Vec<MultipleChoiceQuestionRaw>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipleChoiceQuestionRaw {
    pub id: u32,
    pub question: String,
    pub options: HashMap<String, String>,
    pub answer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Q15To18Raw {
    pub passage: String,
    pub table: TableRaw,
    pub blanks: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRaw {
    pub rows: Vec<TableRowRaw>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRowRaw {
    pub overview: String,
    pub details: Vec<String>,
}

// ===== 1-4 题 =====

/// 生成 1-4 题：4 段短对话，每段配 1 题
pub async fn generate_q1_4(
    http_client: &reqwest::Client,
    llm_cfg: &ModelConfig,
    llm_params: &LlmParams,
    prompts: &PromptConfig,
) -> Result<Vec<ShortDialogue>, GenError> {
    let mut vars = HashMap::new();
    vars.insert("QUESTION_COUNT", "4");
    let prompt = render(&prompts.q1_4, &vars).map_err(|e| GenError::Prompt(e.to_string()))?;

    let retry = RetryConfig {
        max_retries: 2,
        ..Default::default()
    };

    let http = http_client.clone();
    let cfg = llm_cfg.clone();
    let params = llm_params.clone();

    let raw: Q1To4Raw = retry_async(
        &retry,
        "generate_q1_4",
        || {
            let http = http.clone();
            let cfg = cfg.clone();
            let params = params.clone();
            let prompt = prompt.clone();
            async move {
                let text = call_llm(&http, &cfg, &params, prompt).await?;
                parse_with_validation::<Q1To4Raw>(&text, validate_q1_4)
            }
        },
        |e| matches!(e, GenError::Llm(_) | GenError::Json(_) | GenError::Compliance(_)),
    )
    .await?;

    Ok(raw.into_iter().map(into_short_dialogue).collect())
}

// ===== 5-14 题 =====

/// 生成 5-14 题：4 段长对话 + 1 段独白
///
/// 返回 (long_dialogues, monologue)
pub async fn generate_q5_14(
    http_client: &reqwest::Client,
    llm_cfg: &ModelConfig,
    llm_params: &LlmParams,
    prompts: &PromptConfig,
) -> Result<(Vec<LongDialogue>, Monologue), GenError> {
    let vars = HashMap::new();
    let prompt = render(&prompts.q5_14, &vars).map_err(|e| GenError::Prompt(e.to_string()))?;

    let retry = RetryConfig {
        max_retries: 2,
        ..Default::default()
    };

    let http = http_client.clone();
    let cfg = llm_cfg.clone();
    let params = llm_params.clone();

    let raw: Q5To14Raw = retry_async(
        &retry,
        "generate_q5_14",
        || {
            let http = http.clone();
            let cfg = cfg.clone();
            let params = params.clone();
            let prompt = prompt.clone();
            async move {
                let text = call_llm(&http, &cfg, &params, prompt).await?;
                parse_with_validation::<Q5To14Raw>(&text, validate_q5_14)
            }
        },
        |e| matches!(e, GenError::Llm(_) | GenError::Json(_) | GenError::Compliance(_)),
    )
    .await?;

    let long_dialogues: Vec<LongDialogue> = raw.dialogues.into_iter().map(into_long_dialogue).collect();
    let monologue = into_monologue(raw.monologue);
    Ok((long_dialogues, monologue))
}

// ===== 15-18 题 =====

/// 生成 15-19 题听力材料与挖空表格
pub async fn generate_q15_18(
    http_client: &reqwest::Client,
    llm_cfg: &ModelConfig,
    llm_params: &LlmParams,
    prompts: &PromptConfig,
) -> Result<RetellMaterial, GenError> {
    let vars = HashMap::new();
    let prompt = render(&prompts.q15_18, &vars).map_err(|e| GenError::Prompt(e.to_string()))?;

    let retry = RetryConfig {
        max_retries: 2,
        ..Default::default()
    };

    let http = http_client.clone();
    let cfg = llm_cfg.clone();
    let params = llm_params.clone();

    let raw: Q15To18Raw = retry_async(
        &retry,
        "generate_q15_18",
        || {
            let http = http.clone();
            let cfg = cfg.clone();
            let params = params.clone();
            let prompt = prompt.clone();
            async move {
                let text = call_llm(&http, &cfg, &params, prompt).await?;
                parse_with_validation::<Q15To18Raw>(&text, validate_q15_18)
            }
        },
        |e| matches!(e, GenError::Llm(_) | GenError::Json(_) | GenError::Compliance(_)),
    )
    .await?;

    Ok(into_retell_material(raw))
}

// ===== 内部辅助 =====

/// 解析 LLM 输出 + 自定义校验
fn parse_with_validation<T>(text: &str, validate: fn(&T) -> Result<(), String>) -> Result<T, GenError>
where
    T: serde::de::DeserializeOwned,
{
    let parsed: T = try_parse(text)?;
    validate(&parsed).map_err(GenError::Compliance)?;
    Ok(parsed)
}

/// 校验 1-4 题结构
fn validate_q1_4(raw: &Q1To4Raw) -> Result<(), String> {
    if raw.len() != 4 {
        return Err(format!("期望 4 段对话，实际 {} 段", raw.len()));
    }
    for (i, d) in raw.iter().enumerate() {
        if d.dialogue.is_empty() {
            return Err(format!("第 {} 段对话为空", i + 1));
        }
        validate_choice_question(&d.options, &d.answer, &format!("第 {} 段对话", i + 1))?;
    }
    Ok(())
}

/// 校验 5-14 题结构
fn validate_q5_14(raw: &Q5To14Raw) -> Result<(), String> {
    if raw.dialogues.len() != 4 {
        return Err(format!("期望 4 段长对话，实际 {} 段", raw.dialogues.len()));
    }
    for (i, d) in raw.dialogues.iter().enumerate() {
        if d.dialogue.is_empty() {
            return Err(format!("第 {} 段长对话内容为空", i + 1));
        }
        if d.questions.len() != 2 {
            return Err(format!(
                "第 {} 段长对话应配 2 题，实际 {} 题",
                i + 1,
                d.questions.len()
            ));
        }
        for (j, q) in d.questions.iter().enumerate() {
            validate_choice_question(&q.options, &q.answer, &format!("第 {} 段长对话第 {} 题", i + 1, j + 1))?;
        }
    }
    if raw.monologue.text.trim().is_empty() {
        return Err("独白文本为空".to_string());
    }
    if raw.monologue.questions.len() != 2 {
        return Err(format!(
            "独白应配 2 题，实际 {} 题",
            raw.monologue.questions.len()
        ));
    }
    for (j, q) in raw.monologue.questions.iter().enumerate() {
        validate_choice_question(&q.options, &q.answer, &format!("独白第 {} 题", j + 1))?;
    }
    Ok(())
}

/// 校验单道选择题：答案 ∈ {A,B,C} 且选项数 = 3
fn validate_choice_question(
    options: &HashMap<String, String>,
    answer: &str,
    label: &str,
) -> Result<(), String> {
    if !["A", "B", "C"].contains(&answer) {
        return Err(format!("{} 答案不合法: {}", label, answer));
    }
    if options.len() != 3 {
        return Err(format!(
            "{} 选项数量应为 3，实际 {}",
            label,
            options.len()
        ));
    }
    Ok(())
}

/// 校验 15-18 题结构
fn validate_q15_18(raw: &Q15To18Raw) -> Result<(), String> {
    let wc = raw.passage.split_whitespace().count();
    if !(150..=300).contains(&wc) {
        warn!(words = wc, "听力材料词数偏离 200-220 范围");
    }
    if raw.table.rows.len() != 3 {
        return Err(format!("表格应为 3 行，实际 {} 行", raw.table.rows.len()));
    }
    for key in ["15", "16", "17", "18"] {
        if !raw.blanks.contains_key(key) {
            return Err(format!("缺少第 {} 题标准答案", key));
        }
    }
    Ok(())
}

fn into_short_dialogue(raw: ShortDialogueRaw) -> ShortDialogue {
    let id = raw.id;
    let question_text = raw.question;
    let options = raw.options;
    let answer = raw.answer;
    let dialogue = raw.dialogue;
    ShortDialogue {
        id,
        dialogue: dialogue
            .into_iter()
            .map(|t| crate::models::question::DialogueTurn {
                speaker: t.speaker,
                text: t.text,
            })
            .collect(),
        question: MultipleChoiceQuestion {
            id,
            question: question_text,
            options: normalize_options(options),
            answer: normalize_answer(answer),
        },
    }
}

fn into_long_dialogue(raw: LongDialogueRaw) -> LongDialogue {
    let id = raw.id;
    let dialogue = raw.dialogue;
    let mut qs: Vec<MultipleChoiceQuestion> = raw
        .questions
        .into_iter()
        .map(|q| MultipleChoiceQuestion {
            id: q.id,
            question: q.question,
            options: normalize_options(q.options),
            answer: normalize_answer(q.answer),
        })
        .collect();
    // 防御性：确保长度 = 2（虽然 LLM 应该输出 2 题）
    while qs.len() < 2 {
        qs.push(MultipleChoiceQuestion {
            id: 0,
            question: String::new(),
            options: default_options(),
            answer: "A".to_string(),
        });
    }
    let q1 = qs.remove(0);
    let q2 = qs.remove(0);
    LongDialogue {
        id,
        dialogue: dialogue
            .into_iter()
            .map(|t| crate::models::question::DialogueTurn {
                speaker: t.speaker,
                text: t.text,
            })
            .collect(),
        questions: [q1, q2],
    }
}

fn into_monologue(raw: MonologueRaw) -> Monologue {
    let mut qs: Vec<MultipleChoiceQuestion> = raw
        .questions
        .into_iter()
        .map(|q| MultipleChoiceQuestion {
            id: q.id,
            question: q.question,
            options: normalize_options(q.options),
            answer: normalize_answer(q.answer),
        })
        .collect();
    while qs.len() < 2 {
        qs.push(MultipleChoiceQuestion {
            id: 0,
            question: String::new(),
            options: default_options(),
            answer: "A".to_string(),
        });
    }
    let q1 = qs.remove(0);
    let q2 = qs.remove(0);
    Monologue {
        text: raw.text,
        questions: [q1, q2],
    }
}

fn into_retell_material(raw: Q15To18Raw) -> RetellMaterial {
    use crate::models::question::{SummaryTable, TableRow};

    let mut rows: Vec<TableRow> = raw
        .table
        .rows
        .into_iter()
        .map(|r| TableRow {
            overview: r.overview,
            details: r.details,
        })
        .collect();
    // 防御性补齐到 3 行
    while rows.len() < 3 {
        rows.push(TableRow {
            overview: String::new(),
            details: vec![],
        });
    }
    let r1 = rows.remove(0);
    let r2 = rows.remove(0);
    let r3 = rows.remove(0);

    let table = SummaryTable { rows: [r1, r2, r3] };

    let mut blanks_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for key in ["15", "16", "17", "18"] {
        let v = raw
            .blanks
            .get(key)
            .cloned()
            .unwrap_or_else(|| String::new());
        blanks_map.insert(key.to_string(), v);
    }

    RetellMaterial {
        passage: raw.passage,
        table,
        blanks: blanks_map,
    }
}

/// 把 LLM 输出的 options HashMap 标准化为 {"A": "...", "B": "...", "C": "..."}
fn normalize_options(
    raw: HashMap<String, String>,
) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for k in ["A", "B", "C"] {
        let v = raw.get(k).cloned().unwrap_or_default();
        out.insert(k.to_string(), v);
    }
    out
}

fn default_options() -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    out.insert("A".to_string(), String::new());
    out.insert("B".to_string(), String::new());
    out.insert("C".to_string(), String::new());
    out
}

fn normalize_answer(a: String) -> String {
    match a.to_uppercase().as_str() {
        "A" => "A".to_string(),
        "B" => "B".to_string(),
        "C" => "C".to_string(),
        _ => "A".to_string(),
    }
}

// ShortDialogueRaw 的 question 字段需要派生 id，这里手动补（实际未使用）
#[allow(dead_code)]
impl ShortDialogueRaw {
    pub fn question_id(&self) -> u32 {
        self.id
    }
}

/// Phase 2 验证辅助：用一个最小 mock config 生成
#[allow(dead_code)]
pub async fn generate_all_for_test(
    http_client: &reqwest::Client,
    config: &AppConfig,
) -> Result<
    (
        Vec<ShortDialogue>,
        Vec<LongDialogue>,
        Monologue,
        RetellMaterial,
    ),
    GenError,
> {
    info!("开始生成 1-4 题");
    let short = generate_q1_4(
        http_client,
        &config.llm,
        &config.llm_params,
        &config.prompts,
    )
    .await?;

    info!("开始生成 5-14 题");
    let (long, monologue) = generate_q5_14(
        http_client,
        &config.llm,
        &config.llm_params,
        &config.prompts,
    )
    .await?;

    info!("开始生成 15-18 题");
    let retell = generate_q15_18(
        http_client,
        &config.llm,
        &config.llm_params,
        &config.prompts,
    )
    .await?;

    Ok((short, long, monologue, retell))
}
