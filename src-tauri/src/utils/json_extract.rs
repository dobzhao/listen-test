//! JSON 解析容错工具
//!
//! LLM 经常在 JSON 外面包一层 ```json ... ``` 围栏，或者在前面加些解释文字。
//! 这个模块负责从 LLM 输出中提取第一个合法的 JSON 块。

/// 从 LLM 输出文本中提取第一个 JSON 块（自动剥离 ```json 围栏与首尾杂文）。
///
/// 支持以下输入形式：
/// - 纯 JSON（无围栏）
/// - ```json\n{...}\n```
/// - ```\n{...}\n```
/// - "以下是 JSON: {...} 完毕"
pub fn extract_first_json(text: &str) -> Option<String> {
    let trimmed = text.trim();

    // 1. 优先尝试匹配 ```json ... ``` 代码块
    if let Some(start) = trimmed.find("```json") {
        let body_start = start + "```json".len();
        if let Some(end) = trimmed[body_start..].find("```") {
            let candidate = &trimmed[body_start..body_start + end];
            return Some(candidate.trim().to_string());
        }
    }

    // 2. 尝试匹配通用 ``` ... ``` 代码块
    if let Some(start) = trimmed.find("```") {
        let body_start = start + "```".len();
        // 跳过可能的语言标记（如 ```json\n）
        let after = &trimmed[body_start..];
        let body_start = if let Some(newline) = after.find('\n') {
            body_start + newline + 1
        } else {
            body_start
        };
        if let Some(end) = trimmed[body_start..].find("```") {
            let candidate = &trimmed[body_start..body_start + end];
            return Some(candidate.trim().to_string());
        }
    }

    // 3. 退化：扫描第一个 { 或 [，匹配对应的 } 或 ]
    let bytes = trimmed.as_bytes();
    let mut open_idx = None;
    let mut open_char = ' ';
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'{' {
            open_idx = Some(i);
            open_char = '{';
            break;
        }
        if b == b'[' {
            open_idx = Some(i);
            open_char = '[';
            break;
        }
    }
    let Some(open_idx) = open_idx else {
        return None;
    };

    let close_char = if open_char == '{' { '}' } else { ']' };
    // 从末尾向前找最近的 close_char
    for i in (open_idx..bytes.len()).rev() {
        if bytes[i] as u32 == close_char as u32 {
            return Some(trimmed[open_idx..=i].to_string());
        }
    }

    None
}

/// 尝试解析 JSON，失败时返回最后一次错误。
pub fn try_parse<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, serde_json::Error> {
    let candidate = extract_first_json(text)
        .ok_or_else(|| serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "no JSON block found in LLM output",
        )))?;
    serde_json::from_str(&candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_pure_json() {
        let input = r#"{"a": 1, "b": 2}"#;
        let out = extract_first_json(input).unwrap();
        assert_eq!(out, r#"{"a": 1, "b": 2}"#);
    }

    #[test]
    fn test_extract_json_codeblock() {
        let input = "以下是 JSON：\n```json\n{\"a\": 1}\n```\n完毕";
        let out = extract_first_json(input).unwrap();
        assert_eq!(out, "{\"a\": 1}");
    }

    #[test]
    fn test_extract_json_with_chatter() {
        let input = "思考过程... 这里输出 JSON：\n[{\"x\": 1}] 结束";
        let out = extract_first_json(input).unwrap();
        assert_eq!(out, "[{\"x\": 1}]");
    }

    #[test]
    fn test_parse_object() {
        let input = "```json\n{\"name\": \"test\"}\n```";
        let v: serde_json::Value = try_parse(input).unwrap();
        assert_eq!(v["name"], "test");
    }
}
