use super::*;
use areka_sylphya::persist::{FakePersistIo, PersistIo};
use areka_sylphya::{
    Axis, PersistKey, PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya,
};
use placement::persist::{PersistWiring, char_pos_entries, persist_entries};
use placement::resolver::PointPx;
use std::path::Path;
use std::sync::Arc;

/// 共有 fake IO（アクターへ `Box<dyn PersistIo>` として移送しつつ、同一ストアを別ハンドルの
/// `load_scope` で観測するための newtype ラッパ。persist.rs の write-through 檻と同流儀）。
struct SharedFakeIo(Arc<FakePersistIo>);
impl PersistIo for SharedFakeIo {
    fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
        self.0.read(path)
    }
    fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
        self.0.commit(path, content)
    }
}

/// 挿入シーム檻（3.1／1.9／C4/C5）: `insert_persist_wiring` で headless World へ実 publisher を
/// 挿入すると、(a) NonSend `PersistWiring` が存在し、(b) その World 越しの `persist_entries` 投函が
/// Ghost スコープへ write-through され、barrier 後に別ハンドルの `load_scope` で読み戻せる。
#[test]
fn insert_persist_wiring_establishes_world_conduit_reaching_the_store() {
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

    // wired／fallback 両経路が使う本物の挿入ヘルパで World へ結線する。
    let mut world = World::new();
    insert_persist_wiring(&mut world, parts.publisher.clone());

    // (a) NonSend リソース PersistWiring が World に存在する。
    assert!(
        world.get_non_send_resource::<PersistWiring>().is_some(),
        "insert_persist_wiring 後、World に PersistWiring が挿入されているべき（C4/C5）"
    );

    // (b) その World 越しの persist_entries 投函がストアへ到達する（DragEnd→file の World シーム）。
    let entries = char_pos_entries(0, PointPx { x: 1486, y: 353 });
    persist_entries(&world, entries);

    // barrier 復帰＝上記 put の write-through 保存（save_scope）まで完了（同一送信端 FIFO）。
    parts
        .publisher
        .barrier()
        .expect("barrier should resolve while actor is alive");

    // アクターと同一ストアを別ハンドルの load_scope で観測（実 IO 通過＝投函の証明）。
    let loaded = load_scope(PersistScope::Ghost, &roots, &SharedFakeIo(shared.clone()));
    assert!(
        loaded.contains(&(
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::X
            },
            "1486".to_string()
        )),
        "World 導管越しの persist_entries が Ghost へ write-through していない（WindowPos.x）: {loaded:?}"
    );
    assert!(
        loaded.contains(&(
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::Y
            },
            "353".to_string()
        )),
        "World 導管越しの persist_entries が Ghost へ write-through していない（WindowPos.y）: {loaded:?}"
    );

    // 正典終了（アクター join）——テスト後始末（リーク回避・非本質）。
    parts.publisher.close();
    let _ = parts.handle.join();
}
