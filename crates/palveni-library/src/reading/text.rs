//! Plain-text (`.txt`) and Markdown (`.md`) chapter splitter and HTML renderer.

use std::io::{self, Read};

use palveni_core::reading::{ReadingChapter, ReadingChapterSummary};

use super::archive::MAX_ENTRY_BYTES;

const TARGET_CHAPTER_CHARS: usize = 6_500; // ~18–20 KB of UTF-8 CJK/Latin prose
const MIN_SPLIT_CHARS: usize = 600;

#[derive(Clone, Debug)]
pub struct ParsedTextBook {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub chapters: Vec<TextChapterRaw>,
}

#[derive(Clone, Debug)]
pub struct TextChapterRaw {
    pub title: String,
    pub body: String,
}

pub fn read_decoded_text<R: Read>(reader: R) -> io::Result<String> {
    let mut bytes = Vec::new();
    reader
        .take(u64::try_from(MAX_ENTRY_BYTES).unwrap_or(u64::MAX) + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_ENTRY_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "text book exceeds maximum size",
        ));
    }
    Ok(decode_text_bytes(&bytes))
}

pub fn decode_text_bytes(bytes: &[u8]) -> String {
    if let Some(stripped) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(stripped).into_owned();
    }
    if let Some(stripped) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        let units: Vec<u16> = stripped
            .as_chunks::<2>()
            .0
            .iter()
            .copied()
            .map(u16::from_le_bytes)
            .collect();
        return String::from_utf16_lossy(&units);
    }
    if let Some(stripped) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let units: Vec<u16> = stripped
            .as_chunks::<2>()
            .0
            .iter()
            .copied()
            .map(u16::from_be_bytes)
            .collect();
        return String::from_utf16_lossy(&units);
    }
    String::from_utf8_lossy(bytes).into_owned()
}

pub fn parse_text_book(raw: &str, is_markdown: bool, fallback_title: &str) -> ParsedTextBook {
    let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
    let mut detected_title: Option<String> = None;

    let mut sections: Vec<TextChapterRaw> = Vec::new();
    let mut current_title: Option<String> = None;
    let mut current_body = String::new();
    let mut current_chars = 0usize;

    for line in normalized.lines() {
        let trimmed = line.trim();
        let heading = if is_markdown {
            parse_markdown_heading(trimmed)
        } else {
            parse_txt_heading(trimmed)
        };

        if let Some((level, h_title)) = heading {
            if level == 1 && detected_title.is_none() && sections.is_empty() && current_chars < 120
            {
                detected_title = Some(h_title.clone());
            }
            if !current_body.trim().is_empty()
                && (current_title.is_some() || current_chars >= MIN_SPLIT_CHARS)
            {
                let idx = sections.len() + 1;
                sections.push(TextChapterRaw {
                    title: current_title
                        .take()
                        .unwrap_or_else(|| default_chapter_title(idx, fallback_title)),
                    body: std::mem::take(&mut current_body),
                });
                current_chars = 0;
            }
            if current_title.is_none() {
                current_title = Some(h_title);
            }
            current_body.push_str(line);
            current_body.push('\n');
            current_chars += line.chars().count() + 1;
            continue;
        }

        current_body.push_str(line);
        current_body.push('\n');
        current_chars += line.chars().count() + 1;

        // Soft split long unheaded prose at paragraph breaks (~18-25 KB)
        if trimmed.is_empty() && current_chars >= TARGET_CHAPTER_CHARS {
            let idx = sections.len() + 1;
            sections.push(TextChapterRaw {
                title: current_title
                    .take()
                    .unwrap_or_else(|| default_chapter_title(idx, fallback_title)),
                body: std::mem::take(&mut current_body),
            });
            current_chars = 0;
        }
    }

    if !current_body.trim().is_empty() || sections.is_empty() {
        let idx = sections.len() + 1;
        sections.push(TextChapterRaw {
            title: current_title.unwrap_or_else(|| {
                if idx == 1 {
                    detected_title
                        .clone()
                        .unwrap_or_else(|| fallback_title.to_owned())
                } else {
                    default_chapter_title(idx, fallback_title)
                }
            }),
            body: current_body,
        });
    }

    let summary = extract_preview_summary(&normalized);
    ParsedTextBook {
        title: detected_title,
        summary,
        chapters: sections,
    }
}

pub fn chapter_summaries(book: &ParsedTextBook) -> Vec<ReadingChapterSummary> {
    book.chapters
        .iter()
        .enumerate()
        .map(|(i, c)| ReadingChapterSummary {
            index: u32::try_from(i).unwrap_or(u32::MAX),
            title: c.title.clone(),
        })
        .collect()
}

pub fn render_text_chapter(
    book: &ParsedTextBook,
    index: usize,
    is_markdown: bool,
) -> Option<ReadingChapter> {
    let ch = book.chapters.get(index)?;
    let (html, plain_text) = if is_markdown {
        render_markdown_to_html(&ch.body)
    } else {
        render_plain_to_html(&ch.body)
    };
    Some(ReadingChapter {
        index: u32::try_from(index).unwrap_or(u32::MAX),
        title: ch.title.clone(),
        html,
        plain_text,
    })
}

fn default_chapter_title(num: usize, fallback: &str) -> String {
    if num == 1 {
        format!("{fallback} · 第 1 节")
    } else {
        format!("第 {num} 节")
    }
}

fn parse_markdown_heading(trimmed: &str) -> Option<(u8, String)> {
    if let Some(rest) = trimmed.strip_prefix("# ") {
        let t = rest.trim().trim_end_matches('#').trim();
        (!t.is_empty()).then(|| (1, t.to_owned()))
    } else if let Some(rest) = trimmed.strip_prefix("## ") {
        let t = rest.trim().trim_end_matches('#').trim();
        (!t.is_empty()).then(|| (2, t.to_owned()))
    } else {
        None
    }
}

fn parse_txt_heading(trimmed: &str) -> Option<(u8, String)> {
    let char_len = trimmed.chars().count();
    if !(2..=60).contains(&char_len) {
        return None;
    }
    // Chinese chapter pattern: 第...章/回/节/卷/部/篇
    if let Some(after_di) = trimmed.strip_prefix('第') {
        let num_part: String = after_di
            .chars()
            .take_while(|c| {
                c.is_ascii_digit()
                    || matches!(
                        c,
                        '零' | '〇'
                            | '一'
                            | '二'
                            | '两'
                            | '三'
                            | '四'
                            | '五'
                            | '六'
                            | '七'
                            | '八'
                            | '九'
                            | '十'
                            | '百'
                            | '千'
                            | '万'
                            | ' '
                    )
            })
            .collect();
        let rest = &after_di[num_part.len()..];
        if !num_part.trim().is_empty()
            && rest
                .chars()
                .next()
                .is_some_and(|u| matches!(u, '章' | '回' | '节' | '卷' | '部' | '篇'))
        {
            return Some((2, trimmed.to_owned()));
        }
    }
    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("chapter ")
        && rest
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
    {
        return Some((2, trimmed.to_owned()));
    }
    if matches!(
        trimmed,
        "序" | "序章" | "序言" | "楔子" | "前言" | "尾声" | "后记"
    ) {
        return Some((2, trimmed.to_owned()));
    }
    None
}

fn extract_preview_summary(text: &str) -> Option<String> {
    let mut out = String::new();
    for line in text.lines() {
        let t = line.trim().trim_start_matches('#').trim();
        if t.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(t);
        if out.chars().count() >= 220 {
            break;
        }
    }
    if out.is_empty() {
        None
    } else {
        let truncated: String = out.chars().take(220).collect();
        Some(truncated)
    }
}

pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn render_plain_to_html(body: &str) -> (String, String) {
    let mut html = String::with_capacity(body.len() + 128);
    let mut plain = String::with_capacity(body.len());
    let mut para = Vec::new();

    let flush_para = |para: &mut Vec<String>, html: &mut String, plain: &mut String| {
        if para.is_empty() {
            return;
        }
        let joined = para.join(" ");
        para.clear();
        html.push_str("<p>");
        html.push_str(&escape_html(&joined));
        html.push_str("</p>\n");
        if !plain.is_empty() {
            plain.push_str("\n\n");
        }
        plain.push_str(&joined);
    };

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush_para(&mut para, &mut html, &mut plain);
        } else if parse_txt_heading(trimmed).is_some() && para.is_empty() {
            html.push_str("<h2>");
            html.push_str(&escape_html(trimmed));
            html.push_str("</h2>\n");
            if !plain.is_empty() {
                plain.push_str("\n\n");
            }
            plain.push_str(trimmed);
        } else {
            // Single line breaks in Chinese TXT novels usually denote new paragraphs
            if trimmed.chars().any(|c| c > '\u{2E80}') {
                flush_para(&mut para, &mut html, &mut plain);
                html.push_str("<p>");
                html.push_str(&escape_html(trimmed));
                html.push_str("</p>\n");
                if !plain.is_empty() {
                    plain.push_str("\n\n");
                }
                plain.push_str(trimmed);
            } else {
                para.push(trimmed.to_owned());
            }
        }
    }
    flush_para(&mut para, &mut html, &mut plain);
    (html, plain)
}

fn render_markdown_to_html(body: &str) -> (String, String) {
    let mut html = String::with_capacity(body.len() + 256);
    let mut plain = String::with_capacity(body.len());
    let mut in_code = false;
    let mut in_list = false;
    let mut para: Vec<String> = Vec::new();

    let flush_para = |para: &mut Vec<String>, html: &mut String, plain: &mut String| {
        if para.is_empty() {
            return;
        }
        let text = para.join(" ");
        para.clear();
        html.push_str("<p>");
        html.push_str(&render_inline_markdown(&text));
        html.push_str("</p>\n");
        if !plain.is_empty() {
            plain.push_str("\n\n");
        }
        plain.push_str(&text);
    };

    let close_list = |in_list: &mut bool, html: &mut String| {
        if *in_list {
            html.push_str("</ul>\n");
            *in_list = false;
        }
    };

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            flush_para(&mut para, &mut html, &mut plain);
            close_list(&mut in_list, &mut html);
            if in_code {
                html.push_str("</code></pre>\n");
                in_code = false;
            } else {
                html.push_str("<pre><code>");
                in_code = true;
            }
            continue;
        }
        if in_code {
            html.push_str(&escape_html(line));
            html.push('\n');
            plain.push_str(line);
            plain.push('\n');
            continue;
        }
        if trimmed.is_empty() {
            flush_para(&mut para, &mut html, &mut plain);
            close_list(&mut in_list, &mut html);
            continue;
        }
        if let Some((tag, content)) = trimmed
            .strip_prefix("### ")
            .map(|r| ("h3", r))
            .or_else(|| trimmed.strip_prefix("## ").map(|r| ("h2", r)))
            .or_else(|| trimmed.strip_prefix("# ").map(|r| ("h1", r)))
        {
            flush_para(&mut para, &mut html, &mut plain);
            close_list(&mut in_list, &mut html);
            let clean = content.trim().trim_end_matches('#').trim();
            html.push('<');
            html.push_str(tag);
            html.push('>');
            html.push_str(&render_inline_markdown(clean));
            html.push_str("</");
            html.push_str(tag);
            html.push_str(">\n");
            if !plain.is_empty() {
                plain.push_str("\n\n");
            }
            plain.push_str(clean);
            continue;
        }
        if let Some(quote) = trimmed.strip_prefix("> ") {
            flush_para(&mut para, &mut html, &mut plain);
            close_list(&mut in_list, &mut html);
            html.push_str("<blockquote><p>");
            html.push_str(&render_inline_markdown(quote.trim()));
            html.push_str("</p></blockquote>\n");
            if !plain.is_empty() {
                plain.push('\n');
            }
            plain.push_str(quote.trim());
            continue;
        }
        if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            flush_para(&mut para, &mut html, &mut plain);
            if !in_list {
                html.push_str("<ul>\n");
                in_list = true;
            }
            html.push_str("<li>");
            html.push_str(&render_inline_markdown(item.trim()));
            html.push_str("</li>\n");
            if !plain.is_empty() {
                plain.push('\n');
            }
            plain.push_str(item.trim());
            continue;
        }
        if matches!(trimmed, "---" | "***" | "___") {
            flush_para(&mut para, &mut html, &mut plain);
            close_list(&mut in_list, &mut html);
            html.push_str("<hr />\n");
            continue;
        }
        close_list(&mut in_list, &mut html);
        para.push(trimmed.to_owned());
    }

    if in_code {
        html.push_str("</code></pre>\n");
    }
    flush_para(&mut para, &mut html, &mut plain);
    close_list(&mut in_list, &mut html);
    (html, plain)
}

fn render_inline_markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '`'
            && let Some(end) = chars[i + 1..].iter().position(|&c| c == '`')
        {
            let inner: String = chars[i + 1..i + 1 + end].iter().collect();
            out.push_str("<code>");
            out.push_str(&escape_html(&inner));
            out.push_str("</code>");
            i += end + 2;
            continue;
        }
        if i + 1 < chars.len()
            && chars[i] == '*'
            && chars[i + 1] == '*'
            && let Some(end) = find_double_star(&chars[i + 2..])
        {
            let inner: String = chars[i + 2..i + 2 + end].iter().collect();
            out.push_str("<strong>");
            out.push_str(&escape_html(&inner));
            out.push_str("</strong>");
            i += end + 4;
            continue;
        }
        if chars[i] == '*'
            && let Some(end) = chars[i + 1..].iter().position(|&c| c == '*')
            && end > 0
        {
            let inner: String = chars[i + 1..i + 1 + end].iter().collect();
            out.push_str("<em>");
            out.push_str(&escape_html(&inner));
            out.push_str("</em>");
            i += end + 2;
            continue;
        }
        out.push_str(&escape_html(&chars[i].to_string()));
        i += 1;
    }
    out
}

fn find_double_star(slice: &[char]) -> Option<usize> {
    if slice.len() < 2 {
        return None;
    }
    (0..slice.len() - 1).find(|&idx| slice[idx] == '*' && slice[idx + 1] == '*')
}
