use super::*;

fn track(id: &str, title: &str, artist: &str, album: &str) -> SearchTrack {
    SearchTrack {
        id: id.into(),
        title: title.into(),
        artist: Some("Guest".into()),
        artist_key: artist.to_lowercase(),
        album_artist: Some(artist.into()),
        album_key: album.to_lowercase(),
        album: Some(album.into()),
    }
}

#[test]
fn pinyin_and_simplified_queries_sort_and_page_groups() {
    let mut builder = Builder::default();
    builder.extend(vec![
        track("1", "鑽石", "海邊合奏", "早春"),
        track("2", "愛之歌", "另一位", "早春"),
        track("3", "海邊%_", "海邊合奏", "夜曲"),
    ]);
    let index = builder.finish(LibraryId::generate(), 1);
    for q in ["海边%_", "hb%_", "haibian%_", "海邊 夜曲 Guest"] {
        let hits = index
            .matches(MusicSearchKind::Track, &terms(q).unwrap(), 0, 30)
            .unwrap()
            .entries;
        assert_eq!(hits.len(), 1, "{q}");
        assert_eq!(hits[0].id, "3", "{q}");
    }
    let guests = index
        .matches(MusicSearchKind::Track, &terms("guest").unwrap(), 0, 3)
        .unwrap();
    assert_eq!(
        guests
            .entries
            .iter()
            .map(|x| x.id.as_str())
            .collect::<Vec<_>>(),
        ["2", "3", "1"]
    );
    let first = index
        .matches(MusicSearchKind::Album, &terms("zc").unwrap(), 0, 1)
        .unwrap();
    let second = index
        .matches(
            MusicSearchKind::Album,
            &terms("zc").unwrap(),
            first.next.unwrap(),
            1,
        )
        .unwrap();
    assert_ne!(first.entries[0].artist_id, second.entries[0].artist_id);
    assert!(second.next.is_none());
    assert!(terms("   ").is_err());
    assert!(terms("1 2 3 4 5 6 7 8 9").is_err());
    assert!(terms(&"歌".repeat(129)).is_err());
}

#[test]
fn incremental_changes_keep_shared_groups_and_remove_empty_ones() {
    let mut builder = Builder::default();
    builder.extend(vec![
        track("1", "One", "A", "Sessions"),
        track("2", "Two", "A", "Sessions"),
        track("3", "Three", "B", "Sessions"),
    ]);
    let mut index = builder.finish(LibraryId::generate(), 3);
    let ids = HashSet::from(["1".to_owned(), "3".to_owned()]);
    assert_eq!(index.affected_groups(&ids).len(), 4);
    index.apply(
        &ids,
        vec![track("3", "Encore", "C", "Finale")],
        vec![track("2", "Two", "A", "Sessions")],
        4,
    );
    assert_eq!(index.revision, 4);
    assert_eq!(index.artists.len(), 2);
    assert_eq!(index.albums.len(), 2);
    assert_eq!(index.tracks.len(), 2);
    let albums = index
        .matches(MusicSearchKind::Album, &terms("sessions").unwrap(), 0, 30)
        .unwrap()
        .entries;
    assert_eq!(albums[0].artist.as_deref(), Some("A"));
    let ids = HashSet::from(["2".to_owned()]);
    index.apply(&ids, Vec::new(), Vec::new(), 5);
    assert!(
        index
            .matches(MusicSearchKind::Album, &terms("sessions").unwrap(), 0, 30)
            .unwrap()
            .entries
            .is_empty()
    );
}
