use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chunk {
    pub id: String,
    pub text: String,
    pub metadata: ChunkMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkMetadata {
    pub source: String,
    pub source_path: String,
    pub file_name: String,
    pub format: String,
    pub title: String,
    pub heading_path: Vec<String>,
    pub page: Option<u32>,
    pub hash: String,
    pub engine: String,
    pub params: Value,
    pub chars: usize,
    /// 粗略 token 估算：CJK 按 2 字符/token、其余按 4 字符/token 的均值。
    #[serde(default)]
    pub tokens: usize,
    /// OCR 专有信息只在实际使用 OCR 时出现，旧语料读取不受影响。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ocr_engine: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ocr_pages: Vec<u32>,
}

pub struct ChunkContext<'a> {
    pub source: &'a str,
    pub source_path: &'a str,
    pub file_name: &'a str,
    pub format: &'a str,
    pub hash: &'a str,
    pub engine: &'a str,
    pub params: Value,
    pub ocr_engine: Option<&'a str>,
    pub ocr_pages: &'a [u32],
}

/// 按 H1–H3 切块；表格与 fenced code 作为原子块，超长时保留整块并写警告。
pub fn chunk_markdown(
    markdown: &str,
    context: &ChunkContext<'_>,
    max_chars: usize,
    overlap: usize,
) -> (Vec<Chunk>, Vec<String>) {
    let max_chars = max_chars.clamp(500, 4000);
    let overlap = overlap.min(300).min(max_chars.saturating_sub(1));
    let mut sections: Vec<(Vec<String>, Vec<String>)> = Vec::new();
    let mut path: Vec<(usize, String)> = Vec::new();
    let mut content: Vec<String> = Vec::new();
    let mut fence: Option<&str> = None;

    for line in markdown.split('\n') {
        let trimmed = line.trim();
        if fence.is_none() {
            if let Some((level, text)) = heading(trimmed) {
                if level <= 3 {
                    if !content.iter().all(|line| line.trim().is_empty()) {
                        sections.push((
                            path.iter().map(|(_, text)| text.clone()).collect(),
                            std::mem::take(&mut content),
                        ));
                    } else {
                        content.clear();
                    }
                    while path
                        .last()
                        .map(|(current, _)| *current >= level)
                        .unwrap_or(false)
                    {
                        path.pop();
                    }
                    path.push((level, text.to_string()));
                    continue;
                }
            }
        }
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let marker = if trimmed.starts_with("```") {
                "```"
            } else {
                "~~~"
            };
            fence = if fence.is_some() { None } else { Some(marker) };
        }
        content.push(line.to_string());
    }
    if !content.iter().all(|line| line.trim().is_empty()) {
        sections.push((path.iter().map(|(_, text)| text.clone()).collect(), content));
    }

    let title = sections
        .first()
        .and_then(|(heading, _)| heading.first().cloned())
        .unwrap_or_else(|| stem(context.file_name));
    let file_hash = sha256_hex(markdown.as_bytes());
    let mut chunks = Vec::new();
    let mut warnings = Vec::new();
    let mut ordinal = 0usize;

    for (heading_path, lines) in sections {
        let text = lines.join("\n");
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        let pieces = split_text(text, max_chars, overlap, &mut warnings);
        for piece in pieces {
            let piece = piece.trim();
            if piece.is_empty() {
                continue;
            }
            let id = chunk_id(context.source, &heading_path, ordinal, piece);
            chunks.push(Chunk {
                id,
                text: piece.to_string(),
                metadata: ChunkMetadata {
                    source: context.source.to_string(),
                    source_path: context.source_path.to_string(),
                    file_name: context.file_name.to_string(),
                    format: context.format.to_string(),
                    title: title.clone(),
                    heading_path: heading_path.clone(),
                    page: None,
                    hash: if context.hash.is_empty() {
                        file_hash.clone()
                    } else {
                        context.hash.to_string()
                    },
                    engine: context.engine.to_string(),
                    params: context.params.clone(),
                    chars: piece.chars().count(),
                    tokens: (piece.chars().count() / 2).max(1),
                    ocr_engine: context.ocr_engine.map(|value| value.to_string()),
                    ocr_pages: context.ocr_pages.to_vec(),
                },
            });
            ordinal += 1;
        }
    }
    (chunks, warnings)
}

fn heading(line: &str) -> Option<(usize, &str)> {
    let level = line.chars().take_while(|c| *c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = line[level..].strip_prefix(' ')?;
    Some((level, rest.trim()))
}

fn stem(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(stem, _)| stem.to_string())
        .unwrap_or_else(|| name.to_string())
}

fn split_text(text: &str, max: usize, overlap: usize, warnings: &mut Vec<String>) -> Vec<String> {
    if text.chars().count() <= max {
        return vec![text.to_string()];
    }
    let blocks = blocks(text);
    let mut pieces = Vec::new();
    let mut current = String::new();
    for block in blocks {
        let block_len = block.chars().count();
        let is_atomic = is_atomic_block(&block);
        if block_len > max {
            if !current.trim().is_empty() {
                pieces.push(current.trim().to_string());
                current.clear();
            }
            if is_atomic {
                warnings.push(format!(
                    "有一个 {} 字符的表格或代码块超过上限，已保留整块",
                    block_len
                ));
                pieces.push(block.trim().to_string());
            } else {
                pieces.extend(hard_split(&block, max, overlap));
            }
            continue;
        }
        if current.chars().count() + block_len + 2 > max && !current.trim().is_empty() {
            pieces.push(current.trim().to_string());
            current.clear();
        }
        if !current.is_empty() {
            current.push_str("\n\n");
        }
        current.push_str(block.trim());
    }
    if !current.trim().is_empty() {
        pieces.push(current.trim().to_string());
    }
    pieces
}

fn blocks(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut fence: Option<&str> = None;
    let mut in_table = false;

    let flush = |current: &mut Vec<&str>, result: &mut Vec<String>| {
        if !current.is_empty() {
            result.push(current.join("\n"));
            current.clear();
        }
    };

    for line in text.split('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let marker = if trimmed.starts_with("```") {
                "```"
            } else {
                "~~~"
            };
            if fence.is_some() {
                current.push(line);
                fence = None;
                flush(&mut current, &mut result);
            } else {
                flush(&mut current, &mut result);
                fence = Some(marker);
                current.push(line);
            }
            continue;
        }
        if fence.is_some() {
            current.push(line);
            continue;
        }
        let is_table = trimmed.starts_with('|') && trimmed.ends_with('|');
        if is_table {
            if !in_table {
                flush(&mut current, &mut result);
                in_table = true;
            }
            current.push(line);
            continue;
        }
        if in_table {
            flush(&mut current, &mut result);
            in_table = false;
        }
        if trimmed.is_empty() {
            flush(&mut current, &mut result);
        } else {
            current.push(line);
        }
    }
    flush(&mut current, &mut result);
    result
}

fn is_atomic_block(block: &str) -> bool {
    let trimmed = block.trim();
    trimmed.starts_with("```")
        || trimmed.starts_with("~~~")
        || trimmed.lines().all(|line| {
            let line = line.trim();
            line.is_empty() || (line.starts_with('|') && line.ends_with('|'))
        })
}

fn hard_split(text: &str, max: usize, overlap: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut result = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let end = (start + max).min(chars.len());
        let piece: String = chars[start..end].iter().collect();
        if !piece.trim().is_empty() {
            result.push(piece.trim().to_string());
        }
        if end == chars.len() {
            break;
        }
        // 重叠不能停留在起点，否则会无限循环。
        start = end.saturating_sub(overlap).max(start + 1);
    }
    result
}

fn chunk_id(source: &str, path: &[String], ordinal: usize, text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hasher.update(path.join("/").as_bytes());
    hasher.update(ordinal.to_le_bytes());
    hasher.update(sha256_hex(text.as_bytes()).as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_string()
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn context<'a>() -> ChunkContext<'a> {
        ChunkContext {
            source: "mirror/a.md",
            source_path: "/tmp/a.docx",
            file_name: "a.docx",
            format: "DOCX",
            hash: "abc",
            engine: "anydoc",
            params: json!({"chunkSize": 1200, "overlap": 50}),
            ocr_engine: None,
            ocr_pages: &[],
        }
    }

    #[test]
    fn splits_by_heading_and_is_stable() {
        let markdown = "# 第一章\n\n内容一\n\n## 1.1 小节\n\n内容二\n";
        let (first, _) = chunk_markdown(markdown, &context(), 1200, 50);
        let (second, _) = chunk_markdown(markdown, &context(), 1200, 50);
        assert_eq!(first.len(), 2);
        assert_eq!(
            first.iter().map(|c| &c.id).collect::<Vec<_>>(),
            second.iter().map(|c| &c.id).collect::<Vec<_>>()
        );
        assert_eq!(first[1].metadata.heading_path, vec!["第一章", "1.1 小节"]);
    }

    #[test]
    fn keeps_table_intact_and_warns_when_too_large() {
        let mut table = String::from("| a | b |\n| --- | --- |\n");
        for index in 0..200 {
            table.push_str(&format!("| {index} | 内容{index} |\n"));
        }
        let markdown = format!("# 表\n\n{table}");
        let (chunks, warnings) = chunk_markdown(&markdown, &context(), 500, 50);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.starts_with("| a | b |"));
        assert!(!warnings.is_empty());
    }

    #[test]
    fn splits_long_paragraph_with_overlap() {
        let text: String = "字".repeat(1300);
        let markdown = format!("# 长文\n\n{text}");
        let (chunks, _) = chunk_markdown(&markdown, &context(), 600, 50);
        assert!(chunks.len() >= 2);
        assert!(chunks.iter().all(|chunk| chunk.text.chars().count() <= 600));
    }
}
