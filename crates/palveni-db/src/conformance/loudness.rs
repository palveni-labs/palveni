use super::*;
use palveni_core::music::LoudnessValues;

#[tokio::test]
async fn conformance_loudness_skip_changed_and_deleted_tracks() {
    let h = Harness::new().await;
    let library = new_library(&h.db, "Loudness fixtures").await.id;
    let other = new_library(&h.db, "Other fixtures").await.id;
    let mut source = track("Tone", "/media/tone.wav", None, 32);
    h.db.items()
        .apply_scan_batch(library, std::slice::from_ref(&source), &[])
        .await
        .unwrap();
    let music = h.db.music();
    let first = music.loudness_page(library, "").await.unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].measured_revision, None);
    assert!(music.loudness(source.id).await.unwrap().is_none());
    let value = LoudnessValues {
        integrated_lufs: Some(-23.14),
        range_lu: Some(0.0),
        true_peak_dbtp: Some(-19.2),
    };
    music
        .save_loudness(source.id, first[0].revision, value)
        .await
        .unwrap();
    let saved = music.loudness(source.id).await.unwrap().unwrap();
    assert!(saved.current);
    assert_eq!(saved.integrated_lufs, Some(-23.14));
    assert!(music.loudness_page(other, "").await.unwrap().is_empty());
    assert!(
        music
            .loudness_page(library, &first[0].id)
            .await
            .unwrap()
            .is_empty()
    );
    source.size_bytes = Some(124);
    h.db.items()
        .apply_scan_batch(library, std::slice::from_ref(&source), &[])
        .await
        .unwrap();
    let prior = music.loudness(source.id).await.unwrap().unwrap();
    assert!(!prior.current);
    assert_eq!(prior.measured_at, saved.measured_at);
    let current = music.loudness_page(library, "").await.unwrap();
    music
        .save_loudness(
            source.id,
            current[0].revision,
            LoudnessValues {
                integrated_lufs: None,
                range_lu: None,
                true_peak_dbtp: None,
            },
        )
        .await
        .unwrap();
    let silence = music.loudness(source.id).await.unwrap().unwrap();
    assert!(
        silence.current && silence.integrated_lufs.is_none() && silence.true_peak_dbtp.is_none()
    );
    h.db.items()
        .apply_scan_batch(library, &[], &[source.id])
        .await
        .unwrap();
    assert!(matches!(
        music.loudness(source.id).await,
        Err(DbError::NotFound(_))
    ));
    h.finish().await;
}
