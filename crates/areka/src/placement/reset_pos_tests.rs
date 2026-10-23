//! 並べ直し（[`reset_window_positions`]）の檻（areka-P0-move-canon 要件 8.6・8.8・8.9・9.2・
//! design Testing Strategy `reset_pos_tests.rs`）。
//!
//! 窓の一式は本番と同じ `spawn_ghost_windows` で組み、記憶は実物のアクター（`spawn_sylphya`）と
//! 偽の保存先 `FakePersistIo` に置いて `barrier` の後に読み戻す（`persist_finalize_save_tests.rs`
//! と同じやり方）。主モニタは偽の `Monitor` entity で置く。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use areka_sylphya::persist::{FakePersistIo, PersistIo};
use areka_sylphya::{
    Axis, PersistKey, PersistScope, ScopeRoots, SylphyaInit, SylphyaParts, load_scope,
    spawn_sylphya,
};
use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{HINSTANCE, HWND, RECT};
use wintf::ecs::window::drain_window_pos_commands;
use wintf::ecs::window::monitor::Monitor;
use wintf::ecs::{WindowHandle, WindowPos};

use super::{DefaultPlacementBasis, reset_window_positions};
use crate::placement::config::build_placement_config;
use crate::placement::follow::{Anchored, BalloonFollow, OffsetBase};
use crate::placement::persist::PersistWiring;
use crate::placement::prepare_ghost_windows_with_work_area;
use crate::placement::reseed::BalloonPlacementInputs;
use crate::placement::resolver::{
    Anchor, PointPx, ScopeInput, ScopePlacement, SizePx, resolve_placement,
};
use crate::placement::shared_test_support::{WA, balloon_root, emo2_root, with_com_initialized};
use crate::placement::source::{DescriptSource, GhostTitles};
use crate::placement::spawn::{GhostWindows, spawn_ghost_windows};
use crate::placement::test_support::{LogEvent, capture_logs};

/// 主モニタの作業領域（物理 px）。
const WORK: (i32, i32, i32, i32) = (0, 0, 1920, 1040);
/// バルーンのずらし（キャラ窓の左上から）。
const BALLOON_OFFSET: PointPx = PointPx { x: -200, y: 30 };
/// 2 スコープの「ずらした」今の位置と、今表示している大きさ。
const SCOPES: [(usize, PointPx, SizePx); 2] = [
    (0, PointPx { x: 111, y: 222 }, SizePx { w: 301, h: 501 }),
    (1, PointPx { x: 50, y: 60 }, SizePx { w: 201, h: 401 }),
];
/// 配置の規則（下端・右から隙間なく）を手で解いた位置: scope0 は右端から、scope1 はその左。
const EXPECTED: [PointPx; 2] = [
    PointPx {
        x: 1920 - 301,
        y: 1040 - 501,
    },
    PointPx {
        x: 1920 - 301 - 201,
        y: 1040 - 401,
    },
];

struct SharedFakeIo(Arc<FakePersistIo>);
impl PersistIo for SharedFakeIo {
    fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
        self.0.read(path)
    }
    fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
        self.0.commit(path, content)
    }
}

struct Rig {
    world: World,
    shared: Arc<FakePersistIo>,
    roots: ScopeRoots,
    parts: SylphyaParts,
}

impl Rig {
    /// 2 スコープの窓の一式（ずらした位置）・主と副のモニタ・記憶の送り口・材料を持つ World。
    fn new() -> Self {
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
        let placements: Vec<ScopePlacement> = SCOPES
            .iter()
            .map(|&(scope, char_pos, char_size)| ScopePlacement {
                scope,
                char_pos,
                char_size,
                balloon_pos: PointPx {
                    x: char_pos.x + BALLOON_OFFSET.x,
                    y: char_pos.y + BALLOON_OFFSET.y,
                },
                balloon_size: SizePx { w: 223, h: 158 },
                balloon_offset: BALLOON_OFFSET,
                balloon_offset_base: OffsetBase::unpinned(BALLOON_OFFSET),
                balloon_limit: false,
                anchor: Anchor::Bottom,
                balloon_keyword_base: None,
            })
            .collect();
        let gw = spawn_ghost_windows(
            &mut world,
            &placements,
            &GhostTitles::from_scope_titles(SCOPES.iter().map(|&(s, _, _)| (s, format!("s{s}")))),
        );
        let mut raw = 0x100;
        for scope in gw.scopes() {
            for entity in [gw.char_window(scope), gw.balloon_window(scope)] {
                raw += 1;
                world.entity_mut(entity.unwrap()).insert(WindowHandle {
                    hwnd: HWND(raw as *mut _),
                    instance: HINSTANCE::default(),
                });
            }
        }
        // 副モニタを先に置く（先頭を選んでしまう誤りを噛ませる）。
        world.spawn(monitor(2, (-1280, 0, 0, 984), false));
        world.spawn(monitor(1, WORK, true));
        world.insert_non_send(PersistWiring {
            publisher: parts.publisher.clone(),
        });
        world.insert_resource(DefaultPlacementBasis {
            source: DescriptSource {
                ghost_kv: Default::default(),
                shell_kv: Default::default(),
                shell_dir: PathBuf::new(),
                titles: GhostTitles::from_scope_titles(std::iter::empty()),
            },
            balloon: BalloonPlacementInputs {
                author_dpi: 96,
                windowpositions: Default::default(),
            },
        });
        Rig {
            world,
            shared,
            roots,
            parts,
        }
    }

    fn gw(&self) -> &GhostWindows {
        self.world.resource::<GhostWindows>()
    }

    fn pos(&self, entity: Entity) -> PointPx {
        let p = self
            .world
            .get::<WindowPos>(entity)
            .unwrap()
            .position
            .unwrap();
        PointPx { x: p.x, y: p.y }
    }

    fn char_pos(&self, scope: usize) -> PointPx {
        self.pos(self.gw().char_window(scope).unwrap())
    }

    fn balloon_pos(&self, scope: usize) -> PointPx {
        self.pos(self.gw().balloon_window(scope).unwrap())
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

fn monitor(handle: isize, work: (i32, i32, i32, i32), is_primary: bool) -> Monitor {
    let (left, top, right, bottom) = work;
    let rect = RECT {
        left,
        top,
        right,
        bottom,
    };
    Monitor {
        handle,
        bounds: rect,
        work_area: rect,
        dpi: 96,
        is_primary,
    }
}

fn saved(map: &HashMap<PersistKey, String>, scope: u32) -> Option<(String, String)> {
    let get = |axis| map.get(&PersistKey::WindowPos { scope, axis }).cloned();
    Some((get(Axis::X)?, get(Axis::Y)?))
}

fn reset_events(events: &[LogEvent], name: &str) -> Vec<LogEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .cloned()
        .collect()
}

/// 8.6・8.8・8.9・9.2: ずらした 2 スコープの窓が配置の規則の位置へ戻り、既定の位置が記され
/// （記憶から戻したスコープ 1 も）、記憶へ両スコープの位置が届き、バルーンが付いて動き、
/// `Anchored` とバルーンのずらしは変わらない。スコープごとに前後の位置が記録される。
#[test]
fn shifted_windows_return_to_default_and_are_persisted() {
    let mut rig = Rig::new();
    // スコープ 1 は記憶から戻した窓（既定の位置が無い）。
    assert!(
        rig.world
            .resource_mut::<GhostWindows>()
            .clear_default_char_pos(1)
    );
    let chars: Vec<Entity> = (0..2).map(|s| rig.gw().char_window(s).unwrap()).collect();
    let anchored_before: Vec<Anchored> = chars
        .iter()
        .map(|&e| *rig.world.get::<Anchored>(e).unwrap())
        .collect();
    let follow_before: Vec<BalloonFollow> = chars
        .iter()
        .map(|&e| *rig.world.get::<BalloonFollow>(e).unwrap())
        .collect();

    let (moved, events) = capture_logs(|| reset_window_positions(&mut rig.world));
    assert!(moved, "並べ直した窓がある");

    for scope in 0..2 {
        assert_eq!(rig.char_pos(scope), EXPECTED[scope], "scope {scope} の位置");
        assert_eq!(
            rig.gw().default_char_pos(scope),
            Some(EXPECTED[scope]),
            "scope {scope} の既定の位置が記される"
        );
        assert_eq!(
            rig.balloon_pos(scope),
            PointPx {
                x: EXPECTED[scope].x + BALLOON_OFFSET.x,
                y: EXPECTED[scope].y + BALLOON_OFFSET.y,
            },
            "scope {scope} のバルーンが今のずらしで付いて動く"
        );
        assert_eq!(
            *rig.world.get::<Anchored>(chars[scope]).unwrap(),
            anchored_before[scope]
        );
        assert_eq!(
            *rig.world.get::<BalloonFollow>(chars[scope]).unwrap(),
            follow_before[scope]
        );

        let logs: Vec<LogEvent> = reset_events(&events, "reset_window_pos")
            .into_iter()
            .filter(|e| e.field("scope") == Some(scope.to_string().as_str()))
            .collect();
        assert_eq!(logs.len(), 1, "scope {scope} の記録が 1 行: {events:?}");
        let (_, from, _) = SCOPES[scope];
        for (key, want) in [
            ("from_x", from.x),
            ("from_y", from.y),
            ("to_x", EXPECTED[scope].x),
            ("to_y", EXPECTED[scope].y),
        ] {
            assert_eq!(logs[0].field(key), Some(want.to_string().as_str()), "{key}");
        }
    }

    let store = rig.load();
    for scope in 0..2 {
        let w = SCOPES[scope].2.w;
        assert_eq!(
            saved(&store, scope as u32),
            Some((
                (EXPECTED[scope].x + w / 2).to_string(),
                EXPECTED[scope].y.to_string()
            )),
            "scope {scope} の位置が記憶へ届く: {store:?}"
        );
    }
    rig.finish();
}

/// 材料を 1 つ外す手。
type Strip = fn(&mut World);

/// 材料（`DefaultPlacementBasis`・主モニタ・`GhostWindows`）が無ければ `warn!` だけで何もしない。
#[test]
fn missing_material_only_warns() {
    let cases: [(&str, Strip); 3] = [
        ("no_default_basis", |w| {
            w.remove_resource::<DefaultPlacementBasis>();
        }),
        ("no_primary_monitor", |w| {
            let ms: Vec<Entity> = w
                .query_filtered::<Entity, With<Monitor>>()
                .iter(w)
                .collect();
            for m in ms {
                w.despawn(m);
            }
        }),
        ("no_ghost_windows", |w| {
            w.remove_resource::<GhostWindows>();
        }),
    ];
    for (reason, strip) in cases {
        let mut rig = Rig::new();
        let chars: Vec<Entity> = (0..2).map(|s| rig.gw().char_window(s).unwrap()).collect();
        strip(&mut rig.world);
        let (moved, events) = capture_logs(|| reset_window_positions(&mut rig.world));
        assert!(!moved, "{reason}: 何もしない");
        let skips = reset_events(&events, "reset_window_pos_skipped");
        assert_eq!(skips.len(), 1, "{reason}: warn が 1 行: {events:?}");
        assert_eq!(skips[0].level, tracing::Level::WARN);
        assert_eq!(skips[0].field_str("reason"), Some(reason));
        assert!(reset_events(&events, "reset_window_pos").is_empty());
        for (scope, &e) in chars.iter().enumerate() {
            assert_eq!(
                rig.pos(e),
                SCOPES[scope].1,
                "{reason}: scope {scope} は動かない"
            );
        }
        assert_eq!(saved(&rig.load(), 0), None, "{reason}: 記憶へ書かない");
        rig.finish();
    }
}

/// 大きさの揃わないスコープは `warn!` で外し、残りで解く（外したスコープは動かさない）。
#[test]
fn scope_without_size_is_left_out_and_rest_is_solved() {
    let mut rig = Rig::new();
    let balloon0 = rig.gw().balloon_window(0).unwrap();
    rig.world.get_mut::<WindowPos>(balloon0).unwrap().size = None;

    let (moved, events) = capture_logs(|| reset_window_positions(&mut rig.world));
    assert!(moved);
    let skips = reset_events(&events, "reset_window_pos_skipped");
    assert_eq!(skips.len(), 1, "{events:?}");
    assert_eq!(skips[0].level, tracing::Level::WARN);
    assert_eq!(skips[0].field("scope"), Some("0"));
    assert_eq!(skips[0].field_str("reason"), Some("no_window_size"));
    assert_eq!(rig.char_pos(0), SCOPES[0].1, "外したスコープは動かない");
    // 残ったスコープ 1 だけで解くので右端から並ぶ。
    assert_eq!(
        rig.char_pos(1),
        PointPx {
            x: 1920 - 201,
            y: 1040 - 401
        }
    );
    rig.finish();
}

/// 起動の準備の結果は並べ直しの材料（descript の値）を運ぶ（design ResetPos State Management (1)）:
/// 運んだ値から並べ直しと同じく設定を組み、準備と同じ窓の大きさ・作業領域・DPI で解き直すと、
/// キャラクターの窓の位置（並べ直しが書くもの）は準備が解いた配置と同じになる。
#[test]
fn boot_prepare_carries_the_default_placement_basis() {
    with_com_initialized(|| {
        let p = prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(96))
            .expect("emo2 fixture の配置準備は成功する");
        let inputs: Vec<ScopeInput> = p
            .placements
            .iter()
            .map(|s| ScopeInput {
                scope: s.scope,
                char_size: s.char_size,
                balloon_size: s.balloon_size,
            })
            .collect();
        let src = &p.default_basis.source;
        let cfg = build_placement_config(&src.ghost_kv, &src.shell_kv);
        let char_pos = |placements: Vec<ScopePlacement>| -> Vec<_> {
            placements
                .iter()
                .map(|s| (s.scope, s.char_pos, s.anchor))
                .collect()
        };
        assert_eq!(
            char_pos(resolve_placement(
                &cfg,
                WA,
                &inputs,
                wintf::ecs::DPI::from_dpi(96, 96)
            )),
            char_pos(p.placements.clone()),
            "運んだ値で解き直すとキャラクターの窓の位置は準備の配置と同じ"
        );
        assert_eq!(cfg.zorder_raw, p.zorder_raw, "同じ読取の値");
    });
}

/// areka-P0-window-link-directives 要件 4.7・5.6: 組 {0,1} があっても、並べ直しは各窓を
/// それぞれの既定の位置へ置き（相手の動いた量で相手をずらさない）、組は解かない。
#[test]
fn grouped_windows_each_return_to_their_own_default_and_the_group_stays() {
    use crate::placement::sticky_group_ledger::{
        StickyGroupLedger, apply_set_sticky, seed_sticky_descript,
    };
    let mut rig = Rig::new();
    seed_sticky_descript(&mut rig.world, None);
    assert!(apply_set_sticky(
        &mut rig.world,
        &["0".to_string(), "1".to_string()]
    ));

    assert!(reset_window_positions(&mut rig.world));

    for (scope, want) in EXPECTED.iter().enumerate() {
        assert_eq!(rig.char_pos(scope), *want, "scope {scope} の位置");
    }
    assert_eq!(
        rig.world.resource::<StickyGroupLedger>().partners_of(0),
        vec![1],
        "組は解かない"
    );
    rig.finish();
}

/// 並べ直しで既定の位置へ戻した窓からは台本の印 `ScriptPlaced` を外す（起動の確定の前に
/// 並べ直しても、戻した窓を台本の置いた窓と見ない）。戻した窓は記憶へ届く。
#[test]
fn reset_clears_script_placed_mark_and_persists() {
    use crate::placement::chain_finalize::ScriptPlaced;
    let mut rig = Rig::new();
    let char0 = rig.gw().char_window(0).unwrap();
    rig.world.entity_mut(char0).insert(ScriptPlaced);

    assert!(reset_window_positions(&mut rig.world));

    assert!(
        rig.world.get::<ScriptPlaced>(char0).is_none(),
        "戻した窓の印は外れる"
    );
    let store = rig.load();
    let w = SCOPES[0].2.w;
    assert_eq!(
        saved(&store, 0),
        Some((
            (EXPECTED[0].x + w / 2).to_string(),
            EXPECTED[0].y.to_string()
        )),
        "戻した窓の位置が記憶へ届く: {store:?}"
    );
    rig.finish();
}
