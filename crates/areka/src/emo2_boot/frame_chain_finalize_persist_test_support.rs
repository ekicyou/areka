//! 並べ直しから記憶までを通すテストの共有の部品（areka-P0-char-position-save-on-exit タスク 3.2・
//! 要件 5.1・5.2・5.3・design Testing Strategy「並べ直しから記憶まで」）。
//!
//! 使い手は 3 つのテストファイル（`frame_chain_finalize_persist_tests.rs`・
//! `frame_chain_finalize_persist_restart_tests.rs`・`frame_chain_finalize_persist_nowrite_tests.rs`）。
//! `frame_test_support.rs` は 1,000 行に近いので、こちらへ分けて置く。
//!
//! 置くのは 3 つの口:
//! - 毎フレームの処理の World に記憶の送り口（`PersistWiring`）を置く口 [`PersistStore`]
//!   （偽の保存先 `FakePersistIo` と、ワークツリーの `target\` の下の実物のファイルの両方）。
//! - 任意の配置（起動の準備の関数 `crate::restore_merged_placements` が返した配置）から窓を作り、
//!   準備の幅と違う幅の絵を出して置き直す（再スナップ）口 [`spawn_shown`]。同じ幅のまま組むと、
//!   書くとき（出ている幅の半分を足す）・戻すとき（準備の幅の半分を引く）・絵が出て中央を保って
//!   付け替えるとき、の 3 つの計算を通らずに緑になる。
//! - `target\` の下に最小のゴーストを植える口 [`MinimalGhost`]（OS の一時フォルダは使わない）。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use areka_actor::ActorHandle;
use areka_ghost::sylphya_wiring::profile_areka_root;
use areka_sylphya::persist::{FakePersistIo, FsPersistIo, PersistIo};
use areka_sylphya::{PersistKey, PersistScope, ScopeRoots, SylphyaInit, SylphyaPublisher};
use areka_sylphya::{load_scope, spawn_sylphya};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use wintf::ecs::WindowHandle;

use super::test_support::PerTargetSizes;
use super::*;
use crate::placement::follow::{MonitorSnapshot, OffsetBase};
use crate::placement::resolver::{Anchor, PointPx, RectPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::spawn_ghost_windows;

// ── 記憶の送り口 ────────────────────────────────────────────────────────────

/// アクターへ渡す分と、読み戻す分が同じ保存先を指すための包み。
struct SharedIo(Arc<dyn PersistIo + Sync>);

impl PersistIo for SharedIo {
    fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
        self.0.read(path)
    }
    fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
        self.0.commit(path, content)
    }
}

/// 記憶の保存先と、それを書く sylphya のアクター（本番と同じ `spawn_sylphya`）。
pub(super) struct PersistStore {
    io: Arc<dyn PersistIo + Sync>,
    roots: ScopeRoots,
    publisher: SylphyaPublisher,
    /// 偽の保存先のとき、その本体（書き込みを失敗させる口 [`Self::fail_next_commit`] に使う）。
    fake: Option<Arc<FakePersistIo>>,
    /// 破棄で止めて待つ（テストが途中で panic しても、植えたゴーストを消す前に書き手が止まる）。
    handle: Option<ActorHandle>,
}

impl PersistStore {
    /// 偽の保存先（`FakePersistIo`）。
    pub(super) fn fake() -> Self {
        let roots = ScopeRoots {
            ghost: Some(PathBuf::from("/g")),
            ..ScopeRoots::default()
        };
        let fake = Arc::new(FakePersistIo::new());
        let mut store = Self::spawn(fake.clone(), roots);
        store.fake = Some(fake);
        store
    }

    /// 植えたゴーストの実物のファイル（本番と同じ置き場 `ghost\master\profile\areka\`・
    /// 本番と同じ `FsPersistIo`）。起動の準備の関数がそのまま読み直せる。
    pub(super) fn real(ghost: &MinimalGhost) -> Self {
        let roots = ScopeRoots {
            ghost: Some(profile_areka_root(&ghost.ghost_master())),
            ..ScopeRoots::default()
        };
        Self::spawn(Arc::new(FsPersistIo), roots)
    }

    fn spawn(io: Arc<dyn PersistIo + Sync>, roots: ScopeRoots) -> Self {
        let parts = spawn_sylphya(SylphyaInit {
            roots: roots.clone(),
            io: Box::new(SharedIo(io.clone())),
            runtime_sink: None,
        });
        PersistStore {
            io,
            roots,
            publisher: parts.publisher,
            fake: None,
            handle: Some(parts.handle),
        }
    }

    /// World に記憶の送り口を置く（本番の起動と同じ `insert_persist_wiring` を通す）。
    /// 置き直せば、前の送り口は差し替わる。
    pub(super) fn wire(&self, world: &mut World) {
        crate::insert_persist_wiring(world, self.publisher.clone());
    }

    /// 送ったものの書き込みが済むのを待ってから、記憶を読み戻す。
    pub(super) fn load(&self) -> HashMap<PersistKey, String> {
        self.publisher
            .barrier()
            .expect("barrier はアクターが生きている間は返る");
        load_scope(PersistScope::Ghost, &self.roots, &SharedIo(self.io.clone()))
            .into_iter()
            .collect()
    }

    /// 次の書き込み（ファイルの確定）を 1 回だけ失敗させる（偽の保存先だけ・`FakePersistIo` の
    /// 同名の口）。先に送ったものの書き込みが済むのを待ってから仕掛ける（仕掛けが前の書き込みに
    /// 当たらないように）。
    pub(super) fn fail_next_commit(&self) {
        self.publisher
            .barrier()
            .expect("barrier はアクターが生きている間は返る");
        self.fake
            .as_ref()
            .expect("書き込みを失敗させる口は偽の保存先だけ（PersistStore::fake）")
            .fail_next_commit();
    }

    /// アクターを止めて待つ（World に残った送り口の写しは、止めた後は何も書かない）。
    pub(super) fn finish(self) {
        drop(self);
    }
}

impl Drop for PersistStore {
    fn drop(&mut self) {
        self.publisher.close();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

// ── 窓の一式 ───────────────────────────────────────────────────────────────

/// 作業領域（下端に接地させる基準）。
pub(super) const WORK_AREA: RectPx = RectPx {
    left: 0,
    top: 0,
    right: 2560,
    bottom: 1400,
};

/// [`WORK_AREA`] 1 枚の作業領域の表。
pub(super) fn work_area_snapshot() -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![WORK_AREA],
    }
}

/// 作業領域の表だけを置いた空の World。
pub(super) fn persist_world() -> World {
    let mut world = World::new();
    world.insert_resource(work_area_snapshot());
    world
}

/// 準備のときの大きさ（本体・相方）。出ている絵の大きさ [`shown_sizes`] と幅が違う。
pub(super) const PREP_SIZES: [SizePx; 2] = [SizePx { w: 400, h: 600 }, SizePx { w: 300, h: 500 }];

/// 準備の配置（2 スコープ・下端に接地）。相方は本体の左に離して置く（並べ直しで動く）。
pub(super) fn prep_placements() -> Vec<ScopePlacement> {
    [(0usize, 1800), (1, 1000)]
        .into_iter()
        .map(|(scope, x)| {
            let char_size = PREP_SIZES[scope];
            let char_pos = PointPx {
                x,
                y: WORK_AREA.bottom - char_size.h,
            };
            let balloon_offset = PointPx { x: -230, y: 0 };
            ScopePlacement {
                scope,
                char_pos,
                char_size,
                balloon_pos: PointPx {
                    x: char_pos.x + balloon_offset.x,
                    y: char_pos.y + balloon_offset.y,
                },
                balloon_size: SizePx { w: 223, h: 158 },
                balloon_offset,
                balloon_offset_base: OffsetBase::unpinned(balloon_offset),
                // バルーンの作業領域への引き戻しはこのテストの主題ではない。
                balloon_limit: false,
                anchor: Anchor::Bottom,
                balloon_keyword_base: None,
            }
        })
        .collect()
}

/// 出ている絵の幅（準備の幅と違う・半分の切り捨てが効く奇数）。高さは準備と同じ。
pub(super) const SHOWN_SIZES: [(u32, u32); 2] = [(437, 600), (281, 500)];

/// 両方の絵が出ている状態。
pub(super) fn shown_sizes() -> PerTargetSizes {
    PerTargetSizes::new([(0, Some(SHOWN_SIZES[0])), (1, Some(SHOWN_SIZES[1]))])
}

/// 任意の配置から窓の一式を作り（偽 `WindowHandle` 付き）、`shown` の大きさの絵が出たとして
/// 置き直し（再スナップ・本番の `resnap_with`）を 1 回回す。`shown` で `None` のスコープは絵が
/// 出ていないので置き直さない。並べ直しは回さない（呼び手が `finalize_chain_once_with` を呼ぶ）。
pub(super) fn spawn_shown(
    world: &mut World,
    placements: &[ScopePlacement],
    shown: &PerTargetSizes,
) -> GhostWindows {
    let gw = spawn_ghost_windows(
        world,
        placements,
        &GhostTitles::from_scope_titles(
            placements
                .iter()
                .map(|p| (p.scope, format!("s{}", p.scope))),
        ),
    );
    // 偽の WindowHandle（窓への書き込みが WindowPos に届く条件）。
    let mut raw = 0x100usize;
    for scope in gw.scopes().collect::<Vec<_>>() {
        for e in [
            gw.char_window(scope).expect("char 窓がある"),
            gw.balloon_window(scope).expect("balloon 窓がある"),
        ] {
            world.entity_mut(e).insert(WindowHandle {
                hwnd: HWND(raw as *mut _),
                instance: HINSTANCE::default(),
            });
            raw += 0x10;
        }
    }
    resnap_with(shown, world);
    gw
}

// ── 最小のゴースト ──────────────────────────────────────────────────────────

/// 呼び出しごとに別のフォルダにするための連番（並走するテストが互いのフォルダを消さない）。
static NEXT_GHOST: AtomicU32 = AtomicU32::new(0);

/// ワークツリーの `target\test-tmp\` の下に植えた最小のゴースト（マウントの解決が通る形）。
/// 破棄で中身ごと消える。
pub(super) struct MinimalGhost(PathBuf);

impl MinimalGhost {
    /// `ghost\master\descript.txt` と `shell\master\` だけを植える。記憶のファイルは植えない。
    pub(super) fn plant(label: &str) -> Self {
        // ワークスペースの根は `CARGO_MANIFEST_DIR` の 2 つ上（`..` を含めない）。
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("CARGO_MANIFEST_DIR の 2 つ上（ワークスペースの根）");
        let root = workspace.join("target").join("test-tmp").join(format!(
            "areka-cpsoe-{label}-{}-{}",
            std::process::id(),
            NEXT_GHOST.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let ghost = MinimalGhost(root);
        std::fs::create_dir_all(ghost.ghost_master()).expect("ghost/master を作れる");
        std::fs::write(
            ghost.ghost_master().join("descript.txt"),
            "charset,UTF-8\nname,テスト\nsakura.name,さくら\nkero.name,うにゅう\n".as_bytes(),
        )
        .expect("descript.txt を書ける");
        std::fs::create_dir_all(ghost.0.join("shell").join("master"))
            .expect("shell/master を作れる");
        ghost
    }

    /// ゴーストの根（起動の準備の関数へ渡す）。
    pub(super) fn root(&self) -> &Path {
        &self.0
    }

    fn ghost_master(&self) -> PathBuf {
        self.0.join("ghost").join("master")
    }
}

impl Drop for MinimalGhost {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
