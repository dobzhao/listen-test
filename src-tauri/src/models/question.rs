//! 题目数据结构
//!
//! 所有题目都由后端 LLM 生成，前端只负责展示与计时。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 对话中的一个发言轮次
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueTurn {
    /// 说话人标记："M"（男）或 "W"（女），与 prompt 中约定的说话人一致
    pub speaker: String,
    /// 本轮的英文文本
    pub text: String,
}

/// 单道选择题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipleChoiceQuestion {
    pub id: u32,
    pub question: String,
    pub options: HashMap<String, String>, // A/B/C -> 文本
    pub answer: String, // "A" | "B" | "C"
}

/// 第 1-4 题：一段对话配 1 道题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortDialogue {
    pub id: u32,
    pub dialogue: Vec<DialogueTurn>,
    pub question: MultipleChoiceQuestion,
}

/// 第 5-12 题：一段对话配 2 道题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongDialogue {
    pub id: u32,
    pub dialogue: Vec<DialogueTurn>,
    pub questions: [MultipleChoiceQuestion; 2],
}

/// 第 13-14 题：独白配 2 道题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monologue {
    pub text: String,
    pub questions: [MultipleChoiceQuestion; 2],
}

/// 第 15-18 题挖空表格中的一行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRow {
    pub overview: String,
    pub details: Vec<String>, // 包含 `___15___` 等占位符
}

/// 第 15-18 题挖空表格
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryTable {
    pub rows: [TableRow; 3],
}

/// 第 15-19 题听力材料与挖空
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetellMaterial {
    /// 听力材料原文
    pub passage: String,
    /// 总-分结构表格（含挖空占位符 `___NN___`）
    pub table: SummaryTable,
    /// 4 个挖空的标准答案，key 是 "15".."18"
    pub blanks: HashMap<String, String>,
}

/// 完整的题目会话（一次测试的所有题目 + 音频路径）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSession {
    /// 唯一会话 ID（用于本地缓存目录）
    pub session_id: String,
    /// 1-4 题（4 段短对话）
    pub short_dialogues: Vec<ShortDialogue>,
    /// 5-12 题（4 段长对话）
    pub long_dialogues: Vec<LongDialogue>,
    /// 13-14 题（独白）
    pub monologue: Monologue,
    /// 15-19 题听力材料（含挖空表格）
    pub retell: RetellMaterial,
    /// 各题目对应音频文件的本地路径（绝对路径）
    pub audio_paths: HashMap<String, String>,
}
