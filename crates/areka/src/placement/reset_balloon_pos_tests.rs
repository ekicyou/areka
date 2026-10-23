//! バルーン位置初期化（[`reset_balloon_positions`]）の檻（areka-P0-window-link-directives
//! 要件 7.1〜7.4・9.5・design「reset_balloon_pos」）。
//!
//! 窓の一式は起動の準備（`prepare_ghost_windows_with_work_area`）が解いた配置を本番と同じ
//! `spawn_ghost_windows` で組み、材料は準備が運んだ [`DefaultPlacementBasis`] を置く。標準の位置は
//! 準備が解いた位置そのもの（同じ窓寸・作業領域・DPI）なので、ドラッグでずらした後に初期化すると
//! そこへ戻ることを確かめる。記憶は実物のアクター（`spawn_sylphya`）と偽の保存先に置いて読み戻す
//! （`reset_pos_tests.rs` と同じやり方）。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use areka_sylphya::persist::{FakePersistIo, PersistIo};
use areka_sylphya::{
    Axis, PersistKey, PersistScope, ScopeRoots, SylphyaInit, SylphyaParts, load_scope,
    spawn_sylphya,
};
use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use wintf::ecs::window::drain_window_pos_commands;
use wintf::ecs::{DPI, Point, WindowHandle, WindowPos};

use super::{BalloonResetTrigger, reset_balloon_positions};
use crate::emo2_boot::switch_assets::{SwapBuilt, SwitchBuildRequest, build_swap};
use crate::placement::follow::{
    AnchorSource, Anchored, BalloonFollow, MonitorSnapshot, OffsetBase,
};
use crate::placement::persist::PersistWiring;
use crate::placement::prepare_ghost_windows_with_work_area;
use crate::placement::reset_pos::DefaultPlacementBasis;
use crate::placement::resolver::{Anchor, PointPx, ScopePlacement};
use crate::placement::shared_test_support::{
    TempDir, WA, balloon_root, emo2_root, synth_declared_dpi_ghost, with_com_initialized,
};
use crate::placement::spawn::{BalloonKeywordBase, GhostWindows, spawn_ghost_windows};
use crate::placement::sticky_group_ledger::StickyGroupLedger;
use crate::placement::test_support::{LogEvent, capture_logs};

/// ドラッグでずらしたバルーンのずらし（標準のずらしと必ず違う値）。
const DRAGGED: PointPx = PointPx { x: 777, y: -333 };

struct SharedFakeIo(Arc<FakePersistIo>);
impl PersistIo for SharedFakeIo {
    fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
        self.0.read(path)
    }
    fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
        self.0.commit(path, content)
    }
}

/// 起動の準備の配置で組んだ窓・材料・記憶の送り口を持つ World。
struct Rig {
    world: World,
    windows: GhostWindows,
    placements: Vec<ScopePlacement>,
    shared: Arc<FakePersistIo>,
    roots: ScopeRoots,
    parts: SylphyaParts,
}

impl Rig {
    /// `ghost`・`balloon` の起動の準備（作業領域 `WA`・DPI 96）で窓を組み、`basis` を材料に置く
    /// （`None` なら準備が運んだ材料）。
    fn new(ghost: &Path, balloon: &Path, basis: Option<DefaultPlacementBasis>) -> Self {
        let prepared = prepare_ghost_windows_with_work_area(ghost, balloon, WA, Some(96))
            .expect("起動の配置の準備は成功する");
        let shared = Arc::new(FakePersistIo::new());
        let roots = ScopeRoots {
            ghost: Some(PathBuf::from("/g")),
            ..ScopeRoots::default()
        };
        let parts = spawn_sylphya(SylphyaInit {
            roots: roots.clone(),
            io: Box::new(SharedFakeIo(shared.clone())),
            runtime_sink: None,
        });
        let mut world = World::new();
        world.insert_resource(MonitorSnapshot {
            work_areas: vec![WA],
        });
        let windows = spawn_ghost_windows(&mut world, &prepared.placements, &prepared.titles);
        let mut raw = 0x100;
        for scope in windows.scopes() {
            for e in [windows.char_window(scope), windows.balloon_window(scope)] {
                raw += 1;
                world.entity_mut(e.unwrap()).insert(WindowHandle {
                    hwnd: HWND(raw as *mut _),
                    instance: HINSTANCE::default(),
                });
            }
        }
        // `WindowHandle` の付与の hook が偽 HWND の DPI を遅延で書くので、流してから置く。
        world.flush();
        for scope in windows.scopes() {
            for e in [windows.char_window(scope), windows.balloon_window(scope)] {
                world.entity_mut(e.unwrap()).insert(DPI::from_dpi(96, 96));
            }
        }
        world.insert_non_send(PersistWiring {
            publisher: parts.publisher.clone(),
        });
        world.insert_resource(basis.unwrap_or(prepared.default_basis));
        Rig {
            world,
            windows,
            placements: prepared.placements,
            shared,
            roots,
            parts,
        }
    }

    fn char_window(&self, scope: usize) -> Entity {
        self.windows.char_window(scope).unwrap()
    }

    fn pos(&self, e: Entity) -> Point {
        self.world.get::<WindowPos>(e).unwrap().position.unwrap()
    }

    fn balloon_pos(&self, scope: usize) -> Point {
        self.pos(self.windows.balloon_window(scope).unwrap())
    }

    /// ドラッグの後と同じ形にする: バルーンをずらし、ずらしを部品へ、キーワードの素材を退役。
    fn drag_balloons(&mut self) {
        for scope in self.windows.scopes().collect::<Vec<_>>() {
            let c = self.char_window(scope);
            let b = self.windows.balloon_window(scope).unwrap();
            let char_pos = self.pos(c);
            self.world
                .entity_mut(c)
                .insert(BalloonFollow::new(b, OffsetBase::unpinned(DRAGGED)));
            self.world.entity_mut(c).remove::<BalloonKeywordBase>();
            self.world.get_mut::<WindowPos>(b).unwrap().position =
                Some(Point::new(char_pos.x + DRAGGED.x, char_pos.y + DRAGGED.y));
        }
    }

    fn load(&self) -> HashMap<PersistKey, String> {
        self.parts.publisher.barrier().expect("barrier が返る");
        load_scope(
            PersistScope::Ghost,
            &self.roots,
            &SharedFakeIo(self.shared.clone()),
        )
        .into_iter()
        .collect()
    }

    fn finish(self) {
        let _residue = drain_window_pos_commands();
        self.parts.publisher.close();
        let _ = self.parts.handle.join();
    }
}

fn saved_offset(map: &HashMap<PersistKey, String>, scope: u32) -> Option<(String, String)> {
    let get = |axis| map.get(&PersistKey::BalloonOffset { scope, axis }).cloned();
    Some((get(Axis::X)?, get(Axis::Y)?))
}

fn events_named(events: &[LogEvent], name: &str) -> Vec<LogEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .cloned()
        .collect()
}

/// キーワードのバルーン（`windowposition.x,center`）で組む合成の検体（`reseed_tests` t_rs05 と同じ）。
fn keyword_fixture(root: &TempDir) -> (PathBuf, PathBuf) {
    let (ghost, balloon) = synth_declared_dpi_ghost(root, "96", "96", None);
    fs::write(
        balloon.join("descript.txt"),
        "charset,UTF-8\ndpi,96\nwindowposition.x,center\nwindowposition.y,-20\n",
    )
    .expect("キーワードのバルーンを書けるはず");
    (ghost, balloon)
}

/// 7.1・7.2・7.3・9.5: ドラッグでずらしたバルーンが標準の位置（起動の準備が解いた位置）へ戻り、
/// 部品のずらしの基準とキーワードの素材が標準どおりに入り、記憶に標準のずらしが届き、
/// スコープごとに 1 行記録される。キャラクターの窓・`Anchored`・`AnchorSource`・組の帳簿は変わらない。
#[test]
fn dragged_balloons_return_to_standard_and_are_persisted() {
    with_com_initialized(|| {
        let kw_root = TempDir::new();
        let (kw_ghost, kw_balloon) = keyword_fixture(&kw_root);
        for (ghost, balloon, expect_keyword) in [
            (emo2_root(), balloon_root(), false),
            (kw_ghost, kw_balloon, true),
        ] {
            let mut rig = Rig::new(&ghost, &balloon, None);
            // 吸着の向きを台本で変えた形・組を持つ形にしておく（初期化が触らないことを見る）。
            let c0 = rig.char_window(0);
            rig.world.entity_mut(c0).insert((
                Anchored(Anchor::Top),
                AnchorSource {
                    descript: Anchor::Bottom,
                    by_script: true,
                },
            ));
            let mut ledger = StickyGroupLedger::default();
            ledger
                .try_add_script_group([0, 1].into_iter().collect())
                .unwrap();
            rig.world.insert_resource(ledger.clone());
            rig.drag_balloons();

            let scopes: Vec<usize> = rig.windows.scopes().collect();
            let chars_before: Vec<(Point, Option<Anchored>, Option<AnchorSource>)> = scopes
                .iter()
                .map(|&s| {
                    let c = rig.char_window(s);
                    (
                        rig.pos(c),
                        rig.world.get::<Anchored>(c).copied(),
                        rig.world.get::<AnchorSource>(c).copied(),
                    )
                })
                .collect();

            let (reset, events) =
                capture_logs(|| reset_balloon_positions(&mut rig.world, BalloonResetTrigger::Menu));
            assert!(reset, "{}: 戻したバルーンがある", ghost.display());
            assert!(events_named(&events, "balloon_pos_reset_skipped").is_empty());

            let store = rig.load();
            let logs = events_named(&events, "balloon_pos_reset");
            assert_eq!(logs.len(), scopes.len(), "スコープごとに 1 行: {events:?}");
            for (i, p) in rig.placements.iter().enumerate() {
                let case = format!("{} scope={}", ghost.display(), p.scope);
                let c = rig.char_window(p.scope);
                assert_eq!(
                    rig.balloon_pos(p.scope),
                    Point::new(p.balloon_pos.x, p.balloon_pos.y),
                    "{case}: 標準の位置へ戻る"
                );
                assert_eq!(
                    rig.world.get::<BalloonFollow>(c).unwrap().base(),
                    p.balloon_offset_base,
                    "{case}: 標準のずらしが部品に入る"
                );
                let keyword = rig
                    .world
                    .get::<BalloonKeywordBase>(c)
                    .map(|k| (k.mode, k.adjust));
                assert_eq!(keyword, p.balloon_keyword_base, "{case}: キーワードの素材");
                // キャラクターの窓・向き・出どころは変わらない（7.2）。
                assert_eq!(
                    (
                        rig.pos(c),
                        rig.world.get::<Anchored>(c).copied(),
                        rig.world.get::<AnchorSource>(c).copied(),
                    ),
                    chars_before[i],
                    "{case}: キャラクターの窓は触らない"
                );
                // 記憶には標準のずらし（7.3）。
                let off = p.balloon_offset_base.offset;
                assert_eq!(
                    saved_offset(&store, p.scope as u32),
                    Some((off.x.to_string(), off.y.to_string())),
                    "{case}: 記憶に標準のずらし: {store:?}"
                );
                // 記録の欄（9.5）。
                let log = logs
                    .iter()
                    .find(|e| e.field("scope") == Some(p.scope.to_string().as_str()))
                    .unwrap_or_else(|| panic!("{case}: 記録が無い: {events:?}"));
                assert_eq!(log.level, tracing::Level::INFO);
                for (key, want) in [
                    ("to_x", p.balloon_pos.x),
                    ("to_y", p.balloon_pos.y),
                    ("offset_x", off.x),
                    ("offset_y", off.y),
                ] {
                    assert_eq!(
                        log.field(key),
                        Some(want.to_string().as_str()),
                        "{case} {key}"
                    );
                }
                assert_eq!(log.field_str("trigger"), Some("menu"), "{case}");
            }
            if expect_keyword {
                assert!(
                    rig.placements
                        .iter()
                        .any(|p| p.balloon_keyword_base.is_some()),
                    "前提: キーワードの検体はキーワードの素材を持つ"
                );
            }
            assert_eq!(
                rig.world.get_resource::<StickyGroupLedger>(),
                Some(&ledger),
                "組の帳簿は変わらない"
            );
            rig.finish();
        }
    });
}

/// 7.4: 起動の準備が置く材料と、同じシェルへ切り替えたときの資産づくりから置く材料で、
/// 初期化したバルーンの位置が同じになる（どちらも起動の標準の位置）。
#[test]
fn boot_basis_and_switch_basis_give_the_same_positions() {
    with_com_initialized(|| {
        let built = build_swap(&SwitchBuildRequest::Shell {
            ghost_root: emo2_root(),
            folder: "master".to_string(),
            balloon_dir: balloon_root(),
        })
        .unwrap_or_else(|_| panic!("同じシェルの資産づくりは成功する"));
        let SwapBuilt::Shell {
            source, balloon, ..
        } = built
        else {
            panic!("シェルの依頼にはシェルの結果が返る");
        };
        let switch_basis = DefaultPlacementBasis { source, balloon };

        let mut positions = Vec::new();
        for basis in [None, Some(switch_basis)] {
            let mut rig = Rig::new(&emo2_root(), &balloon_root(), basis);
            rig.drag_balloons();
            assert!(reset_balloon_positions(
                &mut rig.world,
                BalloonResetTrigger::Script
            ));
            let got: Vec<Point> = rig.windows.scopes().map(|s| rig.balloon_pos(s)).collect();
            let want: Vec<Point> = rig
                .placements
                .iter()
                .map(|p| Point::new(p.balloon_pos.x, p.balloon_pos.y))
                .collect();
            assert_eq!(got, want, "標準の位置へ戻る");
            positions.push(got);
            rig.finish();
        }
        assert_eq!(positions[0], positions[1], "起動と切り替えで同じ位置");
    });
}

/// 材料を 1 つ外す手。
type Strip = fn(&mut World);

/// 7.1: 材料（`DefaultPlacementBasis`・`GhostWindows`）が無ければ `warn!` だけで何もしない
/// （バルーンは動かず、記憶へ書かない）。
#[test]
fn missing_material_only_warns() {
    with_com_initialized(|| {
        let cases: [(&str, Strip); 2] = [
            ("no_default_basis", |w| {
                w.remove_resource::<DefaultPlacementBasis>();
            }),
            ("no_ghost_windows", |w| {
                w.remove_resource::<GhostWindows>();
            }),
        ];
        for (reason, strip) in cases {
            let mut rig = Rig::new(&emo2_root(), &balloon_root(), None);
            rig.drag_balloons();
            let before: Vec<Point> = rig.windows.scopes().map(|s| rig.balloon_pos(s)).collect();
            strip(&mut rig.world);
            let (reset, events) = capture_logs(|| {
                reset_balloon_positions(&mut rig.world, BalloonResetTrigger::Shortcut)
            });
            assert!(!reset, "{reason}: 何もしない");
            let skips = events_named(&events, "balloon_pos_reset_skipped");
            assert_eq!(skips.len(), 1, "{reason}: warn が 1 行: {events:?}");
            assert_eq!(skips[0].level, tracing::Level::WARN);
            assert_eq!(skips[0].field_str("reason"), Some(reason));
            assert_eq!(skips[0].field_str("trigger"), Some("shortcut"));
            assert!(events_named(&events, "balloon_pos_reset").is_empty());
            let after: Vec<Point> = rig.windows.scopes().map(|s| rig.balloon_pos(s)).collect();
            assert_eq!(after, before, "{reason}: バルーンは動かない");
            assert_eq!(
                saved_offset(&rig.load(), 0),
                None,
                "{reason}: 記憶へ書かない"
            );
            rig.finish();
        }
    });
}

/// 標準の位置を解けないスコープ（窓の部品が欠けた）は scope・reason つきの `warn!` で外し、
/// 残りのスコープは戻す。
#[test]
fn unresolved_scope_is_skipped_with_scope_and_the_rest_reset() {
    with_com_initialized(|| {
        let mut rig = Rig::new(&emo2_root(), &balloon_root(), None);
        rig.drag_balloons();
        let b0 = rig.windows.balloon_window(0).unwrap();
        rig.world.get_mut::<WindowPos>(b0).unwrap().size = None;
        let stuck = rig.balloon_pos(0);

        let (reset, events) =
            capture_logs(|| reset_balloon_positions(&mut rig.world, BalloonResetTrigger::Script));
        assert!(reset, "残りのスコープは戻す");
        let skips = events_named(&events, "balloon_pos_reset_skipped");
        assert_eq!(skips.len(), 1, "{events:?}");
        assert_eq!(skips[0].level, tracing::Level::WARN);
        assert_eq!(skips[0].field("scope"), Some("0"));
        assert_eq!(skips[0].field_str("reason"), Some("not_resolved"));
        assert_eq!(rig.balloon_pos(0), stuck, "外したスコープは動かない");
        // 残ったスコープ 1 だけで解くので位置は起動と違い得る——今の部品のずらしで付いたことを見る。
        let c1 = rig.char_window(1);
        let off = rig.world.get::<BalloonFollow>(c1).unwrap().offset();
        assert_ne!(off, DRAGGED, "スコープ 1 は標準のずらしへ入れ直される");
        let char1 = rig.pos(c1);
        assert_eq!(
            rig.balloon_pos(1),
            Point::new(char1.x + off.x, char1.y + off.y)
        );
        let logs = events_named(&events, "balloon_pos_reset");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].field_str("trigger"), Some("script"));
        rig.finish();
    });
}
