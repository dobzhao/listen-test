//! 测试结果与评分数据结构

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 第 1-14 题的单题作答结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McqResult {
    pub question_id: u32,
    /// 用户选择的选项（"A"/"B"/"C"），未作答为 None
    pub user_answer: Option<String>,
    pub correct_answer: String,
    pub is_correct: bool,
}

/// 第 15-18 题的单空作答结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlankResult {
    pub blank_id: String, // "15".."18"
    pub user_answer: String,
    pub correct_answer: String,
    pub is_correct: bool,
    /// LLM 评分的单项得分（0 / 1.5 / ...）
    pub score: f32,
}

/// 第 19 题转述评分结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetellResult {
    pub score: f32,
    pub max_score: f32,
    pub comment: String,
    /// STT 转写文本，方便用户核对
    pub stt_text: String,
}

/// 完整测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub session_id: String,
    pub mcq_results: Vec<McqResult>,           // 1-14 题
    pub blank_results: Vec<BlankResult>,       // 15-18 题
    pub retell_result: Option<RetellResult>,   // 19 题
    pub blank_total_score: f32,                // 15-18 总分（满分 6）
    pub total_score: f32,                      // 1-14 正确数 + 15-18 得分 + 19 得分
    pub max_score: f32,                        // 14 + 6 + 10 = 30
    /// 各题目对应的原文（用于结算页展示对话原文）
    pub dialogue_texts: HashMap<String, String>,
}
