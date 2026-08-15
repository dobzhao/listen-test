//! Prompt 模板引擎：替换 `{{PLACEHOLDER}}` 形式的占位符
//!
//! 调用方负责确保所有用到的占位符都已传入；缺失时返回明确错误，方便
//! 前端在设置界面提示"缺失变量 XXX"，避免运行时 panic。

use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PromptError {
    #[error("Prompt 模板缺失占位符: {0}")]
    MissingVar(String),
}

/// 渲染 Prompt：用 vars 中的值替换模板中所有 `{{KEY}}`。
///
/// 替换规则：
/// - 仅识别严格符合 `{{KEY}}`（两侧 2 个花括号）的占位符
/// - KEY 由 ASCII 字母/数字/下划线组成
/// - 缺失的占位符返回 `PromptError::MissingVar(KEY)`
pub fn render(template: &str, vars: &HashMap<&str, &str>) -> Result<String, PromptError> {
    let mut out = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // 寻找 {{
        if i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            // 寻找 }}
            if let Some(close_offset) = find_close(&bytes[i + 2..]) {
                let key_bytes = &bytes[i + 2..i + 2 + close_offset];
                let key = std::str::from_utf8(key_bytes)
                    .map_err(|_| PromptError::MissingVar("<invalid utf8>".into()))?
                    .trim();
                if !is_valid_key(key) {
                    return Err(PromptError::MissingVar(key.to_string()));
                }
                let value = vars
                    .get(key)
                    .ok_or_else(|| PromptError::MissingVar(key.to_string()))?;
                out.push_str(value);
                i += 2 + close_offset + 2;
                continue;
            }
        }
        // 普通字符
        let c = template[i..].chars().next().unwrap();
        out.push(c);
        i += c.len_utf8();
    }
    Ok(out)
}

fn find_close(bytes: &[u8]) -> Option<usize> {
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == b'}' && bytes[i + 1] == b'}' {
            return Some(i);
        }
    }
    None
}

fn is_valid_key(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_render() {
        let mut vars = HashMap::new();
        vars.insert("NAME", "Alice");
        let out = render("Hello, {{NAME}}!", &vars).unwrap();
        assert_eq!(out, "Hello, Alice!");
    }

    #[test]
    fn test_missing_var() {
        let vars = HashMap::new();
        let err = render("Hello, {{NAME}}", &vars).unwrap_err();
        match err {
            PromptError::MissingVar(k) => assert_eq!(k, "NAME"),
        }
    }

    #[test]
    fn test_multiple_vars() {
        let mut vars = HashMap::new();
        vars.insert("A", "1");
        vars.insert("B", "2");
        let out = render("{{A}}-{{B}}-{{A}}", &vars).unwrap();
        assert_eq!(out, "1-2-1");
    }

    #[test]
    fn test_no_placeholders() {
        let vars = HashMap::new();
        let out = render("Just text.", &vars).unwrap();
        assert_eq!(out, "Just text.");
    }
}
