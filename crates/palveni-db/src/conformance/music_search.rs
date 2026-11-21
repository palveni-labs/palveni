use super::*;
use palveni_core::music::TrackTags;

#[tokio::test]
async fn conformance_music_search_revision_and_scoped_keysets() {
    let h = Harness::new().await;
    let db = &h.db;
    let library = new_library(db, "Music search fixture").await.id;
    let other = new_library(db, "Another music library").await.id;
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
    assert!(matches!(
        db.music().search_revision(library).await,
        Err(DbError::NotFound(_))
    ));
    h.finish().await;
}
