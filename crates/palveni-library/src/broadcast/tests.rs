use super::*;

#[test]
fn grouped_text_and_extinf_merge_lines_within_groups() {
    let text = "Local,#genre#\nNews,https://channels.example/news.m3u8$HD#https://backup.example/news\nNews,https://channels.example/news.m3u8\nOther,#genre#\nNews,https://channels.example/news2\n";
    let entries = parse_catalog(BroadcastKind::Iptv, text, None, None).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(
        entries[0].streams,
        [
            "https://channels.example/news.m3u8",
            "https://backup.example/news"
        ]
    );
    assert_eq!(entries[0].group.as_deref(), Some("Local"));
    assert_ne!(entries[0].key, entries[1].key);

    let m3u = "#EXTM3U\n#EXTINF:-1 group-title=\"News, local\" tvg-id=\"one\",Evening\nhttps://one.example/master.m3u8";
    let guide = r#"<!DOCTYPE tv SYSTEM "xmltv.dtd"><tv><channel id="one"><display-name>Local</display-name></channel><programme channel="one" start="20261201100000 +0800" stop="20261201103000 +0800"><title>Arts &amp; Culture</title></programme></tv>"#;
    let entries = parse_catalog(BroadcastKind::Iptv, m3u, Some(guide), None).unwrap();
    assert_eq!(entries[0].group.as_deref(), Some("News, local"));
    let p = &entries[0].programmes[0];
    assert_eq!(p.title, "Arts & Culture");
    assert_eq!(p.start.to_rfc3339(), "2026-12-01T02:00:00+00:00");
}

#[test]
fn rss_and_atom_use_enclosures_and_deduplicate_guids() {
    let rss = r#"<rss><channel><item><title>First &amp; foremost</title><guid>a</guid><pubDate>Thu, 5 Nov 2026 09:00:00 +0000</pubDate><enclosure type="audio/mpeg" url="../media/1.mp3"/></item><item><guid>a</guid><enclosure url="https://another.example/file"/></item><item><title>Article only</title><link>https://blog.example/text</link></item></channel></rss>"#;
    let base = Url::parse("https://pod.example/feeds/show.xml").unwrap();
    let entries = parse_catalog(BroadcastKind::Podcast, rss, None, Some(&base)).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, "First & foremost");
    assert_eq!(entries[0].streams, ["https://pod.example/media/1.mp3"]);
    assert!(entries[0].published_at.is_some());
    let atom = r#"<feed xmlns="http://www.w3.org/2005/Atom"><entry><id>episode:a</id><title><![CDATA[Film & Sound]]></title><link rel="alternate" href="https://site.example"/><link rel="enclosure" type="video/mp4" href="https://media.example/episode.mp4"/></entry></feed>"#;
    let entries = parse_catalog(BroadcastKind::Podcast, atom, None, None).unwrap();
    assert_eq!(entries[0].key, "episode:a");
    assert_eq!(entries[0].streams, ["https://media.example/episode.mp4"]);
}
#[test]
fn rejects_non_catalogues_and_unsafe_input_without_network_access() {
    assert!(
        parse_catalog(
            BroadcastKind::Iptv,
            "#EXTM3U\n#EXT-X-TARGETDURATION:5\n#EXTINF:5,\nhttps://media.example/1.ts",
            None,
            None
        )
        .is_err()
    );
    for xml in [
        "<!DOCTYPE rss><rss/>",
        "<rss><channel></rss>",
        "<rss/><rss/>",
    ] {
        assert!(parse_catalog(BroadcastKind::Podcast, xml, None, None).is_err());
    }
    for guide in [
        r#"<!DOCTYPE tv SYSTEM "other.dtd"><tv/>"#,
        r#"<!DOCTYPE tv [<!ENTITY sample "ignored">]><tv/>"#,
    ] {
        assert!(
            parse_catalog(
                BroadcastKind::Iptv,
                "Station,https://media.example/one",
                Some(guide),
                None
            )
            .is_err()
        );
    }
    for bad in [
        "file:///var/media",
        "javascript:alert(1)",
        "https://user:pw@media.example/file",
        "https://media.example/file?access_token=abc",
    ] {
        assert!(public_media_url(bad, None).is_none());
    }
}
