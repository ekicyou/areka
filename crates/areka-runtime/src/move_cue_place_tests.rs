// `\![move]` の適用のテスト。
//
// 偽の窓（`spawn_ghost_windows`＋偽の `WindowHandle`）・主と副の偽の `Monitor`・その写しの
// `MonitorSnapshot` を置いた headless World で `apply_move_directive` を駆動する。

use std::path::PathBuf;
use std::sync::Arc;

use areka_emo_compose::ScaleRatio;
use areka_sylphya::persist::FakePersistIo;
use areka_sylphya::{PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya};
use bevy_ecs::prelude::*;
use windows::Win32::Foundation::RECT;
use wintf::ecs::window::monitor::Monitor;
use wintf::ecs::{WindowHandle, WindowPos};

use super::apply_move_directive;
use crate::move_cue::apply_move_tests::{BALLOON_OFFSET, events_named, placement};
use crate::move_cue::{MoveDirective, parse_move_directive};
use areka_placement::diag::PlacementRoute;
use areka_placement::follow::{
    Anchored, BalloonFollow, MonitorSnapshot, apply_char_shift, resize_window_to,
};
use areka_placement::persist::PersistWiring;
use areka_placement::resolver::{Anchor, PointPx, SizePx};
use areka_placement::source::GhostTitles;
use areka_placement::spawn::{GhostWindows, spawn_ghost_windows};
use areka_placement::sticky_group_ledger::{apply_set_sticky, seed_sticky_descript};
use areka_placement::test_support::{SharedFakeIo, give_fake_handles};
use log_capture_kit::capture;

/// 主モニタの作業領域（下にタスクバー）。
const PRIMARY: (i32, i32, i32, i32) = (0, 0, 1920, 1040);
/// 副モニタの作業領域（主の左）。
const SECONDARY: (i32, i32, i32, i32) = (-1280, 0, 0, 984);
/// scope0 の窓（左上・大きさ）。矩形 (1000,300)-(1400,987)。
const S0: (PointPx, SizePx) = (PointPx { x: 1000, y: 300 }, SizePx { w: 400, h: 687 });
/// scope1 の窓（左上・大きさ）。矩形 (200,500)-(500,934)。
const S1: (PointPx, SizePx) = (PointPx { x: 200, y: 500 }, SizePx { w: 300, h: 434 });

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

/// 2 スコープの窓（scope1 の吸い付きは `anchor1`）・副を先に置いた 2 台のモニタ・その写し。
fn world_with(anchor1: Anchor) -> (World, GhostWindows) {
    let mut world = World::new();
    let gw = spawn_ghost_windows(
        &mut world,
        &[
            placement(0, S0.0, S0.1, Anchor::Free, false),
            placement(1, S1.0, S1.1, anchor1, false),
        ],
        &GhostTitles::from_scope_titles([(0, "a".to_string()), (1, "b".to_string())]),
    );
    give_fake_handles(&mut world, &gw);
    // 副モニタを先に置く（先頭を主と取り違える誤りを噛ませる）。
    let monitors = [monitor(2, SECONDARY, false), monitor(1, PRIMARY, true)];
    world.insert_resource(MonitorSnapshot::from_monitors(&monitors));
    for m in monitors {
        world.spawn(m);
    }
    (world, gw)
}

fn args(items: &[&str]) -> MoveDirective {
    let tokens: Vec<String> = items.iter().map(|s| s.to_string()).collect();
    parse_move_directive(1, &tokens).directive
}

fn pos(world: &World, e: Entity) -> PointPx {
    let p = world.get::<WindowPos>(e).unwrap().position.unwrap();
    PointPx { x: p.x, y: p.y }
}

/// 動かす窓を `\![move]` を通さずに置く（移動の前の位置を作る）。
fn place(world: &mut World, e: Entity, x: i32, y: i32) {
    let mut wp = world.get_mut::<WindowPos>(e).unwrap();
    wp.position = Some(wintf::ecs::Point { x, y });
}

// 基準 5 種と無いスコープ

/// (名前, 移動の前の左上, 台本, 後の左上)。
type BaseRow = (
    &'static str,
    (i32, i32),
    &'static [&'static str],
    (i32, i32),
);

/// 基準ごとの移動先（吸い付きは `Free`・k=1）。
#[test]
fn every_base_resolves_to_its_frame() {
    let rows: &[BaseRow] = &[
        // screen: 主にいる窓は主の作業領域の左上から。
        ("screen", (200, 500), &["--X=10", "--Y=20"], (10, 20)),
        // screen: 副にいる窓は副の作業領域から（窓の中心で決める）。
        (
            "screen/secondary",
            (-1000, 100),
            &["--X=10", "--Y=20"],
            (-1270, 20),
        ),
        // screen: どのモニタにもいない窓は最近傍（副）の作業領域から。
        (
            "screen/nearest",
            (-3000, 100),
            &["--X=10", "--Y=20"],
            (-1270, 20),
        ),
        // primaryscreen: 副にいても主の作業領域（右下どうし）。
        (
            "primaryscreen",
            (-1000, 100),
            &[
                "--base=primaryscreen",
                "--X=10",
                "--Y=20",
                "--base-offset=right.bottom",
                "--move-offset=right.bottom",
            ],
            (1920 + 10 - 300, 1040 + 20 - 434),
        ),
        // global: 原点（参照点は使わない）。
        (
            "global",
            (200, 500),
            &[
                "--base=global",
                "--X=10",
                "--Y=20",
                "--base-offset=right.bottom",
            ],
            (10, 20),
        ),
        // me: 動かす前の自分の矩形（右上から）。
        (
            "me",
            (200, 500),
            &["--base=me", "--X=10", "--Y=20", "--base-offset=right.top"],
            (500 + 10, 500 + 20),
        ),
        // scope 0: その窓の矩形（左下に右下をそろえる）。
        (
            "scope0",
            (200, 500),
            &[
                "--base=0",
                "--X=0",
                "--Y=0",
                "--base-offset=left.bottom",
                "--move-offset=right.bottom",
            ],
            (1000 - 300, 987 - 434),
        ),
        // 無いスコープ: screen へ倒す。
        (
            "missing",
            (200, 500),
            &["--base=5", "--X=10", "--Y=20"],
            (10, 20),
        ),
    ];
    for &(name, (bx, by), script, (ax, ay)) in rows {
        let (mut world, gw) = world_with(Anchor::Free);
        let target = gw.char_window(1).unwrap();
        place(&mut world, target, bx, by);
        let (moved, events) =
            capture(|| apply_move_directive(&mut world, &args(script), ScaleRatio::ONE));
        assert!(moved, "{name}: 動く（非スコープの基準でも動く）");
        assert_eq!(pos(&world, target), PointPx { x: ax, y: ay }, "{name}");
        assert_eq!(
            events_named(&events, "move_applied").len(),
            1,
            "{name}: {events:?}"
        );
    }
}

/// 無いスコープと `global` の参照点は `move_frame_substituted` の `warn!` で 1 件ずつ記録する。
#[test]
fn substitutions_are_logged_one_by_one() {
    let cases: &[(&[&str], &[&str])] = &[
        (&["--base=5", "--X=10", "--Y=20"], &["MissingBaseScope"]),
        (
            &["--base=5", "--X=10", "--Y=20", "--base-offset=base.base"],
            &["MissingBaseScope", "AreaBaseToCenter", "AreaBaseToCenter"],
        ),
        (
            &[
                "--base=global",
                "--X=10",
                "--Y=20",
                "--base-offset=right.bottom",
            ],
            &["GlobalOffsetIgnored"],
        ),
        (&["--base=0", "--X=10", "--Y=20"], &[]),
    ];
    for &(script, want) in cases {
        let (mut world, _gw) = world_with(Anchor::Free);
        let (moved, events) =
            capture(|| apply_move_directive(&mut world, &args(script), ScaleRatio::ONE));
        assert!(moved, "{script:?}");
        let subs = events_named(&events, "move_frame_substituted");
        assert_eq!(subs.len(), want.len(), "{script:?}: {events:?}");
        for (ev, w) in subs.iter().zip(want) {
            assert_eq!(ev.level, tracing::Level::WARN);
            assert!(
                ev.field("note").is_some_and(|n| n.starts_with(w)),
                "{script:?}: {ev:?}"
            );
        }
    }
}

// 吸い付きの当て直し

const EMO2_ADJUST: &[&str] = &[
    "--X=0",
    "--Y=0",
    "--base=0",
    "--base-offset=left.bottom",
    "--move-offset=right.bottom",
];

/// 下端に吸い付いた窓の Y は台本でなく作業領域の下端 − 高さ。`free` は台本どおり。
#[test]
fn bottom_anchor_ignores_script_y_and_free_keeps_it() {
    for (anchor, want_y) in [(Anchor::Bottom, 1040 - 434), (Anchor::Free, 987 - 434)] {
        let (mut world, gw) = world_with(anchor);
        let target = gw.char_window(1).unwrap();
        assert!(apply_move_directive(
            &mut world,
            &args(EMO2_ADJUST),
            ScaleRatio::ONE
        ));
        assert_eq!(
            pos(&world, target),
            PointPx {
                x: 1000 - 300,
                y: want_y
            },
            "{anchor:?}"
        );
    }
}

/// 適用の後に大きさが変わっても、下端にそろったまま横の中央を保つ。
#[test]
fn resize_after_move_keeps_bottom_and_center() {
    let (mut world, gw) = world_with(Anchor::Bottom);
    let target = gw.char_window(1).unwrap();
    assert!(apply_move_directive(
        &mut world,
        &args(EMO2_ADJUST),
        ScaleRatio::ONE
    ));
    let before = pos(&world, target);
    resize_window_to(
        &mut world,
        target,
        SizePx { w: 200, h: 300 },
        PlacementRoute::Resnap,
    );
    assert_eq!(
        pos(&world, target),
        PointPx {
            x: before.x + 300 / 2 - 200 / 2,
            y: 1040 - 300,
        },
        "下端は作業領域の下端・横の中央は動かない"
    );
}

// バルーン・記憶・画面の外・Anchored

/// 画面の外へ出ても寄せず、バルーンは今のずらしで付いて動き、記憶には何も送られず、
/// `Anchored` とバルーンのずらしはビット同一。
#[test]
fn move_keeps_balloon_offset_and_writes_nothing_else() {
    let shared = Arc::new(FakePersistIo::new());
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    let parts = spawn_sylphya(SylphyaInit {
        roots: roots.clone(),
        io: Box::new(SharedFakeIo::new(shared.clone())),
        runtime_sink: None,
    });
    let (mut world, gw) = world_with(Anchor::Free);
    world.insert_non_send(PersistWiring {
        publisher: parts.publisher.clone(),
    });
    let target = gw.char_window(1).unwrap();
    let balloon = gw.balloon_window(1).unwrap();
    let anchored = *world.get::<Anchored>(target).unwrap();
    let follow = *world.get::<BalloonFollow>(target).unwrap();
    let default_pos = gw.default_char_pos(1);

    assert!(apply_move_directive(
        &mut world,
        &args(&["--base=global", "--X=5000", "--Y=-3000"]),
        ScaleRatio::ONE
    ));

    assert_eq!(
        pos(&world, target),
        PointPx { x: 5000, y: -3000 },
        "画面の外でも寄せない"
    );
    assert_eq!(
        pos(&world, balloon),
        PointPx {
            x: 5000 + BALLOON_OFFSET.x,
            y: -3000 + BALLOON_OFFSET.y
        },
        "バルーンは今のずらしで付いて動く"
    );
    assert_eq!(*world.get::<Anchored>(target).unwrap(), anchored);
    assert_eq!(*world.get::<BalloonFollow>(target).unwrap(), follow);
    assert_eq!(
        world.resource::<GhostWindows>().default_char_pos(1),
        default_pos
    );

    parts.publisher.barrier().expect("barrier が返る");
    let stored = load_scope(
        PersistScope::Ghost,
        &roots,
        &SharedFakeIo::new(shared.clone()),
    );
    assert!(stored.is_empty(), "記憶に何も送られない: {stored:?}");
    parts.publisher.close();
    crate::spine::join_bounded("sylphya", parts.handle).expect("sylphya が終わる");
}

// 記録

/// `move_applied` の 1 行に基準・参照点・基準の枠・前・台本・後の欄が出る。
#[test]
fn move_applied_records_base_and_ref_points() {
    let (mut world, _gw) = world_with(Anchor::Bottom);
    let (_, events) =
        capture(|| apply_move_directive(&mut world, &args(EMO2_ADJUST), ScaleRatio::ONE));
    let applied = events_named(&events, "move_applied");
    assert_eq!(applied.len(), 1, "{events:?}");
    let ev = &applied[0];
    assert_eq!(ev.level, tracing::Level::INFO);
    for (key, want) in [
        ("scope", "1"),
        ("base", "Scope(0)"),
        ("anchor", "Some(Bottom)"),
        ("base_left", "1000"),
        ("base_top", "300"),
        ("base_right", "1400"),
        ("base_bottom", "987"),
        ("from_x", "200"),
        ("from_y", "500"),
        ("script_x", "700"),
        ("script_y", "553"),
        ("to_x", "700"),
        ("to_y", "606"),
    ] {
        assert_eq!(ev.field(key), Some(want), "{key}: {ev:?}");
    }
    for key in ["base_offset", "move_offset"] {
        assert!(ev.field(key).is_some(), "{key}: {ev:?}");
    }
}

// 材料が無いとき

type Strip = fn(&mut World);

/// 窓の組・対象の窓・その位置・作業領域・主モニタが無いか、窓へ書けなければ `move_skipped` の `warn!` で動かさない。
#[test]
fn missing_material_skips_with_warn() {
    let cases: [(&str, &[&str], Strip); 6] = [
        ("no_ghost_windows", &["--base=0", "--X=1"], |w| {
            w.remove_resource::<GhostWindows>();
        }),
        ("no_target_window", &["--base=0", "--X=1"], |w| {
            let e = w.resource::<GhostWindows>().char_window(1).unwrap();
            w.resource_mut::<GhostWindows>().remove_entry_of(e);
        }),
        ("write_refused", &["--base=0", "--X=1"], |w| {
            let e = w.resource::<GhostWindows>().char_window(1).unwrap();
            w.entity_mut(e).remove::<WindowHandle>();
        }),
        ("no_target_position", &["--base=0", "--X=1"], |w| {
            let e = w.resource::<GhostWindows>().char_window(1).unwrap();
            w.entity_mut(e).remove::<WindowPos>();
        }),
        ("no_work_area", &["--X=1"], |w| {
            w.remove_resource::<MonitorSnapshot>();
        }),
        (
            "no_primary_monitor",
            &["--base=primaryscreen", "--X=1"],
            |w| {
                let ms: Vec<Entity> = w
                    .query_filtered::<Entity, With<Monitor>>()
                    .iter(w)
                    .collect();
                for m in ms {
                    w.despawn(m);
                }
            },
        ),
    ];
    for (reason, script, strip) in cases {
        let (mut world, gw) = world_with(Anchor::Free);
        let target = gw.char_window(1).unwrap();
        strip(&mut world);
        let before = world.get::<WindowPos>(target).and_then(|w| w.position);
        let (moved, events) =
            capture(|| apply_move_directive(&mut world, &args(script), ScaleRatio::ONE));
        assert!(!moved, "{reason}: 動かさない");
        let skips = events_named(&events, "move_skipped");
        assert_eq!(skips.len(), 1, "{reason}: {events:?}");
        assert_eq!(skips[0].level, tracing::Level::WARN);
        assert_eq!(skips[0].field_str("reason"), Some(reason));
        assert!(events_named(&events, "move_applied").is_empty());
        assert_eq!(
            world.get::<WindowPos>(target).and_then(|w| w.position),
            before,
            "{reason}: 位置は変わらない"
        );
    }
}

/// `Anchored` が無ければ `move_anchor_missing` の `warn!` のうえで当て直さずに書く。
#[test]
fn missing_anchored_writes_script_position_with_warn() {
    let (mut world, gw) = world_with(Anchor::Bottom);
    let target = gw.char_window(1).unwrap();
    world.entity_mut(target).remove::<Anchored>();
    let (moved, events) =
        capture(|| apply_move_directive(&mut world, &args(EMO2_ADJUST), ScaleRatio::ONE));
    assert!(moved);
    let missing = events_named(&events, "move_anchor_missing");
    assert_eq!(missing.len(), 1, "{events:?}");
    assert_eq!(missing[0].level, tracing::Level::WARN);
    assert_eq!(
        pos(&world, target),
        PointPx { x: 700, y: 553 },
        "当て直さない"
    );
}

// 面の `move` のずれとの関係

/// 絶対の位置へ書いた軸は行き先が新しい元の位置になり、ずれはその上へ当て直し、0 で行き先へ戻る。
/// `fix` の軸は今の位置からの相対なので、ずれを二度数えずに元の位置へ戻る。
#[test]
fn absolute_move_rebases_the_pattern_shift_per_axis() {
    let (mut world, gw) = world_with(Anchor::Free);
    let target = gw.char_window(1).unwrap();
    assert!(apply_char_shift(
        &mut world,
        target,
        (10, 20),
        ScaleRatio::ONE
    ));
    assert_eq!(pos(&world, target), PointPx { x: 210, y: 520 });

    // X だけ絶対（screen の左端から 50）・Y は fix。
    assert!(apply_move_directive(
        &mut world,
        &args(&["--X=50"]),
        ScaleRatio::ONE
    ));
    assert_eq!(pos(&world, target), PointPx { x: 60, y: 520 });

    assert!(apply_char_shift(
        &mut world,
        target,
        (0, 0),
        ScaleRatio::ONE
    ));
    assert_eq!(pos(&world, target), PointPx { x: 50, y: 500 });
}

/// ずれを当て直した窓の組の相手は、当て直した後の位置までの量だけ連れ動き、`move_applied` の
/// 行き先も当て直した後の位置になる。
#[test]
fn absolute_move_with_shift_moves_the_partner_by_the_final_amount() {
    let (mut world, gw) = world_with(Anchor::Free);
    let target = gw.char_window(1).unwrap();
    let partner = gw.char_window(0).unwrap();
    seed_sticky_descript(&mut world, None);
    assert!(apply_set_sticky(
        &mut world,
        &["0".to_string(), "1".to_string()]
    ));
    assert!(apply_char_shift(
        &mut world,
        target,
        (10, 20),
        ScaleRatio::ONE
    ));
    let (target_before, partner_before) = (pos(&world, target), pos(&world, partner));

    let (moved, events) =
        capture(|| apply_move_directive(&mut world, &args(&["--X=50"]), ScaleRatio::ONE));
    assert!(moved);

    let target_now = pos(&world, target);
    assert_eq!(target_now, PointPx { x: 60, y: 520 });
    let partner_now = pos(&world, partner);
    assert_eq!(
        (
            partner_now.x - partner_before.x,
            partner_now.y - partner_before.y
        ),
        (
            target_now.x - target_before.x,
            target_now.y - target_before.y
        )
    );
    let applied = events_named(&events, "move_applied");
    assert_eq!(applied.len(), 1, "{events:?}");
    assert_eq!(applied[0].field("to_x"), Some("60"));
    assert_eq!(applied[0].field("to_y"), Some("520"));
}

/// 自分の窓を基準にした `\![move]` は相対なので、当てた分を残す（ずれを二度数えない）。
#[test]
fn relative_move_keeps_the_pattern_shift() {
    for base in ["--base=me", "--base=1"] {
        let (mut world, gw) = world_with(Anchor::Free);
        let target = gw.char_window(1).unwrap();
        assert!(apply_char_shift(
            &mut world,
            target,
            (10, 20),
            ScaleRatio::ONE
        ));

        assert!(apply_move_directive(
            &mut world,
            &args(&["--X=5", "--Y=5", base]),
            ScaleRatio::ONE
        ));
        assert_eq!(pos(&world, target), PointPx { x: 215, y: 525 }, "{base}");

        assert!(apply_char_shift(
            &mut world,
            target,
            (0, 0),
            ScaleRatio::ONE
        ));
        assert_eq!(pos(&world, target), PointPx { x: 205, y: 505 }, "{base}");
    }
}
