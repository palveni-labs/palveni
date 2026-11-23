//! EPUB 2 & EPUB 3 parser (`META-INF/container.xml`, OPF metadata/manifest/spine,
//! NCX/Nav TOC) and strict allowlist XHTML/HTML chapter sanitizer.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::{Read, Seek};

use palveni_core::ItemId;
use palveni_core::reading::{ReadingChapter, ReadingChapterSummary};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use super::archive::{
    ArchiveError, MAX_ENTRY_BYTES, ZipArchive, normalize_archive_path,
    resolve_relative_archive_path,
};
use super::comic::detect_image_mime;
use super::text::escape_html;

const MAX_XML_BYTES: usize = 8 * 1024 * 1024;
const MAX_CHAPTERS: usize = 4_096;

#[derive(Clone, Debug, Default)]
pub struct EpubBook {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub summary: Option<String>,
    pub publisher: Option<String>,
    pub language: Option<String>,
    pub tags: Vec<String>,
    pub series_name: Option<String>,
    pub series_index: Option<f64>,
    pub cover_path: Option<String>,
    pub spine: Vec<EpubSpineItem>,
}

#[derive(Clone, Debug)]
pub struct EpubSpineItem {
    pub path: String,
    pub title: String,
}

#[derive(Clone, Debug)]
struct ManifestItem {
    href: String,
    resolved_path: String,
    media_type: String,
    properties: String,
}

pub fn open_epub<R: Read + Seek>(reader: R) -> Result<(ZipArchive<R>, EpubBook), ArchiveError> {
    let mut zip = ZipArchive::open(reader)?;
    let opf_path = find_opf_path(&mut zip)?;
    let opf_xml = read_utf8_entry(&mut zip, &opf_path, MAX_XML_BYTES)?;
    let mut book = parse_opf(&opf_xml, &opf_path, &mut zip)?;

    // Verify cover actually exists and is a supported image inside the ZIP
    if let Some(ref cpath) = book.cover_path {
        let valid = find_zip_entry(&zip, cpath)
            .and_then(|idx| zip.read_entry(idx, MAX_ENTRY_BYTES).ok())
            .and_then(|bytes| detect_image_mime(&bytes))
            .is_some();
        if !valid {
            book.cover_path = None;
        }
    }

    Ok((zip, book))
}

pub fn epub_chapter_summaries(book: &EpubBook) -> Vec<ReadingChapterSummary> {
    book.spine
        .iter()
        .enumerate()
        .map(|(idx, item)| ReadingChapterSummary {
            index: u32::try_from(idx).unwrap_or(u32::MAX),
            title: item.title.clone(),
        })
        .collect()
}

pub fn read_epub_chapter<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    book: &EpubBook,
    index: usize,
    item_id: ItemId,
) -> Result<Option<ReadingChapter>, ArchiveError> {
    let Some(spine_item) = book.spine.get(index) else {
        return Ok(None);
    };
    let Some(entry_idx) = find_zip_entry(zip, &spine_item.path) else {
        return Err(ArchiveError::Corrupt(
            "EPUB spine chapter missing from archive",
        ));
    };
    let bytes = zip.read_entry(entry_idx, MAX_ENTRY_BYTES)?;
    let raw = String::from_utf8_lossy(&bytes);
    let (detected_title, html, plain_text) = sanitize_epub_xhtml(&raw, &spine_item.path, item_id);
    let title = if is_generic_chapter_title(&spine_item.title)
        && let Some(h) = detected_title.filter(|s| !s.is_empty())
    {
        h
    } else {
        spine_item.title.clone()
    };

    Ok(Some(ReadingChapter {
        index: u32::try_from(index).unwrap_or(u32::MAX),
        title,
        html,
        plain_text,
    }))
}

pub fn read_epub_asset<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    raw_path: &str,
) -> Result<Option<(&'static str, Vec<u8>)>, ArchiveError> {
    let Some(normalized) = normalize_archive_path(raw_path) else {
        return Ok(None);
    };
    let Some(idx) = find_zip_entry(zip, &normalized) else {
        return Ok(None);
    };
    let bytes = zip.read_entry(idx, MAX_ENTRY_BYTES)?;
    let Some(mime) = detect_image_mime(&bytes) else {
        return Ok(None);
    };
    Ok(Some((mime, bytes)))
}

fn find_zip_entry<R: Read + Seek>(zip: &ZipArchive<R>, path: &str) -> Option<usize> {
    let normalized = normalize_archive_path(path)?;
    zip.entries()
        .iter()
        .position(|e| e.name == normalized)
        .or_else(|| {
            zip.entries()
                .iter()
                .position(|e| e.name.eq_ignore_ascii_case(&normalized))
        })
}

fn read_utf8_entry<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    path: &str,
    max_bytes: usize,
) -> Result<String, ArchiveError> {
    let idx = find_zip_entry(zip, path)
        .ok_or(ArchiveError::Corrupt("required EPUB XML file not found"))?;
    let bytes = zip.read_entry(idx, max_bytes)?;
    let stripped = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
    Ok(String::from_utf8_lossy(stripped).into_owned())
}

fn find_opf_path<R: Read + Seek>(zip: &mut ZipArchive<R>) -> Result<String, ArchiveError> {
    if let Ok(container_xml) = read_utf8_entry(zip, "META-INF/container.xml", 256 * 1024)
        && let Some(full_path) = parse_container_xml(&container_xml)
        && let Some(normalized) = normalize_archive_path(&full_path)
    {
        return Ok(normalized);
    }
    zip.entries()
        .iter()
        .find(|e| e.name.to_ascii_lowercase().ends_with(".opf"))
        .map(|e| e.name.clone())
        .ok_or(ArchiveError::Corrupt("EPUB OPF package document not found"))
}

fn parse_container_xml(xml: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event().ok()? {
            Event::Start(tag) | Event::Empty(tag) => {
                if tag.local_name().as_ref().eq_ignore_ascii_case("rootfile")
                    && let Some(path) = attr(&tag, "full-path")
                {
                    return Some(path);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    None
}

fn attr(tag: &BytesStart<'_>, name: &str) -> Option<String> {
    tag.attributes()
        .flatten()
        .find(|a| a.key.local_name().as_ref().eq_ignore_ascii_case(name))
        .and_then(|a| {
            a.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()
                .map(|v| v.into_owned())
        })
}

fn parse_opf<R: Read + Seek>(
    xml: &str,
    opf_path: &str,
    zip: &mut ZipArchive<R>,
) -> Result<EpubBook, ArchiveError> {
    let mut reader = Reader::from_str(xml);
    let mut book = EpubBook::default();
    let mut manifest: HashMap<String, ManifestItem> = HashMap::new();
    let mut spine_idrefs: Vec<String> = Vec::new();
    let mut toc_ncx_id: Option<String> = None;
    let mut cover_meta_id: Option<String> = None;
    let mut guide_cover_href: Option<String> = None;
    let mut collection_ids: HashMap<String, String> = HashMap::new();
    let mut group_positions: HashMap<String, f64> = HashMap::new();

    let mut current_tag: Option<String> = None;
    let mut current_meta_prop: Option<String> = None;
    let mut current_meta_id: Option<String> = None;
    let mut current_meta_refines: Option<String> = None;
    let mut text_buf = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(tag)) => {
                let local = tag.local_name().as_ref().to_ascii_lowercase();
                text_buf.clear();
                match local.as_str() {
                    "spine" => {
                        toc_ncx_id = attr(&tag, "toc");
                    }
                    "meta" => {
                        handle_empty_or_start_meta(
                            &tag,
                            &mut book,
                            &mut cover_meta_id,
                            &mut current_meta_prop,
                            &mut current_meta_id,
                            &mut current_meta_refines,
                        );
                    }
                    "item" => {
                        record_manifest_item(&tag, opf_path, &mut manifest);
                    }
                    "itemref" => {
                        if spine_idrefs.len() < MAX_CHAPTERS
                            && let Some(idref) = attr(&tag, "idref")
                        {
                            spine_idrefs.push(idref);
                        }
                    }
                    "reference" => {
                        if attr(&tag, "type").is_some_and(|t| t.eq_ignore_ascii_case("cover"))
                            && let Some(href) = attr(&tag, "href")
                        {
                            guide_cover_href = Some(href);
                        }
                    }
                    _ => {}
                }
                current_tag = Some(local);
            }
            Ok(Event::Empty(tag)) => {
                let local = tag.local_name().as_ref().to_ascii_lowercase();
                match local.as_str() {
                    "meta" => {
                        let mut p = None;
                        let mut i = None;
                        let mut r = None;
                        handle_empty_or_start_meta(
                            &tag,
                            &mut book,
                            &mut cover_meta_id,
                            &mut p,
                            &mut i,
                            &mut r,
                        );
                    }
                    "item" => {
                        record_manifest_item(&tag, opf_path, &mut manifest);
                    }
                    "itemref" => {
                        if spine_idrefs.len() < MAX_CHAPTERS
                            && let Some(idref) = attr(&tag, "idref")
                        {
                            spine_idrefs.push(idref);
                        }
                    }
                    "reference" => {
                        if attr(&tag, "type").is_some_and(|t| t.eq_ignore_ascii_case("cover"))
                            && let Some(href) = attr(&tag, "href")
                        {
                            guide_cover_href = Some(href);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(end)) => {
                let local = end.local_name().as_ref().to_ascii_lowercase();
                let val = text_buf.trim().to_owned();
                if !val.is_empty() {
                    match local.as_str() {
                        "title" if book.title.is_none() => {
                            book.title = Some(val);
                        }
                        "creator" => {
                            if !book.authors.iter().any(|a| a == &val) && book.authors.len() < 32 {
                                book.authors.push(val);
                            }
                        }
                        "description" if book.summary.is_none() => {
                            let plain = strip_tags_to_plain(&val);
                            if !plain.is_empty() {
                                book.summary = Some(plain);
                            }
                        }
                        "publisher" if book.publisher.is_none() => {
                            book.publisher = Some(val);
                        }
                        "language" if book.language.is_none() => {
                            book.language = Some(val);
                        }
                        "subject" => {
                            for part in val.split([',', ';', '/']) {
                                let t = part.trim();
                                if !t.is_empty()
                                    && book.tags.len() < 32
                                    && !book
                                        .tags
                                        .iter()
                                        .any(|existing| existing.eq_ignore_ascii_case(t))
                                {
                                    book.tags.push(t.to_owned());
                                }
                            }
                        }
                        "meta" => {
                            if let Some(ref prop) = current_meta_prop {
                                if prop.eq_ignore_ascii_case("belongs-to-collection") {
                                    if book.series_name.is_none() {
                                        book.series_name = Some(val.clone());
                                    }
                                    if let Some(ref cid) = current_meta_id {
                                        collection_ids.insert(format!("#{cid}"), val);
                                    }
                                } else if prop.eq_ignore_ascii_case("group-position")
                                    && let Ok(pos) = val.parse::<f64>()
                                    && pos.is_finite()
                                {
                                    if let Some(ref refines) = current_meta_refines {
                                        group_positions.insert(refines.clone(), pos);
                                    }
                                    if book.series_index.is_none() {
                                        book.series_index = Some(pos);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                if local == "meta" {
                    current_meta_prop = None;
                    current_meta_id = None;
                    current_meta_refines = None;
                }
                current_tag = None;
                text_buf.clear();
            }
            Ok(Event::Text(e)) if current_tag.is_some() => {
                text_buf.push_str(&e.xml10_content());
            }
            Ok(Event::CData(e)) if current_tag.is_some() => {
                text_buf.push_str(&e.xml10_content());
            }
            Ok(Event::GeneralRef(e)) if current_tag.is_some() => {
                append_xml_entity(&e, &mut text_buf);
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    // Match EPUB 3 collection refinement if needed
    for (target, pos) in group_positions {
        if let Some(col_name) = collection_ids.get(&target)
            && book.series_name.as_deref() == Some(col_name.as_str())
        {
            book.series_index = Some(pos);
        }
    }

    // Resolve cover path
    book.cover_path = resolve_cover_path(
        opf_path,
        &manifest,
        cover_meta_id.as_deref(),
        guide_cover_href.as_deref(),
    );

    // Build TOC map (`resolved_path -> label`) from EPUB 3 Nav and/or EPUB 2 NCX
    let mut toc_map: HashMap<String, String> = HashMap::new();
    if let Some(nav_item) = manifest.values().find(|m| {
        m.properties
            .split_whitespace()
            .any(|p| p.eq_ignore_ascii_case("nav"))
    }) && let Ok(nav_xml) = read_utf8_entry(zip, &nav_item.resolved_path, MAX_XML_BYTES)
    {
        parse_epub3_nav(&nav_xml, &nav_item.resolved_path, &mut toc_map);
    }
    if toc_map.is_empty() {
        let ncx_item = toc_ncx_id
            .as_deref()
            .and_then(|id| manifest.get(id))
            .or_else(|| {
                manifest.values().find(|m| {
                    m.media_type
                        .eq_ignore_ascii_case("application/x-dtbncx+xml")
                })
            });
        if let Some(ncx) = ncx_item
            && let Ok(ncx_xml) = read_utf8_entry(zip, &ncx.resolved_path, MAX_XML_BYTES)
        {
            parse_ncx(&ncx_xml, &ncx.resolved_path, &mut toc_map);
        }
    }

    // Build ordered spine
    for (idx, idref) in spine_idrefs.iter().enumerate() {
        let Some(item) = manifest.get(idref) else {
            continue;
        };
        let mt = item.media_type.to_ascii_lowercase();
        let is_html = mt.contains("html")
            || mt.contains("xml")
            || item.href.to_ascii_lowercase().ends_with(".xhtml")
            || item.href.to_ascii_lowercase().ends_with(".html")
            || item.href.to_ascii_lowercase().ends_with(".htm");
        if !is_html || mt.contains("ncx") {
            continue;
        }
        let title = toc_map
            .get(&item.resolved_path)
            .cloned()
            .or_else(|| peek_chapter_heading(zip, &item.resolved_path))
            .unwrap_or_else(|| format!("第 {} 章", idx + 1));
        book.spine.push(EpubSpineItem {
            path: item.resolved_path.clone(),
            title,
        });
    }

    Ok(book)
}

fn handle_empty_or_start_meta(
    tag: &BytesStart<'_>,
    book: &mut EpubBook,
    cover_meta_id: &mut Option<String>,
    current_prop: &mut Option<String>,
    current_id: &mut Option<String>,
    current_refines: &mut Option<String>,
) {
    if let Some(name) = attr(tag, "name") {
        let content = attr(tag, "content").unwrap_or_default();
        let content = content.trim();
        if !content.is_empty() {
            if name.eq_ignore_ascii_case("cover") {
                *cover_meta_id = Some(content.to_owned());
            } else if name.eq_ignore_ascii_case("calibre:series") && book.series_name.is_none() {
                book.series_name = Some(content.to_owned());
            } else if name.eq_ignore_ascii_case("calibre:series_index")
                && book.series_index.is_none()
                && let Ok(idx) = content.parse::<f64>()
                && idx.is_finite()
            {
                book.series_index = Some(idx);
            }
        }
    }
    *current_prop = attr(tag, "property");
    *current_id = attr(tag, "id");
    *current_refines = attr(tag, "refines");
}

fn record_manifest_item(
    tag: &BytesStart<'_>,
    opf_path: &str,
    manifest: &mut HashMap<String, ManifestItem>,
) {
    let Some(id) = attr(tag, "id") else { return };
    let Some(href) = attr(tag, "href") else {
        return;
    };
    let Some(resolved_path) = resolve_relative_archive_path(opf_path, &href) else {
        return;
    };
    let media_type = attr(tag, "media-type").unwrap_or_default();
    let properties = attr(tag, "properties").unwrap_or_default();
    manifest.insert(
        id,
        ManifestItem {
            href,
            resolved_path,
            media_type,
            properties,
        },
    );
}

fn resolve_cover_path(
    opf_path: &str,
    manifest: &HashMap<String, ManifestItem>,
    cover_meta_id: Option<&str>,
    guide_cover_href: Option<&str>,
) -> Option<String> {
    let is_image_item = |m: &ManifestItem| -> bool {
        m.media_type.to_ascii_lowercase().starts_with("image/")
            || matches!(
                m.resolved_path
                    .rsplit_once('.')
                    .map(|(_, e)| e.to_ascii_lowercase())
                    .as_deref(),
                Some("jpg" | "jpeg" | "png" | "webp" | "gif" | "avif")
            )
    };

    // 1. EPUB 3 `properties="cover-image"`
    if let Some(m) = manifest.values().find(|m| {
        m.properties
            .split_whitespace()
            .any(|p| p.eq_ignore_ascii_case("cover-image"))
            && is_image_item(m)
    }) {
        return Some(m.resolved_path.clone());
    }

    // 2. EPUB 2 `<meta name="cover" content="..."/>`
    if let Some(cid) = cover_meta_id {
        if let Some(m) = manifest.get(cid).filter(|m| is_image_item(m)) {
            return Some(m.resolved_path.clone());
        }
        if let Some(resolved) = resolve_relative_archive_path(opf_path, cid)
            && let Some(m) = manifest
                .values()
                .find(|m| m.resolved_path == resolved && is_image_item(m))
        {
            return Some(m.resolved_path.clone());
        }
    }

    // 3. Guide cover if directly pointing to an image
    if let Some(ghref) = guide_cover_href
        && let Some(resolved) = resolve_relative_archive_path(opf_path, ghref)
        && manifest
            .values()
            .any(|m| m.resolved_path == resolved && is_image_item(m))
    {
        return Some(resolved);
    }

    // 4. Manifest image item named cover
    manifest
        .iter()
        .find(|(id, m)| {
            is_image_item(m)
                && (id.to_ascii_lowercase().contains("cover")
                    || m.href.to_ascii_lowercase().contains("cover"))
        })
        .map(|(_, m)| m.resolved_path.clone())
}

fn parse_ncx(xml: &str, ncx_path: &str, toc_map: &mut HashMap<String, String>) {
    let mut reader = Reader::from_str(xml);
    let mut in_text = false;
    let mut current_label = String::new();
    let mut label_stack: Vec<String> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(tag)) => {
                let local = tag.local_name().as_ref().to_ascii_lowercase();
                if local == "navpoint" {
                    label_stack.push(String::new());
                } else if local == "text" {
                    in_text = true;
                    current_label.clear();
                } else if local == "content"
                    && let Some(src) = attr(&tag, "src")
                    && let Some(resolved) = resolve_relative_archive_path(ncx_path, &src)
                    && let Some(label) = label_stack.last().filter(|l| !l.is_empty())
                {
                    toc_map.entry(resolved).or_insert_with(|| label.clone());
                }
            }
            Ok(Event::Empty(tag)) => {
                if tag.local_name().as_ref().eq_ignore_ascii_case("content")
                    && let Some(src) = attr(&tag, "src")
                    && let Some(resolved) = resolve_relative_archive_path(ncx_path, &src)
                    && let Some(label) = label_stack.last().filter(|l| !l.is_empty())
                {
                    toc_map.entry(resolved).or_insert_with(|| label.clone());
                }
            }
            Ok(Event::End(end)) => {
                let local = end.local_name().as_ref().to_ascii_lowercase();
                if local == "text" {
                    in_text = false;
                    let trimmed = current_label.trim().to_owned();
                    if !trimmed.is_empty()
                        && let Some(top) = label_stack.last_mut()
                        && top.is_empty()
                    {
                        *top = trimmed;
                    }
                } else if local == "navpoint" {
                    label_stack.pop();
                }
            }
            Ok(Event::Text(e)) if in_text => {
                current_label.push_str(&e.xml10_content());
            }
            Ok(Event::GeneralRef(e)) if in_text => {
                append_xml_entity(&e, &mut current_label);
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
}

fn parse_epub3_nav(xml: &str, nav_path: &str, toc_map: &mut HashMap<String, String>) {
    // Use our lenient HTML tag scanner so HTML5 entities in nav.xhtml never break TOC parsing.
    let mut current_href: Option<String> = None;
    let mut current_text = String::new();

    for token in tokenize_html(xml) {
        match token {
            HtmlToken::StartTag {
                name, attributes, ..
            } if name == "a" => {
                current_href = attributes
                    .iter()
                    .find(|(k, _)| k == "href")
                    .map(|(_, v)| v.clone());
                current_text.clear();
            }
            HtmlToken::EndTag { name } if name == "a" => {
                if let Some(href) = current_href.take() {
                    let label = collapse_ws(&decode_html_entities(&current_text));
                    if !label.is_empty()
                        && let Some(resolved) = resolve_relative_archive_path(nav_path, &href)
                    {
                        toc_map.entry(resolved).or_insert(label);
                    }
                }
                current_text.clear();
            }
            HtmlToken::Text(t) if current_href.is_some() => {
                current_text.push_str(t);
            }
            _ => {}
        }
    }
}

fn peek_chapter_heading<R: Read + Seek>(zip: &mut ZipArchive<R>, path: &str) -> Option<String> {
    let idx = find_zip_entry(zip, path)?;
    let bytes = zip.read_entry(idx, 256 * 1024).ok()?;
    let raw = String::from_utf8_lossy(&bytes);
    extract_first_heading(&raw)
}

fn is_generic_chapter_title(title: &str) -> bool {
    let lower = title.trim().to_ascii_lowercase();
    lower.is_empty()
        || lower == "untitled"
        || lower.ends_with(".xhtml")
        || lower.ends_with(".html")
        || (title.starts_with("第 ") && title.ends_with(" 章"))
}

fn append_xml_entity(e: &quick_xml::events::BytesRef<'_>, out: &mut String) {
    if let Ok(Some(ch)) = e.resolve_char_ref() {
        out.push(ch);
    } else {
        match &**e {
            "amp" => out.push('&'),
            "apos" => out.push('\''),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "quot" => out.push('"'),
            "nbsp" => out.push('\u{00A0}'),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Strict Allowlist HTML / XHTML Sanitizer
// ---------------------------------------------------------------------------

enum HtmlToken<'a> {
    StartTag {
        name: String,
        attributes: Vec<(String, String)>,
        self_closing: bool,
    },
    EndTag {
        name: String,
    },
    Text(&'a str),
}

fn tokenize_html(input: &str) -> Vec<HtmlToken<'_>> {
    let mut tokens = Vec::new();
    let bytes = input.as_bytes();
    let mut pos = 0usize;

    while pos < bytes.len() {
        let Some(rel_lt) = input[pos..].find('<') else {
            tokens.push(HtmlToken::Text(&input[pos..]));
            break;
        };
        let lt = pos + rel_lt;
        if lt > pos {
            tokens.push(HtmlToken::Text(&input[pos..lt]));
        }

        let after_lt = &input[lt + 1..];
        // Comments `<!-- ... -->`
        if let Some(comment_rest) = after_lt.strip_prefix("!--") {
            if let Some(end_comment) = comment_rest.find("-->") {
                pos = lt + 4 + end_comment + 3;
            } else {
                break;
            }
            continue;
        }
        // CDATA `<![CDATA[ ... ]]>`
        if let Some(cdata_rest) = after_lt.strip_prefix("![CDATA[") {
            if let Some(end_cdata) = cdata_rest.find("]]>") {
                tokens.push(HtmlToken::Text(&cdata_rest[..end_cdata]));
                pos = lt + 9 + end_cdata + 3;
            } else {
                break;
            }
            continue;
        }
        // Processing instructions `<?xml ... ?>` or declarations `<!DOCTYPE ...>`
        if after_lt.starts_with('?') || after_lt.starts_with('!') {
            if let Some(gt) = after_lt.find('>') {
                pos = lt + 1 + gt + 1;
            } else {
                break;
            }
            continue;
        }

        // Find matching `>` respecting attribute quotes
        let Some(tag_len) = find_tag_close(after_lt) else {
            tokens.push(HtmlToken::Text(&input[lt..]));
            break;
        };
        let tag_inner = after_lt[..tag_len].trim();
        pos = lt + 1 + tag_len + 1;

        if let Some(end_raw) = tag_inner.strip_prefix('/') {
            let raw_name = end_raw
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .trim_end_matches('/');
            let local = local_tag_name(raw_name);
            if !local.is_empty() {
                tokens.push(HtmlToken::EndTag { name: local });
            }
            continue;
        }

        let self_closing = tag_inner.ends_with('/');
        let trimmed_inner = tag_inner.trim_end_matches('/').trim();
        let (raw_name, attr_part) = split_tag_name_and_attrs(trimmed_inner);
        let local = local_tag_name(raw_name);
        if local.is_empty() {
            continue;
        }
        let attributes = parse_html_attributes(attr_part);
        tokens.push(HtmlToken::StartTag {
            name: local,
            attributes,
            self_closing,
        });

        // Raw text elements (`script` / `style`): consume directly until matching closing tag
        if !self_closing
            && matches!(tokens.last(), Some(HtmlToken::StartTag { name, .. }) if name == "script" || name == "style")
        {
            let target = match tokens.last() {
                Some(HtmlToken::StartTag { name, .. }) => name.clone(),
                _ => String::new(),
            };
            if let Some((raw_end, after_close)) = find_raw_close_tag(&input[pos..], &target) {
                let _ignored = &input[pos..pos + raw_end];
                tokens.push(HtmlToken::EndTag { name: target });
                pos += after_close;
            } else {
                tokens.push(HtmlToken::EndTag { name: target });
                break;
            }
        }
    }

    tokens
}

fn find_tag_close(s: &str) -> Option<usize> {
    let mut quote: Option<u8> = None;
    for (idx, b) in s.bytes().enumerate() {
        match quote {
            Some(q) if b == q => quote = None,
            Some(_) => {}
            None if b == b'"' || b == b'\'' => quote = Some(b),
            None if b == b'>' => return Some(idx),
            _ => {}
        }
    }
    None
}

fn find_raw_close_tag(s: &str, tag_name: &str) -> Option<(usize, usize)> {
    let lower = s.to_ascii_lowercase();
    let needle = format!("</{tag_name}");
    let mut search_from = 0usize;
    while let Some(rel) = lower[search_from..].find(&needle) {
        let start = search_from + rel;
        let rest = &lower[start + needle.len()..];
        if rest
            .bytes()
            .next()
            .is_some_and(|b| b == b'>' || b.is_ascii_whitespace() || b == b'/')
            && let Some(gt) = rest.find('>')
        {
            return Some((start, start + needle.len() + gt + 1));
        }
        search_from = start + needle.len();
    }
    None
}

fn local_tag_name(raw: &str) -> String {
    raw.rsplit(':')
        .next()
        .unwrap_or(raw)
        .trim()
        .to_ascii_lowercase()
}

fn split_tag_name_and_attrs(s: &str) -> (&str, &str) {
    for (idx, ch) in s.char_indices() {
        if ch.is_whitespace() {
            return (&s[..idx], s[idx..].trim_start());
        }
    }
    (s, "")
}

fn parse_html_attributes(s: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let name_start = i;
        while i < bytes.len()
            && !bytes[i].is_ascii_whitespace()
            && bytes[i] != b'='
            && bytes[i] != b'/'
        {
            i += 1;
        }
        if i == name_start {
            i += 1;
            continue;
        }
        let raw_key = &s[name_start..i];
        let key = raw_key.to_ascii_lowercase();

        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let val = if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let q = bytes[i];
                i += 1;
                let v_start = i;
                while i < bytes.len() && bytes[i] != q {
                    i += 1;
                }
                let raw_val = &s[v_start..i];
                if i < bytes.len() {
                    i += 1;
                }
                decode_html_entities(raw_val)
            } else {
                let v_start = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                decode_html_entities(&s[v_start..i])
            }
        } else {
            String::new()
        };
        out.push((key, val));
    }
    out
}

pub fn decode_html_entities(input: &str) -> String {
    if !input.contains('&') {
        return input.to_owned();
    }
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp + 1..];
        if let Some(semi) = after.bytes().take(12).position(|b| b == b';') {
            let entity = &after[..semi];
            if let Some(decoded) = resolve_html_entity(entity) {
                out.push(decoded);
                rest = &after[semi + 1..];
                continue;
            }
        }
        out.push('&');
        rest = after;
    }
    out.push_str(rest);
    out
}

fn resolve_html_entity(entity: &str) -> Option<char> {
    if let Some(hex_str) = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"))
    {
        return u32::from_str_radix(hex_str, 16)
            .ok()
            .and_then(char::from_u32);
    }
    if let Some(dec_str) = entity.strip_prefix('#') {
        return dec_str.parse::<u32>().ok().and_then(char::from_u32);
    }
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" | "#39" => Some('\''),
        "nbsp" => Some('\u{00A0}'),
        "mdash" => Some('—'),
        "ndash" => Some('–'),
        "hellip" => Some('…'),
        "ldquo" => Some('“'),
        "rdquo" => Some('”'),
        "lsquo" => Some('‘'),
        "rsquo" => Some('’'),
        "middot" => Some('·'),
        "copy" => Some('©'),
        _ => None,
    }
}

fn strip_tags_to_plain(raw: &str) -> String {
    let mut out = String::new();
    let mut suppressed = 0usize;
    for tok in tokenize_html(raw) {
        match tok {
            HtmlToken::StartTag {
                name, self_closing, ..
            } => {
                if is_dropped_subtree(&name) && !self_closing {
                    suppressed += 1;
                } else if is_block_tag(&name) && !out.ends_with(' ') && !out.is_empty() {
                    out.push(' ');
                }
            }
            HtmlToken::EndTag { name } => {
                if is_dropped_subtree(&name) {
                    suppressed = suppressed.saturating_sub(1);
                } else if is_block_tag(&name) && !out.ends_with(' ') && !out.is_empty() {
                    out.push(' ');
                }
            }
            HtmlToken::Text(t) if suppressed == 0 => {
                out.push_str(&decode_html_entities(t));
            }
            _ => {}
        }
    }
    collapse_ws(&out)
}

fn extract_first_heading(raw: &str) -> Option<String> {
    let mut capture_tag: Option<String> = None;
    let mut h_buf = String::new();
    let mut title_fallback: Option<String> = None;

    for tok in tokenize_html(raw) {
        match tok {
            HtmlToken::StartTag {
                name, self_closing, ..
            } if !self_closing && matches!(name.as_str(), "h1" | "h2" | "h3" | "title") => {
                capture_tag = Some(name);
                h_buf.clear();
            }
            HtmlToken::EndTag { name } => {
                if capture_tag.as_deref() == Some(name.as_str()) {
                    let clean = collapse_ws(&decode_html_entities(&h_buf));
                    capture_tag = None;
                    if !clean.is_empty() {
                        if name == "title" {
                            if title_fallback.is_none() {
                                title_fallback = Some(clean);
                            }
                        } else {
                            return Some(clean);
                        }
                    }
                }
            }
            HtmlToken::Text(t) if capture_tag.is_some() => {
                h_buf.push_str(t);
            }
            _ => {}
        }
    }
    title_fallback
}

/// Strictly sanitizes EPUB chapter XHTML/HTML:
/// - Strips `<script>`, `<style>`, `<iframe>`, `<object>`, `<embed>`, `<form>`, `<head>`, etc.
/// - Strips all `on*` event attributes, inline styles, and `javascript:` / external URLs.
/// - Rewrites package-relative `<img src="...">` and SVG `<image href="...">` to
///   `/api/v1/items/{id}/reading-asset?path=...` with `data-reading-asset`.
pub fn sanitize_epub_xhtml(
    raw: &str,
    chapter_path: &str,
    item_id: ItemId,
) -> (Option<String>, String, String) {
    let detected_heading = extract_first_heading(raw);
    let mut html = String::with_capacity(raw.len());
    let mut plain = String::with_capacity(raw.len() / 2);
    let mut open_stack: Vec<String> = Vec::new();
    let mut suppressed: Vec<String> = Vec::new();

    for token in tokenize_html(raw) {
        match token {
            HtmlToken::StartTag {
                name,
                attributes,
                self_closing,
            } => {
                if !suppressed.is_empty() {
                    if !self_closing && is_dropped_subtree(&name) {
                        suppressed.push(name);
                    }
                    continue;
                }
                if is_dropped_subtree(&name) {
                    if !self_closing {
                        suppressed.push(name);
                    }
                    continue;
                }

                // Convert `<img src="...">` or SVG `<image href="..." / xlink:href="...">`
                if name == "img" || name == "image" {
                    let raw_src = attributes
                        .iter()
                        .find(|(k, _)| matches!(k.as_str(), "src" | "href" | "xlink:href"))
                        .map(|(_, v)| v.as_str());
                    let alt = attributes
                        .iter()
                        .find(|(k, _)| k == "alt")
                        .map(|(_, v)| v.as_str())
                        .unwrap_or("");
                    if let Some(src) = raw_src
                        && is_local_relative_ref(src)
                        && let Some(resolved) = resolve_relative_archive_path(chapter_path, src)
                    {
                        let encoded_path = url_encode_query(&resolved);
                        let _ = write!(
                            html,
                            r#"<img src="/api/v1/items/{item_id}/reading-asset?path={encoded_path}" data-reading-asset="{}" alt="{}" loading="lazy" />"#,
                            escape_html(&resolved),
                            escape_html(alt)
                        );
                    }
                    continue;
                }

                if name == "br" || name == "hr" {
                    let _ = write!(html, "<{name} />");
                    if name == "br" {
                        plain.push('\n');
                    }
                    continue;
                }

                if !is_allowed_tag(&name) {
                    continue;
                }

                if is_block_tag(&name) && !plain.is_empty() && !plain.ends_with('\n') {
                    plain.push('\n');
                }

                html.push('<');
                html.push_str(&name);
                if name == "a"
                    && let Some((_, href)) = attributes.iter().find(|(k, _)| k == "href")
                    && let Some(safe_href) = sanitize_anchor_href(href)
                {
                    let _ = write!(html, r#" href="{}""#, escape_html(&safe_href));
                    if safe_href.starts_with("http://") || safe_href.starts_with("https://") {
                        html.push_str(r#" target="_blank" rel="noopener noreferrer""#);
                    }
                }
                if matches!(name.as_str(), "td" | "th") {
                    for (k, v) in &attributes {
                        if matches!(k.as_str(), "colspan" | "rowspan")
                            && let Ok(n) = v.trim().parse::<u16>()
                            && (1..=32).contains(&n)
                        {
                            let _ = write!(html, r#" {k}="{n}""#);
                        }
                    }
                }
                html.push('>');
                if !self_closing {
                    open_stack.push(name);
                } else {
                    let _ = write!(html, "</{name}>");
                }
            }
            HtmlToken::EndTag { name } => {
                if let Some(pos) = suppressed.iter().rposition(|s| s == &name) {
                    suppressed.truncate(pos);
                    continue;
                }
                if !suppressed.is_empty()
                    || !is_allowed_tag(&name)
                    || matches!(name.as_str(), "br" | "hr" | "img")
                {
                    continue;
                }
                if let Some(pos) = open_stack.iter().rposition(|open| open == &name) {
                    while open_stack.len() > pos {
                        if let Some(popped) = open_stack.pop() {
                            let _ = write!(html, "</{popped}>");
                        }
                    }
                    if is_block_tag(&name) && !plain.ends_with('\n') {
                        plain.push('\n');
                    }
                }
            }
            HtmlToken::Text(t) => {
                if !suppressed.is_empty() {
                    continue;
                }
                let decoded = decode_html_entities(t);
                html.push_str(&escape_html(&decoded));
                plain.push_str(&decoded);
            }
        }
    }

    while let Some(unclosed) = open_stack.pop() {
        let _ = write!(html, "</{unclosed}>");
    }

    let clean_plain = plain
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    (detected_heading, html.trim().to_owned(), clean_plain)
}

fn is_dropped_subtree(tag: &str) -> bool {
    matches!(
        tag,
        "head"
            | "title"
            | "script"
            | "style"
            | "iframe"
            | "frame"
            | "frameset"
            | "object"
            | "embed"
            | "applet"
            | "noscript"
            | "template"
            | "form"
            | "input"
            | "button"
            | "select"
            | "textarea"
            | "audio"
            | "video"
            | "source"
            | "canvas"
            | "foreignobject"
    )
}

fn is_allowed_tag(tag: &str) -> bool {
    matches!(
        tag,
        "p" | "div"
            | "section"
            | "article"
            | "aside"
            | "blockquote"
            | "pre"
            | "code"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "ul"
            | "ol"
            | "li"
            | "dl"
            | "dt"
            | "dd"
            | "span"
            | "em"
            | "i"
            | "strong"
            | "b"
            | "u"
            | "s"
            | "sub"
            | "sup"
            | "small"
            | "mark"
            | "q"
            | "cite"
            | "ruby"
            | "rt"
            | "rp"
            | "figure"
            | "figcaption"
            | "table"
            | "thead"
            | "tbody"
            | "tfoot"
            | "tr"
            | "th"
            | "td"
            | "a"
    )
}

fn is_block_tag(tag: &str) -> bool {
    matches!(
        tag,
        "p" | "div"
            | "section"
            | "article"
            | "blockquote"
            | "pre"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "li"
            | "dt"
            | "dd"
            | "tr"
            | "figcaption"
    )
}

fn is_local_relative_ref(url: &str) -> bool {
    let cleaned: String = url
        .chars()
        .filter(|c| !c.is_control() && !c.is_whitespace())
        .collect();
    if cleaned.is_empty() || cleaned.starts_with("//") {
        return false;
    }
    if let Some((scheme, _)) = cleaned.split_once(':')
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    {
        return false;
    }
    true
}

fn sanitize_anchor_href(href: &str) -> Option<String> {
    let trimmed = href.trim();
    if trimmed.is_empty() {
        return None;
    }
    let squashed: String = trimmed
        .chars()
        .filter(|c| !c.is_control() && !c.is_whitespace())
        .collect();
    let lower = squashed.to_ascii_lowercase();
    if lower.starts_with('#') {
        return Some(trimmed.to_owned());
    }
    if lower.starts_with("https://") || lower.starts_with("http://") || lower.starts_with("mailto:")
    {
        return Some(trimmed.to_owned());
    }
    // Fragment inside package-local link (keep `#anchor` if present, drop dangerous schemes)
    if !lower.contains(':') && !lower.starts_with("//") {
        if let Some((_, frag)) = trimmed.split_once('#') {
            return Some(format!("#{frag}"));
        }
        return Some("#".to_owned());
    }
    None
}

fn url_encode_query(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~' | b'/') {
            out.push(char::from(b));
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
