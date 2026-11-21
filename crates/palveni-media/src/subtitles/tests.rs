use super::*;

#[test]
fn reports_only_text_streams_convertible_with_the_installed_tools() {
    let formats = " E webvtt WebVTT\n D mov,mp4,m4a QuickTime\n D hls Playlist\n";
    let codecs = " DES... webvtt WebVTT\n DES... mov_text MOV text\n D.S... dvd_subtitle DVD bitmap\n .ES... subrip SubRip\n";
    let caps = capabilities(formats, codecs);
    assert_eq!(caps.demuxers, "mov");
    let probe = br#"{"streams":[
        {"index":1,"codec_type":"audio","codec_name":"webvtt"},
        {"index":2,"codec_type":"subtitle","codec_name":"dvd_subtitle"},
        {"index":3,"codec_type":"subtitle","codec_name":"subrip"},
        {"index":4,"codec_type":"subtitle","codec_name":"mov_text","tags":{"language":"eng"},"disposition":{"forced":1}},
        {"index":6,"codec_type":"subtitle","codec_name":"webvtt","disposition":{"hearing_impaired":1}}
    ]}"#;
    let tracks = parse_tracks(probe, &caps);
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].index, 4);
    assert_eq!(tracks[0].language.as_deref(), Some("eng"));
    assert!(tracks[0].forced);
    assert!(!tracks[0].hearing_impaired);
    assert_eq!(tracks[1].index, 6);
    assert_eq!(tracks[1].language, None);
    assert!(tracks[1].hearing_impaired);

    assert!(
        capabilities(formats, " D.S... webvtt\n")
            .decoders
            .is_empty()
    );
    assert!(
        capabilities(" D webvtt\n D mov\n", codecs)
            .decoders
            .is_empty()
    );
}
