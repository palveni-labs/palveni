use super::*;
use palveni_core::music::TrackTags;

#[tokio::test]
async fn conformance_music_search_revision_and_scoped_keysets() {
    let h = Harness::new().await;
    let db = &h.db;
    let library = new_library(db, "Music search fixture").await.id;
    let other = new_library(db, "Another music library").await.id;
    db.libraries()
        .create(NewLibrary {
            name: "Spoken search fixture".into(),
            kind: LibraryKind::Audiobook,
            paths: vec!["/media/spoken".into()],
        })
        .await
        .unwrap();
    let initial = db.music().search_catalog().await.unwrap();
    assert_eq!(initial.libraries.len(), 2);
    assert_eq!(db.music().search_revision(library).await.unwrap(), 0);
    let mut one = track("One", "/media/one.flac", None, 32);
    one.music = Some(TrackTags {
        title: Some("夜曲".into()),
        artist: Some("Guest".into()),
        album_artist: Some("Ensemble".into()),
        album: Some("Sessions".into()),
        ..Default::default()
    });
    let mut two = track("Two", "/media/two.flac", None, 32);
    two.music = one.music.clone();
    db.items()
        .apply_scan_batch(library, &[one.clone(), two.clone()], &[])
        .await
        .unwrap();
    let revision = db.music().search_revision(library).await.unwrap();
    assert!(revision > 0);
    let catalog = db.music().search_catalog().await.unwrap();
    assert_ne!(initial.stamp, catalog.stamp);
    assert!(
        catalog
            .libraries
            .iter()
            .any(|l| l.id == library && l.name == "Music search fixture" && l.revision == revision)
    );
    assert!(
        catalog
            .libraries
            .iter()
            .any(|l| l.id == other && l.revision == 0)
    );
    assert_eq!(db.music().search_revision(other).await.unwrap(), 0);
    let rows = db.music().search_tracks(library, "").await.unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows[0].id < rows[1].id);
    assert_eq!(rows[0].title, "夜曲");
    assert_eq!(rows[0].artist.as_deref(), Some("Guest"));
    assert_eq!(rows[0].album_artist.as_deref(), Some("Ensemble"));
    assert_eq!(
        db.music()
            .search_tracks(library, &rows[0].id)
            .await
            .unwrap()[0]
            .id,
        rows[1].id
    );
    assert!(
        db.music()
            .search_tracks(other, "")
            .await
            .unwrap()
            .is_empty()
    );
    two.music.as_mut().unwrap().title = Some("Encore".into());
    db.items()
        .apply_scan_batch(library, &[two.clone()], &[one.id])
        .await
        .unwrap();
    let edit_revision = db.music().search_revision(library).await.unwrap();
    assert!(edit_revision > revision);
    let changes = db
        .music()
        .search_changes(library, revision)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(changes.ids.len(), 2);
    assert_eq!(changes.tracks.len(), 1);
    let current = &changes.tracks[0];
    assert_eq!(current.id, two.id.to_string());
    assert_eq!(
        db.music()
            .search_group_sample(library, &current.artist_key, Some(&current.album_key))
            .await
            .unwrap()
            .unwrap()
            .id,
        current.id
    );
    assert!(
        db.music()
            .search_group_sample(other, &current.artist_key, None)
            .await
            .unwrap()
            .is_none()
    );

    assert_eq!(
        db.music().search_tracks(library, "").await.unwrap()[0].title,
        "Encore"
    );
    db.items()
        .apply_scan_batch(library, &[], &[two.id])
        .await
        .unwrap();
    assert!(db.music().search_revision(library).await.unwrap() > edit_revision);
    assert!(
        db.music()
            .search_tracks(library, "")
            .await
            .unwrap()
            .is_empty()
    );
    db.libraries().delete(library).await.unwrap();
    let catalog = db.music().search_catalog().await.unwrap();
    assert_eq!(catalog.libraries.len(), 1);
    assert_eq!(catalog.libraries[0].id, other);
    assert!(matches!(
        db.music().search_revision(library).await,
        Err(DbError::NotFound(_))
    ));
    h.finish().await;
}

#[tokio::test]
async fn conformance_cue_tracks_have_separate_files_catalog_and_revived_ids() {
    use palveni_core::music::CueSegment;
    let h = Harness::new().await;
    let db = &h.db;
    let library = new_library(db, "Cue").await.id;
    let mut backing = track("Suite", "/media/Cue/Suite.flac", None, 100);
    backing.music = Some(TrackTags {
        cue_backing: true,
        ..Default::default()
    });
    let mut part = track("Adagio", "/media/Cue/Suite.cue/02", None, 2396);
    part.id = crate::music::cue_id(library, &part.path);
    part.music = Some(TrackTags {
        cue: Some(CueSegment {
            source: backing.path.clone(),
            start_frame: 16,
            end_frame: 17,
            stamp: crate::music::cue_fingerprint("cue"),
        }),
        album: Some("Suite".into()),
        ..Default::default()
    });
    db.items()
        .apply_scan_batch(library, &[backing.clone(), part.clone()], &[])
        .await
        .unwrap();
    let physical = all_titles(
        db,
        ItemQuery {
            library_id: library,
            parent_id: None,
            kind: None,
            limit: 50,
            cursor: None,
        },
    )
    .await;
    assert_eq!(physical, ["Suite"]);
    assert_eq!(
        db.music().search_tracks(library, "").await.unwrap().len(),
        1
    );
    assert!(
        db.music()
            .catalog_song(&backing.id.to_string())
            .await
            .unwrap()
            .is_none()
    );
    let catalog = db
        .music()
        .catalog_song(&part.id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!((catalog.cue_start, catalog.cue_end), (Some(16), Some(17)));
    let loc = db.music().audio_location(part.id).await.unwrap().unwrap();
    assert_eq!(loc.relative_path, std::path::Path::new("Suite.flac"));
    assert_eq!(loc.cue_frames, Some(16..17));
    assert!(
        db.items()
            .get(part.id)
            .await
            .unwrap()
            .unwrap()
            .path
            .is_none()
    );
    let mark = db.music().search_revision(library).await.unwrap();
    backing.music = Some(TrackTags::default());
    db.items()
        .apply_scan_batch(library, &[backing.clone()], &[part.id])
        .await
        .unwrap();
    assert!(db.music().audio_location(part.id).await.unwrap().is_none());
    let delta = db
        .music()
        .search_changes(library, mark)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(delta.tracks[0].id, backing.id.to_string());
    backing.music.as_mut().unwrap().cue_backing = true;
    db.items()
        .apply_scan_batch(library, &[backing, part.clone()], &[])
        .await
        .unwrap();
    assert_eq!(
        db.music().search_tracks(library, "").await.unwrap()[0].id,
        part.id.to_string()
    );
    h.finish().await;
}
