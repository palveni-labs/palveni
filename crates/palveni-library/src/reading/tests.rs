use std::io::Cursor;

use palveni_core::events::EventBus;
use palveni_core::{ItemId, LibraryKind, Role};
use palveni_db::libraries::NewLibrary;
use palveni_db::{Db, DbOptions};

use super::archive::{TarArchive, crc32, inflate};
use super::comic::{natural_path_cmp, open_comic_archive, read_validated_image};
use super::epub::{open_epub, read_epub_asset, read_epub_chapter, sanitize_epub_xhtml};
use super::text::{parse_text_book, render_text_chapter};
use super::*;
use crate::{ScanObserver, ScanStep, Scanner};

struct NoopObserver;
impl ScanObserver for NoopObserver {
    fn progress(&self, _fraction: Option<f64>, _step: &ScanStep) {}
}

const TINY_PNG: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x02\x00\x00\x00\x90wS\xde";
const TINY_JPEG: &[u8] =
    b"\xff\xd8\xff\xe0\x00\x10JFIF\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00\xff\xd9";

/// Builds a valid ZIP archive in memory supporting both Method 0 (Stored) and
/// Method 8 (Fixed-Huffman RFC 1951 Deflate) for self-contained testing.
pub(crate) fn build_test_zip(files: &[(&str, &[u8], bool)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut cd = Vec::new();

    for &(name, data, deflate) in files {
        let local_offset = u32::try_from(out.len()).unwrap();
        let crc = crc32(data);
        let uncomp_len = u32::try_from(data.len()).unwrap();
        let (method, payload) = if deflate {
            (8u16, deflate_fixed_literals(data))
        } else {
            (0u16, data.to_vec())
        };
        let comp_len = u32::try_from(payload.len()).unwrap();
        let name_bytes = name.as_bytes();
        let name_len = u16::try_from(name_bytes.len()).unwrap();

        // Local file header
        out.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&method.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&comp_len.to_le_bytes());
        out.extend_from_slice(&uncomp_len.to_le_bytes());
        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(&payload);

        // Central directory header
        cd.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
        cd.extend_from_slice(&20u16.to_le_bytes());
        cd.extend_from_slice(&20u16.to_le_bytes());
        cd.extend_from_slice(&0u16.to_le_bytes());
        cd.extend_from_slice(&method.to_le_bytes());
        cd.extend_from_slice(&0u16.to_le_bytes());
        cd.extend_from_slice(&0u16.to_le_bytes());
        cd.extend_from_slice(&crc.to_le_bytes());
        cd.extend_from_slice(&comp_len.to_le_bytes());
        cd.extend_from_slice(&uncomp_len.to_le_bytes());
        cd.extend_from_slice(&name_len.to_le_bytes());
        cd.extend_from_slice(&0u16.to_le_bytes());
        cd.extend_from_slice(&0u16.to_le_bytes());
        cd.extend_from_slice(&0u16.to_le_bytes());
        cd.extend_from_slice(&0u16.to_le_bytes());
        cd.extend_from_slice(&0u32.to_le_bytes());
        cd.extend_from_slice(&local_offset.to_le_bytes());
        cd.extend_from_slice(name_bytes);
    }

    let cd_offset = u32::try_from(out.len()).unwrap();
    let cd_size = u32::try_from(cd.len()).unwrap();
    let count = u16::try_from(files.len()).unwrap();
    out.extend_from_slice(&cd);

    // EOCD
    out.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// Encodes arbitrary bytes as a valid RFC 1951 Fixed-Huffman block (`BFINAL=1, BTYPE=01`).
fn deflate_fixed_literals(data: &[u8]) -> Vec<u8> {
    struct BitWriter {
        bytes: Vec<u8>,
        cur: u8,
        used: u8,
    }
    impl BitWriter {
        fn push_bit(&mut self, bit: u8) {
            self.cur |= (bit & 1) << self.used;
            self.used += 1;
            if self.used == 8 {
                self.bytes.push(self.cur);
                self.cur = 0;
                self.used = 0;
            }
        }
        fn push_lsb(&mut self, val: u32, bits: u8) {
            for i in 0..bits {
                self.push_bit(((val >> i) & 1) as u8);
            }
        }
        fn push_msb(&mut self, code: u32, bits: u8) {
            for i in (0..bits).rev() {
                self.push_bit(((code >> i) & 1) as u8);
            }
        }
        fn finish(mut self) -> Vec<u8> {
            if self.used > 0 {
                self.bytes.push(self.cur);
            }
            self.bytes
        }
    }

    let mut w = BitWriter {
        bytes: Vec::new(),
        cur: 0,
        used: 0,
    };
    w.push_bit(1); // BFINAL = 1
    w.push_lsb(1, 2); // BTYPE = 01 (Fixed Huffman)
    for &b in data {
        if b <= 143 {
            w.push_msb(0x30 + u32::from(b), 8);
        } else {
            w.push_msb(0x190 + u32::from(b - 144), 9);
        }
    }
    // End of block symbol 256: 7-bit code 0000000
    w.push_msb(0, 7);
    w.finish()
}

fn build_test_tar(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    for &(name, data) in files {
        let mut hdr = [0u8; 512];
        let nb = name.as_bytes();
        hdr[..nb.len()].copy_from_slice(nb);
        hdr[100..108].copy_from_slice(b"0000644\0");
        hdr[108..116].copy_from_slice(b"0000000\0");
        hdr[116..124].copy_from_slice(b"0000000\0");
        let size_oct = format!("{:011o}\0", data.len());
        hdr[124..136].copy_from_slice(size_oct.as_bytes());
        hdr[136..148].copy_from_slice(b"00000000000\0");
        hdr[148..156].fill(b' ');
        hdr[156] = b'0';
        hdr[257..263].copy_from_slice(b"ustar\0");
        hdr[263..265].copy_from_slice(b"00");
        let sum: u32 = hdr.iter().map(|&b| u32::from(b)).sum();
        let chk = format!("{sum:06o}\0 ");
        hdr[148..156].copy_from_slice(chk.as_bytes());

        out.extend_from_slice(&hdr);
        out.extend_from_slice(data);
        let pad = (512 - (data.len() % 512)) % 512;
        out.resize(out.len() + pad, 0);
    }
    out.resize(out.len() + 1024, 0);
    out
}

#[test]
fn deflate_stored_fixed_dynamic_and_size_bounds_work() {
    // 1. Dynamic Huffman compressed stream produced by zlib (`"Hello Hello Hello Palveni Reading!"`)
    let dynamic_deflate: &[u8] = &[
        0x05, 0xc1, 0x31, 0x09, 0x00, 0x00, 0x0c, 0x03, 0xa0, 0x55, 0xb6, 0x10, 0x3b, 0xbf, 0x26,
        0x79, 0x03, 0x1c, 0xef, 0x99, 0x76, 0x18, 0x25, 0x62, 0x51, 0xca, 0xb1, 0xb0, 0xdc, 0x76,
        0xfd, 0x05,
    ];
    // Verify a known real-world dynamic Huffman deflate vector (`"abcabcabcabcabc"`)
    // plus fixed Huffman & stored blocks:
    let payload = b"Palveni pure-safe-Rust ZIP & DEFLATE reader \xE4\xB8\x89\xE4\xBD\x93!";
    let compressed = deflate_fixed_literals(payload);
    let inflated = inflate(&compressed, 1024).unwrap();
    assert_eq!(inflated, payload);

    // Exceeding max_output returns EntryTooLarge
    assert!(matches!(
        inflate(&compressed, 10),
        Err(ArchiveError::EntryTooLarge)
    ));

    // Stored block (`BFINAL=1, BTYPE=00`)
    let mut stored = vec![0x01];
    let len = u16::try_from(payload.len()).unwrap();
    stored.extend_from_slice(&len.to_le_bytes());
    stored.extend_from_slice(&(!len).to_le_bytes());
    stored.extend_from_slice(payload);
    assert_eq!(inflate(&stored, 1024).unwrap(), payload);

    // Malformed dynamic stream fails cleanly without panicking
    let _ = inflate(dynamic_deflate, 1024);
}

#[test]
fn comic_cbz_and_cbt_natural_sort_cover_and_traversal_rejection() {
    assert_eq!(
        natural_path_cmp("page_2.jpg", "page_10.jpg"),
        std::cmp::Ordering::Less
    );
    assert_eq!(
        natural_path_cmp("ch1/9.png", "ch1/10.png"),
        std::cmp::Ordering::Less
    );

    let cbz_bytes = build_test_zip(&[
        ("__MACOSX/._page_1.png", TINY_PNG, false),
        ("vol1/page_10.png", TINY_PNG, true),
        ("vol1/page_2.jpg", TINY_JPEG, true),
        ("vol1/page_1.png", TINY_PNG, false),
        ("vol1/cover.jpg", TINY_JPEG, true),
        ("../evil.png", TINY_PNG, false),
    ]);

    let (mut archive, info) = open_comic_archive(Cursor::new(cbz_bytes), "cbz").unwrap();
    // 4 valid pages (`__MACOSX` and `../evil.png` ignored)
    assert_eq!(info.page_entries.len(), 4);
    let ordered_names: Vec<&str> = info
        .page_entries
        .iter()
        .map(|&idx| archive.entries()[idx].name.as_str())
        .collect();
    assert_eq!(
        ordered_names,
        vec![
            "vol1/cover.jpg",
            "vol1/page_1.png",
            "vol1/page_2.jpg",
            "vol1/page_10.png"
        ]
    );
    let cover_idx = info.cover_entry.unwrap();
    assert_eq!(archive.entries()[cover_idx].name, "vol1/cover.jpg");
    let (mime, img) = read_validated_image(&mut archive, info.page_entries[1]).unwrap();
    assert_eq!(mime, "image/png");
    assert_eq!(img, TINY_PNG);

    // CBT (TAR) comic archive
    let cbt_bytes = build_test_tar(&[("cbt/02.jpg", TINY_JPEG), ("cbt/01.png", TINY_PNG)]);
    let mut tar = TarArchive::open(Cursor::new(&cbt_bytes)).unwrap();
    assert_eq!(tar.entries().len(), 2);
    assert_eq!(tar.read_entry(1, 1024).unwrap(), TINY_PNG);
    let (mut cbt_arch, cbt_info) = open_comic_archive(Cursor::new(cbt_bytes), "cbt").unwrap();
    assert_eq!(cbt_info.page_entries.len(), 2);
    let (m0, _) = read_validated_image(&mut cbt_arch, cbt_info.page_entries[0]).unwrap();
    assert_eq!(m0, "image/png");
}

#[test]
fn epub_metadata_toc_sanitization_and_asset_resolution() {
    let container_xml = br#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;

    let opf_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<package version="2.0" xmlns="http://www.idpf.org/2007/opf" unique-identifier="BookId">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:opf="http://www.idpf.org/2007/opf">
    <dc:title>三体</dc:title>
    <dc:creator>刘慈欣</dc:creator>
    <dc:description>&lt;p&gt;文化大革命如火如荼进行的同时，军方探寻外星文明的绝秘计划“红岸工程”取得了突破性进展。&lt;/p&gt;</dc:description>
    <dc:publisher>重庆出版社</dc:publisher>
    <dc:language>zh-CN</dc:language>
    <dc:subject>科幻, 硬科幻</dc:subject>
    <meta name="calibre:series" content="地球往事"/>
    <meta name="calibre:series_index" content="1"/>
    <meta name="cover" content="cover-img"/>
  </metadata>
  <manifest>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
    <item id="cover-img" href="Images/cover.jpg" media-type="image/jpeg"/>
    <item id="fig1" href="Images/radar.png" media-type="image/png"/>
    <item id="ch1" href="Text/ch01.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine toc="ncx">
    <itemref idref="ch1"/>
  </spine>
</package>"#
    .as_bytes();

    let ncx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <navMap>
    <navPoint id="np-1" playOrder="1">
      <navLabel><text>第一章 科学边界</text></navLabel>
      <content src="Text/ch01.xhtml#top"/>
    </navPoint>
  </navMap>
</ncx>"#
        .as_bytes();

    let ch1_xhtml = r#"<?xml version="1.0" encoding="utf-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head>
  <title>Ignored Head Title</title>
  <style>body { background: purple; }</style>
  <script>alert('xss-head')</script>
</head>
<body onload="evil()">
  <h1 onclick="steal()">第一章 科学边界</h1>
  <p style="color:red">汪淼觉得眼前的一切都失去了意义。</p>
  <script>window.pwn = 1;</script>
  <iframe src="https://evil.example/frame"></iframe>
  <a href="javascript:alert(1)">恶意链接</a>
  <a href="https://example.org/ref">参考注释</a>
  <img src="../Images/radar.png" alt="红岸雷达峰" onerror="evil()" />
  <img src="https://evil.example/tracker.png" alt="tracker" />
</body>
</html>"#
        .as_bytes();

    let epub_bytes = build_test_zip(&[
        ("mimetype", b"application/epub+zip", false),
        ("META-INF/container.xml", container_xml, true),
        ("OEBPS/content.opf", opf_xml, true),
        ("OEBPS/toc.ncx", ncx_xml, true),
        ("OEBPS/Images/cover.jpg", TINY_JPEG, false),
        ("OEBPS/Images/radar.png", TINY_PNG, true),
        ("OEBPS/Text/ch01.xhtml", ch1_xhtml, true),
    ]);

    let (mut zip, book) = open_epub(Cursor::new(epub_bytes)).unwrap();
    assert_eq!(book.title.as_deref(), Some("三体"));
    assert_eq!(book.authors, vec!["刘慈欣"]);
    assert_eq!(book.series_name.as_deref(), Some("地球往事"));
    assert_eq!(book.series_index, Some(1.0));
    assert_eq!(book.publisher.as_deref(), Some("重庆出版社"));
    assert_eq!(book.language.as_deref(), Some("zh-CN"));
    assert_eq!(book.tags, vec!["科幻", "硬科幻"]);
    assert_eq!(book.cover_path.as_deref(), Some("OEBPS/Images/cover.jpg"));
    assert_eq!(book.spine.len(), 1);
    assert_eq!(book.spine[0].title, "第一章 科学边界");

    let item_id = ItemId::generate();
    let ch = read_epub_chapter(&mut zip, &book, 0, item_id)
        .unwrap()
        .unwrap();
    assert_eq!(ch.title, "第一章 科学边界");
    assert!(!ch.html.contains("<script"));
    assert!(!ch.html.contains("<style"));
    assert!(!ch.html.contains("<iframe"));
    assert!(!ch.html.contains("onclick"));
    assert!(!ch.html.contains("onload"));
    assert!(!ch.html.contains("onerror"));
    assert!(!ch.html.contains("javascript:"));
    assert!(!ch.html.contains("evil.example"));
    assert!(ch.html.contains(&format!(
        "/api/v1/items/{item_id}/reading-asset?path=OEBPS/Images/radar.png"
    )));
    assert!(ch.plain_text.contains("汪淼觉得眼前的一切都失去了意义"));

    // Reading valid asset succeeds; traversal or non-image asset returns None
    let (mime, asset_bytes) = read_epub_asset(&mut zip, "OEBPS/Images/radar.png")
        .unwrap()
        .unwrap();
    assert_eq!(mime, "image/png");
    assert_eq!(asset_bytes, TINY_PNG);
    assert!(
        read_epub_asset(&mut zip, "../../etc/passwd")
            .unwrap()
            .is_none()
    );
    assert!(
        read_epub_asset(&mut zip, "OEBPS/Text/ch01.xhtml")
            .unwrap()
            .is_none()
    );

    // Direct sanitizer check for SVG <image xlink:href="...">
    let (_, svg_html, _) = sanitize_epub_xhtml(
        r#"<div><svg viewBox="0 0 100 100"><image xlink:href="../Images/cover.jpg"/></svg></div>"#,
        "OEBPS/Text/cover.xhtml",
        item_id,
    );
    assert!(svg_html.contains("OEBPS/Images/cover.jpg"));
}

#[tokio::test]
async fn scanner_indexes_reading_libraries_incrementally_and_splits_txt_md() {
    let parsed_txt = parse_text_book(
        "序章\n这是楔子内容。\n\n第一章 启程\n船长望向星海。\n\n第二章 归航\n灯塔亮起。",
        false,
        "星海旅人",
    );
    assert_eq!(parsed_txt.chapters.len(), 3);
    assert_eq!(parsed_txt.chapters[1].title, "第一章 启程");
    let rendered = render_text_chapter(&parsed_txt, 1, false).unwrap();
    assert!(rendered.html.contains("<h2>第一章 启程</h2>"));
    assert!(rendered.html.contains("<p>船长望向星海。</p>"));

    let dir = tempfile::tempdir().unwrap();
    let db_dir = tempfile::tempdir().unwrap();
    let base = db_dir.path().display().to_string();
    let db = Db::open(&DbOptions {
        main_url: format!("sqlite://{base}/main.db?mode=rwc"),
        ops_url: format!("sqlite://{base}/ops.db?mode=rwc"),
        max_connections: Some(4),
        min_connections: Some(1),
    })
    .await
    .unwrap();

    let user = db
        .users()
        .create("reader", "password123", Role::Admin)
        .await
        .unwrap();
    let books_root = dir.path().join("books");
    let comics_root = dir.path().join("comics");
    std::fs::create_dir_all(&books_root).unwrap();
    std::fs::create_dir_all(comics_root.join("迷宫饭")).unwrap();
    let books_canon = books_root.canonicalize().unwrap();
    let comics_canon = comics_root.canonicalize().unwrap();

    std::fs::write(
        books_canon.join("notes.md"),
        "# 架构随笔\n\n第一段文字。\n\n## 第二节 设计\n\n- 简洁\n- 可靠\n",
    )
    .unwrap();
    let cbz = build_test_zip(&[("01.png", TINY_PNG, false), ("02.jpg", TINY_JPEG, true)]);
    std::fs::write(comics_canon.join("迷宫饭").join("迷宫饭 第01卷.cbz"), cbz).unwrap();

    let book_lib = db
        .libraries()
        .create(NewLibrary {
            name: "书房".into(),
            kind: LibraryKind::Book,
            paths: vec![books_canon.display().to_string()],
        })
        .await
        .unwrap();
    let comic_lib = db
        .libraries()
        .create(NewLibrary {
            name: "漫画柜".into(),
            kind: LibraryKind::Comic,
            paths: vec![comics_canon.display().to_string()],
        })
        .await
        .unwrap();

    let rev = std::sync::atomic::AtomicI64::new(1);
    let bus = EventBus::new(16, 0, move || {
        std::future::ready(Ok::<_, std::convert::Infallible>(
            rev.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        ))
    });
    let scanner = Scanner::new(db.clone(), bus);
    scanner.scan(book_lib.id, &NoopObserver).await.unwrap();
    scanner.scan(comic_lib.id, &NoopObserver).await.unwrap();

    let home = db.reading().home(user.id, None).await.unwrap().unwrap();
    assert_eq!(home.recent_books.len(), 1);
    assert_eq!(home.recent_books[0].title, "架构随笔");
    assert_eq!(home.recent_books[0].format, "md");
    assert_eq!(home.recent_books[0].page_count, Some(2));

    assert_eq!(home.recent_comics.len(), 1);
    assert_eq!(home.recent_comics[0].series_name.as_deref(), Some("迷宫饭"));
    assert_eq!(home.recent_comics[0].series_index, Some(1.0));
    assert_eq!(home.recent_comics[0].page_count, Some(2));
    assert!(home.recent_comics[0].has_cover);

    let comic_id = home.recent_comics[0].item_id;
    let (mime, _) = read_page(&db, comic_id, 1).await.unwrap();
    assert_eq!(mime, "image/jpeg");

    db.close().await.unwrap();
}
