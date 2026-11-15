use super::*;
use crate::source::tests::{unique_temp_dir, write_minimal_ghost_descript};

// ------------------------------------------------------------------
// load_restored_state（唯一の IO 点・8.1 前哨・6.1/6.3 寛容縮退）
// ------------------------------------------------------------------

/// 起動時先読み: profile_areka_root(shiori.dir) に植えた sylphya.toml が
/// `(PersistKey, String)` エントリとして読み戻る（load_scope 契約経由・8.1 前哨）。
///
/// ゴーストは `ghost/master/descript.txt` だけでシェルのフォルダを持たない。永続の置き場は
/// SHIORI の置き場だけで決まり、シェルの解決に縛られないこと（既定のシェルが無くても読める）も
/// ここで確かめる。
#[test]
fn load_restored_state_reads_planted_sylphya_toml() {
    let temp = unique_temp_dir("load-reads-planted");
    let root = temp.path().to_path_buf();
    write_minimal_ghost_descript(&root);
    assert!(!root.join("shell").exists(), "前提: シェルのフォルダが無い");
    // profile root = <ghost/master>/profile/areka（boot 結線と同一構築）。
    let profile = profile_areka_root(&root.join("ghost").join("master"));
    std::fs::create_dir_all(&profile).expect("create profile/areka");
    std::fs::write(
        profile.join("sylphya.toml"),
        "format-version = 1\n[window.0]\nx = \"1486\"\ny = \"353\"\n[boot]\ncount = \"1\"\n"
            .as_bytes(),
    )
    .expect("plant sylphya.toml");

    let entries = load_restored_state(&root);

    assert!(
        entries.contains(&(
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::X
            },
            "1486".to_string()
        )),
        "植えた WindowPos.x が読める: {entries:?}"
    );
    assert!(
        entries.contains(&(
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::Y
            },
            "353".to_string()
        )),
        "植えた WindowPos.y が読める: {entries:?}"
    );
    assert!(
        entries.contains(&(PersistKey::BootCount, "1".to_string())),
        "植えた BootCount が読める: {entries:?}"
    );
}

/// 起動時先読み: 永続ファイル不在（profile 未作成）→ 空（load_scope の不在縮退・6.1）。
#[test]
fn load_restored_state_absent_file_is_empty() {
    let temp = unique_temp_dir("load-absent-file");
    let root = temp.path().to_path_buf();
    write_minimal_ghost_descript(&root); // ゴーストは読めるが sylphya.toml は植えない
    let entries = load_restored_state(&root);
    assert!(entries.is_empty(), "永続ファイル不在は空縮退: {entries:?}");
}

/// 起動時先読み: mount 解決失敗（ghost_root 不在）→ 空 Vec（寛容縮退・panic なし・6.1/6.3）。
/// 空は「記憶のファイルが無い」と同じ結果なので、ゴーストを読めなかったことは `warn!` の
/// 1 行だけが伝える（その行が ghost_root を運ぶ）。
#[test]
fn load_restored_state_mount_resolve_failure_is_empty() {
    let temp = unique_temp_dir("load-mount-fail");
    let root = temp.path().join("no_such_ghost");
    let (entries, events) = capture(|| load_restored_state(&root));
    assert!(
        entries.is_empty(),
        "mount 解決失敗は空縮退（起動を止めない）"
    );
    let warns: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .collect();
    assert_eq!(
        warns.len(),
        1,
        "読めないゴーストの warn が 1 行: {events:?}"
    );
    assert!(
        warns[0]
            .field("ghost_root")
            .is_some_and(|v| v.contains("no_such_ghost")),
        "warn が ghost_root を運ぶ: {:?}",
        warns[0]
    );
}

// ------------------------------------------------------------------
// PersistWiring / persist_entries・forget_entries（送り口・design C1 State Management・
//   Req1.1/1.9/6.2/7.1・Testing Strategy §1/§6）
//   NonSend リソース存在 → persist_put（Ghost 固定・fire-and-forget）／不在 → debug!＋no-op。
// ------------------------------------------------------------------

use super::finalize_save_tests::Store;
use log_capture_kit::capture;

/// 存在檻（7.1/1.1）: headless World に `PersistWiring` を挿入して `persist_entries` を呼ぶと、
/// Ghost スコープへ write-through され、barrier 後に別ハンドルの `load_scope` で読み戻せる
/// （＝`persist_put` が実際に投函され実 IO へ確定した証明）。
#[test]
fn persist_entries_with_wiring_write_through_to_ghost_scope() {
    let store = Store::new();
    let mut world = World::new();
    store.wire(&mut world);

    // char_pos_entries を persist_entries 経由で投函（Ghost 固定はヘルパ内で強制・7.1）。
    persist_entries(&world, char_pos_entries(0, PointPx { x: 1486, y: 353 }));

    let loaded = store.load();
    for (axis, want) in [(Axis::X, "1486"), (Axis::Y, "353")] {
        assert_eq!(
            loaded
                .get(&PersistKey::WindowPos { scope: 0, axis })
                .map(String::as_str),
            Some(want),
            "persist_entries が Ghost へ write-through していない（{axis:?}）: {loaded:?}"
        );
    }
    store.finish();
}

/// 不在縮退檻（6.2/design「PersistWiring 不在は debug!＋no-op」）: `PersistWiring` 未挿入の
/// World で `forget_entries` を呼ぶと、panic せず `debug!` を 1 行残して何もしない
/// （`persist_entries` の同じ行は `follow_keyword_base_tests.rs` の
/// `a_drag_on_a_side_scope_is_a_no_op_for_the_material` が確かめる）。
#[test]
fn forget_entries_without_wiring_logs_debug_and_does_nothing() {
    let world = World::new();
    let keys = vec![PersistKey::BalloonOffset {
        scope: 0,
        axis: Axis::X,
    }];
    let ((), events) = capture(|| forget_entries(&world, keys));
    let hits: Vec<_> = events
        .iter()
        .filter(|e| e.message().contains("鍵を消す投函を no-op"))
        .collect();
    assert_eq!(hits.len(), 1, "不在の debug! が 1 行: {events:?}");
    assert_eq!(hits[0].level, tracing::Level::DEBUG);
    assert_eq!(hits[0].field("key_count"), Some("1"));
}
