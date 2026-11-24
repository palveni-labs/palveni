use std::collections::HashSet;

use palveni_core::plugins::PluginRuntimeKind;
use palveni_core::{ItemId, ItemKind, LibraryKind, Role};
use serde_json::json;

use super::Harness;
use crate::DbError;
use crate::items::ItemUpsert;
use crate::libraries::NewLibrary;
use crate::photos::PhotoMetadataUpsert;
use crate::plugins_photo_ai::{PhotoAiTagUpsert, PluginInstallationUpsert};

fn make_photo_item(title: &str, path: &str, size: i64, modified: i64) -> ItemUpsert {
    ItemUpsert {
        music: None,
        id: ItemId::generate(),
        parent_id: None,
        kind: ItemKind::Photo,
        title: title.to_owned(),
        sort_title: title.to_lowercase(),
        search_text: title.to_lowercase(),
        path: path.to_owned(),
        size_bytes: Some(size),
        modified_at: Some(modified),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_plugins_and_photo_ai_tables() {
    let h = Harness::new().await;

    // -----------------------------------------------------------------------
    // 1. plugin_installations CRUD, upgrade previous_version & rollback
    // -----------------------------------------------------------------------
    let installed = h
        .db
        .plugins_photo_ai()
        .upsert_plugin_installation(PluginInstallationUpsert {
            plugin_id: "palveni.recipe.tmdb".into(),
            name: "TMDB Metadata Recipe".into(),
            version: "1.1.0".into(),
            previous_version: None,
            runtime: PluginRuntimeKind::Recipe,
            provides: vec!["metadata".into()],
            permissions: vec![
                "network:api.themoviedb.org".into(),
                "library:write_metadata".into(),
            ],
            granted_permissions: vec![
                "network:api.themoviedb.org".into(),
                "library:write_metadata".into(),
            ],
            settings_schema: json!({"type": "object", "properties": {"language": {"type": "string"}}}),
            settings: json!({"language": "zh-CN"}),
            enabled: true,
            source: "official".into(),
        })
        .await
        .unwrap();
    assert_eq!(installed.version, "1.1.0");
    assert_eq!(installed.previous_version, None);
    assert!(installed.enabled);

    // Rollback with no previous_version fails cleanly
    assert!(matches!(
        h.db.plugins_photo_ai()
            .rollback_plugin_installation("palveni.recipe.tmdb")
            .await,
        Err(DbError::Invalid(_))
    ));

    // Upgrade to 1.2.0 records previous_version = 1.1.0
    let upgraded = h
        .db
        .plugins_photo_ai()
        .upsert_plugin_installation(PluginInstallationUpsert {
            plugin_id: "palveni.recipe.tmdb".into(),
            name: "TMDB Metadata Recipe".into(),
            version: "1.2.0".into(),
            previous_version: Some("1.1.0".into()),
            runtime: PluginRuntimeKind::Recipe,
            provides: vec!["metadata".into()],
            permissions: vec![
                "network:api.themoviedb.org".into(),
                "library:write_metadata".into(),
            ],
            granted_permissions: vec![
                "network:api.themoviedb.org".into(),
                "library:write_metadata".into(),
            ],
            settings_schema: json!({"type": "object", "properties": {"language": {"type": "string"}}}),
            settings: json!({"language": "en-US"}),
            enabled: true,
            source: "official".into(),
        })
        .await
        .unwrap();
    assert_eq!(upgraded.version, "1.2.0");
    assert_eq!(upgraded.previous_version.as_deref(), Some("1.1.0"));

    // Patch enabled & settings
    let patched =
        h.db.plugins_photo_ai()
            .update_plugin_installation(
                "palveni.recipe.tmdb",
                Some(false),
                Some(json!({"language": "ja-JP"})),
            )
            .await
            .unwrap();
    assert!(!patched.enabled);
    assert_eq!(patched.settings["language"], "ja-JP");

    // Rollback swaps back to 1.1.0 and clears previous_version
    let rolled =
        h.db.plugins_photo_ai()
            .rollback_plugin_installation("palveni.recipe.tmdb")
            .await
            .unwrap();
    assert_eq!(rolled.version, "1.1.0");
    assert_eq!(rolled.previous_version, None);

    assert_eq!(
        h.db.plugins_photo_ai()
            .list_installed_plugins()
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        h.db.plugins_photo_ai()
            .delete_plugin_installation("palveni.recipe.tmdb")
            .await
            .unwrap()
    );
    assert!(
        h.db.plugins_photo_ai()
            .get_installed_plugin("palveni.recipe.tmdb")
            .await
            .unwrap()
            .is_none()
    );

    // -----------------------------------------------------------------------
    // 2. photo_ai_tags, photo_people & photo_person_tags (FK & user isolation)
    // -----------------------------------------------------------------------
    let alice =
        h.db.users()
            .create("alice", "pw", Role::User)
            .await
            .unwrap()
            .id;
    let bob =
        h.db.users()
            .create("bob", "pw", Role::Admin)
            .await
            .unwrap()
            .id;

    let lib =
        h.db.libraries()
            .create(NewLibrary {
                name: "Photos".into(),
                kind: LibraryKind::Photo,
                paths: vec!["/photos".into()],
            })
            .await
            .unwrap();

    let p1 = make_photo_item("IMG_1", "/photos/IMG_1.jpg", 2_000_000, 1_760_000_000_000);
    let p2 = make_photo_item("IMG_2", "/photos/IMG_2.jpg", 3_000_000, 1_760_100_000_000);
    let (id1, id2) = (p1.id, p2.id);

    h.db.items()
        .apply_scan_batch(lib.id, &[p1, p2], &[])
        .await
        .unwrap();

    h.db.photos()
        .upsert_batch(
            lib.id,
            &[
                PhotoMetadataUpsert {
                    item_id: id1,
                    stamp: "s1".into(),
                    taken_at: 1_760_000_000_000,
                    day_bucket: "2025-10-09".into(),
                    month_bucket: "2025-10".into(),
                    width: Some(4000),
                    height: Some(3000),
                    orientation: 1,
                    camera_make: Some("SONY".into()),
                    camera_model: Some("A7M4".into()),
                    lens_model: None,
                    iso: Some(100),
                    f_number: Some(2.8),
                    exposure_time: None,
                    focal_length: None,
                    latitude: Some(31.2),
                    longitude: Some(121.4),
                    content_hash: Some("a".repeat(64)),
                },
                PhotoMetadataUpsert {
                    item_id: id2,
                    stamp: "s2".into(),
                    taken_at: 1_760_100_000_000,
                    day_bucket: "2025-10-10".into(),
                    month_bucket: "2025-10".into(),
                    width: Some(3000),
                    height: Some(4000),
                    orientation: 1,
                    camera_make: Some("Apple".into()),
                    camera_model: Some("iPhone 15".into()),
                    lens_model: None,
                    iso: Some(200),
                    f_number: Some(1.8),
                    exposure_time: None,
                    focal_length: None,
                    latitude: None,
                    longitude: None,
                    content_hash: Some("b".repeat(64)),
                },
            ],
        )
        .await
        .unwrap();

    let unanalyzed =
        h.db.plugins_photo_ai()
            .list_unanalyzed_photos(None, 50)
            .await
            .unwrap();
    assert_eq!(unanalyzed.len(), 2);

    let (tagged_before, rem_before) =
        h.db.plugins_photo_ai()
            .count_photo_ai_progress(None)
            .await
            .unwrap();
    assert_eq!((tagged_before, rem_before), (0, 2));

    h.db.plugins_photo_ai()
        .upsert_photo_ai_tags(&[PhotoAiTagUpsert {
            item_id: id1,
            dhash: Some(0x1234_5678_9abc_def0_u64 as i64),
            dominant_color: Some("blue".into()),
            dominant_hex: Some("#2868A8".into()),
            scene_tags: vec!["landscape".into(), "geotagged".into(), "blue".into()],
            time_of_day: Some("night".into()),
            season: Some("autumn".into()),
            analyzed_at: 1_760_200_000_000,
        }])
        .await
        .unwrap();

    let (tagged_after, rem_after) =
        h.db.plugins_photo_ai()
            .count_photo_ai_progress(None)
            .await
            .unwrap();
    assert_eq!((tagged_after, rem_after), (1, 1));

    let tag =
        h.db.plugins_photo_ai()
            .get_photo_ai_tag(id1)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(tag.dominant_color.as_deref(), Some("blue"));
    assert_eq!(tag.dominant_hex.as_deref(), Some("#2868A8"));
    assert_eq!(tag.scene_tags, vec!["landscape", "geotagged", "blue"]);

    // People CRUD & auto-cover + owner isolation
    let person =
        h.db.plugins_photo_ai()
            .create_person(alice, "Alice Chen", None)
            .await
            .unwrap();
    assert_eq!(person.photo_count, 0);
    assert_eq!(person.cover_item_id, None);
    assert!(
        h.db.plugins_photo_ai()
            .get_person(bob, person.id, None)
            .await
            .unwrap()
            .is_none()
    );

    // Tagging auto-fills cover_item_id
    let tagged_person =
        h.db.plugins_photo_ai()
            .tag_person_on_item(
                alice,
                id1,
                person.id,
                Some(r#"{"x":0.2,"y":0.2,"w":0.3,"h":0.3}"#.into()),
            )
            .await
            .unwrap();
    assert_eq!(tagged_person.photo_count, 1);
    assert_eq!(tagged_person.cover_item_id, Some(id1));

    // Tagging second photo preserves existing cover
    let tagged_person2 =
        h.db.plugins_photo_ai()
            .tag_person_on_item(alice, id2, person.id, None)
            .await
            .unwrap();
    assert_eq!(tagged_person2.photo_count, 2);
    assert_eq!(tagged_person2.cover_item_id, Some(id1));

    // Accessible photos list includes AI tag + people
    let mut allowed = HashSet::new();
    allowed.insert(lib.id);
    let candidates =
        h.db.plugins_photo_ai()
            .list_accessible_photos_for_ai(alice, Some(&allowed))
            .await
            .unwrap();
    assert_eq!(candidates.len(), 2);
    let c1 = candidates.iter().find(|c| c.id == id1).unwrap();
    assert_eq!(c1.dominant_color.as_deref(), Some("blue"));
    assert_eq!(c1.people.len(), 1);
    assert_eq!(c1.people[0].name, "Alice Chen");

    // Rename & untag cover falls back to remaining item
    let renamed =
        h.db.plugins_photo_ai()
            .update_person(alice, person.id, Some("Alice Lin"), None)
            .await
            .unwrap();
    assert_eq!(renamed.name, "Alice Lin");

    assert!(
        h.db.plugins_photo_ai()
            .untag_person_from_item(alice, id1, person.id)
            .await
            .unwrap()
    );
    let after_untag =
        h.db.plugins_photo_ai()
            .get_person(alice, person.id, None)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(after_untag.photo_count, 1);
    assert_eq!(after_untag.cover_item_id, Some(id2));

    assert!(
        !h.db
            .plugins_photo_ai()
            .delete_person(bob, person.id)
            .await
            .unwrap()
    );
    assert!(
        h.db.plugins_photo_ai()
            .delete_person(alice, person.id)
            .await
            .unwrap()
    );

    h.finish().await;
}
