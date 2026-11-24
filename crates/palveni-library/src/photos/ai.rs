//! Pure-local Photo AI engine: 64-bit perceptual `dHash`, dominant color extraction,
//! scene/time/season derivation, bilingual CN/EN natural-language query parsing,
//! and exact + perceptual duplicate clustering.
//!
//! Zero external crates beyond workspace std/tokio/chrono; zero outbound network calls.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Duration;

use chrono::{Datelike, Timelike};
use palveni_core::photos::{
    PhotoDuplicateCluster, PhotoDuplicateItem, PhotoDuplicateKind, PhotoDuplicatesResponse,
    PhotoQueryFilter,
};
use palveni_core::{ItemId, PhotoPersonId, clock};
use palveni_db::plugins_photo_ai::{PhotoAiCandidateRow, PhotoAiTagUpsert, UnanalyzedPhotoRow};
use tokio::process::Command;

const FFMPEG_TIMEOUT: Duration = Duration::from_secs(4);
const MAX_FALLBACK_READ_BYTES: usize = 8 * 1024 * 1024;
const RGB_GRID_BYTES: usize = 9 * 8 * 3; // 216

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PixelAnalysis {
    pub dhash: i64,
    pub dominant_color: String,
    pub dominant_hex: String,
}

/// Computes the Hamming distance (`0..=64`) between two 64-bit perceptual dHash values.
pub fn hamming_distance(a: i64, b: i64) -> u32 {
    #[allow(clippy::cast_sign_loss)]
    let diff = (a as u64) ^ (b as u64);
    diff.count_ones()
}

/// Computes a 64-bit horizontal difference hash (`dHash`) from a 9x8 RGB24 buffer.
pub fn compute_dhash(rgb: &[u8; RGB_GRID_BYTES]) -> i64 {
    let mut hash: u64 = 0;
    for y in 0..8usize {
        for x in 0..8usize {
            let left_idx = (y * 9 + x) * 3;
            let right_idx = (y * 9 + (x + 1)) * 3;
            let luma_l = u32::from(rgb[left_idx]) * 299
                + u32::from(rgb[left_idx + 1]) * 587
                + u32::from(rgb[left_idx + 2]) * 114;
            let luma_r = u32::from(rgb[right_idx]) * 299
                + u32::from(rgb[right_idx + 1]) * 587
                + u32::from(rgb[right_idx + 2]) * 114;
            if luma_l > luma_r {
                hash |= 1u64 << (y * 8 + x);
            }
        }
    }
    #[allow(clippy::cast_possible_wrap)]
    let signed = hash as i64;
    signed
}

/// Maps an average/dominant RGB triple to `#RRGGBB` and a semantic color bucket:
/// `red | orange | yellow | green | blue | warm | dark | bright`.
pub fn classify_rgb_color(r: u8, g: u8, b: u8) -> (String, String) {
    let hex = format!("#{r:02X}{g:02X}{b:02X}");
    let rf = f64::from(r) / 255.0;
    let gf = f64::from(g) / 255.0;
    let bf = f64::from(b) / 255.0;

    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;
    let l = f64::midpoint(max, min);

    let s = if delta < 1e-6 {
        0.0
    } else {
        delta / (1.0 - (2.0 * l - 1.0).abs())
    };

    let h = if delta < 1e-6 {
        0.0
    } else if (max - rf).abs() < 1e-6 {
        60.0 * (((gf - bf) / delta).rem_euclid(6.0))
    } else if (max - gf).abs() < 1e-6 {
        60.0 * (((bf - rf) / delta) + 2.0)
    } else {
        60.0 * (((rf - gf) / delta) + 4.0)
    };

    let bucket = if l < 0.20 {
        "dark"
    } else if l > 0.85 && s < 0.25 {
        "bright"
    } else if s < 0.15 {
        if l < 0.36 {
            "dark"
        } else if l > 0.68 {
            "bright"
        } else if i16::from(r) >= i16::from(b) + 8 {
            "warm"
        } else if l < 0.50 {
            "dark"
        } else {
            "bright"
        }
    } else if s < 0.34 && (h < 58.0 || h >= 335.0) {
        "warm"
    } else if !(18.0..340.0).contains(&h) {
        "red"
    } else if h < 45.0 {
        "orange"
    } else if h < 72.0 {
        "yellow"
    } else if h < 170.0 {
        "green"
    } else if h < 265.0 {
        "blue"
    } else if r >= b {
        "red"
    } else {
        "warm"
    };

    (bucket.to_owned(), hex)
}

/// Computes `dHash`, `dominant_color`, and `dominant_hex` directly from 9x8 RGB24 pixels.
pub fn analyze_rgb_9x8(rgb: &[u8; RGB_GRID_BYTES]) -> PixelAnalysis {
    let dhash = compute_dhash(rgb);
    let mut sum_r: u32 = 0;
    let mut sum_g: u32 = 0;
    let mut sum_b: u32 = 0;
    let pixels = 72u32;
    for chunk in rgb.as_chunks::<3>().0 {
        sum_r += u32::from(chunk[0]);
        sum_g += u32::from(chunk[1]);
        sum_b += u32::from(chunk[2]);
    }
    #[allow(clippy::cast_possible_truncation)]
    let avg_r = (sum_r / pixels) as u8;
    #[allow(clippy::cast_possible_truncation)]
    let avg_g = (sum_g / pixels) as u8;
    #[allow(clippy::cast_possible_truncation)]
    let avg_b = (sum_b / pixels) as u8;
    let (dominant_color, dominant_hex) = classify_rgb_color(avg_r, avg_g, avg_b);
    PixelAnalysis {
        dhash,
        dominant_color,
        dominant_hex,
    }
}

/// Attempts to decode and scale an image to 9x8 RGB24 via local `ffmpeg`.
async fn extract_rgb_via_ffmpeg(path: &Path) -> Option<[u8; RGB_GRID_BYTES]> {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-v", "error", "-i"])
        .arg(path)
        .args([
            "-frames:v",
            "1",
            "-vf",
            "scale=9:8",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "pipe:1",
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);

    let output = tokio::time::timeout(FFMPEG_TIMEOUT, cmd.output())
        .await
        .ok()?
        .ok()?;
    if !output.status.success() || output.stdout.len() != RGB_GRID_BYTES {
        return None;
    }
    let mut buf = [0u8; RGB_GRID_BYTES];
    buf.copy_from_slice(&output.stdout);
    Some(buf)
}

/// Parses an uncompressed binary (`P6`) or ASCII (`P3`) PPM image and downsamples to 9x8 RGB24.
fn parse_ppm_to_9x8(bytes: &[u8]) -> Option<[u8; RGB_GRID_BYTES]> {
    if bytes.len() < 7 || bytes[0] != b'P' || (bytes[1] != b'6' && bytes[1] != b'3') {
        return None;
    }
    let is_binary = bytes[1] == b'6';
    let mut pos = 2usize;

    let next_token = |data: &[u8], idx: &mut usize| -> Option<usize> {
        while *idx < data.len() {
            let b = data[*idx];
            if b.is_ascii_whitespace() {
                *idx += 1;
            } else if b == b'#' {
                while *idx < data.len() && data[*idx] != b'\n' {
                    *idx += 1;
                }
            } else {
                break;
            }
        }
        let start = *idx;
        while *idx < data.len() && !data[*idx].is_ascii_whitespace() && data[*idx] != b'#' {
            *idx += 1;
        }
        if *idx == start {
            return None;
        }
        std::str::from_utf8(&data[start..*idx]).ok()?.parse().ok()
    };

    let width = next_token(bytes, &mut pos)?;
    let height = next_token(bytes, &mut pos)?;
    let maxval = next_token(bytes, &mut pos)?;
    if width == 0 || height == 0 || width > 32_768 || height > 32_768 || maxval == 0 || maxval > 255
    {
        return None;
    }

    if is_binary {
        // Single whitespace delimiter before binary raster
        if pos >= bytes.len() || !bytes[pos].is_ascii_whitespace() {
            return None;
        }
        pos += 1;
        let expected = width.checked_mul(height)?.checked_mul(3)?;
        let raster = bytes.get(pos..pos.checked_add(expected)?)?;
        Some(downsample_rgb_grid(width, height, |x, y| {
            let idx = (y * width + x) * 3;
            (raster[idx], raster[idx + 1], raster[idx + 2])
        }))
    } else {
        let mut vals = Vec::with_capacity(width.saturating_mul(height).saturating_mul(3));
        while vals.len() < width * height * 3 {
            let v = next_token(bytes, &mut pos)?;
            #[allow(clippy::cast_possible_truncation)]
            vals.push(((v * 255) / maxval).min(255) as u8);
        }
        Some(downsample_rgb_grid(width, height, |x, y| {
            let idx = (y * width + x) * 3;
            (vals[idx], vals[idx + 1], vals[idx + 2])
        }))
    }
}

/// Parses an uncompressed 24-bit or 32-bit BMP (`BM`, `BI_RGB`) and downsamples to 9x8 RGB24.
fn parse_bmp_to_9x8(bytes: &[u8]) -> Option<[u8; RGB_GRID_BYTES]> {
    if bytes.len() < 54 || &bytes[0..2] != b"BM" {
        return None;
    }
    let pixel_offset = u32::from_le_bytes(bytes[10..14].try_into().ok()?) as usize;
    let header_size = u32::from_le_bytes(bytes[14..18].try_into().ok()?);
    if header_size < 40 {
        return None;
    }
    let raw_w = i32::from_le_bytes(bytes[18..22].try_into().ok()?);
    let raw_h = i32::from_le_bytes(bytes[22..26].try_into().ok()?);
    let bpp = u16::from_le_bytes(bytes[28..30].try_into().ok()?);
    let compression = u32::from_le_bytes(bytes[30..34].try_into().ok()?);
    if compression != 0 || (bpp != 24 && bpp != 32) || raw_w <= 0 || raw_h == 0 {
        return None;
    }
    let top_down = raw_h < 0;
    let width = usize::try_from(raw_w).ok()?;
    let height = usize::try_from(raw_h.unsigned_abs()).ok()?;
    if width > 32_768 || height > 32_768 {
        return None;
    }
    let bytes_per_pixel = usize::from(bpp / 8);
    let row_stride = (width.checked_mul(bytes_per_pixel)?).div_ceil(4) * 4;
    let needed = pixel_offset.checked_add(row_stride.checked_mul(height)?)?;
    if bytes.len() < needed {
        return None;
    }

    Some(downsample_rgb_grid(width, height, |x, y| {
        let src_y = if top_down { y } else { height - 1 - y };
        let idx = pixel_offset + src_y * row_stride + x * bytes_per_pixel;
        // BMP stores channels in BGR(A) order
        (bytes[idx + 2], bytes[idx + 1], bytes[idx])
    }))
}

fn downsample_rgb_grid(
    width: usize,
    height: usize,
    sample: impl Fn(usize, usize) -> (u8, u8, u8),
) -> [u8; RGB_GRID_BYTES] {
    let mut out = [0u8; RGB_GRID_BYTES];
    for gy in 0..8usize {
        let y0 = (gy * height) / 8;
        let y1 = (((gy + 1) * height) / 8).max(y0 + 1).min(height);
        for gx in 0..9usize {
            let x0 = (gx * width) / 9;
            let x1 = (((gx + 1) * width) / 9).max(x0 + 1).min(width);
            let mut sr: u64 = 0;
            let mut sg: u64 = 0;
            let mut sb: u64 = 0;
            let mut count: u64 = 0;
            for y in y0..y1 {
                for x in x0..x1 {
                    let (r, g, b) = sample(x, y);
                    sr += u64::from(r);
                    sg += u64::from(g);
                    sb += u64::from(b);
                    count += 1;
                }
            }
            let idx = (gy * 9 + gx) * 3;
            let denom = count.max(1);
            #[allow(clippy::cast_possible_truncation)]
            {
                out[idx] = (sr / denom) as u8;
                out[idx + 1] = (sg / denom) as u8;
                out[idx + 2] = (sb / denom) as u8;
            }
        }
    }
    out
}

/// Deterministic pure-Rust fallback: parses PPM/BMP directly when possible,
/// otherwise samples payload bytes into a repeatable 9x8 RGB grid.
pub fn extract_rgb_9x8_fallback(bytes: &[u8]) -> [u8; RGB_GRID_BYTES] {
    if let Some(ppm) = parse_ppm_to_9x8(bytes) {
        return ppm;
    }
    if let Some(bmp) = parse_bmp_to_9x8(bytes) {
        return bmp;
    }
    let mut out = [0u8; RGB_GRID_BYTES];
    if bytes.is_empty() {
        return out;
    }
    let len = bytes.len();
    for (idx, slot) in out.iter_mut().enumerate() {
        let pos = (idx.saturating_mul(len)) / RGB_GRID_BYTES;
        let b = bytes[pos % len];
        let b2 = bytes[(pos + 1) % len];
        *slot = if len >= RGB_GRID_BYTES {
            b
        } else {
            b.wrapping_add(b2.rotate_left(1))
        };
    }
    out
}

/// Analyzes a photo's pixels via `ffmpeg` first, falling back gracefully to pure-Rust extraction.
pub async fn analyze_photo_pixels(path: Option<&Path>, raw_bytes: Option<&[u8]>) -> PixelAnalysis {
    if let Some(p) = path
        && p.is_file()
        && let Some(rgb) = extract_rgb_via_ffmpeg(p).await
    {
        return analyze_rgb_9x8(&rgb);
    }

    if let Some(bytes) = raw_bytes {
        let rgb = extract_rgb_9x8_fallback(bytes);
        return analyze_rgb_9x8(&rgb);
    }

    if let Some(p) = path
        && let Ok(bytes) = read_capped_bytes(p).await
    {
        let rgb = extract_rgb_9x8_fallback(&bytes);
        return analyze_rgb_9x8(&rgb);
    }

    // Even if a file is temporarily missing on disk, return a deterministic neutral tag
    analyze_rgb_9x8(&[128u8; RGB_GRID_BYTES])
}

async fn read_capped_bytes(path: &Path) -> std::io::Result<Vec<u8>> {
    use tokio::io::AsyncReadExt;
    let file = tokio::fs::File::open(path).await?;
    let mut buf = Vec::new();
    let mut limited = file.take(MAX_FALLBACK_READ_BYTES as u64);
    limited.read_to_end(&mut buf).await?;
    Ok(buf)
}

/// Derives `time_of_day` (`morning | afternoon | sunset | night`) from Unix timestamp ms.
pub fn derive_time_of_day(timestamp_ms: i64) -> &'static str {
    let dt = clock::from_millis(timestamp_ms).unwrap_or_else(clock::now);
    match dt.hour() {
        5..=11 => "morning",
        12..=16 => "afternoon",
        17..=19 => "sunset",
        _ => "night",
    }
}

/// Derives `season` (`spring | summer | autumn | winter`) from Unix timestamp ms.
pub fn derive_season(timestamp_ms: i64) -> &'static str {
    let dt = clock::from_millis(timestamp_ms).unwrap_or_else(clock::now);
    match dt.month() {
        3..=5 => "spring",
        6..=8 => "summer",
        9..=11 => "autumn",
        _ => "winter",
    }
}

/// Computes effective orientation (`portrait` or `landscape`) accounting for EXIF rotation.
pub fn derive_orientation(
    width: Option<i32>,
    height: Option<i32>,
    exif_orientation: i32,
) -> Option<&'static str> {
    let (w, h) = (width?, height?);
    if w <= 0 || h <= 0 {
        return None;
    }
    let (eff_w, eff_h) = if matches!(exif_orientation, 5..=8) {
        (h, w)
    } else {
        (w, h)
    };
    if eff_h > eff_w {
        Some("portrait")
    } else if eff_w > eff_h {
        Some("landscape")
    } else {
        None
    }
}

/// Extracts a normalized camera brand tag (`sony | apple | canon | nikon | fuji | huawei | xiaomi | leica | ricoh`).
pub fn normalize_camera_brand(make: Option<&str>, model: Option<&str>) -> Option<&'static str> {
    let combined = format!(
        "{} {}",
        make.unwrap_or_default().to_ascii_lowercase(),
        model.unwrap_or_default().to_ascii_lowercase()
    );
    if combined.contains("sony") || combined.contains("ilce") {
        Some("sony")
    } else if combined.contains("apple") || combined.contains("iphone") || combined.contains("ipad")
    {
        Some("apple")
    } else if combined.contains("canon") || combined.contains("eos") {
        Some("canon")
    } else if combined.contains("nikon") {
        Some("nikon")
    } else if combined.contains("fuji") || combined.contains("fujifilm") {
        Some("fuji")
    } else if combined.contains("huawei") {
        Some("huawei")
    } else if combined.contains("xiaomi") || combined.contains("redmi") {
        Some("xiaomi")
    } else if combined.contains("leica") {
        Some("leica")
    } else if combined.contains("ricoh") {
        Some("ricoh")
    } else {
        None
    }
}

/// Derives deduplicated scene tags from pixel analysis + photo metadata.
#[allow(clippy::too_many_arguments)]
pub fn derive_scene_tags(
    width: Option<i32>,
    height: Option<i32>,
    orientation: i32,
    has_gps: bool,
    camera_make: Option<&str>,
    camera_model: Option<&str>,
    time_of_day: &str,
    season: &str,
    dominant_color: &str,
) -> Vec<String> {
    let mut tags = Vec::new();
    let mut push = |tag: &str| {
        if !tag.is_empty() && !tags.iter().any(|t| t == tag) {
            tags.push(tag.to_owned());
        }
    };
    if let Some(orient) = derive_orientation(width, height, orientation) {
        push(orient);
    }
    if has_gps {
        push("geotagged");
    }
    if let Some(brand) = normalize_camera_brand(camera_make, camera_model) {
        push(brand);
    }
    push(time_of_day);
    push(season);
    push(dominant_color);
    tags
}

/// Runs complete local AI analysis on an unanalyzed photo row.
pub async fn analyze_unanalyzed_photo(row: &UnanalyzedPhotoRow) -> PhotoAiTagUpsert {
    let pixels = analyze_photo_pixels(Some(Path::new(&row.path)), None).await;
    let timestamp_ms = row
        .taken_at
        .or(row.modified_at)
        .unwrap_or_else(|| clock::to_millis(clock::now()));
    let time_of_day = derive_time_of_day(timestamp_ms);
    let season = derive_season(timestamp_ms);
    let has_gps = row.latitude.is_some() && row.longitude.is_some();
    let scene_tags = derive_scene_tags(
        row.width,
        row.height,
        row.orientation,
        has_gps,
        row.camera_make.as_deref(),
        row.camera_model.as_deref(),
        time_of_day,
        season,
        &pixels.dominant_color,
    );

    PhotoAiTagUpsert {
        item_id: row.item_id,
        dhash: Some(pixels.dhash),
        dominant_color: Some(pixels.dominant_color),
        dominant_hex: Some(pixels.dominant_hex),
        scene_tags,
        time_of_day: Some(time_of_day.to_owned()),
        season: Some(season.to_owned()),
        analyzed_at: clock::to_millis(clock::now()),
    }
}

// ---------------------------------------------------------------------------
// Bilingual CN/EN Natural-Language Query Parser
// ---------------------------------------------------------------------------

fn is_ascii_word(s: &str) -> bool {
    s.bytes().all(|b| b.is_ascii())
}

fn is_ascii_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Removes all occurrences of `term` from `text` (using ASCII word boundaries for ASCII terms).
/// Returns `true` if at least one match was found and removed.
fn strip_term(text: &mut String, term: &str) -> bool {
    if term.is_empty() {
        return false;
    }
    let mut found = false;
    if is_ascii_word(term) {
        let mut cursor = 0usize;
        let mut out = String::with_capacity(text.len());
        let bytes = text.as_bytes();
        let tlen = term.len();
        while cursor < bytes.len() {
            if text[cursor..].starts_with(term) {
                let left_ok = cursor == 0 || !is_ascii_word_char(bytes[cursor - 1]);
                let right_ok =
                    cursor + tlen == bytes.len() || !is_ascii_word_char(bytes[cursor + tlen]);
                if left_ok && right_ok {
                    found = true;
                    out.push(' ');
                    cursor += tlen;
                    continue;
                }
            }
            let ch = text[cursor..].chars().next().unwrap_or(' ');
            out.push(ch);
            cursor += ch.len_utf8();
        }
        if found {
            *text = out;
        }
    } else if text.contains(term) {
        found = true;
        *text = text.replace(term, " ");
    }
    found
}

fn match_category(text: &mut String, rules: &[(&str, &[&str])]) -> Option<String> {
    for &(canonical, synonyms) in rules {
        let mut matched = false;
        for &syn in synonyms {
            if strip_term(text, syn) {
                matched = true;
            }
        }
        if matched {
            return Some(canonical.to_owned());
        }
    }
    None
}

fn extract_year(text: &mut String, now_year: i32) -> Option<i32> {
    if strip_term(text, "去年") || strip_term(text, "last year") {
        return Some(now_year - 1);
    }
    if strip_term(text, "今年") || strip_term(text, "this year") {
        return Some(now_year);
    }
    if strip_term(text, "前年") {
        return Some(now_year - 2);
    }

    // Look for a 4-digit year (1990..=2035), optionally followed by '年'
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    for i in 0..chars.len() {
        if i + 4 > chars.len() {
            break;
        }
        let left_ok = i == 0 || !chars[i - 1].1.is_ascii_digit();
        let four_digits = (0..4).all(|k| chars[i + k].1.is_ascii_digit());
        if !left_ok || !four_digits {
            continue;
        }
        let start = chars[i].0;
        let end_digits = if i + 4 < chars.len() {
            chars[i + 4].0
        } else {
            text.len()
        };
        let right_digit = i + 4 < chars.len() && chars[i + 4].1.is_ascii_digit();
        if right_digit {
            continue;
        }
        if let Ok(year) = text[start..end_digits].parse::<i32>()
            && (1990..=2035).contains(&year)
        {
            let mut end = end_digits;
            if text[end..].starts_with('年') {
                end += '年'.len_utf8();
            }
            text.replace_range(start..end, " ");
            return Some(year);
        }
    }
    None
}

fn extract_month(text: &mut String) -> Option<u32> {
    // Chinese numeric months: match 12..1 longest-first so "12月" precedes "2月"
    const CN_MONTHS: &[(u32, &[&str])] = &[
        (12, &["12月", "十二月"]),
        (11, &["11月", "十一月"]),
        (10, &["10月", "十月"]),
        (9, &["09月", "9月", "九月"]),
        (8, &["08月", "8月", "八月"]),
        (7, &["07月", "7月", "七月"]),
        (6, &["06月", "6月", "六月"]),
        (5, &["05月", "5月", "五月"]),
        (4, &["04月", "4月", "四月"]),
        (3, &["03月", "3月", "三月"]),
        (2, &["02月", "2月", "二月"]),
        (1, &["01月", "1月", "一月"]),
    ];
    for &(month, terms) in CN_MONTHS {
        for &term in terms {
            if strip_term(text, term) {
                return Some(month);
            }
        }
    }

    const EN_MONTHS: &[(u32, &[&str])] = &[
        (1, &["january", "jan"]),
        (2, &["february", "feb"]),
        (3, &["march", "mar"]),
        (4, &["april", "apr"]),
        (5, &["may"]),
        (6, &["june", "jun"]),
        (7, &["july", "jul"]),
        (8, &["august", "aug"]),
        (9, &["september", "sept", "sep"]),
        (10, &["october", "oct"]),
        (11, &["november", "nov"]),
        (12, &["december", "dec"]),
    ];
    for &(month, terms) in EN_MONTHS {
        for &term in terms {
            if strip_term(text, term) {
                return Some(month);
            }
        }
    }
    None
}

/// Parses a bilingual CN/EN natural-language photo search query into structured filters + keywords.
pub fn parse_photo_query(q: &str, now_year: i32) -> PhotoQueryFilter {
    let mut text = q.trim().to_lowercase();
    if text.is_empty() {
        return PhotoQueryFilter::default();
    }

    let year = extract_year(&mut text, now_year);
    let month = extract_month(&mut text);

    const SEASONS: &[(&str, &[&str])] = &[
        ("spring", &["春季", "春天", "春", "spring"]),
        ("summer", &["夏季", "夏天", "夏", "summer"]),
        ("autumn", &["秋季", "秋天", "秋", "autumn", "fall"]),
        ("winter", &["冬季", "冬天", "冬", "winter"]),
    ];
    let season = match_category(&mut text, SEASONS);

    const TIMES: &[(&str, &[&str])] = &[
        (
            "morning",
            &[
                "清晨", "早上", "早晨", "上午", "日出", "morning", "sunrise", "dawn",
            ],
        ),
        ("afternoon", &["下午", "午后", "中午", "afternoon", "noon"]),
        (
            "sunset",
            &[
                "日落",
                "黄昏",
                "夕阳",
                "傍晚",
                "晚霞",
                "落日",
                "golden hour",
                "sunset",
                "dusk",
            ],
        ),
        (
            "night",
            &[
                "夜景", "夜晚", "晚上", "夜间", "深夜", "黑夜", "夜", "night",
            ],
        ),
    ];
    let time_of_day = match_category(&mut text, TIMES);

    const COLORS: &[(&str, &[&str])] = &[
        (
            "blue",
            &[
                "蓝色", "蓝天", "天空", "海洋", "大海", "海边", "蓝", "海", "blue", "sky", "ocean",
                "sea",
            ],
        ),
        ("red", &["红色", "红", "red"]),
        ("orange", &["橙色", "橘色", "橙", "橘", "orange"]),
        (
            "green",
            &[
                "绿色", "草地", "森林", "树木", "植物", "绿", "草", "树", "green", "grass", "tree",
                "forest",
            ],
        ),
        ("warm", &["暖色", "暖调", "暖", "warm"]),
        ("yellow", &["黄色", "金色", "黄", "yellow", "gold"]),
        (
            "dark",
            &["暗调", "深色", "黑色", "暗", "黑", "dark", "black"],
        ),
        (
            "bright",
            &["明亮", "白色", "亮色", "亮", "白", "bright", "white"],
        ),
    ];
    let color = match_category(&mut text, COLORS);

    const ORIENTATIONS: &[(&str, &[&str])] = &[
        (
            "portrait",
            &["竖拍", "竖图", "竖屏", "portrait", "vertical"],
        ),
        (
            "landscape",
            &["横拍", "横图", "横屏", "landscape", "horizontal"],
        ),
    ];
    let orientation = match_category(&mut text, ORIENTATIONS);

    const CAMERAS: &[(&str, &[&str])] = &[
        ("sony", &["索尼", "sony"]),
        ("apple", &["苹果", "iphone", "apple"]),
        ("canon", &["佳能", "canon"]),
        ("nikon", &["尼康", "nikon"]),
        ("fuji", &["富士", "fujifilm", "fuji"]),
        ("huawei", &["华为", "huawei"]),
        ("xiaomi", &["小米", "xiaomi"]),
        ("leica", &["徕卡", "leica"]),
        ("ricoh", &["理光", "ricoh"]),
    ];
    let camera_brand = match_category(&mut text, CAMERAS);

    let geotagged = if strip_term(&mut text, "带定位")
        || strip_term(&mut text, "有定位")
        || strip_term(&mut text, "有坐标")
        || strip_term(&mut text, "定位")
        || strip_term(&mut text, "geotagged")
        || strip_term(&mut text, "gps")
    {
        Some(true)
    } else {
        None
    };

    // Strip common Chinese/English filler words
    for stop in [
        "拍的", "拍摄", "照片", "图片", "相片", "色调", "风格", "的", "在", "里", "中", "和", "与",
    ] {
        strip_term(&mut text, stop);
    }

    const EN_STOPS: &[&str] = &[
        "in", "of", "the", "with", "from", "a", "an", "at", "on", "and", "photo", "photos",
        "picture", "pictures", "shot", "shots",
    ];
    let mut keywords = Vec::new();
    for raw in text.split(|c: char| {
        c.is_whitespace() || matches!(c, ',' | '，' | '、' | ';' | '；' | '|' | '。')
    }) {
        let tok = raw.trim();
        if tok.is_empty() || EN_STOPS.contains(&tok) {
            continue;
        }
        if !keywords.iter().any(|k| k == tok) {
            keywords.push(tok.to_owned());
        }
    }

    PhotoQueryFilter {
        year,
        month,
        season,
        time_of_day,
        color,
        orientation,
        camera_brand,
        geotagged,
        keywords,
    }
}

fn color_matches(filter_color: &str, dominant: Option<&str>, scene_tags: &[String]) -> bool {
    let check = |value: &str| match filter_color {
        "warm" => matches!(value, "warm" | "orange" | "yellow"),
        "yellow" => matches!(value, "yellow" | "orange" | "warm"),
        "orange" => matches!(value, "orange" | "warm"),
        other => value.eq_ignore_ascii_case(other),
    };
    if dominant.is_some_and(check) {
        return true;
    }
    scene_tags.iter().any(|tag| check(tag))
}

/// Evaluates whether a candidate photo row satisfies a parsed query and optional person filter.
pub fn matches_photo_query(
    row: &PhotoAiCandidateRow,
    filter: &PhotoQueryFilter,
    person_id_filter: Option<PhotoPersonId>,
) -> bool {
    if let Some(pid) = person_id_filter
        && !row.people.iter().any(|p| p.id == pid)
    {
        return false;
    }

    if let Some(year) = filter.year
        && row.taken_at.year() != year
    {
        return false;
    }

    if let Some(month) = filter.month
        && row.taken_at.month() != month
    {
        return false;
    }

    if let Some(ref season) = filter.season {
        let actual = row
            .season
            .as_deref()
            .unwrap_or_else(|| derive_season(row.taken_at_ms));
        if !actual.eq_ignore_ascii_case(season) {
            return false;
        }
    }

    if let Some(ref tod) = filter.time_of_day {
        let actual = row
            .time_of_day
            .as_deref()
            .unwrap_or_else(|| derive_time_of_day(row.taken_at_ms));
        if !actual.eq_ignore_ascii_case(tod) {
            return false;
        }
    }

    if let Some(ref color) = filter.color
        && !color_matches(color, row.dominant_color.as_deref(), &row.scene_tags)
    {
        return false;
    }

    if let Some(ref orient) = filter.orientation {
        let actual = derive_orientation(row.width, row.height, row.orientation);
        let tag_match = row
            .scene_tags
            .iter()
            .any(|t| t.eq_ignore_ascii_case(orient));
        if actual != Some(orient.as_str()) && !tag_match {
            return false;
        }
    }

    if let Some(ref brand) = filter.camera_brand {
        let actual =
            normalize_camera_brand(row.camera_make.as_deref(), row.camera_model.as_deref());
        let raw_match = row
            .camera_make
            .as_deref()
            .is_some_and(|m| m.to_ascii_lowercase().contains(brand.as_str()))
            || row
                .camera_model
                .as_deref()
                .is_some_and(|m| m.to_ascii_lowercase().contains(brand.as_str()));
        if actual != Some(brand.as_str()) && !raw_match {
            return false;
        }
    }

    if let Some(geo) = filter.geotagged {
        let has_gps = row.latitude.is_some() && row.longitude.is_some();
        if has_gps != geo {
            return false;
        }
    }

    for kw in &filter.keywords {
        let kw_lower = kw.to_lowercase();
        let hit = row.title.to_lowercase().contains(&kw_lower)
            || row.file_name.to_lowercase().contains(&kw_lower)
            || row.day_bucket.to_lowercase().contains(&kw_lower)
            || row
                .camera_make
                .as_deref()
                .is_some_and(|s| s.to_lowercase().contains(&kw_lower))
            || row
                .camera_model
                .as_deref()
                .is_some_and(|s| s.to_lowercase().contains(&kw_lower))
            || row
                .lens_model
                .as_deref()
                .is_some_and(|s| s.to_lowercase().contains(&kw_lower))
            || row
                .album_titles
                .iter()
                .any(|t| t.to_lowercase().contains(&kw_lower))
            || row
                .people
                .iter()
                .any(|p| p.name.to_lowercase().contains(&kw_lower))
            || row
                .scene_tags
                .iter()
                .any(|t| t.to_lowercase().contains(&kw_lower));
        if !hit {
            return false;
        }
    }

    true
}

// ---------------------------------------------------------------------------
// Exact & Perceptual Duplicate Grouping
// ---------------------------------------------------------------------------

struct DisjointSet {
    parent: Vec<usize>,
}

impl DisjointSet {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            let p = self.parent[x];
            self.parent[x] = self.parent[p];
            x = p;
        }
        x
    }

    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra != rb {
            self.parent[rb] = ra;
        }
    }
}

fn quality_rank(row: &PhotoAiCandidateRow) -> (u64, u64, std::cmp::Reverse<i64>, ItemId) {
    let pixels = match (row.width, row.height) {
        (Some(w), Some(h)) if w > 0 && h > 0 => {
            u64::try_from(w).unwrap_or(0) * u64::try_from(h).unwrap_or(0)
        }
        _ => 0,
    };
    let size = row
        .size_bytes
        .and_then(|s| u64::try_from(s).ok())
        .unwrap_or(0);
    (pixels, size, std::cmp::Reverse(row.taken_at_ms), row.id)
}

/// Groups accessible photos into `exact` (same `content_hash`) and `similar` (`dHash`
/// Hamming distance `<= threshold`) duplicate clusters with keep-recommendation & reclaimable bytes.
pub fn build_duplicate_clusters(
    rows: &[PhotoAiCandidateRow],
    threshold: u32,
) -> PhotoDuplicatesResponse {
    let mut clusters: Vec<PhotoDuplicateCluster> = Vec::new();
    let mut exact_grouped: HashSet<ItemId> = HashSet::new();

    // 1. Exact duplicate clusters by SHA-256 content_hash
    let mut by_hash: HashMap<&str, Vec<&PhotoAiCandidateRow>> = HashMap::new();
    for row in rows {
        if let Some(hash) = row
            .content_hash
            .as_deref()
            .map(str::trim)
            .filter(|h| !h.is_empty())
        {
            by_hash.entry(hash).or_default().push(row);
        }
    }
    let mut exact_groups: Vec<(&str, Vec<&PhotoAiCandidateRow>)> = by_hash
        .into_iter()
        .filter(|(_, group)| group.len() >= 2)
        .collect();
    exact_groups.sort_by(|a, b| a.0.cmp(b.0));

    for (hash, mut group) in exact_groups {
        group.sort_by_key(|a| std::cmp::Reverse(quality_rank(a)));
        let keep_id = group[0].id;
        let mut reclaimable: u64 = 0;
        let mut items = Vec::with_capacity(group.len());
        for (idx, row) in group.into_iter().enumerate() {
            exact_grouped.insert(row.id);
            let keep = idx == 0;
            if !keep {
                reclaimable = reclaimable.saturating_add(
                    row.size_bytes
                        .and_then(|s| u64::try_from(s).ok())
                        .unwrap_or(0),
                );
            }
            items.push(to_duplicate_item(row, keep, 0));
        }
        clusters.push(PhotoDuplicateCluster {
            cluster_id: format!("exact:{}", &hash[..hash.len().min(12)]),
            kind: PhotoDuplicateKind::Exact,
            keep_item_id: keep_id,
            reclaimable_bytes: reclaimable,
            max_distance: 0,
            items,
        });
    }

    // 2. Perceptual similar clusters via dHash Hamming distance <= threshold
    let candidates: Vec<&PhotoAiCandidateRow> = rows
        .iter()
        .filter(|r| !exact_grouped.contains(&r.id) && r.dhash.is_some())
        .collect();

    if candidates.len() >= 2 {
        let mut dsu = DisjointSet::new(candidates.len());
        for (i, item_i) in candidates.iter().enumerate() {
            let hi = item_i.dhash.unwrap_or(0);
            for (j, item_j) in candidates.iter().enumerate().skip(i + 1) {
                let hj = item_j.dhash.unwrap_or(0);
                if hamming_distance(hi, hj) <= threshold {
                    dsu.union(i, j);
                }
            }
        }

        let mut comps: HashMap<usize, Vec<&PhotoAiCandidateRow>> = HashMap::new();
        for (idx, row) in candidates.iter().enumerate() {
            comps.entry(dsu.find(idx)).or_default().push(*row);
        }

        let mut similar_groups: Vec<Vec<&PhotoAiCandidateRow>> = comps
            .into_values()
            .filter(|group| group.len() >= 2)
            .collect();
        similar_groups.sort_by_key(|g| g.first().map(|r| r.id));

        for mut group in similar_groups {
            group.sort_by_key(|a| std::cmp::Reverse(quality_rank(a)));
            let keep_row = group[0];
            let keep_hash = keep_row.dhash.unwrap_or(0);
            let keep_id = keep_row.id;
            let mut reclaimable: u64 = 0;
            let mut max_distance: u32 = 0;
            let mut items = Vec::with_capacity(group.len());

            for (idx, row) in group.into_iter().enumerate() {
                let keep = idx == 0;
                let dist = if keep {
                    0
                } else {
                    hamming_distance(keep_hash, row.dhash.unwrap_or(0))
                };
                max_distance = max_distance.max(dist);
                if !keep {
                    reclaimable = reclaimable.saturating_add(
                        row.size_bytes
                            .and_then(|s| u64::try_from(s).ok())
                            .unwrap_or(0),
                    );
                }
                items.push(to_duplicate_item(row, keep, dist));
            }
            clusters.push(PhotoDuplicateCluster {
                cluster_id: format!("similar:{keep_id}"),
                kind: PhotoDuplicateKind::Similar,
                keep_item_id: keep_id,
                reclaimable_bytes: reclaimable,
                max_distance,
                items,
            });
        }
    }

    // Sort clusters: exact first, then largest reclaimable bytes descending
    clusters.sort_by(|a, b| {
        let ka = matches!(a.kind, PhotoDuplicateKind::Similar);
        let kb = matches!(b.kind, PhotoDuplicateKind::Similar);
        ka.cmp(&kb)
            .then_with(|| b.reclaimable_bytes.cmp(&a.reclaimable_bytes))
            .then_with(|| a.cluster_id.cmp(&b.cluster_id))
    });

    let total_clusters = u64::try_from(clusters.len()).unwrap_or(u64::MAX);
    let total_duplicate_photos: u64 = clusters
        .iter()
        .map(|c| u64::try_from(c.items.len().saturating_sub(1)).unwrap_or(0))
        .sum();
    let total_reclaimable_bytes: u64 = clusters.iter().map(|c| c.reclaimable_bytes).sum();

    PhotoDuplicatesResponse {
        threshold,
        total_clusters,
        total_duplicate_photos,
        total_reclaimable_bytes,
        clusters,
    }
}

fn to_duplicate_item(
    row: &PhotoAiCandidateRow,
    recommended_keep: bool,
    distance: u32,
) -> PhotoDuplicateItem {
    PhotoDuplicateItem {
        id: row.id,
        library_id: row.library_id,
        title: row.title.clone(),
        file_name: row.file_name.clone(),
        taken_at: row.taken_at,
        day_bucket: row.day_bucket.clone(),
        month_bucket: row.month_bucket.clone(),
        width: row.width,
        height: row.height,
        orientation: row.orientation,
        size_bytes: row.size_bytes,
        camera_make: row.camera_make.clone(),
        camera_model: row.camera_model.clone(),
        latitude: row.latitude,
        longitude: row.longitude,
        favorite: row.favorite,
        recommended_keep,
        distance,
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use palveni_core::{ItemId, LibraryId};

    use super::*;

    fn make_ppm_gradient(
        width: usize,
        height: usize,
        rgb: impl Fn(usize, usize) -> (u8, u8, u8),
    ) -> Vec<u8> {
        let mut out = format!("P6\n{width} {height}\n255\n").into_bytes();
        for y in 0..height {
            for x in 0..width {
                let (r, g, b) = rgb(x, y);
                out.push(r);
                out.push(g);
                out.push(b);
            }
        }
        out
    }

    #[test]
    fn ppm_dhash_and_dominant_color_extraction() {
        // Blue horizontal gradient: bright on left, dark on right -> all 64 dHash bits set!
        let blue_grad = make_ppm_gradient(18, 16, |x, _y| {
            let level = 240u8.saturating_sub(u8::try_from(x).unwrap_or(255).saturating_mul(10));
            (20, 90, level)
        });
        let rgb = extract_rgb_9x8_fallback(&blue_grad);
        let analysis = analyze_rgb_9x8(&rgb);
        assert_eq!(analysis.dominant_color, "blue");
        assert!(analysis.dominant_hex.starts_with('#'));
        assert_eq!(analysis.dominant_hex.len(), 7);
        assert_eq!(analysis.dhash, -1i64); // all 64 bits set

        // Slightly perturbed version should have tiny Hamming distance
        let blue_grad2 = make_ppm_gradient(18, 16, |x, y| {
            let level = 238u8
                .saturating_sub(u8::try_from(x).unwrap_or(255).saturating_mul(10))
                .saturating_add(u8::try_from(y % 2).unwrap_or(0));
            (22, 88, level)
        });
        let analysis2 = analyze_rgb_9x8(&extract_rgb_9x8_fallback(&blue_grad2));
        assert!(hamming_distance(analysis.dhash, analysis2.dhash) <= 2);

        // Opposite gradient (dark on left, bright on right) -> distance 64!
        let opposite = make_ppm_gradient(18, 16, |x, _y| {
            let level = 30u8.saturating_add(u8::try_from(x).unwrap_or(255).saturating_mul(10));
            (level, 30, 30)
        });
        let analysis_opp = analyze_rgb_9x8(&extract_rgb_9x8_fallback(&opposite));
        assert_eq!(hamming_distance(analysis.dhash, analysis_opp.dhash), 64);
    }

    #[test]
    fn bilingual_natural_language_query_parser_handles_cn_and_en() {
        let q1 = parse_photo_query("去年秋天的夜景", 2026);
        assert_eq!(q1.year, Some(2025));
        assert_eq!(q1.season.as_deref(), Some("autumn"));
        assert_eq!(q1.time_of_day.as_deref(), Some("night"));
        assert!(q1.keywords.is_empty());

        let q2 = parse_photo_query("蓝色天空", 2026);
        assert_eq!(q2.color.as_deref(), Some("blue"));
        assert!(q2.keywords.is_empty());

        let q3 = parse_photo_query("索尼竖拍 小明", 2026);
        assert_eq!(q3.camera_brand.as_deref(), Some("sony"));
        assert_eq!(q3.orientation.as_deref(), Some("portrait"));
        assert_eq!(q3.keywords, vec!["小明"]);

        let q4 = parse_photo_query("2025年11月 日落 iPhone", 2026);
        assert_eq!(q4.year, Some(2025));
        assert_eq!(q4.month, Some(11));
        assert_eq!(q4.time_of_day.as_deref(), Some("sunset"));
        assert_eq!(q4.camera_brand.as_deref(), Some("apple"));
        assert!(q4.keywords.is_empty());

        let q5 = parse_photo_query("Jan 2025 blue sunset canon landscape", 2026);
        assert_eq!(q5.year, Some(2025));
        assert_eq!(q5.month, Some(1));
        assert_eq!(q5.color.as_deref(), Some("blue"));
        assert_eq!(q5.time_of_day.as_deref(), Some("sunset"));
        assert_eq!(q5.camera_brand.as_deref(), Some("canon"));
        assert_eq!(q5.orientation.as_deref(), Some("landscape"));
        assert!(q5.keywords.is_empty());
    }

    #[test]
    fn duplicate_clustering_groups_exact_and_similar_with_keep_recommendation() {
        let lib = LibraryId::generate();
        let mk =
            |title: &str, w: i32, h: i32, size: i64, hash: Option<&str>, dhash: Option<i64>| {
                let dt = chrono::Utc
                    .with_ymd_and_hms(2025, 10, 1, 21, 0, 0)
                    .single()
                    .unwrap();
                PhotoAiCandidateRow {
                    id: ItemId::generate(),
                    library_id: lib,
                    title: title.into(),
                    file_name: format!("{title}.jpg"),
                    path: Some(format!("/photos/{title}.jpg")),
                    size_bytes: Some(size),
                    taken_at_ms: dt.timestamp_millis(),
                    taken_at: dt,
                    day_bucket: "2025-10-01".into(),
                    month_bucket: "2025-10".into(),
                    width: Some(w),
                    height: Some(h),
                    orientation: 1,
                    camera_make: Some("SONY".into()),
                    camera_model: Some("A7M4".into()),
                    lens_model: None,
                    latitude: None,
                    longitude: None,
                    content_hash: hash.map(str::to_owned),
                    favorite: false,
                    dhash,
                    dominant_color: Some("blue".into()),
                    dominant_hex: Some("#1E5AA8".into()),
                    scene_tags: vec!["landscape".into(), "blue".into()],
                    time_of_day: Some("night".into()),
                    season: Some("autumn".into()),
                    album_titles: vec![],
                    people: vec![],
                }
            };

        let exact_hi = mk(
            "EXACT_HI",
            4000,
            3000,
            5_000_000,
            Some("deadbeef"),
            Some(0x1111),
        );
        let exact_lo = mk(
            "EXACT_LO",
            2000,
            1500,
            1_200_000,
            Some("deadbeef"),
            Some(0x1111),
        );
        let sim_hi = mk(
            "SIM_HI",
            6000,
            4000,
            8_000_000,
            Some("hash1"),
            Some(0b1111_0000),
        );
        let sim_lo = mk(
            "SIM_LO",
            3000,
            2000,
            2_500_000,
            Some("hash2"),
            Some(0b1111_0011),
        ); // dist = 2
        let unique = mk(
            "UNIQUE",
            4000,
            3000,
            4_000_000,
            Some("hash3"),
            Some(0x00FF_FF00_00FF_FF00u64 as i64),
        );

        let resp = build_duplicate_clusters(
            &[
                exact_lo.clone(),
                exact_hi.clone(),
                sim_lo.clone(),
                sim_hi.clone(),
                unique,
            ],
            6,
        );
        assert_eq!(resp.total_clusters, 2);
        assert_eq!(resp.total_duplicate_photos, 2);
        assert_eq!(resp.total_reclaimable_bytes, 1_200_000 + 2_500_000);
        assert_eq!(resp.clusters[0].kind, PhotoDuplicateKind::Exact);
        assert_eq!(resp.clusters[0].keep_item_id, exact_hi.id);
        assert_eq!(resp.clusters[1].kind, PhotoDuplicateKind::Similar);
        assert_eq!(resp.clusters[1].keep_item_id, sim_hi.id);
        assert_eq!(resp.clusters[1].max_distance, 2);
    }
}
