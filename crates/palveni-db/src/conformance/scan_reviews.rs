use super::*;
use palveni_core::music::{CueSegment, TrackTags};

#[tokio::test]
async fn conformance_scan_reviews_are_stable_paged_and_versioned_per_outage() {
    let h = Harness::new().await;
    let library = new_library(&h.db, "Snapshot").await.id;
    let other = new_library(&h.db, "Other").await.id;
    let reviews = h.db.scan_reviews();
    let scope = "/music/lost";
    let mut missing: Vec<_> = (0..521)
        .map(|i| (ItemId::generate(), format!("{scope}/{i:03}.flac")))
        .collect();
    missing.reverse();
    let (a, b) = tokio::join!(
        reviews.record(library, scope, &missing),
        reviews.record(library, scope, &missing)
    );
    let a = a.unwrap();
    for malformed in [format!("A{}", &a.revision[1..]), format!("{} ", a.revision)] {
        assert!(!reviews.contains(library, &malformed).await.unwrap());
        assert!(!reviews.clear(library, &malformed).await.unwrap());
    }
    assert_eq!(b.unwrap(), a);
    assert_eq!(a.missing_count, 521);
    let paths: Vec<_> = missing.iter().rev().map(|(_, p)| p.clone()).collect();
    assert_eq!(a.sample_paths, paths[..10]);
    let revision_before = h.db.current_revision().await.unwrap();
    missing.reverse();
    assert_eq!(reviews.record(library, scope, &missing).await.unwrap(), a);
    assert_eq!(h.db.current_revision().await.unwrap(), revision_before);
    let reopened = Db::open(&h.options).await.unwrap();
    let saved = reopened.scan_reviews();
    assert_eq!(saved.list(library).await.unwrap(), vec![a.clone()]);
    assert!(
        saved
            .paths(other, &a.revision, 0, 10)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        saved.paths(library, &a.revision, 417, 100).await.unwrap(),
        Some((paths[417..517].to_vec(), 521))
    );
    assert_eq!(
        saved.paths(library, &a.revision, 517, 50).await.unwrap(),
        Some((paths[517..].to_vec(), 521))
    );
    assert_eq!(
        saved
            .paths(library, &a.revision, u64::MAX, 50)
            .await
            .unwrap(),
        Some((vec![], 521))
    );
    reopened.close().await.unwrap();
    let changed = reviews.record(library, scope, &missing[1..]).await.unwrap();
    assert_ne!(a.revision, changed.revision);
    assert!(!reviews.clear(library, &a.revision).await.unwrap());
    assert!(!reviews.contains(other, &changed.revision).await.unwrap());
    assert!(
        reviews
            .paths(library, &a.revision, 0, 50)
            .await
            .unwrap()
            .is_none()
    );
    assert!(reviews.clear(library, &changed.revision).await.unwrap());
    assert!(reviews.list(library).await.unwrap().is_empty());
    let again = reviews.record(library, scope, &missing).await.unwrap();
    assert_ne!(again.revision, a.revision);
    let mut new_id = missing.clone();
    new_id[0].0 = ItemId::generate();
    let replaced = reviews.record(library, scope, &new_id).await.unwrap();
    assert_ne!(replaced.revision, again.revision);
    new_id[1].1 = format!("{scope}/renamed.flac");
    let renamed = reviews.record(library, scope, &new_id).await.unwrap();
    assert_ne!(renamed.revision, replaced.revision);
    let unaffected = reviews
        .record(
            library,
            "/music/other",
            &[(ItemId::generate(), "/music/other/z".to_owned())],
        )
        .await
        .unwrap();
    assert_eq!(
        reviews.list(library).await.unwrap(),
        vec![renamed.clone(), unaffected]
    );
    assert!(h.db.libraries().delete(library).await.unwrap());
    assert!(reviews.list(library).await.unwrap().is_empty());
    assert!(!reviews.contains(library, &renamed.revision).await.unwrap());
    h.finish().await;
}

#[tokio::test]
async fn conformance_scan_review_confirmation_is_atomic_and_scoped() {
    let h = Harness::new().await;
    let library = new_library(&h.db, "Confirmation").await.id;
    let other = new_library(&h.db, "Foreign").await.id;
    let scope = "/music/lost";
    let mut songs: Vec<_> = (0..510)
        .map(|i| track("Music", &format!("{scope}/{i:03}.flac"), None, 42))
        .collect();
    songs[0].music = Some(TrackTags::default());
    for batch in songs.chunks(40) {
        h.db.items()
            .apply_scan_batch(library, batch, &[])
            .await
            .unwrap();
    }
    let dir = folder("Album", "/music/lost/album", None);
    let mut cue = track("Movement", "/music/lost/suite.cue/01", None, 42);
    cue.music = Some(TrackTags {
        cue: Some(CueSegment {
            source: songs[0].path.clone(),
            start_frame: 0,
            end_frame: 75,
            stamp: "test".into(),
        }),
        ..Default::default()
    });
    let outside = folder("Other", "/music/lost-and-found", None);
    let extra = track("Survivor", "/music/lost/survivor.flac", None, 42);
    h.db.items()
        .apply_scan_batch(
            library,
            &[dir.clone(), cue.clone(), outside.clone(), extra.clone()],
            &[],
        )
        .await
        .unwrap();
    let foreign = track("Foreign", "/music/lost/foreign.flac", None, 42);
    h.db.items()
        .apply_scan_batch(other, std::slice::from_ref(&foreign), &[])
        .await
        .unwrap();
    let missing = songs
        .iter()
        .map(|s| (s.id, s.path.clone()))
        .collect::<Vec<_>>();
    let reviews = h.db.scan_reviews();
    let review = reviews.record(library, scope, &missing).await.unwrap();
    let ids = songs.iter().map(|s| s.id).collect::<Vec<_>>();
    let rev = h.db.current_revision().await.unwrap();
    for wrong in [foreign.id, outside.id, extra.id] {
        let mut attempted = ids.clone();
        attempted.push(wrong);
        assert!(
            !reviews
                .apply_confirmed(library, &review.revision, &attempted)
                .await
                .unwrap()
        );
    }
    assert!(
        !reviews
            .apply_confirmed(library, &review.revision, &ids[..509])
            .await
            .unwrap()
    );
    assert!(
        !reviews
            .apply_confirmed(other, &review.revision, &ids)
            .await
            .unwrap()
    );
    assert_eq!(h.db.current_revision().await.unwrap(), rev);
    assert!(reviews.contains(library, &review.revision).await.unwrap());
    assert!(h.db.items().get(songs[0].id).await.unwrap().is_some());
    let on_mount = reviews
        .record_on(library, scope, "mount-a", &missing)
        .await
        .unwrap();
    assert_ne!(on_mount.revision, review.revision);
    assert!(
        reviews
            .matches_source(library, &on_mount.revision, "mount-a")
            .await
            .unwrap()
    );
    assert!(
        !reviews
            .matches_source(library, &on_mount.revision, "mount-b")
            .await
            .unwrap()
    );
    assert!(
        !reviews
            .apply_confirmed_on(library, &on_mount.revision, "mount-b", &ids)
            .await
            .unwrap()
    );
    assert!(reviews.contains(library, &on_mount.revision).await.unwrap());
    let review = reviews.record(library, scope, &missing).await.unwrap();
    assert_ne!(review.revision, on_mount.revision);
    let mut confirmed = ids;
    confirmed.extend([cue.id, dir.id]);
    let (one, two) = tokio::join!(
        reviews.apply_confirmed(library, &review.revision, &confirmed),
        reviews.apply_confirmed(library, &review.revision, &confirmed)
    );
    assert_ne!(one.unwrap(), two.unwrap());
    assert!(reviews.list(library).await.unwrap().is_empty());
    for id in [songs[0].id, songs[509].id, dir.id, cue.id] {
        assert!(h.db.items().get(id).await.unwrap().is_none());
    }
    assert!(
        h.db.music()
            .indexed_item_ids(library)
            .await
            .unwrap()
            .is_empty()
    );
    for id in [outside.id, foreign.id, extra.id] {
        assert!(h.db.items().get(id).await.unwrap().is_some());
    }
    h.finish().await;
}
