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

#[test]
fn embedded_tracks_keep_sidecars_and_share_the_list_limit() {
    let mut list = SubtitleList {
        tracks: vec![descriptor("fr.vtt").unwrap()],
        truncated: false,
    };
    append_embedded(&mut list, 3, Some("ENG"), true, false);
    append_embedded(&mut list, 5, Some("und"), false, true);
    append_embedded(&mut list, 7, Some("bad-tag!"), false, false);
    let en = &list.tracks[1];
    assert_eq!(list.tracks[0].id, "fr.vtt");
    assert_eq!(
        (&*en.id, en.language.as_deref()),
        ("embedded-3", Some("en"))
    );
    assert!(en.forced && !en.hearing_impaired);
    assert_eq!(en.format, SubtitleFormat::Vtt);
    assert!(list.tracks[2].hearing_impaired);
    assert!(list.tracks[2].language.is_none() && list.tracks[3].language.is_none());
    for i in 4..u32::try_from(MAX_TRACKS).unwrap() {
        append_embedded(&mut list, i + 10, None, false, false);
    }
    assert!(!list.truncated);
    append_embedded(&mut list, 100, None, false, false);
    assert!(list.truncated);
    assert_eq!(list.tracks.len(), MAX_TRACKS);
}

#[tokio::test]
async fn limits_embedded_subtitles_after_escaping_cue_text() {
    let mut content = b"WEBVTT\n\n00:00.250 --> 00:01.400\n".to_vec();
    content.resize(MAX_BYTES / 4, b'&');
    assert!(matches!(
        render_embedded(content).await,
        Err(Error::Invalid(_))
    ));
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
    let (entries, mut input) = list(location()).await.unwrap();
    let mut contents = String::new();
    input.read_to_string(&mut contents).unwrap();
    assert_eq!(contents, "frame");
    assert_eq!(entries.tracks.len(), 1);
    assert_eq!(entries.tracks[0].id, "zh.srt");
    assert!(!entries.truncated);
    assert!(
        matches!(read(location(), "zh.srt").await.unwrap(), SubtitleSource::WebVtt(text) if text.contains("你好"))
    );
    assert!(matches!(
        read(location(), "embedded-2").await.unwrap(),
        SubtitleSource::Embedded { index: 2, .. }
    ));
    for id in [
        "embedded-",
        "embedded-01",
        "embedded-+1",
        "embedded-2.srt",
        "embedded-4294967296",
    ] {
        assert!(
            matches!(read(location(), id).await, Err(Error::Invalid(_))),
            "{id}"
        );
    }
    assert!(read(location(), "en.vtt").await.is_err());
    assert!(read(location(), "it.srt").await.is_err());
    fs::remove_file(root.join("Film.2024.zh.srt")).unwrap();
    assert!(read(location(), "zh.srt").await.is_err());
    assert!(list(location()).await.unwrap().0.tracks.is_empty());
    fs::remove_file(root.join("Film.2024.mp4")).unwrap();
    assert!(list(location()).await.is_err());
    symlink(dir.path().join("other.vtt"), root.join("Film.2024.mp4")).unwrap();
    assert!(matches!(
        read(location(), "embedded-2").await,
        Err(Error::NotFound(_))
    ));
    fs::remove_file(root.join("Film.2024.mp4")).unwrap();
    fs::create_dir(root.join("Film.2024.mp4")).unwrap();
    assert!(matches!(list(location()).await, Err(Error::NotFound(_))));
}
