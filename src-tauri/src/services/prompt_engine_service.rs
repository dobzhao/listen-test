//! Prompt 引擎服务层封装：把 `utils::prompt_engine` 的错误映射为
//! 适合 Tauri command 返回的字符串错误。

pub use crate::utils::prompt_engine::{render, PromptError};
