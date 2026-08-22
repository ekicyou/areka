use super::*;

// ------------------------------------------------------------------
// load_restored_state（唯一の IO 点・8.1 前哨・6.1/6.3 寛容縮退）
// ------------------------------------------------------------------

/// このテスト専用の一意な一時ディレクトリ（source.rs と同規約・外部 tempfile 非依存）。
fn unique_temp_dir(tag: &str) -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("areka_placement_persist_tests_{tag}"));
    dir
}

/// 最小ゴーストパッケージ（ghost/master/descript.txt ＋ shell/master dir）を組む。
/// resolve が成功する最小構成（source.rs の失敗経路テストと同型）。
fn plant_minimal_ghost(root: &Path) {
    let _ = std::fs::remove_dir_all(root);
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        "charset,UTF-8\nname,テスト\nsakura.name,さくら\n".as_bytes(),
    )
    .expect("write ghost descript");
    std::fs::create_dir_all(root.join("shell").join("master")).expect("create shell/master");
}

/// 起動時先読み: profile_areka_root(shiori.dir) に植えた sylphya.toml が
/// `(PersistKey, String)` エントリとして読み戻る（load_scope 契約経由・8.1 前哨）。
#[test]
fn load_restored_state_reads_planted_sylphya_toml() {
    let root = unique_temp_dir("load_reads_planted");
    plant_minimal_ghost(&root);
    // profile root = <ghost/master>/profile/areka（boot 結線と同一構築）。
    let profile = profile_areka_root(&root.join("ghost").join("master"));
    std::fs::create_dir_all(&profile).expect("create profile/areka");
    std::fs::write(
        profile.join("sylphya.toml"),
        "format-version = 1\n[window.0]\nx = \"1486\"\ny = \"353\"\n[boot]\ncount = \"1\"\n"
            .as_bytes(),
    )
    .expect("plant sylphya.toml");

    let entries = load_restored_state(&root, DefaultEncoding::Ansi);

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

    let _ = std::fs::remove_dir_all(&root);
}

/// 起動時先読み: 永続ファイル不在（profile 未作成）→ 空（load_scope の不在縮退・6.1）。
#[test]
fn load_restored_state_absent_file_is_empty() {
    let root = unique_temp_dir("load_absent_file");
    plant_minimal_ghost(&root); // resolve は成功するが sylphya.toml は植えない
    let entries = load_restored_state(&root, DefaultEncoding::Ansi);
    assert!(entries.is_empty(), "永続ファイル不在は空縮退: {entries:?}");
    let _ = std::fs::remove_dir_all(&root);
}

/// 起動時先読み: mount 解決失敗（ghost_root 不在）→ 空 Vec（寛容縮退・panic なし・6.1/6.3）。
#[test]
fn load_restored_state_mount_resolve_failure_is_empty() {
    let root = unique_temp_dir("load_mount_fail").join("no_such_ghost");
    let entries = load_restored_state(&root, DefaultEncoding::Ansi);
    assert!(
        entries.is_empty(),
        "mount 解決失敗は空縮退（起動を止めない）"
    );
}

// ------------------------------------------------------------------
// PersistWiring / persist_entries（保存投函ヘルパ・design C1 State Management・
//   task 2.1・Req1.1/1.9/6.2/7.1・Testing Strategy §1/§6）
//   NonSend リソース存在 → persist_put（Ghost 固定・fire-and-forget）／不在 → debug!＋no-op。
// ------------------------------------------------------------------

use areka_sylphya::persist::{FakePersistIo, PersistIo};
use areka_sylphya::{SylphyaInit, spawn_sylphya};
use std::sync::Arc;

/// 共有 fake IO（アクターへ `Box<dyn PersistIo>` として移送しつつ、同一ストアを別ハンドルの
/// `load_scope` で観測するための newtype ラッパ。actor.rs の write-through 檻と同流儀）。
struct SharedFakeIo(Arc<FakePersistIo>);
impl PersistIo for SharedFakeIo {
    fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
        self.0.read(path)
    }
    fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
        self.0.commit(path, content)
    }
}

/// 存在檻（7.1/1.1）: headless World に `PersistWiring` を挿入して `persist_entries` を呼ぶと、
/// Ghost スコープへ write-through され、barrier 後に別ハンドルの `load_scope` で読み戻せる
/// （＝`persist_put` が実際に投函され実 IO へ確定した証明）。
#[test]
fn persist_entries_with_wiring_write_through_to_ghost_scope() {
    // 共有 fake IO（アクター Box 移送用と観測用で同一ストアを指す）。
    let shared = Arc::new(FakePersistIo::new());
    let roots = ScopeRoots {
        ghost: Some(std::path::PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    let parts = spawn_sylphya(SylphyaInit {
        roots: roots.clone(),
        io: Box::new(SharedFakeIo(shared.clone())),
        runtime_sink: None,
    });

    // World へ NonSend リソースとして挿入（UI スレッド常駐・MouseWiring/Emo2Wiring 先例）。
    let mut world = World::new();
    world.insert_non_send(PersistWiring {
        publisher: parts.publisher.clone(),
    });

    // char_pos_entries を persist_entries 経由で投函（Ghost 固定はヘルパ内で強制・7.1）。
    let entries = char_pos_entries(0, PointPx { x: 1486, y: 353 });
    persist_entries(&world, entries);

    // barrier 復帰 = 上記 put の write-through 保存（save_scope）まで完了（同一送信端 FIFO）。
    parts
        .publisher
        .barrier()
        .expect("barrier should resolve while actor is alive");

    // アクターと同一ストアを別ハンドルの load_scope で観測（実 IO 通過＝persist_put 投函の証明）。
    let loaded = load_scope(PersistScope::Ghost, &roots, &SharedFakeIo(shared.clone()));
    assert!(
        loaded.contains(&(
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::X
            },
            "1486".to_string()
        )),
        "persist_entries が Ghost へ write-through していない（WindowPos.x）: {loaded:?}"
    );
    assert!(
        loaded.contains(&(
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::Y
            },
            "353".to_string()
        )),
        "persist_entries が Ghost へ write-through していない（WindowPos.y）: {loaded:?}"
    );

    // 正典終了（アクター join）——テスト後始末（リーク回避・非本質）。
    parts.publisher.close();
    let _ = parts.handle.join();
}

/// 不在縮退檻（6.2/design「PersistWiring 不在は debug!＋no-op」）: `PersistWiring` 未挿入の
/// World で `persist_entries` を呼んでも panic せず no-op で戻る（fallback boot 経路の縮退）。
#[test]
fn persist_entries_without_wiring_is_noop_and_never_panics() {
    let world = World::new(); // PersistWiring 未挿入（fallback 未挿入経路を模す）。
    // panic しない・no-op（debug ログのみ）。ここへ到達＝縮退成功。
    persist_entries(&world, char_pos_entries(0, PointPx { x: 1, y: 2 }));
    persist_entries(&world, balloon_offset_entries(0, PointPx { x: -400, y: 0 }));
}
