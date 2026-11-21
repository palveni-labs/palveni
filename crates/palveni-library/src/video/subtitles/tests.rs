use super::*;
use std::fs;

#[test]
fn parses_conventional_language_names_and_dispositions() {
    for (name, lang, forced, sdh) in [
        ("srt", None, false, false),
        ("ENG.FORCED.SRT", Some("en"), true, false),
        ("zh-Hant-TW.sdh.vtt", Some("zh-Hant-TW"), false, true),
        ("繁體中文.srt", Some("zh-Hant"), false, false),
        ("English.cc.srt", Some("en"), false, true),
    ] {
        let t = descriptor(name).unwrap();
        assert_eq!(
            (t.language.as_deref(), t.forced, t.hearing_impaired),
            (lang, forced, sdh)
        );
    }
    for bad in [
        "../en.srt",
        "en/clip.srt",
        "en.us.fr.srt",
        "en-US-US.srt",
        "cc.cc.srt",
        "en.ass",
        ".srt",
    ] {
        assert!(descriptor(bad).is_none(), "{bad}");
    }
}

#[test]
fn renders_browser_cues_preserving_text_and_discards_global_styles() {
    let srt =
        "\u{feff}10\r\n01:15:22,012 --> 01:15:27,918\r\n<i>山 & 海</i>\r\n<x>X</x> &amp; Y\r\n";
    let vtt = webvtt::render(srt.as_bytes(), SubtitleFormat::Srt).unwrap();
    assert_eq!(
        vtt,
        "WEBVTT\n\n01:15:22.012 --> 01:15:27.918\n<i>山 &amp; 海</i>\n&lt;x&gt;X&lt;/x&gt; &amp; Y\n\n"
    );
    let source = "WEBVTT\n\nSTYLE\n::cue { background: red; }\n\nNOTE cue notes\nPrivate annotations\n\ncue name\n01:02.120 --> 01:04.350 align:start\nClosed captions\n";
    assert_eq!(
        webvtt::render(source.as_bytes(), SubtitleFormat::Vtt).unwrap(),
        "WEBVTT\n\n00:01:02.120 --> 00:01:04.350\nClosed captions\n\n"
    );
    for bytes in [
        &b"WEBVTT\n\n00:04.000 --> 00:01.000\ntext"[..],
        b"not webvtt",
        &[0xff],
        b"WEBVTT\n\n00:00.999 --> 00:99.777\ntext",
    ] {
        assert!(webvtt::render(bytes, SubtitleFormat::Vtt).is_none());
    }
}

#[cfg(unix)]
#[tokio::test]
async fn finds_only_same_video_regular_sidecars_and_reopens_with_bounds() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("films");
    fs::create_dir(&root).unwrap();
    let root = root.canonicalize().unwrap();
    let location = || TrackLocation {
        cue_frames: None,
        root: root.clone(),
        relative_path: Path::new("Film.2024.mp4").to_owned(),
    };
    fs::write(root.join("Film.2024.mp4"), "frame").unwrap();
    fs::write(
        root.join("Film.2024.zh.srt"),
        "1\n00:00:00,300 --> 00:00:01,200\n你好",
    )
    .unwrap();
    fs::write(root.join("Film.en.srt"), "another film").unwrap();
    fs::write(root.join("Film.2024.it.srt"), vec![0; MAX_BYTES + 1]).unwrap();
    fs::write(dir.path().join("other.vtt"), "outside").unwrap();
    symlink(dir.path().join("other.vtt"), root.join("Film.2024.en.vtt")).unwrap();
    let entries = list(location()).await.unwrap();
    assert_eq!(entries.tracks.len(), 1);
    assert_eq!(entries.tracks[0].id, "zh.srt");
    assert!(!entries.truncated);
    assert!(read(location(), "zh.srt").await.unwrap().contains("你好"));
    assert!(read(location(), "en.vtt").await.is_err());
    assert!(read(location(), "it.srt").await.is_err());
    fs::remove_file(root.join("Film.2024.zh.srt")).unwrap();
    assert!(read(location(), "zh.srt").await.is_err());
    assert!(list(location()).await.unwrap().tracks.is_empty());
    fs::remove_file(root.join("Film.2024.mp4")).unwrap();
    assert!(list(location()).await.is_err());
    fs::create_dir(root.join("Film.2024.mp4")).unwrap();
    assert!(matches!(list(location()).await, Err(Error::NotFound(_))));
}
