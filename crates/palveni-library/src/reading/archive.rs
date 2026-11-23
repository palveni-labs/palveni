//! Pure safe-Rust bounded ZIP (Stored `0` + RFC 1951 DEFLATE `8` with CRC32)
//! and POSIX/USTAR TAR archive readers.

use std::io::{self, Read, Seek, SeekFrom};

pub const MAX_ENTRY_BYTES: usize = 32 * 1024 * 1024;
const MAX_CENTRAL_DIR_BYTES: usize = 16 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 32_768;
const MAX_HUFFMAN_BITS: usize = 15;

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("corrupt or unsupported archive: {0}")]
    Corrupt(&'static str),
    #[error("unsupported compression method {0}")]
    UnsupportedCompression(u16),
    #[error("archive entry exceeds size limit")]
    EntryTooLarge,
}

#[derive(Clone, Debug)]
pub struct ArchiveEntry {
    pub name: String,
    pub uncompressed_size: u64,
}

pub enum ArchiveReader<R: Read + Seek> {
    Zip(ZipArchive<R>),
    Tar(TarArchive<R>),
}

impl<R: Read + Seek> ArchiveReader<R> {
    pub fn entries(&self) -> &[ArchiveEntry] {
        match self {
            Self::Zip(z) => z.entries(),
            Self::Tar(t) => t.entries(),
        }
    }

    pub fn find_entry(&self, path: &str) -> Option<usize> {
        let normalized = normalize_archive_path(path)?;
        self.entries()
            .iter()
            .position(|e| e.name == normalized)
            .or_else(|| {
                self.entries()
                    .iter()
                    .position(|e| e.name.eq_ignore_ascii_case(&normalized))
            })
    }

    pub fn read_entry(&mut self, index: usize, max_bytes: usize) -> Result<Vec<u8>, ArchiveError> {
        match self {
            Self::Zip(z) => z.read_entry(index, max_bytes),
            Self::Tar(t) => t.read_entry(index, max_bytes),
        }
    }
}

/// Normalizes an in-archive relative path and rejects any `..` traversal.
pub fn normalize_archive_path(raw: &str) -> Option<String> {
    let replaced = raw.replace('\\', "/");
    let mut parts: Vec<&str> = Vec::new();
    for segment in replaced.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." || segment.chars().any(char::is_control) {
            return None;
        }
        parts.push(segment);
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

/// Resolves `href` relative to `base_file_path` inside an archive, rejecting traversal above root.
pub fn resolve_relative_archive_path(base_file_path: &str, href: &str) -> Option<String> {
    let clean_href = href
        .split('#')
        .next()
        .unwrap_or(href)
        .split('?')
        .next()
        .unwrap_or(href)
        .trim();
    if clean_href.is_empty() {
        return None;
    }
    let decoded = percent_decode(clean_href);
    let replaced = decoded.replace('\\', "/");
    let mut stack: Vec<&str> = if replaced.starts_with('/') {
        Vec::new()
    } else {
        match base_file_path.rsplit_once('/') {
            Some((dir, _)) => dir.split('/').filter(|s| !s.is_empty()).collect(),
            None => Vec::new(),
        }
    };
    for part in replaced.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            stack.pop()?;
        } else {
            if part.chars().any(char::is_control) {
                return None;
            }
            stack.push(part);
        }
    }
    if stack.is_empty() {
        None
    } else {
        Some(stack.join("/"))
    }
}

pub fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let (Some(&hi), Some(&lo)) = (bytes.get(i + 1), bytes.get(i + 2))
            && let (Some(h), Some(l)) = (hex_val(hi), hex_val(lo))
        {
            out.push((h << 4) | l);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

const fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// IEEE 802.3 CRC32
// ---------------------------------------------------------------------------

const CRC32_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0u16;
    while i < 256 {
        let mut crc = i as u32;
        let mut bit = 0;
        while bit < 8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
            bit += 1;
        }
        table[i as usize] = crc;
        i += 1;
    }
    table
};

pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        let idx = usize::from(crc.to_le_bytes()[0] ^ byte);
        crc = (crc >> 8) ^ CRC32_TABLE[idx];
    }
    !crc
}

// ---------------------------------------------------------------------------
// RFC 1951 DEFLATE Inflator
// ---------------------------------------------------------------------------

const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
const CL_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    fn read_bit(&mut self) -> Result<u8, ArchiveError> {
        let byte = *self
            .data
            .get(self.byte_pos)
            .ok_or(ArchiveError::Corrupt("unexpected end of deflate stream"))?;
        let bit = (byte >> self.bit_pos) & 1;
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
        Ok(bit)
    }

    fn read_bits(&mut self, count: u8) -> Result<u32, ArchiveError> {
        let mut value = 0u32;
        let mut shift = 0u8;
        while shift < count {
            let bit = u32::from(self.read_bit()?);
            value |= bit << shift;
            shift += 1;
        }
        Ok(value)
    }

    fn align_to_byte(&mut self) {
        if self.bit_pos != 0 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
    }

    fn read_u16_le(&mut self) -> Result<u16, ArchiveError> {
        let bytes = self.read_exact_slice(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_exact_slice(&mut self, len: usize) -> Result<&'a [u8], ArchiveError> {
        let end = self
            .byte_pos
            .checked_add(len)
            .ok_or(ArchiveError::Corrupt("deflate slice overflow"))?;
        let slice = self
            .data
            .get(self.byte_pos..end)
            .ok_or(ArchiveError::Corrupt(
                "unexpected end of uncompressed block",
            ))?;
        self.byte_pos = end;
        Ok(slice)
    }
}

struct HuffmanTree {
    counts: [u16; MAX_HUFFMAN_BITS + 1],
    first_code: [u32; MAX_HUFFMAN_BITS + 1],
    first_index: [usize; MAX_HUFFMAN_BITS + 1],
    symbols: Vec<u16>,
}

impl HuffmanTree {
    fn from_lengths(lengths: &[u8]) -> Result<Self, ArchiveError> {
        let mut counts = [0u16; MAX_HUFFMAN_BITS + 1];
        for &len in lengths {
            let l = usize::from(len);
            if l > MAX_HUFFMAN_BITS {
                return Err(ArchiveError::Corrupt("huffman code length > 15"));
            }
            if l > 0 {
                counts[l] = counts[l]
                    .checked_add(1)
                    .ok_or(ArchiveError::Corrupt("huffman count overflow"))?;
            }
        }

        let mut first_code = [0u32; MAX_HUFFMAN_BITS + 1];
        let mut first_index = [0usize; MAX_HUFFMAN_BITS + 1];
        let mut code = 0u32;
        let mut total_symbols = 0usize;
        for bits in 1..=MAX_HUFFMAN_BITS {
            code = (code + u32::from(counts[bits - 1])) << 1;
            if code.saturating_add(u32::from(counts[bits])) > (1u32 << bits) {
                return Err(ArchiveError::Corrupt("oversubscribed huffman tree"));
            }
            first_code[bits] = code;
            first_index[bits] = total_symbols;
            total_symbols += usize::from(counts[bits]);
        }

        let mut next_index = first_index;
        let mut symbols = vec![0u16; total_symbols];
        for (sym, &len) in lengths.iter().enumerate() {
            let l = usize::from(len);
            if l > 0 {
                let sym_u16 =
                    u16::try_from(sym).map_err(|_| ArchiveError::Corrupt("symbol overflow"))?;
                let slot = next_index[l];
                if let Some(target) = symbols.get_mut(slot) {
                    *target = sym_u16;
                }
                next_index[l] += 1;
            }
        }

        Ok(Self {
            counts,
            first_code,
            first_index,
            symbols,
        })
    }

    fn fixed_lit_len() -> Result<Self, ArchiveError> {
        let mut lengths = [0u8; 288];
        lengths[0..=143].fill(8);
        lengths[144..=255].fill(9);
        lengths[256..=279].fill(7);
        lengths[280..=287].fill(8);
        Self::from_lengths(&lengths)
    }

    fn fixed_dist() -> Result<Self, ArchiveError> {
        let lengths = [5u8; 32];
        Self::from_lengths(&lengths)
    }

    fn decode(&self, reader: &mut BitReader<'_>) -> Result<u16, ArchiveError> {
        let mut code = 0u32;
        for bits in 1..=MAX_HUFFMAN_BITS {
            code = (code << 1) | u32::from(reader.read_bit()?);
            let first = self.first_code[bits];
            let count = u32::from(self.counts[bits]);
            if code >= first && code - first < count {
                let offset = usize::try_from(code - first).unwrap_or(0);
                return self
                    .symbols
                    .get(self.first_index[bits] + offset)
                    .copied()
                    .ok_or(ArchiveError::Corrupt("invalid huffman symbol"));
            }
        }
        Err(ArchiveError::Corrupt("invalid huffman code"))
    }
}

pub fn inflate(input: &[u8], max_output: usize) -> Result<Vec<u8>, ArchiveError> {
    let mut reader = BitReader::new(input);
    let mut out = Vec::with_capacity(input.len().saturating_mul(2).min(max_output).min(65_536));

    loop {
        let bfinal = reader.read_bit()?;
        let btype = reader.read_bits(2)?;
        match btype {
            0 => {
                reader.align_to_byte();
                let len = reader.read_u16_le()?;
                let nlen = reader.read_u16_le()?;
                if len ^ nlen != 0xFFFF {
                    return Err(ArchiveError::Corrupt("invalid stored block length"));
                }
                let block_len = usize::from(len);
                if out.len().saturating_add(block_len) > max_output {
                    return Err(ArchiveError::EntryTooLarge);
                }
                out.extend_from_slice(reader.read_exact_slice(block_len)?);
            }
            1 => {
                let lit_tree = HuffmanTree::fixed_lit_len()?;
                let dist_tree = HuffmanTree::fixed_dist()?;
                decode_huffman_block(&mut reader, &lit_tree, &dist_tree, &mut out, max_output)?;
            }
            2 => {
                let hlit = usize::try_from(reader.read_bits(5)?).unwrap_or(0) + 257;
                let hdist = usize::try_from(reader.read_bits(5)?).unwrap_or(0) + 1;
                let hclen = usize::try_from(reader.read_bits(4)?).unwrap_or(0) + 4;
                if hlit > 286 || hdist > 32 || hclen > 19 {
                    return Err(ArchiveError::Corrupt("invalid dynamic huffman header"));
                }

                let mut cl_lengths = [0u8; 19];
                for &idx in CL_ORDER.iter().take(hclen) {
                    cl_lengths[idx] = u8::try_from(reader.read_bits(3)?).unwrap_or(0);
                }
                let cl_tree = HuffmanTree::from_lengths(&cl_lengths)?;

                let total_codes = hlit + hdist;
                let mut lengths = Vec::with_capacity(total_codes);
                while lengths.len() < total_codes {
                    let sym = cl_tree.decode(&mut reader)?;
                    match sym {
                        0..=15 => {
                            lengths.push(u8::try_from(sym).unwrap_or(0));
                        }
                        16 => {
                            let prev = *lengths
                                .last()
                                .ok_or(ArchiveError::Corrupt("repeat code with no previous"))?;
                            let repeat = usize::try_from(reader.read_bits(2)?).unwrap_or(0) + 3;
                            if lengths.len() + repeat > total_codes {
                                return Err(ArchiveError::Corrupt("code length repeat overflow"));
                            }
                            lengths.resize(lengths.len() + repeat, prev);
                        }
                        17 => {
                            let repeat = usize::try_from(reader.read_bits(3)?).unwrap_or(0) + 3;
                            if lengths.len() + repeat > total_codes {
                                return Err(ArchiveError::Corrupt("zero repeat overflow"));
                            }
                            lengths.resize(lengths.len() + repeat, 0);
                        }
                        18 => {
                            let repeat = usize::try_from(reader.read_bits(7)?).unwrap_or(0) + 11;
                            if lengths.len() + repeat > total_codes {
                                return Err(ArchiveError::Corrupt("long zero repeat overflow"));
                            }
                            lengths.resize(lengths.len() + repeat, 0);
                        }
                        _ => return Err(ArchiveError::Corrupt("invalid code length symbol")),
                    }
                }

                let lit_tree = HuffmanTree::from_lengths(&lengths[..hlit])?;
                let dist_tree = HuffmanTree::from_lengths(&lengths[hlit..])?;
                decode_huffman_block(&mut reader, &lit_tree, &dist_tree, &mut out, max_output)?;
            }
            _ => return Err(ArchiveError::Corrupt("reserved deflate block type 3")),
        }

        if bfinal == 1 {
            break;
        }
    }

    Ok(out)
}

fn decode_huffman_block(
    reader: &mut BitReader<'_>,
    lit_tree: &HuffmanTree,
    dist_tree: &HuffmanTree,
    out: &mut Vec<u8>,
    max_output: usize,
) -> Result<(), ArchiveError> {
    loop {
        let sym = lit_tree.decode(reader)?;
        if sym < 256 {
            if out.len() >= max_output {
                return Err(ArchiveError::EntryTooLarge);
            }
            out.push(u8::try_from(sym).unwrap_or(0));
        } else if sym == 256 {
            return Ok(());
        } else {
            let len_idx = usize::from(sym - 257);
            let base_len = usize::from(
                *LEN_BASE
                    .get(len_idx)
                    .ok_or(ArchiveError::Corrupt("invalid length code"))?,
            );
            let extra_bits = LEN_EXTRA[len_idx];
            let extra_len = usize::try_from(reader.read_bits(extra_bits)?).unwrap_or(0);
            let match_len = base_len + extra_len;

            let dist_sym = usize::from(dist_tree.decode(reader)?);
            let base_dist = usize::from(
                *DIST_BASE
                    .get(dist_sym)
                    .ok_or(ArchiveError::Corrupt("invalid distance code"))?,
            );
            let dist_extra_bits = DIST_EXTRA[dist_sym];
            let extra_dist = usize::try_from(reader.read_bits(dist_extra_bits)?).unwrap_or(0);
            let distance = base_dist + extra_dist;

            if distance == 0 || distance > out.len() {
                return Err(ArchiveError::Corrupt("lz77 distance out of bounds"));
            }
            if out.len().saturating_add(match_len) > max_output {
                return Err(ArchiveError::EntryTooLarge);
            }
            let start = out.len() - distance;
            for i in 0..match_len {
                let b = out[start + i];
                out.push(b);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ZIP Archive Reader
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct ZipRecord {
    method: u16,
    crc32: u32,
    compressed_size: u64,
    uncompressed_size: u64,
    local_header_offset: u64,
}

pub struct ZipArchive<R: Read + Seek> {
    reader: R,
    entries: Vec<ArchiveEntry>,
    records: Vec<ZipRecord>,
}

impl<R: Read + Seek> ZipArchive<R> {
    pub fn open(mut reader: R) -> Result<Self, ArchiveError> {
        let file_len = reader.seek(SeekFrom::End(0))?;
        if file_len < 22 {
            return Err(ArchiveError::Corrupt("file too small to be a ZIP archive"));
        }

        let tail_len = usize::try_from(file_len.min(65_535 + 22)).unwrap_or(22);
        let tail_start = file_len - u64::try_from(tail_len).unwrap_or(0);
        reader.seek(SeekFrom::Start(tail_start))?;
        let mut tail = vec![0u8; tail_len];
        reader.read_exact(&mut tail)?;

        let eocd_rel = find_eocd(&tail).ok_or(ArchiveError::Corrupt(
            "ZIP end-of-central-directory not found",
        ))?;
        let eocd = &tail[eocd_rel..];
        let mut cd_size = u64::from(read_u32_le(&eocd[12..16]));
        let mut cd_offset = u64::from(read_u32_le(&eocd[16..20]));

        // ZIP64 EOCD Locator (20 bytes immediately preceding EOCD)
        if (cd_offset == 0xFFFF_FFFF || cd_size == 0xFFFF_FFFF) && eocd_rel >= 20 {
            let locator = &tail[eocd_rel - 20..eocd_rel];
            if locator.starts_with(&[0x50, 0x4b, 0x06, 0x07]) {
                let zip64_eocd_offset = read_u64_le(&locator[8..16]);
                if zip64_eocd_offset + 56 <= file_len {
                    reader.seek(SeekFrom::Start(zip64_eocd_offset))?;
                    let mut z64 = [0u8; 56];
                    reader.read_exact(&mut z64)?;
                    if z64.starts_with(&[0x50, 0x4b, 0x06, 0x06]) {
                        cd_size = read_u64_le(&z64[40..48]);
                        cd_offset = read_u64_le(&z64[48..56]);
                    }
                }
            }
        }

        if cd_size > u64::try_from(MAX_CENTRAL_DIR_BYTES).unwrap_or(u64::MAX)
            || cd_offset.saturating_add(cd_size) > file_len
        {
            return Err(ArchiveError::Corrupt("ZIP central directory out of bounds"));
        }

        let cd_len = usize::try_from(cd_size).map_err(|_| ArchiveError::EntryTooLarge)?;
        reader.seek(SeekFrom::Start(cd_offset))?;
        let mut cd = vec![0u8; cd_len];
        reader.read_exact(&mut cd)?;

        let mut entries = Vec::new();
        let mut records = Vec::new();
        let mut pos = 0usize;

        while pos + 46 <= cd.len() {
            let hdr = &cd[pos..];
            if !hdr.starts_with(&[0x50, 0x4b, 0x01, 0x02]) {
                break;
            }
            if entries.len() >= MAX_ARCHIVE_ENTRIES {
                break;
            }

            let method = read_u16_le(&hdr[10..12]);
            let crc = read_u32_le(&hdr[16..20]);
            let mut comp_size = u64::from(read_u32_le(&hdr[20..24]));
            let mut uncomp_size = u64::from(read_u32_le(&hdr[24..28]));
            let name_len = usize::from(read_u16_le(&hdr[28..30]));
            let extra_len = usize::from(read_u16_le(&hdr[30..32]));
            let comment_len = usize::from(read_u16_le(&hdr[32..34]));
            let mut local_offset = u64::from(read_u32_le(&hdr[42..46]));

            let name_end = pos + 46 + name_len;
            let extra_end = name_end + extra_len;
            let record_end = extra_end + comment_len;
            if record_end > cd.len() {
                return Err(ArchiveError::Corrupt(
                    "truncated ZIP central directory entry",
                ));
            }

            let raw_name_bytes = &cd[pos + 46..name_end];
            let extra_bytes = &cd[name_end..extra_end];
            parse_zip64_extra(
                extra_bytes,
                &mut uncomp_size,
                &mut comp_size,
                &mut local_offset,
            );

            let is_dir = raw_name_bytes.ends_with(b"/") || raw_name_bytes.ends_with(b"\\");
            if !is_dir {
                let raw_name = String::from_utf8_lossy(raw_name_bytes);
                if let Some(name) = normalize_archive_path(&raw_name) {
                    entries.push(ArchiveEntry {
                        name,
                        uncompressed_size: uncomp_size,
                    });
                    records.push(ZipRecord {
                        method,
                        crc32: crc,
                        compressed_size: comp_size,
                        uncompressed_size: uncomp_size,
                        local_header_offset: local_offset,
                    });
                }
            }
            pos = record_end;
        }

        Ok(Self {
            reader,
            entries,
            records,
        })
    }

    pub fn entries(&self) -> &[ArchiveEntry] {
        &self.entries
    }

    pub fn read_entry(&mut self, index: usize, max_bytes: usize) -> Result<Vec<u8>, ArchiveError> {
        let rec = self
            .records
            .get(index)
            .cloned()
            .ok_or(ArchiveError::Corrupt("entry index out of bounds"))?;
        let max_u64 = u64::try_from(max_bytes).unwrap_or(u64::MAX);
        if rec.uncompressed_size > max_u64 || rec.compressed_size > max_u64.saturating_add(65_536) {
            return Err(ArchiveError::EntryTooLarge);
        }

        self.reader.seek(SeekFrom::Start(rec.local_header_offset))?;
        let mut lfh = [0u8; 30];
        self.reader.read_exact(&mut lfh)?;
        if !lfh.starts_with(&[0x50, 0x4b, 0x03, 0x04]) {
            return Err(ArchiveError::Corrupt("invalid ZIP local file header"));
        }
        let name_len = u64::from(read_u16_le(&lfh[26..28]));
        let extra_len = u64::from(read_u16_le(&lfh[28..30]));
        let data_offset = rec
            .local_header_offset
            .checked_add(30 + name_len + extra_len)
            .ok_or(ArchiveError::Corrupt("ZIP data offset overflow"))?;
        self.reader.seek(SeekFrom::Start(data_offset))?;

        let comp_len =
            usize::try_from(rec.compressed_size).map_err(|_| ArchiveError::EntryTooLarge)?;
        let mut compressed = vec![0u8; comp_len];
        self.reader.read_exact(&mut compressed)?;

        let output = match rec.method {
            0 => {
                if compressed.len() > max_bytes {
                    return Err(ArchiveError::EntryTooLarge);
                }
                compressed
            }
            8 => inflate(&compressed, max_bytes)?,
            other => return Err(ArchiveError::UnsupportedCompression(other)),
        };

        if u64::try_from(output.len()).unwrap_or(u64::MAX) != rec.uncompressed_size {
            return Err(ArchiveError::Corrupt("ZIP uncompressed size mismatch"));
        }
        if crc32(&output) != rec.crc32 {
            return Err(ArchiveError::Corrupt("ZIP CRC32 mismatch"));
        }
        Ok(output)
    }
}

fn find_eocd(tail: &[u8]) -> Option<usize> {
    if tail.len() < 22 {
        return None;
    }
    let mut i = tail.len() - 22;
    loop {
        if tail[i..i + 4] == [0x50, 0x4b, 0x05, 0x06] {
            let comment_len = usize::from(read_u16_le(&tail[i + 20..i + 22]));
            if i + 22 + comment_len == tail.len() {
                return Some(i);
            }
        }
        if i == 0 {
            break;
        }
        i -= 1;
    }
    // Fallback if trailing padding bytes exist after comment
    (0..=tail.len() - 22)
        .rev()
        .find(|&pos| tail[pos..pos + 4] == [0x50, 0x4b, 0x05, 0x06])
}

fn parse_zip64_extra(extra: &[u8], uncomp: &mut u64, comp: &mut u64, offset: &mut u64) {
    let mut pos = 0usize;
    while pos + 4 <= extra.len() {
        let id = read_u16_le(&extra[pos..pos + 2]);
        let size = usize::from(read_u16_le(&extra[pos + 2..pos + 4]));
        let end = pos + 4 + size;
        if end > extra.len() {
            break;
        }
        if id == 0x0001 {
            let mut field = &extra[pos + 4..end];
            if *uncomp == 0xFFFF_FFFF && field.len() >= 8 {
                *uncomp = read_u64_le(&field[..8]);
                field = &field[8..];
            }
            if *comp == 0xFFFF_FFFF && field.len() >= 8 {
                *comp = read_u64_le(&field[..8]);
                field = &field[8..];
            }
            if *offset == 0xFFFF_FFFF && field.len() >= 8 {
                *offset = read_u64_le(&field[..8]);
            }
            break;
        }
        pos = end;
    }
}

fn read_u16_le(b: &[u8]) -> u16 {
    u16::from_le_bytes([b[0], b[1]])
}

fn read_u32_le(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

fn read_u64_le(b: &[u8]) -> u64 {
    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

// ---------------------------------------------------------------------------
// POSIX / USTAR TAR Archive Reader
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct TarRecord {
    data_offset: u64,
    size: u64,
}

pub struct TarArchive<R: Read + Seek> {
    reader: R,
    entries: Vec<ArchiveEntry>,
    records: Vec<TarRecord>,
}

impl<R: Read + Seek> TarArchive<R> {
    pub fn open(mut reader: R) -> Result<Self, ArchiveError> {
        let file_len = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;

        let mut entries = Vec::new();
        let mut records = Vec::new();
        let mut pending_long_name: Option<String> = None;
        let mut offset = 0u64;

        while offset + 512 <= file_len && entries.len() < MAX_ARCHIVE_ENTRIES {
            reader.seek(SeekFrom::Start(offset))?;
            let mut block = [0u8; 512];
            reader.read_exact(&mut block)?;
            offset += 512;

            if block.iter().all(|&b| b == 0) {
                break;
            }
            if !verify_tar_checksum(&block) {
                return Err(ArchiveError::Corrupt("invalid TAR header checksum"));
            }

            let size = parse_tar_size(&block[124..136])?;
            let padded = size.div_ceil(512).saturating_mul(512);
            if offset.saturating_add(size) > file_len {
                return Err(ArchiveError::Corrupt("TAR entry exceeds archive size"));
            }

            let typeflag = block[156];
            match typeflag {
                b'L' => {
                    // GNU LongLink name
                    if size <= 4096 {
                        let len = usize::try_from(size).unwrap_or(0);
                        let mut buf = vec![0u8; len];
                        reader.read_exact(&mut buf)?;
                        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
                        pending_long_name = Some(String::from_utf8_lossy(&buf[..end]).into_owned());
                    }
                }
                b'x' => {
                    // PAX extended header: extract `path=...` if present
                    if size <= 16_384 {
                        let len = usize::try_from(size).unwrap_or(0);
                        let mut buf = vec![0u8; len];
                        reader.read_exact(&mut buf)?;
                        if let Some(pax_path) = parse_pax_path(&buf) {
                            pending_long_name = Some(pax_path);
                        }
                    }
                }
                0 | b'0' => {
                    let raw_name = pending_long_name
                        .take()
                        .unwrap_or_else(|| parse_ustar_name(&block));
                    if let Some(name) = normalize_archive_path(&raw_name) {
                        entries.push(ArchiveEntry {
                            name,
                            uncompressed_size: size,
                        });
                        records.push(TarRecord {
                            data_offset: offset,
                            size,
                        });
                    }
                }
                _ => {
                    pending_long_name = None;
                }
            }

            offset = offset.saturating_add(padded);
        }

        Ok(Self {
            reader,
            entries,
            records,
        })
    }

    pub fn entries(&self) -> &[ArchiveEntry] {
        &self.entries
    }

    pub fn read_entry(&mut self, index: usize, max_bytes: usize) -> Result<Vec<u8>, ArchiveError> {
        let rec = self
            .records
            .get(index)
            .cloned()
            .ok_or(ArchiveError::Corrupt("TAR entry index out of bounds"))?;
        let len = usize::try_from(rec.size).map_err(|_| ArchiveError::EntryTooLarge)?;
        if len > max_bytes {
            return Err(ArchiveError::EntryTooLarge);
        }
        self.reader.seek(SeekFrom::Start(rec.data_offset))?;
        let mut out = vec![0u8; len];
        self.reader.read_exact(&mut out)?;
        Ok(out)
    }
}

fn verify_tar_checksum(block: &[u8; 512]) -> bool {
    let Some(expected) = parse_octal(&block[148..156]) else {
        return false;
    };
    let mut unsigned_sum = 0u64;
    let mut signed_sum = 0i64;
    for (i, &b) in block.iter().enumerate() {
        let val = if (148..156).contains(&i) { b' ' } else { b };
        unsigned_sum += u64::from(val);
        signed_sum += i64::from(i8::from_be_bytes([val]));
    }
    expected == unsigned_sum || i64::try_from(expected).ok() == Some(signed_sum)
}

fn parse_tar_size(field: &[u8]) -> Result<u64, ArchiveError> {
    if field.first().copied() == Some(0x80) {
        let mut acc = 0u64;
        for &b in &field[4..] {
            acc = (acc << 8) | u64::from(b);
        }
        return Ok(acc);
    }
    parse_octal(field).ok_or(ArchiveError::Corrupt("invalid TAR octal size"))
}

fn parse_octal(field: &[u8]) -> Option<u64> {
    let trimmed: Vec<u8> = field
        .iter()
        .copied()
        .take_while(|&b| b != 0)
        .filter(|&b| b != b' ')
        .collect();
    if trimmed.is_empty() {
        return Some(0);
    }
    let mut val = 0u64;
    for b in trimmed {
        if !(b'0'..=b'7').contains(&b) {
            return None;
        }
        val = val.checked_mul(8)?.checked_add(u64::from(b - b'0'))?;
    }
    Some(val)
}

fn parse_ustar_name(block: &[u8; 512]) -> String {
    let cstr = |slice: &[u8]| -> String {
        let end = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
        String::from_utf8_lossy(&slice[..end]).into_owned()
    };
    let name = cstr(&block[0..100]);
    if block[257..262] == *b"ustar" {
        let prefix = cstr(&block[345..500]);
        if !prefix.is_empty() {
            return format!("{prefix}/{name}");
        }
    }
    name
}

fn parse_pax_path(buf: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(buf).ok()?;
    for line in text.lines() {
        if let Some((_len_part, rest)) = line.split_once(' ')
            && let Some(path) = rest.strip_prefix("path=")
        {
            return Some(path.to_owned());
        }
    }
    None
}
