//! シェルの見た目の値の入れ直しの決定論テスト。
//!
//! 窓は bare `World` に spawn の本物で組み、偽 HWND の `WindowHandle` で窓書込の口を通す
//! （`SetWindowPosCommand` は enqueue だけで flush しない＝実窓には触れない）。

use std::fs;

use areka_parsers::balloon::{WindowPosition, WindowPositionRaw};
use bevy_ecs::prelude::*;
use wintf::ecs::drag::DragConfig;
use wintf::ecs::{DPI, Point, WindowPos};

use super::test_support::{
    BALLOON, CHAR0, booted_world, char_positions, new_shell, position_of, world_from,
};
use super::{BalloonPlacementInputs, apply_shell_descript};
use crate::diag::PlacementRoute;
use crate::dpi_sync::DpiSyncHold;
use crate::face_windowposition::BalloonUserMoved;
use crate::follow::{
    AnchorSource, Anchored, BalloonFollow, MonitorSnapshot, OffsetBase, resize_window_to,
};
use crate::persist::balloon_offset_entries;
use crate::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use crate::sample_test_support::{balloon_root, emo2_root};
use crate::shared_test_support::TempDir;
use crate::source::{load_balloon_author_dpi, load_descript_source_for_shell};
use crate::spawn::{BalloonKeywordBase, BalloonLimit, CharWindowMarker, GhostWindows};
use crate::test_support::{
    KEYWORD_BALLOON_DESCRIPT, WA, synth_declared_dpi_ghost, with_com_initialized,
};
use crate::{load_scope_windowpositions, prepare_ghost_windows_with_work_area};
use log_capture_kit::{ExpectField, capture};

/// バルーン側の `windowposition` を持たない走っているバルーン（作者の DPI は 96）。
fn no_balloon() -> BalloonPlacementInputs {
    BalloonPlacementInputs {
        author_dpi: 96,
        windowpositions: Default::default(),
    }
}

fn follow_of(world: &World, windows: &GhostWindows, scope: usize) -> BalloonFollow {
    *world
        .get::<BalloonFollow>(windows.char_window(scope).unwrap())
        .unwrap()
}

fn char_window_count(world: &mut World) -> usize {
    world.query::<&CharWindowMarker>().iter(world).count()
}

/// 新しいシェルのずらしと揃え方がキャラ窓の部品に入り、窓の位置とスコープの集合は保たれ、
/// バルーン窓は新しいずらしで 1 度置き直される。`char2.` のキーがあっても設定にスコープ 2 は
/// 現れないので、読み飛ばしの記録は 0 件。
#[test]
fn t_rs01_new_offset_and_alignment_reach_the_char_window_parts() {
    let (mut world, windows) = booted_world(96);
    let before = char_positions(&world, &windows);

    let ((), events) =
        capture(|| apply_shell_descript(&mut world, &windows, &new_shell(&[]), &no_balloon(), &[]));

    let c0 = windows.char_window(0).unwrap();
    let pinned96 = Some(DPI::from_dpi(96, 96));
    // sakura: 右（キャラ窓の幅）＋ (30,40)。
    let want0 = PointPx {
        x: CHAR0.w - 30,
        y: 40,
    };
    assert_eq!(follow_of(&world, &windows, 0).offset(), want0);
    assert_eq!(follow_of(&world, &windows, 0).base().dpi, pinned96);
    // kero: 左（バルーンの幅）＋ (-7,9)。
    assert_eq!(
        follow_of(&world, &windows, 1).offset(),
        PointPx {
            x: -BALLOON.w - 7,
            y: 9
        }
    );
    for scope in [0, 1] {
        let c = windows.char_window(scope).unwrap();
        assert_eq!(world.get::<Anchored>(c), Some(&Anchored(Anchor::Free)));
        assert!(
            world.get::<DragConfig>(c).unwrap().move_window,
            "free の窓のドラッグが揃え方に合っていない"
        );
    }
    // キーワードの素材は新しい値に無いので外れる。
    assert!(world.get::<BalloonKeywordBase>(c0).is_none());

    assert_eq!(char_positions(&world, &windows), before);
    assert_eq!(
        world
            .resource::<GhostWindows>()
            .scopes()
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(char_window_count(&mut world), 2);

    let p0 = before[0];
    assert_eq!(
        position_of(&world, windows.balloon_window(0).unwrap()),
        Point {
            x: p0.x + want0.x,
            y: p0.y + want0.y
        }
    );

    let skipped: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("reseed_scope_not_running"))
        .collect();
    assert_eq!(skipped.len(), 0, "{events:?}");
}

/// 保存済みのバルーンのずらしは新しいシェルの値より優先して保たれる（起動と同じ規則）。
#[test]
fn t_rs02_saved_balloon_offset_survives_the_reseed() {
    let (mut world, windows) = booted_world(96);

    apply_shell_descript(
        &mut world,
        &windows,
        &new_shell(&[]),
        &no_balloon(),
        &balloon_offset_entries(0, PointPx { x: -123, y: -45 }),
    );

    let f0 = follow_of(&world, &windows, 0);
    assert_eq!(f0.offset(), PointPx { x: -123, y: -45 });
    assert_eq!(f0.base().dpi, None, "保存値は起動と同じく未係留で運ぶ");
    // 記憶から戻したバルーンは利用者が動かしたものとして、次の面の置き直しで動かさない。
    let moved = |scope| {
        let c = windows.char_window(scope).unwrap();
        world.entity(c).contains::<BalloonUserMoved>()
    };
    assert!(moved(0), "記憶から戻した scope 0 に動かした印が無い");
    assert!(!moved(1), "標準の位置の scope 1 に動かした印がある");
    // 保存の無い scope は新しいシェルの値。
    assert_eq!(
        follow_of(&world, &windows, 1).offset(),
        PointPx {
            x: -BALLOON.w - 7,
            y: 9
        }
    );
}

/// 作者の空間のずらしは、起動と同じく窓の DPI ÷ 作者の DPI で物理 px へ換算する。
#[test]
fn t_rs03_author_offset_is_scaled_by_the_window_dpi() {
    let (mut world, windows) = booted_world(192);

    apply_shell_descript(&mut world, &windows, &new_shell(&[]), &no_balloon(), &[]);

    let f0 = follow_of(&world, &windows, 0);
    assert_eq!(
        f0.offset(),
        PointPx {
            x: CHAR0.w - 60,
            y: 80
        }
    );
    assert_eq!(f0.base().dpi, Some(DPI::from_dpi(192, 192)));

    // 作者の DPI が窓と同じなら換算しない。
    let (mut world, windows) = booted_world(192);
    apply_shell_descript(
        &mut world,
        &windows,
        &new_shell(&[("seriko.dpi", "192")]),
        &no_balloon(),
        &[],
    );
    assert_eq!(
        follow_of(&world, &windows, 0).offset(),
        PointPx {
            x: CHAR0.w - 30,
            y: 40
        }
    );
}

/// 同じ作業領域でも窓の DPI が違うスコープは、それぞれの DPI で解く（同じ作業領域と DPI の窓
/// だけを 1 度にまとめる）。拡大率の移り変わりの途中で、スコープ 0 が 96・スコープ 1 が 192 の形。
#[test]
fn t_rs03b_scopes_with_different_window_dpis_are_solved_with_their_own_dpi() {
    let (mut world, windows) = booted_world(96);
    let dpi192 = DPI::from_dpi(192, 192);
    for window in [windows.char_window(1), windows.balloon_window(1)] {
        world.entity_mut(window.unwrap()).insert(dpi192);
    }

    apply_shell_descript(&mut world, &windows, &new_shell(&[]), &no_balloon(), &[]);

    let f0 = follow_of(&world, &windows, 0);
    assert_eq!(
        (f0.offset(), f0.base().dpi),
        (
            PointPx {
                x: CHAR0.w - 30,
                y: 40
            },
            Some(DPI::from_dpi(96, 96))
        )
    );
    let f1 = follow_of(&world, &windows, 1);
    assert_eq!(
        (f1.offset(), f1.base().dpi),
        (
            PointPx {
                x: -BALLOON.w - 14,
                y: 18
            },
            Some(dpi192)
        ),
        "スコープ 1 がスコープ 0 の DPI で解かれた"
    );
}

/// 窓が無いスコープは `warn!` を 1 件残して読み飛ばし、残りのスコープは続ける。
#[test]
fn t_rs04_a_missing_window_is_skipped_with_a_warning_and_the_rest_continue() {
    let (mut world, windows) = booted_world(96);
    world.despawn(windows.char_window(1).unwrap());

    let ((), events) =
        capture(|| apply_shell_descript(&mut world, &windows, &new_shell(&[]), &no_balloon(), &[]));

    let skipped: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("reseed_skipped"))
        .collect();
    assert_eq!(skipped.len(), 1, "{events:?}");
    assert_eq!(skipped[0].level, tracing::Level::WARN);
    assert_eq!(skipped[0].field("scope"), Some("1"));
    assert_eq!(
        follow_of(&world, &windows, 0).offset(),
        PointPx {
            x: CHAR0.w - 30,
            y: 40
        }
    );
}

/// バルーン窓が無いスコープと、バルーン窓の大きさが読めないスコープは別の理由で読み飛ばす。
#[test]
fn t_rs04c_balloon_window_gone_and_size_unknown_have_their_own_reasons() {
    type Break = fn(&mut World, Entity);
    let cases: [(&str, Break); 2] = [
        ("balloon_window_gone", |world, balloon| {
            world.entity_mut(balloon).remove::<WindowPos>();
        }),
        ("balloon_size_unknown", |world, balloon| {
            world.get_mut::<WindowPos>(balloon).unwrap().size = None;
        }),
    ];
    for (reason, strip) in cases {
        let (mut world, windows) = booted_world(96);
        strip(&mut world, windows.balloon_window(1).unwrap());

        let ((), events) = capture(|| {
            apply_shell_descript(&mut world, &windows, &new_shell(&[]), &no_balloon(), &[])
        });

        let skipped: Vec<_> = events
            .iter()
            .filter(|e| e.field_str("event") == Some("reseed_skipped"))
            .collect();
        assert_eq!(skipped.len(), 1, "{reason}: {events:?}");
        assert_eq!(skipped[0].field("scope"), Some("1"), "{reason}");
        assert_eq!(skipped[0].field_str("reason"), Some(reason));
    }
}

/// 起動の準備と同じ入力（同じゴースト・同じバルーン・同じ窓寸・同じ DPI）なら、入れ直した
/// 部品は起動の配置と一致する。バルーンの `windowposition` の数値（emo2）・キーワードと y の
/// 調整（合成バルーンの `center`／`-20`）が、シェルの差し替えをまたいで残ることの錨。
///
/// 最後の行は連鎖の錨: `sakura.defaultx,1000` で本体を左へ寄せ、`kero.balloon.alignment,none`
/// の相方の側を「連鎖した仮の X の中心」で決めさせる（連鎖なら中央より左＝右側、スコープ単独で
/// 解けば右端＝左側になって起動と割れる）。作業領域は起動と同じ `WA`。
#[test]
fn t_rs05_reseed_matches_boot_for_the_same_inputs() {
    with_com_initialized(|| {
        let root = TempDir::new();
        let (kw_ghost, kw_balloon) =
            synth_declared_dpi_ghost(root.path(), &emo2_root(), &balloon_root(), "96", "96", None);
        fs::write(kw_balloon.join("descript.txt"), KEYWORD_BALLOON_DESCRIPT)
            .expect("キーワードのバルーンを書けるはず");
        let chain_root = TempDir::new();
        let (chain_ghost, chain_balloon) = synth_declared_dpi_ghost(
            chain_root.path(),
            &emo2_root(),
            &balloon_root(),
            "96",
            "96",
            None,
        );
        fs::write(
            chain_ghost.join("shell").join("master").join("descript.txt"),
            "charset,UTF-8\nseriko.alignmenttodesktop,bottom\nsakura.defaultx,1000\nkero.defaultx,0\nsakura.balloon.alignment,left\nkero.balloon.alignment,none\n",
        )
        .expect("連鎖の錨のシェルを書けるはず");
        for (ghost, balloon, dpi) in [
            (emo2_root(), balloon_root(), 96u16),
            (emo2_root(), balloon_root(), 192),
            (kw_ghost.clone(), kw_balloon.clone(), 96),
            (chain_ghost.clone(), chain_balloon.clone(), 96),
        ] {
            let boot =
                prepare_ghost_windows_with_work_area(&ghost, &balloon, WA, Some(u32::from(dpi)))
                    .expect("起動の配置の準備は成功する")
                    .placements;
            // 走っている窓は起動の窓寸で、部品だけを崩しておく（入れ直しが空振りなら一致しない）。
            let skewed: Vec<ScopePlacement> = boot
                .iter()
                .map(|p| ScopePlacement {
                    anchor: Anchor::Free,
                    balloon_offset_base: OffsetBase::unpinned(PointPx { x: 1, y: 1 }),
                    balloon_keyword_base: None,
                    ..*p
                })
                .collect();
            let (mut world, windows) = world_from(&skewed, dpi);
            world.insert_resource(MonitorSnapshot {
                work_areas: vec![WA],
            });
            let src = load_descript_source_for_shell(&ghost, None).expect("ゴーストの descript");
            let inputs = BalloonPlacementInputs {
                author_dpi: load_balloon_author_dpi(&balloon),
                windowpositions: load_scope_windowpositions(&balloon, &[0, 1]),
            };

            apply_shell_descript(&mut world, &windows, &src, &inputs, &[]);

            for p in &boot {
                let c = windows.char_window(p.scope).unwrap();
                let case = format!("{} dpi={dpi} scope={}", ghost.display(), p.scope);
                assert_eq!(
                    world.get::<Anchored>(c),
                    Some(&Anchored(p.anchor)),
                    "{case}"
                );
                assert_eq!(
                    world.get::<BalloonFollow>(c).unwrap().base(),
                    p.balloon_offset_base,
                    "{case}"
                );
                assert_eq!(
                    world
                        .get::<BalloonKeywordBase>(c)
                        .map(|k| (k.mode, k.adjust)),
                    p.balloon_keyword_base,
                    "{case}"
                );
            }
        }
    });
}

fn anchor_of(world: &World, windows: &GhostWindows, scope: usize) -> (Anchor, AnchorSource, bool) {
    let c = windows.char_window(scope).unwrap();
    (
        world.get::<Anchored>(c).unwrap().0,
        *world.get::<AnchorSource>(c).unwrap(),
        world.get::<DragConfig>(c).unwrap().move_window,
    )
}

/// シェルの切り替えは台本で変えた向きを保つ:
/// 台本の印のある窓（scope 0・`free`）は向きが残り、印の無い窓（scope 1）は新しいシェルの向き
/// （`top`）になる。どの窓も `AnchorSource.descript` は新しい値。向きが変わった窓だけ
/// `alignment_changed`（shell_switch）が出て、`move_window` は書き手どおり（組なし・`free` だけ真）。
#[test]
fn t_rs06_script_alignment_survives_and_unmarked_scopes_take_the_new_shell() {
    let (mut world, windows) = booted_world(96);
    let c0 = windows.char_window(0).unwrap();
    world.entity_mut(c0).insert((
        Anchored(Anchor::Free),
        AnchorSource {
            descript: Anchor::Bottom,
            by_script: true,
        },
    ));

    let ((), events) = capture(|| {
        apply_shell_descript(
            &mut world,
            &windows,
            &new_shell(&[("seriko.alignmenttodesktop", "top")]),
            &no_balloon(),
            &[],
        )
    });

    let source = |by_script| AnchorSource {
        descript: Anchor::Top,
        by_script,
    };
    assert_eq!(
        (
            anchor_of(&world, &windows, 0),
            anchor_of(&world, &windows, 1)
        ),
        (
            (Anchor::Free, source(true), true),
            (Anchor::Top, source(false), false)
        ),
        "(向き, 出どころ, move_window) の scope 0・1"
    );
    let changed: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("alignment_changed"))
        .map(|e| {
            (
                e.level,
                e.expect_field("scope"),
                e.expect_field("from"),
                e.expect_field("to"),
                e.field_str("source"),
            )
        })
        .collect();
    assert_eq!(
        changed,
        vec![(
            tracing::Level::INFO,
            "1",
            "Bottom",
            "Top",
            Some("shell_switch")
        )],
        "{events:?}"
    );
}

/// `move_window` は書き手（帳簿と向き）から付け直す: 組 `{0,1}` の窓は、新しい
/// シェルが `free` でも wintf に動かさせない。
#[test]
fn t_rs07_move_window_follows_the_drag_writer_for_grouped_windows() {
    let (mut world, windows) = booted_world(96);
    crate::sticky_group_ledger::seed_sticky_descript(&mut world, Some("0,1"));

    apply_shell_descript(&mut world, &windows, &new_shell(&[]), &no_balloon(), &[]);

    let got: Vec<_> = windows
        .scopes()
        .map(|s| anchor_of(&world, &windows, s))
        .map(|(anchor, _, moves)| (anchor, moves))
        .collect();
    assert_eq!(got, vec![(Anchor::Free, false), (Anchor::Free, false)]);
}

/// 知らない `alignmenttodesktop` の値の警告は、走っているスコープの数によらずスコープごとに 1 行
/// （同じ作業領域と DPI の窓は 1 度だけ解く）。
#[test]
fn t_rs08_unknown_alignment_warns_once_per_scope() {
    let (mut world, windows) = booted_world(96);

    let ((), events) = capture(|| {
        apply_shell_descript(
            &mut world,
            &windows,
            &new_shell(&[("seriko.alignmenttodesktop", "sideways")]),
            &no_balloon(),
            &[],
        )
    });

    let mut warned: Vec<&str> = events
        .iter()
        .filter(|e| {
            e.message()
                .contains("alignmenttodesktop の未知アンカー指定")
        })
        .map(|e| e.expect_field("scope"))
        .collect();
    warned.sort_unstable();
    assert_eq!(warned, vec!["0", "1"], "{events:?}");
}

/// 作業領域を引けないスコープは `warn!`（`reseed_work_area_missing`）を残してから窓の矩形で代える。
#[test]
fn t_rs04b_missing_work_area_warns_before_substituting_the_window_rect() {
    let (mut world, windows) = booted_world(96);
    world.remove_resource::<MonitorSnapshot>();

    let ((), events) =
        capture(|| apply_shell_descript(&mut world, &windows, &new_shell(&[]), &no_balloon(), &[]));

    let missing: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("reseed_work_area_missing"))
        .collect();
    assert!(!missing.is_empty(), "{events:?}");
    assert!(missing.iter().all(|e| e.level == tracing::Level::WARN));
}

/// 走っているバルーンの `windowposition.limit,0` は、シェルの切り替えでそのスコープのバルーン窓の
/// `BalloonLimit` へ入る（起動の値 `true` のまま残さない）。
#[test]
fn t_rs09_balloon_limit_is_reinstalled_from_the_new_placement() {
    let (mut world, windows) = booted_world(96);
    let limit = |world: &World, scope| {
        world
            .get::<BalloonLimit>(windows.balloon_window(scope).unwrap())
            .copied()
    };
    assert_eq!(limit(&world, 0), Some(BalloonLimit(true)), "前提: 起動の値");
    let balloon = BalloonPlacementInputs {
        author_dpi: 96,
        windowpositions: [(
            0,
            (
                WindowPosition::new(None, None),
                WindowPositionRaw::new(None, Some("0".to_string())),
            ),
        )]
        .into(),
    };

    apply_shell_descript(&mut world, &windows, &new_shell(&[]), &balloon, &[]);

    assert_eq!(limit(&world, 0), Some(BalloonLimit(false)));
    assert_eq!(limit(&world, 1), Some(BalloonLimit(true)));
}

/// 揃え方 `top` のシェルへ切り替えて置き直す。`held` のスコープのキャラ窓には先に整合待ちの札を付ける。
fn reanchor_to_top(
    held: Option<usize>,
) -> (World, GhostWindows, Vec<log_capture_kit::CapturedEvent>) {
    let (mut world, windows) = booted_world(96);
    if let Some(scope) = held {
        world
            .entity_mut(windows.char_window(scope).unwrap())
            .insert(DpiSyncHold { since_frame: 1 });
    }
    apply_shell_descript(
        &mut world,
        &windows,
        &new_shell(&[("seriko.alignmenttodesktop", "top")]),
        &no_balloon(),
        &[],
    );
    let ((), events) = capture(|| super::reanchor_char_windows(&mut world, &windows));
    (world, windows, events)
}

/// シェルの切り替えの置き直しは整合待ちの札のある窓を
/// 見送り（位置は前のまま・`reseed_held` の `debug!`）、札の無い窓は新しい揃え方へ動かす。
/// 見送りを外すと札のある窓への書込が窓書込の口の `debug_assert!`（`route=Some(AnchorChange)`）で落ちる。
#[test]
fn t_rs10_reanchor_holds_a_window_with_the_dpi_sync_tag() {
    let (before_world, before_windows) = booted_world(96);
    let before = char_positions(&before_world, &before_windows);

    let (world, windows, events) = reanchor_to_top(Some(0));
    let after = char_positions(&world, &windows);

    assert_eq!(after[0], before[0], "札のある窓は前の位置のまま");
    assert_ne!(after[1], before[1], "札の無い窓は新しい揃え方へ動く");
    let held: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("reseed_held"))
        .map(|e| (e.level, e.expect_field("scope")))
        .collect();
    assert_eq!(held, vec![(tracing::Level::DEBUG, "0")], "{events:?}");
}

/// 札を外して拡大率の相の書込（`DpiReproject`）を通すと、見送った窓は入れ直した
/// 揃え方の位置（札が無かったときの置き直しと同じ位置）に置かれる。
#[test]
fn t_rs11_held_window_takes_the_new_alignment_once_the_tag_is_removed() {
    let (untagged_world, untagged_windows, _) = reanchor_to_top(None);
    let want = char_positions(&untagged_world, &untagged_windows)[0];
    let (boot_world, boot_windows) = booted_world(96);
    assert_ne!(
        want,
        char_positions(&boot_world, &boot_windows)[0],
        "新しい揃え方の位置は前の位置と違う（同じなら檻が空回りする）"
    );

    let (mut world, windows, _) = reanchor_to_top(Some(0));
    let c0 = windows.char_window(0).unwrap();
    world.entity_mut(c0).remove::<DpiSyncHold>();
    let size = world.get::<WindowPos>(c0).unwrap().size.unwrap();
    resize_window_to(
        &mut world,
        c0,
        SizePx {
            w: size.width,
            h: size.height,
        },
        PlacementRoute::DpiReproject,
    );

    assert_eq!(position_of(&world, c0), want);
}
