use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

/// 批次级清洗开关。全部关闭时 `clean` 必须原样返回输入，保证 A3 可核对。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanOptions {
    pub headers: bool,
    pub cjk: bool,
    pub tables: bool,
}

impl Default for CleanOptions {
    fn default() -> Self {
        Self {
            headers: true,
            cjk: true,
            tables: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cleaned {
    pub markdown: String,
    pub warnings: Vec<String>,
    pub table_grade: String,
}

pub fn clean(markdown: &str, options: &CleanOptions) -> Cleaned {
    if !options.headers && !options.cjk && !options.tables {
        return Cleaned {
            markdown: markdown.to_string(),
            warnings: Vec::new(),
            table_grade: "A".into(),
        };
    }

    let raw_lines: Vec<&str> = markdown.split('\n').collect();
    let removal = if options.headers {
        header_footer_lines(&raw_lines)
    } else {
        HashSet::new()
    };

    let mut warnings = Vec::new();
    let mut table_grade = "A".to_string();
    if options.tables {
        let (grade, table_warnings) = grade_tables(&raw_lines);
        table_grade = grade;
        warnings.extend(table_warnings);
    }

    let mut output: Vec<String> = Vec::with_capacity(raw_lines.len());
    let mut fence: Option<&str> = None;
    let mut last_blank = false;

    for line in raw_lines {
        let trimmed = line.trim();
        if is_fence(trimmed) {
            fence = toggle_fence(fence, trimmed);
            output.push(line.to_string());
            last_blank = false;
            continue;
        }
        if fence.is_some() {
            output.push(line.to_string());
            last_blank = false;
            continue;
        }
        if options.headers && removal.contains(trimmed) {
            continue;
        }

        let mut next = if options.cjk {
            normalize_heading(line)
        } else {
            line.to_string()
        };
        if options.cjk && !next.trim_start().starts_with('|') {
            next = collapse_cjk_spaces(&next);
            next = normalize_cjk_punctuation(&next);
        }

        if options.cjk && next.trim().is_empty() {
            if last_blank {
                continue;
            }
            last_blank = true;
        } else {
            last_blank = false;
        }
        output.push(next);
    }

    let mut merged: Vec<String> = Vec::with_capacity(output.len());
    let mut fence: Option<&str> = None;
    for line in output {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fence = if fence.is_some() { None } else { Some(if trimmed.starts_with("```") { "```" } else { "~~~" }) };
            merged.push(line);
            continue;
        }
        let removable = fence.is_none()
            && (trimmed.contains(".....") || trimmed.contains("……"))
            && trimmed.chars().rev().take_while(|c| c.is_ascii_digit()).count() > 0;
        if removable {
            continue;
        }
        let can_join = fence.is_none()
            && !merged.is_empty()
            && !trimmed.is_empty()
            && !trimmed.starts_with('#')
            && !trimmed.starts_with('|')
            && !trimmed.starts_with('-')
            && !trimmed.starts_with('*')
            && !trimmed.starts_with('>')
            && !trimmed.starts_with('<');
        if can_join {
            let previous = merged.last().unwrap().trim_end();
            let previous_ends_open = previous
                .chars()
                .last()
                .map(|c| !matches!(c, '。' | '！' | '？' | '.' | '!' | '?' | ':' | '：' | '；' | ';' | '】' | '」' | '）' | ')' | '`'))
                .unwrap_or(false);
            let starts_cjk = trimmed.chars().next().map(is_cjk).unwrap_or(false);
            if previous_ends_open && starts_cjk && !previous.ends_with("  ") {
                let last = merged.last_mut().unwrap();
                last.push(' ');
                last.push_str(trimmed);
                continue;
            }
        }
        merged.push(line);
    }
    let mut result = merged.join("\n");
    // 原文件没有末尾换行时，不额外增加；有则保留。
    if markdown.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }
    Cleaned {
        markdown: result,
        warnings,
        table_grade,
    }
}

/// 表格质量分级只读入口：供引擎路由判断是否需要切换 xberg，不修改正文。
pub fn table_grade(markdown: &str) -> String {
    let lines: Vec<&str> = markdown.split('\n').collect();
    grade_tables(&lines).0
}

fn is_fence(trimmed: &str) -> bool {
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

fn toggle_fence<'a>(current: Option<&'a str>, trimmed: &'a str) -> Option<&'a str> {
    match current {
        None => Some(if trimmed.starts_with("```") {
            "```"
        } else {
            "~~~"
        }),
        Some(marker) if trimmed.starts_with(marker) => None,
        other => other,
    }
}

/// 只移除可明确识别的页眉页脚：页码模式，或同时在文档首尾重复的短行。
fn header_footer_lines(lines: &[&str]) -> HashSet<String> {
    let mut removal = HashSet::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    let non_empty: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, _)| index)
        .collect();
    let mut boundary = HashSet::new();
    for index in non_empty.iter().take(10) {
        boundary.insert(*index);
    }
    for index in non_empty.iter().rev().take(10) {
        boundary.insert(*index);
    }

    let mut in_fence = false;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if is_fence(trimmed) {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || trimmed.is_empty() {
            continue;
        }
        if is_pure_digits(trimmed) {
            // 纯数字只有出现在文档首尾时才当作页码，避免误删正文数字。
            if boundary.contains(&index) {
                removal.insert(trimmed.to_string());
            }
            continue;
        }
        if is_page_label(trimmed) {
            removal.insert(trimmed.to_string());
            continue;
        }
        if trimmed.chars().count() <= 80 {
            *counts.entry(trimmed.to_string()).or_insert(0) += 1;
        }
    }

    let first: HashSet<String> = lines
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .take(5)
        .map(str::to_string)
        .collect();
    let last: HashSet<String> = lines
        .iter()
        .rev()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .take(5)
        .map(str::to_string)
        .collect();
    for (line, count) in counts {
        if count >= 2 && first.contains(&line) && last.contains(&line) {
            removal.insert(line);
        }
    }
    removal
}

fn is_pure_digits(text: &str) -> bool {
    let compact = text
        .trim_matches(|c: char| c == '-' || c == '—' || c == '·' || c.is_whitespace())
        .to_string();
    !compact.is_empty() && compact.chars().all(|c| c.is_ascii_digit())
}

fn is_page_label(text: &str) -> bool {
    let lower = text.to_lowercase();
    if lower.contains("页") && lower.contains("第") {
        return true;
    }
    lower.starts_with("page ") && lower.contains(" of ")
}

fn normalize_heading(line: &str) -> String {
    let trimmed_start = line.trim_start();
    let hashes = trimmed_start.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return line.to_string();
    }
    let rest = &trimmed_start[hashes..];
    if !rest.starts_with(' ') {
        return line.to_string();
    }
    let indent = &line[..line.len() - trimmed_start.len()];
    let text = rest.trim_start();
    let normalized = normalize_numbering(text).unwrap_or_else(|| text.trim_end().to_string());
    format!("{indent}{} {normalized}", "#".repeat(hashes))
}

/// 只处理标题开头的数字编号，例如 `5 . 1 范围` → `5.1 范围`。
fn normalize_numbering(text: &str) -> Option<String> {
    let mut prefix_end = 0;
    let mut has_dot = false;
    for (index, ch) in text.char_indices() {
        if ch.is_ascii_digit() || ch == '.' || ch.is_whitespace() {
            has_dot |= ch == '.';
            prefix_end = index + ch.len_utf8();
        } else {
            break;
        }
    }
    if !has_dot || prefix_end == 0 {
        return None;
    }
    let prefix: String = text[..prefix_end]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let rest = text[prefix_end..].trim();
    Some(if rest.is_empty() {
        prefix
    } else {
        format!("{prefix} {rest}")
    })
}

/// ASCII 逗号/冒号/分号夹在中文之间时改为全角，不改动代码与表格。
fn normalize_cjk_punctuation(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    for (index, ch) in chars.iter().enumerate() {
        let prev = index.checked_sub(1).and_then(|i| chars.get(i)).copied();
        let next = chars.get(index + 1).copied();
        let replace = match ch {
            ',' => Some('，'),
            ';' => Some('；'),
            ':' => Some('：'),
            _ => None,
        };
        if let (Some(full), Some(prev), Some(next)) = (replace, prev, next) {
            if is_cjk(prev) && is_cjk(next) {
                out.push(full);
                continue;
            }
        }
        out.push(*ch);
    }
    out
}

fn is_cjk(ch: char) -> bool {
    matches!(ch,
        '\u{3400}'..='\u{4DBF}'
        | '\u{4E00}'..='\u{9FFF}'
        | '\u{F900}'..='\u{FAFF}'
    )
}

/// 删除 CJK 字符之间的空格；CJK 与 Latin 之间的空格保留。代码/URL/表格由调用方跳过。
fn collapse_cjk_spaces(line: &str) -> String {
    if line.contains('`') || line.contains("http://") || line.contains("https://") {
        return line.to_string();
    }
    let chars: Vec<char> = line.chars().collect();
    let mut output = String::with_capacity(line.len());
    for (index, ch) in chars.iter().enumerate() {
        if ch.is_whitespace() {
            let prev = index.checked_sub(1).and_then(|i| chars.get(i)).copied();
            let next = chars.get(index + 1).copied();
            if prev.map(is_cjk).unwrap_or(false) && next.map(is_cjk).unwrap_or(false) {
                continue;
            }
        }
        output.push(*ch);
    }
    output
}

/// 表格质量只影响警告等级，不修改正文。
fn grade_tables(lines: &[&str]) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let mut grade = "A".to_string();
    let mut block: Vec<&str> = Vec::new();
    let mut in_fence = false;

    let flush = |block: &mut Vec<&str>, grade: &mut String, warnings: &mut Vec<String>| {
        if block.len() < 2 {
            block.clear();
            return;
        }
        let counts: Vec<usize> = block
            .iter()
            .map(|line| line.trim().trim_matches('|').split('|').count())
            .collect();
        let min = *counts.iter().min().unwrap_or(&0);
        let max = *counts.iter().max().unwrap_or(&0);
        let block_grade = if min == max {
            "A"
        } else if max.saturating_sub(min) <= 1 {
            "B"
        } else {
            "C"
        };
        if block_grade != "A" {
            warnings.push(format!(
                "表格列数不一致（{}–{} 列），标记为 {} 级，请人工复核",
                min, max, block_grade
            ));
            let rank = |value: &str| match value {
                "C" => 2,
                "B" => 1,
                _ => 0,
            };
            if rank(block_grade) > rank(grade) {
                *grade = block_grade.to_string();
            }
        }
        block.clear();
    };

    for line in lines {
        let trimmed = line.trim();
        if is_fence(trimmed) {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            block.push(trimmed);
        } else if !block.is_empty() {
            flush(&mut block, &mut grade, &mut warnings);
        }
    }
    flush(&mut block, &mut grade, &mut warnings);
    (grade, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_on() -> CleanOptions {
        CleanOptions {
            headers: true,
            cjk: true,
            tables: true,
        }
    }

    #[test]
    fn all_off_returns_exact_input() {
        let raw = "# 5 . 1 标题\n\n\n\n正文\n";
        let cleaned = clean(
            raw,
            &CleanOptions {
                headers: false,
                cjk: false,
                tables: false,
            },
        );
        assert_eq!(cleaned.markdown, raw);
    }

    #[test]
    fn removes_page_numbers_and_normalizes_headings() {
        let raw = "# 5 . 1 范围\n\n第 1 页\n正文\n\n\n\n## 5 . 2 术语\n";
        let cleaned = clean(raw, &all_on());
        assert!(cleaned.markdown.contains("# 5.1 范围"));
        assert!(cleaned.markdown.contains("## 5.2 术语"));
        assert!(!cleaned.markdown.contains("第 1 页"));
        assert!(!cleaned.markdown.contains("\n\n\n"));
    }

    #[test]
    fn keeps_code_and_tables_and_removes_cjk_spaces() {
        let raw = "# 标题\n\n中 文 字 符\n\n```\n中 文 不 变\n```\n\n| 列 1 | 列 2 |\n| --- | --- |\n| a | b |\n";
        let cleaned = clean(raw, &all_on());
        assert!(cleaned.markdown.contains("中文字符"));
        assert!(cleaned.markdown.contains("中 文 不 变"));
        assert!(cleaned.markdown.contains("| 列 1 | 列 2 |"));
        assert_eq!(cleaned.table_grade, "A");
    }

    #[test]
    fn grades_broken_table_as_warning_only() {
        let raw = "| a | b |\n| --- | --- |\n| 1 |\n";
        let cleaned = clean(raw, &all_on());
        assert_ne!(cleaned.table_grade, "A");
        assert!(cleaned.markdown.contains("| 1 |"));
        assert!(!cleaned.warnings.is_empty());
    }
}
