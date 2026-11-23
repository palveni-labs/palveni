//! Pure-Rust bounded image header and EXIF/TIFF/GPS metadata parser.
//!
//! Supports JPEG (SOF0/SOF1/SOF2 + APP1 Exif II/MM), PNG (IHDR + eXIf),
//! WebP (VP8 / VP8L / VP8X + EXIF), TIFF/DNG/RAW (IFD0 + SubIFDs + Exif + GPS),
//! HEIC/HEIF/AVIF (ISOBMFF meta/iprp/ipco ispe + irot + Exif item), GIF and BMP.

use std::collections::HashSet;
use std::io::{self, Cursor, Read, Seek, SeekFrom};

use chrono::{FixedOffset, NaiveDate, TimeZone};
use palveni_db::photos::sha256_bytes_chunks;

const MAX_METADATA_READ: usize = 4 * 1024 * 1024;
const MAX_CHUNK_PAYLOAD: usize = 2 * 1024 * 1024;
const MAX_CHUNK_PAYLOAD_U32: u32 = 2 * 1024 * 1024;
const MAX_IFD_ENTRIES: u16 = 512;
const MAX_TAG_BYTES: usize = 4096;
const HASH_WINDOW: usize = 64 * 1024;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParsedPhoto {
    pub taken_at_ms: Option<i64>,
    pub day_bucket: Option<String>,
    pub month_bucket: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub orientation: u8,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens_model: Option<String>,
    pub iso: Option<i32>,
    pub f_number: Option<f64>,
    pub exposure_time: Option<String>,
    pub focal_length: Option<f64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// Computes a fast content fingerprint (`sha256` over 8-byte BE file size +
/// first 64 KiB + last 64 KiB) without reading multi-megabyte bodies in full.
pub fn compute_content_hash<R: Read + Seek>(reader: &mut R, file_size: u64) -> io::Result<String> {
    reader.seek(SeekFrom::Start(0))?;
    let size_be = file_size.to_be_bytes();

    let head_len = usize::try_from(file_size.min(HASH_WINDOW as u64)).unwrap_or(HASH_WINDOW);
    let mut head = vec![0u8; head_len];
    reader.read_exact(&mut head)?;

    let digest = if file_size > HASH_WINDOW as u64 {
        let tail_offset = file_size.saturating_sub(HASH_WINDOW as u64);
        reader.seek(SeekFrom::Start(tail_offset))?;
        let mut tail = vec![0u8; HASH_WINDOW];
        reader.read_exact(&mut tail)?;
        sha256_bytes_chunks(&[&size_be, &head, &tail])
    } else {
        sha256_bytes_chunks(&[&size_be, &head])
    };

    let _ = reader.seek(SeekFrom::Start(0));
    Ok(digest)
}

/// Detects the MIME type of a photo from its magic header and filename extension.
pub fn detect_photo_mime(header: &[u8], file_name: &str) -> Option<&'static str> {
    if header.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    if header.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if header.starts_with(b"RIFF") && header.get(8..12) == Some(b"WEBP") {
        return Some("image/webp");
    }
    if header.starts_with(b"GIF87a") || header.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if header.starts_with(b"BM") {
        return Some("image/bmp");
    }
    if header.get(4..8) == Some(b"ftyp") {
        let brand = header.get(8..12).unwrap_or_default();
        if brand == b"avif" || brand == b"avis" {
            return Some("image/avif");
        }
        if matches!(
            brand,
            b"heic" | b"heix" | b"hevc" | b"hevx" | b"mif1" | b"msf1"
        ) {
            return Some("image/heic");
        }
    }
    if is_tiff_magic(header) {
        return match crate::classify::extension_of(file_name).as_deref() {
            Some("dng") => Some("image/x-adobe-dng"),
            Some("cr2") => Some("image/x-canon-cr2"),
            Some("nef") => Some("image/x-nikon-nef"),
            Some("arw") => Some("image/x-sony-arw"),
            Some("orf") => Some("image/x-olympus-orf"),
            Some("rw2") => Some("image/x-panasonic-rw2"),
            _ => Some("image/tiff"),
        };
    }
    if header.starts_with(&[0xFF, 0x0A]) || header.starts_with(b"\x00\x00\x00\x0cJXL ") {
        return Some("image/jxl");
    }
    match crate::classify::extension_of(file_name).as_deref() {
        Some("jpg" | "jpeg") => Some("image/jpeg"),
        Some("png") => Some("image/png"),
        Some("webp") => Some("image/webp"),
        Some("gif") => Some("image/gif"),
        Some("bmp") => Some("image/bmp"),
        Some("avif") => Some("image/avif"),
        Some("heic" | "heif") => Some("image/heic"),
        Some("tif" | "tiff") => Some("image/tiff"),
        Some("dng") => Some("image/x-adobe-dng"),
        Some("jxl") => Some("image/jxl"),
        Some("cr2" | "cr3" | "nef" | "arw" | "orf" | "raf" | "rw2") => {
            Some("application/octet-stream")
        }
        _ => None,
    }
}

/// Checks whether a header buffer starts with recognized image magic bytes.
pub fn has_valid_image_magic(header: &[u8]) -> bool {
    if header.len() < 8 {
        return false;
    }
    header.starts_with(&[0xFF, 0xD8, 0xFF])
        || header.starts_with(b"\x89PNG\r\n\x1a\n")
        || (header.starts_with(b"RIFF") && header.get(8..12) == Some(b"WEBP"))
        || header.starts_with(b"GIF87a")
        || header.starts_with(b"GIF89a")
        || header.starts_with(b"BM")
        || header.get(4..8) == Some(b"ftyp")
        || is_tiff_magic(header)
        || header.starts_with(&[0xFF, 0x0A])
        || header.starts_with(b"\x00\x00\x00\x0cJXL ")
        || header.starts_with(b"FUJIFILMCCD-RAW")
}

fn is_tiff_magic(header: &[u8]) -> bool {
    matches!(
        header.get(..4),
        Some([0x49, 0x49, 0x2A, 0x00])
            | Some([0x4D, 0x4D, 0x00, 0x2A])
            | Some([0x49, 0x49, 0x55, 0x00])
            | Some([0x49, 0x49, 0x52, 0x4F])
            | Some([0x49, 0x49, 0x52, 0x53])
    )
}

struct BoundedReader<'a, R> {
    inner: &'a mut R,
    remaining: usize,
}

impl<R: Read> Read for BoundedReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "photo metadata read budget exhausted",
            ));
        }
        let max = buf.len().min(self.remaining);
        let n = self.inner.read(&mut buf[..max])?;
        self.remaining = self.remaining.saturating_sub(n);
        Ok(n)
    }
}

impl<R: Seek> Seek for BoundedReader<'_, R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}

pub fn parse_photo_bytes(bytes: &[u8]) -> ParsedPhoto {
    let len = bytes.len() as u64;
    let mut cursor = Cursor::new(bytes);
    parse_photo_reader(&mut cursor, len)
}

pub fn parse_photo_reader<R: Read + Seek>(reader: &mut R, file_size: u64) -> ParsedPhoto {
    let mut out = ParsedPhoto {
        orientation: 1,
        ..ParsedPhoto::default()
    };
    if file_size < 6 || reader.seek(SeekFrom::Start(0)).is_err() {
        return out;
    }

    let mut bounded = BoundedReader {
        inner: reader,
        remaining: MAX_METADATA_READ,
    };
    let mut sig = [0u8; 16];
    let sig_len = usize::try_from(file_size.min(16)).unwrap_or(16);
    if bounded.read_exact(&mut sig[..sig_len]).is_err() {
        return out;
    }
    let head = &sig[..sig_len];

    if head.starts_with(&[0xFF, 0xD8]) {
        let _ = parse_jpeg(&mut bounded, file_size, &mut out);
    } else if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        let _ = parse_png(&mut bounded, file_size, &mut out);
    } else if head.starts_with(b"RIFF") && head.get(8..12) == Some(b"WEBP") {
        let _ = parse_webp(&mut bounded, file_size, &mut out);
    } else if is_tiff_magic(head) {
        let _ = parse_tiff_seek(&mut bounded, file_size, &mut out);
    } else if head.get(4..8) == Some(b"ftyp") {
        let _ = parse_isobmff(&mut bounded, file_size, &mut out);
    } else if (head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a")) && head.len() >= 10 {
        let w = u32::from(u16::from_le_bytes([head[6], head[7]]));
        let h = u32::from(u16::from_le_bytes([head[8], head[9]]));
        if w > 0 && h > 0 {
            out.width = Some(w);
            out.height = Some(h);
        }
    } else if head.starts_with(b"BM") && file_size >= 26 {
        let mut bmp = [0u8; 26];
        if bounded.seek(SeekFrom::Start(0)).is_ok() && bounded.read_exact(&mut bmp).is_ok() {
            let w = i32::from_le_bytes([bmp[18], bmp[19], bmp[20], bmp[21]]);
            let h = i32::from_le_bytes([bmp[22], bmp[23], bmp[24], bmp[25]]).unsigned_abs();
            if let Ok(wu) = u32::try_from(w)
                && wu > 0
                && h > 0
            {
                out.width = Some(wu);
                out.height = Some(h);
            }
        }
    }

    if !(1..=8).contains(&out.orientation) {
        out.orientation = 1;
    }
    out
}

fn parse_jpeg<R: Read + Seek>(
    reader: &mut R,
    file_size: u64,
    out: &mut ParsedPhoto,
) -> io::Result<()> {
    reader.seek(SeekFrom::Start(2))?;
    let mut exif_done = false;
    let mut exif_pixel_x: Option<u32> = None;
    let mut exif_pixel_y: Option<u32> = None;

    for _ in 0..512 {
        let pos = reader.stream_position()?;
        if pos >= file_size {
            break;
        }
        let mut b = [0u8; 1];
        if reader.read_exact(&mut b).is_err() {
            break;
        }
        if b[0] != 0xFF {
            continue;
        }
        let marker = loop {
            reader.read_exact(&mut b)?;
            if b[0] != 0xFF {
                break b[0];
            }
        };
        if marker == 0xD9 || marker == 0xDA || marker == 0x00 {
            break;
        }
        if marker == 0xD8 || marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            continue;
        }

        let mut len_bytes = [0u8; 2];
        reader.read_exact(&mut len_bytes)?;
        let seg_len = u16::from_be_bytes(len_bytes);
        if seg_len < 2 {
            break;
        }
        let payload_len = usize::from(seg_len - 2);

        let is_sof = matches!(
            marker,
            0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF
        );
        if is_sof && payload_len >= 5 {
            let mut sof = [0u8; 5];
            reader.read_exact(&mut sof)?;
            let h = u32::from(u16::from_be_bytes([sof[1], sof[2]]));
            let w = u32::from(u16::from_be_bytes([sof[3], sof[4]]));
            if w > 0 && h > 0 {
                out.width = Some(w);
                out.height = Some(h);
            }
            if payload_len > 5 {
                reader.seek(SeekFrom::Current((payload_len - 5) as i64))?;
            }
        } else if marker == 0xE1 && !exif_done && (14..=MAX_CHUNK_PAYLOAD).contains(&payload_len) {
            let mut app1 = vec![0u8; payload_len];
            reader.read_exact(&mut app1)?;
            if let Some(tiff_bytes) = app1.strip_prefix(b"Exif\0\0") {
                let mut sub = ParsedPhoto {
                    orientation: 1,
                    ..ParsedPhoto::default()
                };
                parse_tiff_slice(tiff_bytes, &mut sub, &mut exif_pixel_x, &mut exif_pixel_y);
                merge_exif(out, sub);
                exif_done = true;
            }
        } else {
            reader.seek(SeekFrom::Current(payload_len as i64))?;
        }

        if out.width.is_some() && exif_done {
            break;
        }
    }

    if out.width.is_none()
        && let (Some(w), Some(h)) = (exif_pixel_x, exif_pixel_y)
    {
        out.width = Some(w);
        out.height = Some(h);
    }
    Ok(())
}

fn parse_png<R: Read + Seek>(
    reader: &mut R,
    file_size: u64,
    out: &mut ParsedPhoto,
) -> io::Result<()> {
    reader.seek(SeekFrom::Start(8))?;
    let mut exif_done = false;
    let mut px = None;
    let mut py = None;

    for _ in 0..256 {
        if reader.stream_position()? + 12 > file_size {
            break;
        }
        let mut hdr = [0u8; 8];
        if reader.read_exact(&mut hdr).is_err() {
            break;
        }
        let len = u32::from_be_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]);
        let ctype = &hdr[4..8];
        if ctype == b"IHDR" && len >= 8 {
            let mut ihdr = [0u8; 8];
            reader.read_exact(&mut ihdr)?;
            let w = u32::from_be_bytes([ihdr[0], ihdr[1], ihdr[2], ihdr[3]]);
            let h = u32::from_be_bytes([ihdr[4], ihdr[5], ihdr[6], ihdr[7]]);
            if w > 0 && h > 0 {
                out.width = Some(w);
                out.height = Some(h);
            }
            let skip = i64::from(len - 8) + 4;
            reader.seek(SeekFrom::Current(skip))?;
        } else if ctype == b"eXIf" && !exif_done && (8..=MAX_CHUNK_PAYLOAD_U32).contains(&len) {
            let mut data = vec![0u8; len as usize];
            reader.read_exact(&mut data)?;
            reader.seek(SeekFrom::Current(4))?;
            let tiff_slice = data.strip_prefix(b"Exif\0\0").unwrap_or(&data);
            let mut sub = ParsedPhoto {
                orientation: 1,
                ..ParsedPhoto::default()
            };
            parse_tiff_slice(tiff_slice, &mut sub, &mut px, &mut py);
            merge_exif(out, sub);
            exif_done = true;
        } else if ctype == b"IEND" {
            break;
        } else {
            let skip = i64::from(len) + 4;
            reader.seek(SeekFrom::Current(skip))?;
        }
        if out.width.is_some() && exif_done {
            break;
        }
    }
    Ok(())
}

fn parse_webp<R: Read + Seek>(
    reader: &mut R,
    file_size: u64,
    out: &mut ParsedPhoto,
) -> io::Result<()> {
    reader.seek(SeekFrom::Start(12))?;
    let mut exif_done = false;
    let mut px = None;
    let mut py = None;

    for _ in 0..128 {
        if reader.stream_position()? + 8 > file_size {
            break;
        }
        let mut hdr = [0u8; 8];
        if reader.read_exact(&mut hdr).is_err() {
            break;
        }
        let cid = &hdr[0..4];
        let clen = u32::from_le_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]);
        let padded = i64::from(clen) + i64::from(clen & 1);

        if cid == b"VP8 " && clen >= 10 && out.width.is_none() {
            let mut b = [0u8; 10];
            reader.read_exact(&mut b)?;
            if (b[0] & 1) == 0 && b[3..6] == [0x9D, 0x01, 0x2A] {
                let w = u32::from(u16::from_le_bytes([b[6], b[7]]) & 0x3FFF);
                let h = u32::from(u16::from_le_bytes([b[8], b[9]]) & 0x3FFF);
                if w > 0 && h > 0 {
                    out.width = Some(w);
                    out.height = Some(h);
                }
            }
            reader.seek(SeekFrom::Current(padded - 10))?;
        } else if cid == b"VP8L" && clen >= 5 && out.width.is_none() {
            let mut b = [0u8; 5];
            reader.read_exact(&mut b)?;
            if b[0] == 0x2F {
                let bits = u32::from_le_bytes([b[1], b[2], b[3], b[4]]);
                let w = (bits & 0x3FFF) + 1;
                let h = ((bits >> 14) & 0x3FFF) + 1;
                out.width = Some(w);
                out.height = Some(h);
            }
            reader.seek(SeekFrom::Current(padded - 5))?;
        } else if cid == b"VP8X" && clen >= 10 && out.width.is_none() {
            let mut b = [0u8; 10];
            reader.read_exact(&mut b)?;
            let w = u32::from_le_bytes([b[4], b[5], b[6], 0]) + 1;
            let h = u32::from_le_bytes([b[7], b[8], b[9], 0]) + 1;
            out.width = Some(w);
            out.height = Some(h);
            reader.seek(SeekFrom::Current(padded - 10))?;
        } else if cid == b"EXIF" && !exif_done && (8..=MAX_CHUNK_PAYLOAD_U32).contains(&clen) {
            let mut data = vec![0u8; clen as usize];
            reader.read_exact(&mut data)?;
            let rem = padded - i64::from(clen);
            if rem > 0 {
                reader.seek(SeekFrom::Current(rem))?;
            }
            let tiff_slice = data.strip_prefix(b"Exif\0\0").unwrap_or(&data);
            let mut sub = ParsedPhoto {
                orientation: 1,
                ..ParsedPhoto::default()
            };
            parse_tiff_slice(tiff_slice, &mut sub, &mut px, &mut py);
            merge_exif(out, sub);
            exif_done = true;
        } else {
            reader.seek(SeekFrom::Current(padded))?;
        }
        if out.width.is_some() && exif_done {
            break;
        }
    }
    Ok(())
}

fn parse_isobmff<R: Read + Seek>(
    reader: &mut R,
    file_size: u64,
    out: &mut ParsedPhoto,
) -> io::Result<()> {
    reader.seek(SeekFrom::Start(0))?;
    for _ in 0..64 {
        let pos = reader.stream_position()?;
        if pos + 8 > file_size {
            break;
        }
        let mut hdr = [0u8; 8];
        if reader.read_exact(&mut hdr).is_err() {
            break;
        }
        let size32 = u32::from_be_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]);
        let btype = [hdr[4], hdr[5], hdr[6], hdr[7]];
        let (box_size, hdr_len) = if size32 == 1 {
            let mut ext = [0u8; 8];
            reader.read_exact(&mut ext)?;
            (u64::from_be_bytes(ext), 16u64)
        } else if size32 == 0 {
            (file_size.saturating_sub(pos), 8u64)
        } else {
            (u64::from(size32), 8u64)
        };
        if box_size < hdr_len {
            break;
        }
        let payload_len = box_size - hdr_len;
        let next_pos = pos.saturating_add(box_size);

        if &btype == b"meta"
            && (4..=MAX_CHUNK_PAYLOAD as u64).contains(&payload_len)
            && let Ok(meta_len) = usize::try_from(payload_len)
        {
            let mut meta = vec![0u8; meta_len];
            reader.read_exact(&mut meta)?;
            parse_heic_meta(&meta, reader, file_size, out);
            break;
        }
        if next_pos <= pos || next_pos > file_size {
            break;
        }
        reader.seek(SeekFrom::Start(next_pos))?;
    }
    Ok(())
}

fn iter_boxes(data: &[u8]) -> Vec<([u8; 4], &[u8])> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    while offset + 8 <= data.len() && out.len() < 256 {
        let Some(hdr) = data.get(offset..offset + 8) else {
            break;
        };
        let size32 = u32::from_be_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]);
        let btype = [hdr[4], hdr[5], hdr[6], hdr[7]];
        let (box_size, hdr_len) = if size32 == 1 {
            let Some(ext) = data.get(offset + 8..offset + 16) else {
                break;
            };
            let s = u64::from_be_bytes([
                ext[0], ext[1], ext[2], ext[3], ext[4], ext[5], ext[6], ext[7],
            ]);
            let Ok(sz) = usize::try_from(s) else { break };
            (sz, 16usize)
        } else if size32 == 0 {
            (data.len() - offset, 8usize)
        } else {
            (size32 as usize, 8usize)
        };
        if box_size < hdr_len || offset + box_size > data.len() {
            break;
        }
        if let Some(payload) = data.get(offset + hdr_len..offset + box_size) {
            out.push((btype, payload));
        }
        offset += box_size;
    }
    out
}

fn parse_heic_meta<R: Read + Seek>(
    meta_payload: &[u8],
    reader: &mut R,
    file_size: u64,
    out: &mut ParsedPhoto,
) {
    // `meta` is a FullBox (4-byte version + flags).
    let Some(children_slice) = meta_payload.get(4..) else {
        return;
    };
    let children = iter_boxes(children_slice);
    let mut best_area = 0u64;
    let mut irot_orientation: Option<u8> = None;
    let mut exif_item_id: Option<u32> = None;
    let mut idat_slice: Option<&[u8]> = None;
    let mut iloc_slice: Option<&[u8]> = None;

    for (btype, payload) in &children {
        match btype {
            b"iprp" => {
                for (sub_type, sub_payload) in iter_boxes(payload) {
                    if &sub_type == b"ipco" {
                        for (prop_type, prop_payload) in iter_boxes(sub_payload) {
                            if &prop_type == b"ispe"
                                && let Some(dims) = prop_payload.get(4..12)
                            {
                                let w = u32::from_be_bytes([dims[0], dims[1], dims[2], dims[3]]);
                                let h = u32::from_be_bytes([dims[4], dims[5], dims[6], dims[7]]);
                                let area = u64::from(w) * u64::from(h);
                                if w > 0 && h > 0 && area >= best_area {
                                    best_area = area;
                                    out.width = Some(w);
                                    out.height = Some(h);
                                }
                            } else if &prop_type == b"irot"
                                && let Some(&angle) = prop_payload.first()
                            {
                                irot_orientation = match angle & 0x03 {
                                    1 => Some(8), // 90 deg CCW
                                    2 => Some(3), // 180 deg
                                    3 => Some(6), // 270 deg CCW (90 deg CW)
                                    _ => Some(1),
                                };
                            }
                        }
                    }
                }
            }
            b"iinf" => {
                exif_item_id = find_heic_exif_item_id(payload);
            }
            b"idat" => {
                idat_slice = Some(payload);
            }
            b"iloc" => {
                iloc_slice = Some(payload);
            }
            _ => {}
        }
    }

    if let (Some(item_id), Some(iloc)) = (exif_item_id, iloc_slice)
        && let Some((method, offset, length)) = locate_heic_item(iloc, item_id)
        && (10..=MAX_CHUNK_PAYLOAD as u64).contains(&length)
    {
        let exif_buf = if method == 1 {
            let start = usize::try_from(offset).ok();
            let len = usize::try_from(length).ok();
            start
                .zip(len)
                .and_then(|(s, l)| idat_slice.and_then(|d| d.get(s..s.checked_add(l)?)))
                .map(<[u8]>::to_vec)
        } else if method == 0
            && offset.saturating_add(length) <= file_size
            && let Ok(len_usize) = usize::try_from(length)
        {
            if reader.seek(SeekFrom::Start(offset)).is_ok() {
                let mut buf = vec![0u8; len_usize];
                reader.read_exact(&mut buf).ok().map(|()| buf)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(buf) = exif_buf
            && let Some(hdr) = buf.get(..4)
        {
            let tiff_off = u32::from_be_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]) as usize;
            if let Some(after) = buf.get(4usize.saturating_add(tiff_off)..) {
                let tiff_slice = after.strip_prefix(b"Exif\0\0").unwrap_or(after);
                let mut sub = ParsedPhoto {
                    orientation: 1,
                    ..ParsedPhoto::default()
                };
                let mut px = None;
                let mut py = None;
                parse_tiff_slice(tiff_slice, &mut sub, &mut px, &mut py);
                merge_exif(out, sub);
            }
        }
    }

    if out.orientation == 1
        && let Some(rot) = irot_orientation
    {
        out.orientation = rot;
    }
}

fn find_heic_exif_item_id(iinf: &[u8]) -> Option<u32> {
    let version = *iinf.first()?;
    let entries_start = if version == 0 { 6 } else { 8 };
    let rest = iinf.get(entries_start..)?;
    for (btype, payload) in iter_boxes(rest) {
        if &btype != b"infe" {
            continue;
        }
        let infe_ver = *payload.first()?;
        if infe_ver == 2 && payload.len() >= 12 {
            let item_id = u32::from(u16::from_be_bytes([payload[4], payload[5]]));
            if payload.get(8..12) == Some(b"Exif") {
                return Some(item_id);
            }
        } else if infe_ver >= 3 && payload.len() >= 14 {
            let item_id = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
            if payload.get(10..14) == Some(b"Exif") {
                return Some(item_id);
            }
        }
    }
    None
}

fn read_uint_be(bytes: &[u8]) -> Option<u64> {
    match bytes.len() {
        0 => Some(0),
        4 => Some(u64::from(u32::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ]))),
        8 => Some(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])),
        _ => None,
    }
}

fn locate_heic_item(iloc: &[u8], target_id: u32) -> Option<(u8, u64, u64)> {
    let version = *iloc.first()?;
    let s0 = *iloc.get(4)?;
    let s1 = *iloc.get(5)?;
    let offset_size = usize::from(s0 >> 4);
    let length_size = usize::from(s0 & 0x0F);
    let base_offset_size = usize::from(s1 >> 4);
    let index_size = if version == 1 || version == 2 {
        usize::from(s1 & 0x0F)
    } else {
        0
    };

    let (item_count, mut pos) = if version < 2 {
        let cnt = u32::from(u16::from_be_bytes([*iloc.get(6)?, *iloc.get(7)?]));
        (cnt, 8usize)
    } else if version == 2 {
        let cnt = u32::from_be_bytes([*iloc.get(6)?, *iloc.get(7)?, *iloc.get(8)?, *iloc.get(9)?]);
        (cnt, 10usize)
    } else {
        return None;
    };

    for _ in 0..item_count.min(512) {
        let item_id = if version < 2 {
            let b = iloc.get(pos..pos + 2)?;
            pos += 2;
            u32::from(u16::from_be_bytes([b[0], b[1]]))
        } else {
            let b = iloc.get(pos..pos + 4)?;
            pos += 4;
            u32::from_be_bytes([b[0], b[1], b[2], b[3]])
        };
        let method = if version == 1 || version == 2 {
            let m = iloc.get(pos..pos + 2)?;
            pos += 2;
            m[1] & 0x0F
        } else {
            0
        };
        pos = pos.checked_add(2)?; // data_reference_index
        let base_offset = read_uint_be(iloc.get(pos..pos.checked_add(base_offset_size)?)?)?;
        pos += base_offset_size;
        let extent_eb = iloc.get(pos..pos + 2)?;
        pos += 2;
        let extent_count = u16::from_be_bytes([extent_eb[0], extent_eb[1]]);
        let mut first_extent: Option<(u64, u64)> = None;
        for idx in 0..extent_count.min(64) {
            pos = pos.checked_add(index_size)?;
            let ext_off = read_uint_be(iloc.get(pos..pos.checked_add(offset_size)?)?)?;
            pos += offset_size;
            let ext_len = read_uint_be(iloc.get(pos..pos.checked_add(length_size)?)?)?;
            pos += length_size;
            if idx == 0 {
                first_extent = Some((base_offset.saturating_add(ext_off), ext_len));
            }
        }
        if item_id == target_id
            && let Some((off, len)) = first_extent
        {
            return Some((method, off, len));
        }
    }
    None
}

fn merge_exif(dst: &mut ParsedPhoto, src: ParsedPhoto) {
    if src.taken_at_ms.is_some() {
        dst.taken_at_ms = src.taken_at_ms;
        dst.day_bucket = src.day_bucket;
        dst.month_bucket = src.month_bucket;
    }
    if (1..=8).contains(&src.orientation) && src.orientation != 1 {
        dst.orientation = src.orientation;
    }
    if src.camera_make.is_some() {
        dst.camera_make = src.camera_make;
    }
    if src.camera_model.is_some() {
        dst.camera_model = src.camera_model;
    }
    if src.lens_model.is_some() {
        dst.lens_model = src.lens_model;
    }
    if src.iso.is_some() {
        dst.iso = src.iso;
    }
    if src.f_number.is_some() {
        dst.f_number = src.f_number;
    }
    if src.exposure_time.is_some() {
        dst.exposure_time = src.exposure_time;
    }
    if src.focal_length.is_some() {
        dst.focal_length = src.focal_length;
    }
    if src.latitude.is_some() && src.longitude.is_some() {
        dst.latitude = src.latitude;
        dst.longitude = src.longitude;
    }
}

#[derive(Clone, Copy)]
enum Endian {
    Little,
    Big,
}

impl Endian {
    fn u16(self, b: &[u8]) -> Option<u16> {
        let a: [u8; 2] = b.get(..2)?.try_into().ok()?;
        Some(match self {
            Self::Little => u16::from_le_bytes(a),
            Self::Big => u16::from_be_bytes(a),
        })
    }

    fn u32(self, b: &[u8]) -> Option<u32> {
        let a: [u8; 4] = b.get(..4)?.try_into().ok()?;
        Some(match self {
            Self::Little => u32::from_le_bytes(a),
            Self::Big => u32::from_be_bytes(a),
        })
    }

    fn i32(self, b: &[u8]) -> Option<i32> {
        let a: [u8; 4] = b.get(..4)?.try_into().ok()?;
        Some(match self {
            Self::Little => i32::from_le_bytes(a),
            Self::Big => i32::from_be_bytes(a),
        })
    }
}

fn parse_tiff_slice(
    bytes: &[u8],
    out: &mut ParsedPhoto,
    pixel_x: &mut Option<u32>,
    pixel_y: &mut Option<u32>,
) {
    let len = bytes.len() as u64;
    let mut cursor = Cursor::new(bytes);
    let _ = parse_tiff_internal(&mut cursor, len, out, pixel_x, pixel_y);
}

fn parse_tiff_seek<R: Read + Seek>(
    reader: &mut R,
    file_size: u64,
    out: &mut ParsedPhoto,
) -> io::Result<()> {
    let mut px = None;
    let mut py = None;
    parse_tiff_internal(reader, file_size, out, &mut px, &mut py)?;
    if out.width.is_none()
        && let (Some(w), Some(h)) = (px, py)
    {
        out.width = Some(w);
        out.height = Some(h);
    }
    Ok(())
}

#[derive(Default)]
struct ExifDates {
    original: Option<String>,
    digitized: Option<String>,
    ifd0: Option<String>,
    offset_original: Option<String>,
    offset_any: Option<String>,
}

#[derive(Default)]
struct GpsAcc {
    lat_ref: Option<char>,
    lat_dms: Option<[(f64, f64); 3]>,
    lon_ref: Option<char>,
    lon_dms: Option<[(f64, f64); 3]>,
}

fn parse_tiff_internal<R: Read + Seek>(
    reader: &mut R,
    limit: u64,
    out: &mut ParsedPhoto,
    pixel_x: &mut Option<u32>,
    pixel_y: &mut Option<u32>,
) -> io::Result<()> {
    if limit < 8 {
        return Ok(());
    }
    reader.seek(SeekFrom::Start(0))?;
    let mut hdr = [0u8; 8];
    reader.read_exact(&mut hdr)?;
    let endian = match &hdr[0..2] {
        b"II" => Endian::Little,
        b"MM" => Endian::Big,
        _ => return Ok(()),
    };
    let Some(magic) = endian.u16(&hdr[2..4]) else {
        return Ok(());
    };
    if !matches!(magic, 0x002A | 0x0055 | 0x4F52 | 0x5352) {
        return Ok(());
    }
    let Some(ifd0_offset) = endian.u32(&hdr[4..8]) else {
        return Ok(());
    };

    let mut visited = HashSet::new();
    let mut dates = ExifDates::default();
    let mut gps = GpsAcc::default();
    let mut exif_ifd_offset: Option<u32> = None;
    let mut gps_ifd_offset: Option<u32> = None;
    let mut sub_ifds: Vec<u32> = Vec::new();
    let mut best_area = 0u64;

    if let Some(entries) = read_ifd_entries(reader, endian, ifd0_offset, limit, &mut visited) {
        let mut ifd0_w: Option<u32> = None;
        let mut ifd0_h: Option<u32> = None;
        for entry in &entries {
            match entry.tag {
                0x0100 => ifd0_w = read_tag_u32(reader, endian, entry, limit),
                0x0101 => ifd0_h = read_tag_u32(reader, endian, entry, limit),
                0x010F => out.camera_make = read_tag_ascii(reader, endian, entry, limit),
                0x0110 => out.camera_model = read_tag_ascii(reader, endian, entry, limit),
                0x0112 => {
                    if let Some(o) = read_tag_u32(reader, endian, entry, limit)
                        && let Ok(ob) = u8::try_from(o)
                        && (1..=8).contains(&ob)
                    {
                        out.orientation = ob;
                    }
                }
                0x0132 => dates.ifd0 = read_tag_ascii(reader, endian, entry, limit),
                0x014A => sub_ifds = read_tag_u32_vec(reader, endian, entry, limit, 8),
                0x8769 => exif_ifd_offset = read_tag_u32(reader, endian, entry, limit),
                0x8825 => gps_ifd_offset = read_tag_u32(reader, endian, entry, limit),
                _ => {}
            }
        }
        if let (Some(w), Some(h)) = (ifd0_w, ifd0_h)
            && w > 0
            && h > 0
        {
            best_area = u64::from(w) * u64::from(h);
            out.width = Some(w);
            out.height = Some(h);
        }
    }

    for sub_off in sub_ifds {
        if let Some(entries) = read_ifd_entries(reader, endian, sub_off, limit, &mut visited) {
            let mut sw = None;
            let mut sh = None;
            for entry in &entries {
                match entry.tag {
                    0x0100 => sw = read_tag_u32(reader, endian, entry, limit),
                    0x0101 => sh = read_tag_u32(reader, endian, entry, limit),
                    _ => {}
                }
            }
            if let (Some(w), Some(h)) = (sw, sh) {
                let area = u64::from(w) * u64::from(h);
                if w > 0 && h > 0 && area > best_area {
                    best_area = area;
                    out.width = Some(w);
                    out.height = Some(h);
                }
            }
        }
    }

    if let Some(exif_off) = exif_ifd_offset
        && let Some(entries) = read_ifd_entries(reader, endian, exif_off, limit, &mut visited)
    {
        for entry in &entries {
            match entry.tag {
                0x9003 => dates.original = read_tag_ascii(reader, endian, entry, limit),
                0x9004 => dates.digitized = read_tag_ascii(reader, endian, entry, limit),
                0x9011 => dates.offset_original = read_tag_ascii(reader, endian, entry, limit),
                0x9010 => dates.offset_any = read_tag_ascii(reader, endian, entry, limit),
                0x8827 | 0x8833 => {
                    if out.iso.is_none()
                        && let Some(v) = read_tag_u32(reader, endian, entry, limit)
                        && let Ok(iv) = i32::try_from(v)
                        && iv > 0
                    {
                        out.iso = Some(iv);
                    }
                }
                0x829D => {
                    if let Some((num, den)) = read_tag_rational(reader, endian, entry, limit)
                        && den > 0.0
                    {
                        let f = num / den;
                        if f.is_finite() && f > 0.0 && f < 256.0 {
                            out.f_number = Some((f * 100.0).round() / 100.0);
                        }
                    }
                }
                0x829A => {
                    if let Some((num, den)) = read_tag_u32_rational(reader, endian, entry, limit) {
                        out.exposure_time = format_exposure_time(num, den);
                    }
                }
                0x920A => {
                    if let Some((num, den)) = read_tag_rational(reader, endian, entry, limit)
                        && den > 0.0
                    {
                        let fl = num / den;
                        if fl.is_finite() && fl > 0.0 && fl < 10_000.0 {
                            out.focal_length = Some((fl * 100.0).round() / 100.0);
                        }
                    }
                }
                0xA434 => out.lens_model = read_tag_ascii(reader, endian, entry, limit),
                0xA002 => *pixel_x = read_tag_u32(reader, endian, entry, limit).filter(|&v| v > 0),
                0xA003 => *pixel_y = read_tag_u32(reader, endian, entry, limit).filter(|&v| v > 0),
                _ => {}
            }
        }
    }

    if let Some(gps_off) = gps_ifd_offset
        && let Some(entries) = read_ifd_entries(reader, endian, gps_off, limit, &mut visited)
    {
        for entry in &entries {
            match entry.tag {
                0x0001 => {
                    gps.lat_ref =
                        read_tag_ascii(reader, endian, entry, limit).and_then(|s| s.chars().next());
                }
                0x0002 => {
                    gps.lat_dms = read_tag_rationals_3(reader, endian, entry, limit);
                }
                0x0003 => {
                    gps.lon_ref =
                        read_tag_ascii(reader, endian, entry, limit).and_then(|s| s.chars().next());
                }
                0x0004 => {
                    gps.lon_dms = read_tag_rationals_3(reader, endian, entry, limit);
                }
                _ => {}
            }
        }
    }

    let tz_str = dates
        .offset_original
        .as_deref()
        .or(dates.offset_any.as_deref());
    for candidate in [
        dates.original.as_deref(),
        dates.digitized.as_deref(),
        dates.ifd0.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if let Some((ms, day, month)) = parse_exif_datetime(candidate, tz_str) {
            out.taken_at_ms = Some(ms);
            out.day_bucket = Some(day);
            out.month_bucket = Some(month);
            break;
        }
    }

    if let (Some(lat_dms), Some(lon_dms)) = (gps.lat_dms, gps.lon_dms)
        && let (Some(lat), Some(lon)) = (
            dms_to_degrees(lat_dms, gps.lat_ref, 90.0, ['S', 's']),
            dms_to_degrees(lon_dms, gps.lon_ref, 180.0, ['W', 'w']),
        )
    {
        out.latitude = Some(lat);
        out.longitude = Some(lon);
    }

    Ok(())
}

struct IfdEntry {
    tag: u16,
    field_type: u16,
    count: u32,
    raw_value: [u8; 4],
}

fn type_unit_bytes(field_type: u16) -> Option<usize> {
    match field_type {
        1 | 2 | 6 | 7 => Some(1),
        3 | 8 => Some(2),
        4 | 9 | 11 => Some(4),
        5 | 10 | 12 => Some(8),
        _ => None,
    }
}

fn read_ifd_entries<R: Read + Seek>(
    reader: &mut R,
    endian: Endian,
    offset: u32,
    limit: u64,
    visited: &mut HashSet<u32>,
) -> Option<Vec<IfdEntry>> {
    if visited.len() >= 16 || !visited.insert(offset) {
        return None;
    }
    let off64 = u64::from(offset);
    if off64.saturating_add(2) > limit {
        return None;
    }
    reader.seek(SeekFrom::Start(off64)).ok()?;
    let mut cnt_buf = [0u8; 2];
    reader.read_exact(&mut cnt_buf).ok()?;
    let count = endian.u16(&cnt_buf)?.min(MAX_IFD_ENTRIES);
    let table_bytes = usize::from(count) * 12;
    if off64.saturating_add(2 + table_bytes as u64) > limit {
        return None;
    }
    let mut buf = vec![0u8; table_bytes];
    reader.read_exact(&mut buf).ok()?;
    let mut entries = Vec::with_capacity(usize::from(count));
    for chunk in buf.as_chunks::<12>().0 {
        let tag = endian.u16(&chunk[0..2])?;
        let field_type = endian.u16(&chunk[2..4])?;
        let cnt = endian.u32(&chunk[4..8])?;
        let raw_value = [chunk[8], chunk[9], chunk[10], chunk[11]];
        entries.push(IfdEntry {
            tag,
            field_type,
            count: cnt,
            raw_value,
        });
    }
    Some(entries)
}

fn read_entry_bytes<R: Read + Seek>(
    reader: &mut R,
    endian: Endian,
    entry: &IfdEntry,
    limit: u64,
    max_bytes: usize,
) -> Option<Vec<u8>> {
    let unit = type_unit_bytes(entry.field_type)?;
    let total = usize::try_from(entry.count).ok()?.checked_mul(unit)?;
    if total == 0 {
        return None;
    }
    let capped = total.min(max_bytes);
    if total <= 4 {
        return Some(entry.raw_value.get(..capped)?.to_vec());
    }
    let offset = u64::from(endian.u32(&entry.raw_value)?);
    if offset.saturating_add(capped as u64) > limit {
        return None;
    }
    reader.seek(SeekFrom::Start(offset)).ok()?;
    let mut buf = vec![0u8; capped];
    reader.read_exact(&mut buf).ok()?;
    Some(buf)
}

fn read_tag_ascii<R: Read + Seek>(
    reader: &mut R,
    endian: Endian,
    entry: &IfdEntry,
    limit: u64,
) -> Option<String> {
    if !matches!(entry.field_type, 2 | 1 | 7) {
        return None;
    }
    let bytes = read_entry_bytes(reader, endian, entry, limit, MAX_TAG_BYTES)?;
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let text = String::from_utf8_lossy(&bytes[..end]);
    let cleaned: String = text
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .to_owned();
    (!cleaned.is_empty()).then_some(cleaned)
}

fn read_tag_u32<R: Read + Seek>(
    reader: &mut R,
    endian: Endian,
    entry: &IfdEntry,
    limit: u64,
) -> Option<u32> {
    let bytes = read_entry_bytes(reader, endian, entry, limit, 8)?;
    match entry.field_type {
        1 | 7 => bytes.first().copied().map(u32::from),
        3 => endian.u16(&bytes).map(u32::from),
        4 => endian.u32(&bytes),
        9 => endian.i32(&bytes).and_then(|v| u32::try_from(v).ok()),
        _ => None,
    }
}

fn read_tag_u32_vec<R: Read + Seek>(
    reader: &mut R,
    endian: Endian,
    entry: &IfdEntry,
    limit: u64,
    max_items: usize,
) -> Vec<u32> {
    let unit = match entry.field_type {
        3 => 2usize,
        4 => 4usize,
        _ => return Vec::new(),
    };
    let Some(bytes) = read_entry_bytes(reader, endian, entry, limit, max_items * unit) else {
        return Vec::new();
    };
    bytes
        .chunks_exact(unit)
        .filter_map(|c| {
            if unit == 2 {
                endian.u16(c).map(u32::from)
            } else {
                endian.u32(c)
            }
        })
        .collect()
}

fn read_tag_u32_rational<R: Read + Seek>(
    reader: &mut R,
    endian: Endian,
    entry: &IfdEntry,
    limit: u64,
) -> Option<(u32, u32)> {
    let bytes = read_entry_bytes(reader, endian, entry, limit, 8)?;
    match entry.field_type {
        5 => Some((endian.u32(&bytes[0..4])?, endian.u32(&bytes[4..8])?)),
        10 => {
            let n = u32::try_from(endian.i32(&bytes[0..4])?).ok()?;
            let d = u32::try_from(endian.i32(&bytes[4..8])?).ok()?;
            Some((n, d))
        }
        _ => None,
    }
}

fn read_tag_rational<R: Read + Seek>(
    reader: &mut R,
    endian: Endian,
    entry: &IfdEntry,
    limit: u64,
) -> Option<(f64, f64)> {
    let bytes = read_entry_bytes(reader, endian, entry, limit, 8)?;
    match entry.field_type {
        5 => Some((
            f64::from(endian.u32(&bytes[0..4])?),
            f64::from(endian.u32(&bytes[4..8])?),
        )),
        10 => Some((
            f64::from(endian.i32(&bytes[0..4])?),
            f64::from(endian.i32(&bytes[4..8])?),
        )),
        _ => None,
    }
}

fn read_tag_rationals_3<R: Read + Seek>(
    reader: &mut R,
    endian: Endian,
    entry: &IfdEntry,
    limit: u64,
) -> Option<[(f64, f64); 3]> {
    if entry.count < 3 || !matches!(entry.field_type, 5 | 10) {
        return None;
    }
    let bytes = read_entry_bytes(reader, endian, entry, limit, 24)?;
    if bytes.len() < 24 {
        return None;
    }
    let mut out = [(0.0, 0.0); 3];
    for (idx, chunk) in bytes.as_chunks::<8>().0.iter().take(3).enumerate() {
        out[idx] = if entry.field_type == 5 {
            (
                f64::from(endian.u32(&chunk[0..4])?),
                f64::from(endian.u32(&chunk[4..8])?),
            )
        } else {
            (
                f64::from(endian.i32(&chunk[0..4])?),
                f64::from(endian.i32(&chunk[4..8])?),
            )
        };
    }
    Some(out)
}

fn gcd_u32(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

/// Formats an EXIF exposure rational like `"1/250s"`, `"0.5s"`, or `"2s"`.
pub fn format_exposure_time(num: u32, den: u32) -> Option<String> {
    if num == 0 || den == 0 {
        return None;
    }
    let g = gcd_u32(num, den).max(1);
    let n = num / g;
    let d = den / g;
    let secs = f64::from(n) / f64::from(d);
    if !secs.is_finite() || secs <= 0.0 {
        return None;
    }
    if n < d {
        if n == 1 {
            return Some(format!("1/{d}s"));
        }
        let inv = f64::from(d) / f64::from(n);
        if inv >= 3.0 {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let rounded_den = inv.round() as u64;
            return Some(format!("1/{rounded_den}s"));
        }
        return Some(format!("{secs:.1}s"));
    }
    if d == 1 {
        Some(format!("{n}s"))
    } else {
        Some(format!("{secs:.1}s"))
    }
}

fn dms_to_degrees(
    dms: [(f64, f64); 3],
    hemi: Option<char>,
    max_abs: f64,
    neg_refs: [char; 2],
) -> Option<f64> {
    for &(n, d) in &dms {
        if d == 0.0 || !n.is_finite() || !d.is_finite() || n < 0.0 || d < 0.0 {
            return None;
        }
    }
    let deg = (dms[0].0 / dms[0].1) + (dms[1].0 / dms[1].1) / 60.0 + (dms[2].0 / dms[2].1) / 3600.0;
    if !deg.is_finite() || !(0.0..=max_abs).contains(&deg) {
        return None;
    }
    if deg == 0.0 && hemi.is_none() {
        return None;
    }
    let signed = if hemi.is_some_and(|c| neg_refs.contains(&c)) {
        -deg
    } else {
        deg
    };
    Some((signed * 1_000_000.0).round() / 1_000_000.0)
}

fn parse_exif_offset(raw: &str) -> Option<FixedOffset> {
    let s = raw.trim();
    if s == "Z" || s == "z" {
        return FixedOffset::east_opt(0);
    }
    let sign = match s.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let rest = &s[1..];
    let (hh, mm) = if let Some((h, m)) = rest.split_once(':') {
        (h, m)
    } else if rest.len() == 4 {
        (&rest[0..2], &rest[2..4])
    } else {
        return None;
    };
    let hours: i32 = hh.parse().ok()?;
    let mins: i32 = mm.parse().ok()?;
    if !(0..=14).contains(&hours) || !(0..=59).contains(&mins) {
        return None;
    }
    let total_secs = sign * (hours * 3600 + mins * 60);
    FixedOffset::east_opt(total_secs)
}

/// Parses `"YYYY:MM:DD HH:MM:SS"` (or ISO `"YYYY-MM-DDTHH:MM:SS"`) into `(epoch_ms, YYYY-MM-DD, YYYY-MM)`.
pub fn parse_exif_datetime(raw: &str, tz_offset: Option<&str>) -> Option<(i64, String, String)> {
    let s = raw.trim();
    if s.len() < 19 {
        return None;
    }
    let year: i32 = s.get(0..4)?.parse().ok()?;
    if !(1826..=2200).contains(&year) {
        return None;
    }
    let sep1 = s.as_bytes().get(4).copied()?;
    let sep2 = s.as_bytes().get(7).copied()?;
    if !matches!(sep1, b':' | b'-') || !matches!(sep2, b':' | b'-') {
        return None;
    }
    let month: u32 = s.get(5..7)?.parse().ok()?;
    let day: u32 = s.get(8..10)?.parse().ok()?;
    let hour: u32 = s.get(11..13)?.parse().ok()?;
    let minute: u32 = s.get(14..16)?.parse().ok()?;
    let second: u32 = s.get(17..19)?.parse().ok()?;

    let date = NaiveDate::from_ymd_opt(year, month, day)?;
    let naive = date.and_hms_opt(hour, minute, second)?;
    let day_bucket = format!("{year:04}-{month:02}-{day:02}");
    let month_bucket = format!("{year:04}-{month:02}");

    let inline_tz = s.get(19..).map(str::trim).filter(|t| !t.is_empty());
    let offset = tz_offset
        .and_then(parse_exif_offset)
        .or_else(|| inline_tz.and_then(parse_exif_offset));

    let ms = match offset {
        Some(off) => off.from_local_datetime(&naive).single()?.timestamp_millis(),
        None => naive.and_utc().timestamp_millis(),
    };
    Some((ms, day_bucket, month_bucket))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_ifd_entry_le(buf: &mut Vec<u8>, tag: u16, ftype: u16, count: u32, val: [u8; 4]) {
        buf.extend_from_slice(&tag.to_le_bytes());
        buf.extend_from_slice(&ftype.to_le_bytes());
        buf.extend_from_slice(&count.to_le_bytes());
        buf.extend_from_slice(&val);
    }

    fn push_ifd_entry_be(buf: &mut Vec<u8>, tag: u16, ftype: u16, count: u32, val: [u8; 4]) {
        buf.extend_from_slice(&tag.to_be_bytes());
        buf.extend_from_slice(&ftype.to_be_bytes());
        buf.extend_from_slice(&count.to_be_bytes());
        buf.extend_from_slice(&val);
    }

    fn build_sample_exif_tiff_le() -> Vec<u8> {
        // TIFF header (8 bytes): II, 42, IFD0 at offset 8
        let mut tiff = Vec::new();
        tiff.extend_from_slice(b"II");
        tiff.extend_from_slice(&42u16.to_le_bytes());
        tiff.extend_from_slice(&8u32.to_le_bytes());

        // IFD0 has 5 entries: Make, Model, Orientation, ExifIFD, GPSIFD
        // Offset 8: 2 + 5*12 + 4 = 66 bytes -> ends at offset 74
        let ifd0_count = 5u16;
        let mut data_pool = Vec::new();
        let data_base = 8u32 + 2 + u32::from(ifd0_count) * 12 + 4; // 74

        let add_pool = |pool: &mut Vec<u8>, bytes: &[u8]| -> u32 {
            let off = data_base + u32::try_from(pool.len()).unwrap();
            pool.extend_from_slice(bytes);
            if pool.len() % 2 == 1 {
                pool.push(0);
            }
            off
        };

        let make = b"SONY\0";
        let make_off = add_pool(&mut data_pool, make);
        let model = b"ILCE-7M4\0";
        let model_off = add_pool(&mut data_pool, model);

        // Reserve space for ExifIFD (7 entries: DateTimeOriginal, ISOSpeedRatings, FNumber, ExposureTime, FocalLength, LensModel, PixelXDimension)
        // and GPSIFD (4 entries: LatRef, Lat, LonRef, Lon)
        let fnum_off = add_pool(&mut data_pool, &[28, 0, 0, 0, 10, 0, 0, 0]); // 28/10 = f/2.8
        let exp_off = add_pool(&mut data_pool, &[10, 0, 0, 0, 224, 9, 0, 0]); // 10/2500 = 1/250s (2500 = 0x09C4 -> [0xC4, 0x09, 0, 0])
        // Fix 2500 LE bytes explicitly:
        let pos = (exp_off - data_base) as usize;
        data_pool[pos..pos + 4].copy_from_slice(&10u32.to_le_bytes());
        data_pool[pos + 4..pos + 8].copy_from_slice(&2500u32.to_le_bytes());

        let fl_off = add_pool(
            &mut data_pool,
            &[350u32.to_le_bytes(), 10u32.to_le_bytes()].concat(),
        ); // 35.0mm
        let dt_off = add_pool(&mut data_pool, b"2025:11:23 16:42:09\0");
        let lens_off = add_pool(&mut data_pool, b"FE 35mm F1.4 GM\0");

        let mut lat_bytes = Vec::new();
        for (n, d) in [(35u32, 1u32), (41u32, 1u32), (2220u32, 100u32)] {
            lat_bytes.extend_from_slice(&n.to_le_bytes());
            lat_bytes.extend_from_slice(&d.to_le_bytes());
        }
        let lat_off = add_pool(&mut data_pool, &lat_bytes);

        let mut lon_bytes = Vec::new();
        for (n, d) in [(139u32, 1u32), (41u32, 1u32), (3012u32, 100u32)] {
            lon_bytes.extend_from_slice(&n.to_le_bytes());
            lon_bytes.extend_from_slice(&d.to_le_bytes());
        }
        let lon_off = add_pool(&mut data_pool, &lon_bytes);

        let exif_ifd_off = data_base + u32::try_from(data_pool.len()).unwrap();
        let mut exif_ifd = Vec::new();
        exif_ifd.extend_from_slice(&6u16.to_le_bytes());
        push_ifd_entry_le(&mut exif_ifd, 0x829A, 5, 1, exp_off.to_le_bytes());
        push_ifd_entry_le(&mut exif_ifd, 0x829D, 5, 1, fnum_off.to_le_bytes());
        push_ifd_entry_le(&mut exif_ifd, 0x8827, 3, 1, [160, 0, 0, 0]); // ISO 160
        push_ifd_entry_le(&mut exif_ifd, 0x9003, 2, 20, dt_off.to_le_bytes());
        push_ifd_entry_le(&mut exif_ifd, 0x920A, 5, 1, fl_off.to_le_bytes());
        push_ifd_entry_le(&mut exif_ifd, 0xA434, 2, 16, lens_off.to_le_bytes());
        exif_ifd.extend_from_slice(&0u32.to_le_bytes());
        data_pool.extend_from_slice(&exif_ifd);

        let gps_ifd_off = data_base + u32::try_from(data_pool.len()).unwrap();
        let mut gps_ifd = Vec::new();
        gps_ifd.extend_from_slice(&4u16.to_le_bytes());
        push_ifd_entry_le(&mut gps_ifd, 0x0001, 2, 2, *b"S\0\0\0");
        push_ifd_entry_le(&mut gps_ifd, 0x0002, 5, 3, lat_off.to_le_bytes());
        push_ifd_entry_le(&mut gps_ifd, 0x0003, 2, 2, *b"W\0\0\0");
        push_ifd_entry_le(&mut gps_ifd, 0x0004, 5, 3, lon_off.to_le_bytes());
        gps_ifd.extend_from_slice(&0u32.to_le_bytes());
        data_pool.extend_from_slice(&gps_ifd);

        tiff.extend_from_slice(&ifd0_count.to_le_bytes());
        push_ifd_entry_le(&mut tiff, 0x010F, 2, 5, make_off.to_le_bytes());
        push_ifd_entry_le(&mut tiff, 0x0110, 2, 9, model_off.to_le_bytes());
        push_ifd_entry_le(&mut tiff, 0x0112, 3, 1, [6, 0, 0, 0]); // Orientation = 6
        push_ifd_entry_le(&mut tiff, 0x8769, 4, 1, exif_ifd_off.to_le_bytes());
        push_ifd_entry_le(&mut tiff, 0x8825, 4, 1, gps_ifd_off.to_le_bytes());
        tiff.extend_from_slice(&0u32.to_le_bytes());
        tiff.extend_from_slice(&data_pool);
        tiff
    }

    #[test]
    fn parses_jpeg_sof_and_little_endian_exif_and_signed_gps() {
        let tiff = build_sample_exif_tiff_le();
        let mut jpeg = vec![0xFF, 0xD8];
        // APP1 segment
        jpeg.extend_from_slice(&[0xFF, 0xE1]);
        let app1_len = u16::try_from(2 + 6 + tiff.len()).unwrap();
        jpeg.extend_from_slice(&app1_len.to_be_bytes());
        jpeg.extend_from_slice(b"Exif\0\0");
        jpeg.extend_from_slice(&tiff);
        // SOF2 progressive segment (width = 7008, height = 4672)
        jpeg.extend_from_slice(&[0xFF, 0xC2, 0x00, 0x0B, 0x08]);
        jpeg.extend_from_slice(&4672u16.to_be_bytes());
        jpeg.extend_from_slice(&7008u16.to_be_bytes());
        jpeg.extend_from_slice(&[0x01, 0x01, 0x11, 0x00]);
        jpeg.extend_from_slice(&[0xFF, 0xD9]);

        let parsed = parse_photo_bytes(&jpeg);
        assert_eq!(parsed.width, Some(7008));
        assert_eq!(parsed.height, Some(4672));
        assert_eq!(parsed.orientation, 6);
        assert_eq!(parsed.camera_make.as_deref(), Some("SONY"));
        assert_eq!(parsed.camera_model.as_deref(), Some("ILCE-7M4"));
        assert_eq!(parsed.lens_model.as_deref(), Some("FE 35mm F1.4 GM"));
        assert_eq!(parsed.iso, Some(160));
        assert_eq!(parsed.f_number, Some(2.8));
        assert_eq!(parsed.exposure_time.as_deref(), Some("1/250s"));
        assert_eq!(parsed.focal_length, Some(35.0));
        assert_eq!(parsed.day_bucket.as_deref(), Some("2025-11-23"));
        assert_eq!(parsed.month_bucket.as_deref(), Some("2025-11"));
        assert!(parsed.taken_at_ms.is_some());
        let lat = parsed.latitude.unwrap();
        let lon = parsed.longitude.unwrap();
        // S & W -> negative signed degrees!
        assert!((lat - (-35.6895)).abs() < 0.001, "lat={lat}");
        assert!((lon - (-139.6917)).abs() < 0.001, "lon={lon}");
    }

    #[test]
    fn parses_big_endian_tiff_header_and_gps_north_east() {
        let mut tiff = Vec::new();
        tiff.extend_from_slice(b"MM");
        tiff.extend_from_slice(&42u16.to_be_bytes());
        tiff.extend_from_slice(&8u32.to_be_bytes());

        // IFD0: ImageWidth(3), ImageLength(3), Orientation(3), GPSIFD(4)
        let ifd0_count = 4u16;
        let gps_ifd_off = 8u32 + 2 + u32::from(ifd0_count) * 12 + 4; // 62
        tiff.extend_from_slice(&ifd0_count.to_be_bytes());
        push_ifd_entry_be(&mut tiff, 0x0100, 4, 1, 4032u32.to_be_bytes());
        push_ifd_entry_be(&mut tiff, 0x0101, 4, 1, 3024u32.to_be_bytes());
        push_ifd_entry_be(&mut tiff, 0x0112, 3, 1, [0, 3, 0, 0]); // SHORT BE = 3
        push_ifd_entry_be(&mut tiff, 0x8825, 4, 1, gps_ifd_off.to_be_bytes());
        tiff.extend_from_slice(&0u32.to_be_bytes());

        let coords_off = gps_ifd_off + 2 + 4 * 12 + 4; // 116
        tiff.extend_from_slice(&4u16.to_be_bytes());
        push_ifd_entry_be(&mut tiff, 0x0001, 2, 2, *b"N\0\0\0");
        push_ifd_entry_be(&mut tiff, 0x0002, 5, 3, coords_off.to_be_bytes());
        push_ifd_entry_be(&mut tiff, 0x0003, 2, 2, *b"E\0\0\0");
        push_ifd_entry_be(&mut tiff, 0x0004, 5, 3, (coords_off + 24).to_be_bytes());
        tiff.extend_from_slice(&0u32.to_be_bytes());

        for (n, d) in [
            (31u32, 1u32),
            (13u32, 1u32),
            (4944u32, 100u32),
            (121u32, 1u32),
            (28u32, 1u32),
            (2532u32, 100u32),
        ] {
            tiff.extend_from_slice(&n.to_be_bytes());
            tiff.extend_from_slice(&d.to_be_bytes());
        }

        let parsed = parse_photo_bytes(&tiff);
        assert_eq!(parsed.width, Some(4032));
        assert_eq!(parsed.height, Some(3024));
        assert_eq!(parsed.orientation, 3);
        assert!((parsed.latitude.unwrap() - 31.2304).abs() < 0.001);
        assert!((parsed.longitude.unwrap() - 121.4737).abs() < 0.001);
    }

    #[test]
    fn parses_png_ihdr_webp_vp8x_and_heic_ispe_irot() {
        // PNG with IHDR (1920x1080) + eXIf
        let tiff = build_sample_exif_tiff_le();
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend_from_slice(&13u32.to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&1920u32.to_be_bytes());
        png.extend_from_slice(&1080u32.to_be_bytes());
        png.extend_from_slice(&[8, 6, 0, 0, 0]);
        png.extend_from_slice(&0u32.to_be_bytes()); // CRC
        png.extend_from_slice(&u32::try_from(tiff.len()).unwrap().to_be_bytes());
        png.extend_from_slice(b"eXIf");
        png.extend_from_slice(&tiff);
        png.extend_from_slice(&0u32.to_be_bytes());
        let p_png = parse_photo_bytes(&png);
        assert_eq!(p_png.width, Some(1920));
        assert_eq!(p_png.height, Some(1080));
        assert_eq!(p_png.camera_model.as_deref(), Some("ILCE-7M4"));

        // WebP VP8X (2560 x 1440)
        let mut webp = b"RIFF\x24\x00\x00\x00WEBP".to_vec();
        webp.extend_from_slice(b"VP8X");
        webp.extend_from_slice(&10u32.to_le_bytes());
        webp.extend_from_slice(&[0, 0, 0, 0]);
        let w_m1 = 2559u32.to_le_bytes();
        let h_m1 = 1439u32.to_le_bytes();
        webp.extend_from_slice(&w_m1[0..3]);
        webp.extend_from_slice(&h_m1[0..3]);
        let p_webp = parse_photo_bytes(&webp);
        assert_eq!(p_webp.width, Some(2560));
        assert_eq!(p_webp.height, Some(1440));

        // HEIC ftyp + meta -> iprp -> ipco -> ispe (4032x3024) + irot (90 CCW -> orientation 8)
        let mut ispe = Vec::new();
        ispe.extend_from_slice(&20u32.to_be_bytes());
        ispe.extend_from_slice(b"ispe");
        ispe.extend_from_slice(&[0, 0, 0, 0]);
        ispe.extend_from_slice(&4032u32.to_be_bytes());
        ispe.extend_from_slice(&3024u32.to_be_bytes());

        let mut irot = Vec::new();
        irot.extend_from_slice(&9u32.to_be_bytes());
        irot.extend_from_slice(b"irot");
        irot.push(1); // 90 deg CCW -> Exif 8

        let mut ipco = Vec::new();
        let ipco_len = 8 + ispe.len() + irot.len();
        ipco.extend_from_slice(&u32::try_from(ipco_len).unwrap().to_be_bytes());
        ipco.extend_from_slice(b"ipco");
        ipco.extend_from_slice(&ispe);
        ipco.extend_from_slice(&irot);

        let mut iprp = Vec::new();
        let iprp_len = 8 + ipco.len();
        iprp.extend_from_slice(&u32::try_from(iprp_len).unwrap().to_be_bytes());
        iprp.extend_from_slice(b"iprp");
        iprp.extend_from_slice(&ipco);

        let mut meta = Vec::new();
        let meta_len = 12 + iprp.len();
        meta.extend_from_slice(&u32::try_from(meta_len).unwrap().to_be_bytes());
        meta.extend_from_slice(b"meta");
        meta.extend_from_slice(&[0, 0, 0, 0]); // FullBox version/flags
        meta.extend_from_slice(&iprp);

        let mut heic = Vec::new();
        heic.extend_from_slice(&16u32.to_be_bytes());
        heic.extend_from_slice(b"ftypheic\0\0\0\0");
        heic.extend_from_slice(&meta);

        let p_heic = parse_photo_bytes(&heic);
        assert_eq!(p_heic.width, Some(4032));
        assert_eq!(p_heic.height, Some(3024));
        assert_eq!(p_heic.orientation, 8);
    }

    #[test]
    fn content_hash_fingerprint_distinguishes_size_head_and_tail() {
        let a = vec![0xAAu8; 150_000];
        let mut b = a.clone();
        b[149_999] = 0xBB;
        let ha = compute_content_hash(&mut Cursor::new(&a), a.len() as u64).unwrap();
        let hb = compute_content_hash(&mut Cursor::new(&b), b.len() as u64).unwrap();
        assert_eq!(ha.len(), 64);
        assert_ne!(ha, hb);
    }
}
