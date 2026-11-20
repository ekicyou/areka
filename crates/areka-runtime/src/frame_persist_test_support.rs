//! 並べ直しから記憶までを通すテストの共有の部品。
//!
//! 使い手は `areka-runtime` の鎖のテストと、本体の `chain_finalize_*` のテスト。
//!
//! 置くのは 2 つの口:
//! - 毎フレームの処理の World に記憶の送り口（`PersistWiring`）を置く口 [`PersistStore`]
//!   （偽の保存先 `FakePersistIo` と、ゴーストのフォルダの下の実物のファイルの両方）。
//! - 任意の配置から窓を作り、準備の幅と違う幅の絵を出して置き直す（再スナップ）口 [`spawn_shown`]。
//!   同じ幅のまま組むと、書くとき（出ている幅の半分を足す）・戻すとき（準備の幅の半分を引く）・
//!   絵が出て中央を保って付け替えるとき、の 3 つの計算を通らずに緑になる。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use areka_actor::ActorHandle;
use areka_ghost::sylphya_wiring::profile_areka_root;
use areka_sylphya::persist::{FakePersistIo, FsPersistIo, PersistIo};
use areka_sylphya::{PersistKey, PersistScope, ScopeRoots, SylphyaInit, SylphyaPublisher};
use areka_sylphya::{load_scope, spawn_sylphya};

use super::resnap_with;
use super::test_support::PerTargetSizes;
use areka_placement::follow::{MonitorSnapshot, OffsetBase};
use areka_placement::persist::PersistWiring;
use areka_placement::resolver::{Anchor, PointPx, RectPx, ScopePlacement, SizePx};
use areka_placement::source::GhostTitles;
use areka_placement::spawn::{GhostWindows, spawn_ghost_windows};
use areka_placement::test_support::give_fake_handles;
use bevy_ecs::world::World;

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
pub struct PersistStore {
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
    pub fn fake() -> Self {
        let roots = ScopeRoots {
            ghost: Some(PathBuf::from("/g")),
            ..ScopeRoots::default()
        };
        let fake = Arc::new(FakePersistIo::new());
        let mut store = Self::spawn(fake.clone(), roots);
        store.fake = Some(fake);
        store
    }

    /// ゴーストの `ghost\master` の下の実物のファイル（本番と同じ置き場 `profile\areka\`・
    /// 本番と同じ `FsPersistIo`）。起動の準備の関数がそのまま読み直せる。
    pub fn real(ghost_master: &Path) -> Self {
        let roots = ScopeRoots {
            ghost: Some(profile_areka_root(ghost_master)),
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

    /// World に記憶の送り口を置く。置き直せば、前の送り口は差し替わる。
    pub fn wire(&self, world: &mut World) {
        world.insert_non_send(PersistWiring {
            publisher: self.publisher.clone(),
        });
    }

    /// 送ったものの書き込みが済むのを待ってから、記憶を読み戻す。
    pub fn load(&self) -> HashMap<PersistKey, String> {
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
    pub fn fail_next_commit(&self) {
        self.publisher
            .barrier()
            .expect("barrier はアクターが生きている間は返る");
        self.fake
            .as_ref()
            .expect("書き込みを失敗させる口は偽の保存先だけ（PersistStore::fake）")
            .fail_next_commit();
    }

    /// アクターを止めて待つ（World に残った送り口の写しは、止めた後は何も書かない）。
    pub fn finish(self) {
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
pub const WORK_AREA: RectPx = RectPx {
    left: 0,
    top: 0,
    right: 2560,
    bottom: 1400,
};

/// [`WORK_AREA`] 1 枚の作業領域の表。
pub fn work_area_snapshot() -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![WORK_AREA],
    }
}

/// 作業領域の表だけを置いた空の World。
pub fn persist_world() -> World {
    let mut world = World::new();
    world.insert_resource(work_area_snapshot());
    world
}

/// 準備のときの大きさ（本体・相方）。出ている絵の大きさ [`shown_sizes`] と幅が違う。
pub const PREP_SIZES: [SizePx; 2] = [SizePx { w: 400, h: 600 }, SizePx { w: 300, h: 500 }];

/// 準備の配置（2 スコープ・下端に接地）。相方は本体の左に離して置く（並べ直しで動く）。
pub fn prep_placements() -> Vec<ScopePlacement> {
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
pub const SHOWN_SIZES: [(u32, u32); 2] = [(437, 600), (281, 500)];

/// 両方の絵が出ている状態。
pub fn shown_sizes() -> PerTargetSizes {
    PerTargetSizes::new([(0, Some(SHOWN_SIZES[0])), (1, Some(SHOWN_SIZES[1]))])
}

/// 任意の配置から窓の一式を作り（偽 `WindowHandle` 付き）、`shown` の大きさの絵が出たとして
/// 置き直し（再スナップ・本番の `resnap_with`）を 1 回回す。`shown` で `None` のスコープは絵が
/// 出ていないので置き直さない。並べ直しは回さない（呼び手が `finalize_chain_once_with` を呼ぶ）。
pub fn spawn_shown(
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
    give_fake_handles(world, &gw);
    resnap_with(shown, world);
    gw
}

// ── 主の画面の左端に立つ組 ────────────────────────────────────────────────────

/// 本体が主の画面の左端に立ち、相方を左へ並べると左隣の画面へ渡る並び（x=−1108 の形）。
pub mod left_edge {
    use super::{MonitorSnapshot, OffsetBase, PerTargetSizes, World};
    use areka_placement::resolver::{Anchor, PointPx, RectPx, ScopePlacement, SizePx};

    /// 主の画面（200%）の作業領域。
    pub const PRIMARY: RectPx = RectPx {
        left: 0,
        top: 0,
        right: 2880,
        bottom: 1704,
    };
    /// 左隣の画面（150%）の作業領域。
    pub const LEFT: RectPx = RectPx {
        left: -2560,
        top: 0,
        right: 0,
        bottom: 1400,
    };
    /// 本体・相方の絵の大きさ（準備と同じ幅で出す）。
    pub const SIZES: [(u32, u32); 2] = [(764, 1094), (1108, 1368)];
    /// 相方のバルーンのずらし。
    pub const OFFSET: PointPx = PointPx { x: -230, y: 0 };

    pub fn shown() -> PerTargetSizes {
        PerTargetSizes::new([(0, Some(SIZES[0])), (1, Some(SIZES[1]))])
    }

    /// 本体は主の画面の左端・相方は本体から離して置く（記憶なし＝既定の位置のまま・並べ直しで動く）。
    pub fn placements() -> Vec<ScopePlacement> {
        [(0usize, 0), (1, 1500)]
            .into_iter()
            .map(|(scope, x)| {
                let char_size = SizePx {
                    w: SIZES[scope].0 as i32,
                    h: SIZES[scope].1 as i32,
                };
                let char_pos = PointPx {
                    x,
                    y: PRIMARY.bottom - char_size.h,
                };
                ScopePlacement {
                    scope,
                    char_pos,
                    char_size,
                    balloon_pos: PointPx {
                        x: char_pos.x + OFFSET.x,
                        y: char_pos.y + OFFSET.y,
                    },
                    balloon_size: SizePx { w: 223, h: 158 },
                    balloon_offset: OFFSET,
                    balloon_offset_base: OffsetBase::unpinned(OFFSET),
                    // バルーンの引き戻しを効かせ、キャラ窓を収めれば起きないことを見る（要件 5.2）。
                    balloon_limit: true,
                    anchor: Anchor::Bottom,
                    balloon_keyword_base: None,
                }
            })
            .collect()
    }

    pub fn world() -> World {
        let mut world = World::new();
        world.insert_resource(MonitorSnapshot {
            work_areas: vec![PRIMARY, LEFT],
        });
        world
    }
}
