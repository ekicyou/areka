//! 鎖の規則の檻。
//!
//! 起動の並べ直し・隣り合いの印の保存・拡大率の後の詰め直しの 3 か所が、同じ規則で組を作る:
//! ⑴ 鎖の前の窓は、自分より下の番号で、左右の辺に揃えない一番近いスコープ。⑵ 左の組は隙間 0 か
//! そのスコープの `defaultx` の隙間で隣り合う（右の組は隙間 0 だけ）。⑶ 覚えた印 `left` は
//! 鎖の前の窓の左に `defaultx` の隙間で付く。⑷ 詰め直しは、辺のスコープの隣に隙間 0 で付いていた窓を
//! その辺のスコープの隣へ付け直す。規則の説明は `areka_placement::chain_rule`。
//!
//! 窓は本番と同じく resolver の出力で生やす（`chain_finalize_align_tests` の `spawn_resolved`）。
//! 拡大率の変化は、窓の大きさを変えて下端の中央を保つ置き直し（拡大率の相の再射影の結果の形）で作る。

use bevy_ecs::change_detection::DetectChangesMut;
use wintf::ecs::drag::DraggingState;
use wintf::ecs::{Point, SizeI, WindowPos};

use super::chain_finalize_align_tests::{
    READ_ACROSS, SPAWN, WA, sizes, spawn_resolved, spawn_resolved_scopes, x_of,
};
use super::persist_test_support::PersistStore;
use super::test_support::PerTargetSizes;
use super::*;
use log_capture_kit::{LineFormat, capture_lines};

use areka_placement::chain_finalize::{ADJACENT_TOLERANCE_PX, AdjacencyMark, ScriptPlaced};
use areka_placement::chain_realign::{arm_chain_realign, snapshot_char_rects};
use areka_placement::resolver::Anchor;
use areka_placement::script_alignment::{AlignmentWord, apply_script_alignment};
use areka_placement::sticky_group_ledger::seed_sticky_descript;
use areka_sylphya::PersistKey;

/// 全スコープの窓の大きさを `num / den` 倍にし、下端の中央を保って置き直す。新しい大きさの
/// 絵が出ている形（並べ直しの走査が読む大きさ）を返す。
fn rescale_keeping_bottom_center(world: &mut World, num: i32, den: i32) -> PerTargetSizes {
    let gw = world.resource::<GhostWindows>().clone();
    let mut shown = Vec::new();
    for scope in gw.scopes() {
        let mut wp = world
            .get_mut::<WindowPos>(gw.char_window(scope).expect("char 窓がある"))
            .expect("WindowPos がある");
        let wp = wp.bypass_change_detection();
        let (pos, size) = wp.position.zip(wp.size).expect("位置と大きさがある");
        let (w, h) = (size.width * num / den, size.height * num / den);
        let center = pos.x + size.width / 2;
        wp.position = Some(Point {
            x: center - w / 2,
            y: pos.y + size.height - h,
        });
        wp.size = Some(SizeI::new(w, h));
        shown.push((scope, Some((w as u32, h as u32))));
    }
    PerTargetSizes::new(shown)
}

/// `prev` の左端 − `cur` の右端（`cur` が `prev` の左にある並びの隙間）。
fn left_gap(world: &World, prev: usize, cur: usize) -> i32 {
    let gw = world.resource::<GhostWindows>().clone();
    let w = world
        .get::<WindowPos>(gw.char_window(cur).expect("char 窓がある"))
        .and_then(|wp| wp.size)
        .expect("大きさがある")
        .width;
    x_of(world, prev) - (x_of(world, cur) + w)
}

/// 拡大率の変化を 1 回流し、詰め直しを解く（`num / den` 倍）。
fn change_scale_and_realign(world: &mut World, num: i32, den: i32) {
    let before = snapshot_char_rects(world);
    let shown = rescale_keeping_bottom_center(world, num, den);
    arm_chain_realign(world, before);
    realign_chain_once_with_source(&shown, world);
}

/// `kero.defaultx,36` を記憶なしで起こすと、並べ終えた時点の保存は印 `left` を書き、
/// 100% から 200% へ移っても 2 つの窓は重ならず、36 の隙間で並ぶ。
#[test]
fn the_defaultx_gap_is_saved_as_left_and_survives_100_to_200() {
    let shown = sizes(&SPAWN[..2]);
    let mut world = spawn_resolved(&[("kero.defaultx", "36")], &shown, 2);
    let store = PersistStore::fake();
    store.wire(&mut world);

    finalize_chain_once_with(&shown, &mut world);
    assert_eq!(
        left_gap(&world, 0, 1),
        36,
        "前提: 起動の並べ直しは 36 の隙間を残す"
    );
    assert_eq!(
        store
            .load()
            .get(&PersistKey::WindowAdjacent { scope: 1 })
            .map(String::as_str),
        Some("left"),
        "defaultx の隙間の組は印 left"
    );

    change_scale_and_realign(&mut world, 2, 1);

    assert_eq!(
        left_gap(&world, 0, 1),
        36,
        "200% の後も defaultx の隙間で並ぶ（重なったまま残らない）"
    );
    store.finish();
}

/// 記憶から戻した `defaultx` の隙間の組も、200% から 100% へ移った後に同じ隙間へ詰まる。
#[test]
fn a_remembered_defaultx_gap_pair_rejoins_after_200_to_100() {
    let shown = sizes(&SPAWN[..2]);
    let mut world = spawn_resolved(&[("kero.defaultx", "36")], &shown, 2);
    finalize_chain_once_with(&shown, &mut world);
    {
        let mut gw = world.resource_mut::<GhostWindows>();
        assert!(gw.clear_default_char_pos(0));
        assert!(gw.clear_default_char_pos(1));
    }

    change_scale_and_realign(&mut world, 1, 2);

    assert_eq!(
        left_gap(&world, 0, 1),
        36,
        "詰め直しは defaultx の隙間へ戻す"
    );
}

/// 覚えた右の辺のスコープは、次の（記憶の無い）スコープの基準にならない。
#[test]
fn a_remembered_side_edge_scope_is_not_the_base_of_the_next_scope() {
    let shown = sizes(&SPAWN);
    let mut world = spawn_resolved(&[("kero.seriko.alignmenttodesktop", "right")], &shown, 3);
    assert!(
        world
            .resource_mut::<GhostWindows>()
            .clear_default_char_pos(1)
    );

    finalize_chain_once_with(&shown, &mut world);

    assert_eq!(x_of(&world, 1), WA.right - 278, "辺のスコープは辺のまま");
    assert_eq!(left_gap(&world, 0, 2), 0, "scope 2 の基準は本体");
}

/// 最初の確定より前に台本が辺へ動かしたスコープも、次のスコープの基準にならない。
#[test]
fn a_boot_script_side_edge_scope_is_not_the_base_of_the_next_scope() {
    let shown = sizes(&SPAWN);
    let mut world = spawn_resolved(&[], &shown, 3);
    assert!(apply_script_alignment(
        &mut world,
        1,
        &AlignmentWord::Anchor(Anchor::Left)
    ));
    assert_eq!(x_of(&world, 1), WA.left, "前提: 台本が左の辺へ動かした");

    finalize_chain_once_with(&shown, &mut world);

    assert_eq!(x_of(&world, 1), WA.left, "辺のスコープは辺のまま");
    assert_eq!(left_gap(&world, 0, 2), 0, "scope 2 の基準は本体");
}

/// 詰め直しも左右の辺のスコープを飛ばし、次のスコープを本体との組で詰める。
#[test]
fn realign_skips_a_side_edge_scope_as_the_chain_base() {
    let shown = sizes(&SPAWN);
    let mut world = spawn_resolved(&[("kero.seriko.alignmenttodesktop", "right")], &shown, 3);
    finalize_chain_once_with(&shown, &mut world);
    assert_eq!(left_gap(&world, 0, 2), 0, "前提: scope 2 は本体の左に付く");

    change_scale_and_realign(&mut world, 1, 2);

    let gap = left_gap(&world, 0, 2);
    assert!(
        gap.abs() <= ADJACENT_TOLERANCE_PX,
        "scope 2 が本体の左に詰まっていない（隙間 {gap}）"
    );
}

/// 詰め直しが辺かを決めるのは窓の今の揃え方（`Anchored`）: 最初の確定より前に台本が辺へ動かした
/// スコープも飛ばし、次のスコープを本体との組で詰める（descript は下端のまま）。
#[test]
fn realign_skips_a_boot_script_side_edge_scope_as_the_chain_base() {
    let shown = sizes(&SPAWN);
    let mut world = spawn_resolved(&[], &shown, 3);
    assert!(apply_script_alignment(
        &mut world,
        1,
        &AlignmentWord::Anchor(Anchor::Left)
    ));
    finalize_chain_once_with(&shown, &mut world);
    assert_eq!(left_gap(&world, 0, 2), 0, "前提: scope 2 は本体の左に付く");

    change_scale_and_realign(&mut world, 1, 2);

    let gap = left_gap(&world, 0, 2);
    assert!(
        gap.abs() <= ADJACENT_TOLERANCE_PX,
        "scope 2 が本体の左に詰まっていない（隙間 {gap}）"
    );
}

// ── 鎖の規則の総括（本体の道） ──────────────

/// スコープのキャラ窓の x を書き換える（利用者がドラッグで置いた形）。
fn put_x(world: &mut World, scope: usize, x: i32) {
    let gw = world.resource::<GhostWindows>().clone();
    let mut wp = world
        .get_mut::<WindowPos>(gw.char_window(scope).expect("char 窓がある"))
        .expect("WindowPos がある");
    let wp = wp.bypass_change_detection();
    let pos = wp.position.expect("位置がある");
    wp.position = Some(Point { x, y: pos.y });
}

fn width_of(world: &World, scope: usize) -> i32 {
    let gw = world.resource::<GhostWindows>().clone();
    world
        .get::<WindowPos>(gw.char_window(scope).expect("char 窓がある"))
        .and_then(|wp| wp.size)
        .expect("大きさがある")
        .width
}

/// descript が左右の辺のスコープは、台本が最初の確定より前に揃え方を
/// 変えても起動の鎖の外。辺の窓は動かさず、次の記憶の無いスコープの基準は resolver と同じ本体の
/// まま。行: 下端・free へ揃え直しただけ（x は辺のまま）／free にして台本の移動で辺から
/// 離した／記憶から戻した窓を下端へ揃え直した。
#[test]
fn a_descript_side_edge_scope_realigned_by_the_boot_script_is_not_the_chain_base() {
    let kero = |world: &World| {
        world
            .resource::<GhostWindows>()
            .char_window(1)
            .expect("char 窓がある")
    };
    let moved_by_script = |world: &mut World| {
        put_x(world, 1, 1000);
        let window = kero(world);
        world.entity_mut(window).insert(ScriptPlaced);
    };
    let remembered = |world: &mut World| {
        assert!(
            world
                .resource_mut::<GhostWindows>()
                .clear_default_char_pos(1)
        );
    };
    type Row<'a> = (&'a str, Anchor, Option<&'a dyn Fn(&mut World)>);
    let rows: [Row; 4] = [
        ("T24 下端へ", Anchor::Bottom, None),
        ("T24 free へ", Anchor::Free, None),
        ("T25 台本が動かした", Anchor::Free, Some(&moved_by_script)),
        ("T26 記憶から戻した", Anchor::Bottom, Some(&remembered)),
    ];
    let gaps = rows.map(|(what, word, more)| {
        let shown = sizes(&SPAWN);
        let mut world = spawn_resolved(&[("kero.seriko.alignmenttodesktop", "right")], &shown, 3);
        assert!(apply_script_alignment(
            &mut world,
            1,
            &AlignmentWord::Anchor(word)
        ));
        assert_eq!(
            x_of(&world, 1),
            WA.right - 278,
            "{what} 前提: 揃え直しても x は辺のまま"
        );
        if let Some(more) = more {
            more(&mut world);
        }
        let x1 = x_of(&world, 1);

        finalize_chain_once_with(&shown, &mut world);

        assert_eq!(x_of(&world, 1), x1, "{what}: 鎖の外の窓は動かさない");
        (what, left_gap(&world, 0, 2))
    });
    assert_eq!(
        gaps.map(|(_, gap)| gap),
        [0; 4],
        "scope 2 の基準は本体のまま（鎖の外の窓の隣へ動かさない）: {gaps:?}"
    );
}

/// 辺のスコープの隣へ付ける場合の行: (descript の鍵, 値, スコープの数, 辺のスコープ, 付けた窓, 辺は右か)。
type EdgeRow = (&'static str, &'static str, usize, usize, usize, bool);

/// 3 スコープ: 右の辺の kero の左に scope 2。
const KERO_RIGHT: EdgeRow = ("kero.seriko.alignmenttodesktop", "right", 3, 1, 2, true);

/// 辺のスコープの隣（内側）へ隙間 0 で付けた x。
fn beside_edge(world: &World, (_, _, _, edge, attached, right_edge): EdgeRow) -> i32 {
    if right_edge {
        x_of(world, edge) - width_of(world, attached)
    } else {
        x_of(world, edge) + width_of(world, edge)
    }
}

/// 起動を確定し、付ける窓を辺のスコープの隣へ隙間 0 で置き（`placed` で並びを足せる）、100% → 200% の
/// 詰め直しを流す（`after_arm` は武装の後・解く前）。辺のスコープは辺に付いたまま大きくなる
/// （拡大率の相の再射影の結果の形）。解いた後の World を返す。
fn attach_to_edge_and_rescale(
    row: EdgeRow,
    placed: impl FnOnce(&mut World),
    after_arm: impl FnOnce(&mut World),
) -> World {
    let (key, word, scopes, edge, attached, right_edge) = row;
    let shown = sizes(&SPAWN[..scopes]);
    let mut world = spawn_resolved(&[(key, word)], &shown, scopes);
    finalize_chain_once_with(&shown, &mut world);
    let x = beside_edge(&world, row);
    put_x(&mut world, attached, x);
    placed(&mut world);

    let before = snapshot_char_rects(&world);
    let shown = rescale_keeping_bottom_center(&mut world, 2, 1);
    let edge_x = if right_edge {
        WA.right - width_of(&world, edge)
    } else {
        WA.left
    };
    put_x(&mut world, edge, edge_x);
    arm_chain_realign(&mut world, before);
    after_arm(&mut world);
    realign_chain_once_with_source(&shown, &mut world);

    assert_eq!(
        x_of(&world, edge),
        edge_x,
        "{key},{word}: 辺のスコープは動かない"
    );
    world
}

/// 付けた窓の x と、辺のスコープの隣の x の差（0 なら付いたまま）。
fn off_the_edge(world: &World, row: EdgeRow) -> i32 {
    x_of(world, row.4) - beside_edge(world, row)
}

/// 利用者が左右の辺のスコープの隣へ付けた窓は、拡大率が変わっても辺のスコープの
/// 隣に付いたまま。
#[test]
fn a_window_attached_to_a_side_edge_scope_stays_attached_after_100_to_200() {
    let rows = [
        KERO_RIGHT,
        // 3 スコープ: 左の辺の kero の右に scope 2。
        ("kero.seriko.alignmenttodesktop", "left", 3, 1, 2, false),
        // 2 スコープ: 右の辺の本体の左に kero（鎖の頭が辺のスコープの隣に付く）。
        ("sakura.seriko.alignmenttodesktop", "right", 2, 0, 1, true),
    ];
    let off = rows.map(|row| {
        let world = attach_to_edge_and_rescale(row, |_| {}, |_| {});
        (row.0, row.1, off_the_edge(&world, row))
    });
    assert_eq!(
        off,
        rows.map(|(key, word, ..)| (key, word, 0)),
        "付けた窓は辺のスコープの隣のまま（重なったまま残らない）"
    );
}

/// 辺のスコープの隣に付いていても、鎖の前の窓の隣にも付いていた窓は、鎖の前の窓（本体）へ
/// 付け直す（鎖の組が先）。
#[test]
fn a_window_beside_both_follows_the_previous_chain_window() {
    // 本体の左端を kero の左端に揃える（scope 2 は両方の左に隙間 0 で付く）。
    let world = attach_to_edge_and_rescale(
        KERO_RIGHT,
        |world| {
            let x = x_of(world, 1);
            put_x(world, 0, x);
        },
        |_| {},
    );
    assert_ne!(
        x_of(&world, 0),
        x_of(&world, 1),
        "前提: 200% の後は本体と kero の左端が違う"
    );
    assert_eq!(left_gap(&world, 0, 2), 0, "scope 2 は本体の左に付く");
}

/// 辺のスコープの隣に付いた窓も、詰め直しの「動かさない」の決まり（窓の組・ドラッグの最中・
/// 武装の後の台本）に当たれば動かさない。辺のスコープがドラッグの最中のときも付け直さない。
#[test]
fn a_window_attached_to_a_side_edge_scope_is_kept_by_the_exclusions() {
    let drag = |scope: usize| {
        move |world: &mut World| {
            let window = world
                .resource::<GhostWindows>()
                .char_window(scope)
                .expect("char 窓がある");
            world.entity_mut(window).insert(DraggingState {
                drag_start_pos: Point::new(0, 0),
                initial_window_pos: Some((0.0, 0.0)),
            });
        }
    };
    let script_placed = |world: &mut World| {
        let window = world
            .resource::<GhostWindows>()
            .char_window(2)
            .expect("char 窓がある");
        world.entity_mut(window).insert(ScriptPlaced);
    };
    let sticky = |world: &mut World| seed_sticky_descript(world, Some("0,2"));
    type Row = (&'static str, Box<dyn FnOnce(&mut World)>);
    let rows: [Row; 4] = [
        ("辺のスコープがドラッグの最中", Box::new(drag(1))),
        ("付けた窓がドラッグの最中", Box::new(drag(2))),
        ("付けた窓を武装の後に台本が置いた", Box::new(script_placed)),
        ("付けた窓が窓の組に入っている", Box::new(sticky)),
    ];
    let off = rows.map(|(what, after_arm)| {
        let world = attach_to_edge_and_rescale(KERO_RIGHT, |_| {}, after_arm);
        (what, off_the_edge(&world, KERO_RIGHT))
    });
    assert_eq!(
        off.map(|(_, off)| off),
        [428; 4],
        "置き直しのままで、付け直さない: {off:?}"
    );
}

/// descript が鎖の本体を、台本が最初の確定より前に右の辺へ揃えると、本体は
/// 起動の鎖から外れ、次のスコープが鎖の頭になる（頭は動かさない）。辺の本体と次のスコープは、
/// descript で辺に揃えたときと同じく鎖の組にならない。
#[test]
fn a_main_window_aligned_to_a_side_edge_by_the_boot_script_leaves_the_chain() {
    let shown = sizes(&[(500, 687), SPAWN[1]]);
    let mut world = spawn_resolved(&[], &shown, 2);
    assert!(apply_script_alignment(
        &mut world,
        0,
        &AlignmentWord::Anchor(Anchor::Right)
    ));
    assert_eq!(x_of(&world, 0), WA.right - 500, "前提: 本体は右の辺");
    let x1 = x_of(&world, 1);

    finalize_chain_once_with(&shown, &mut world);

    assert_eq!(x_of(&world, 0), WA.right - 500, "本体は辺のまま");
    assert_eq!(x_of(&world, 1), x1, "次のスコープは鎖の頭で、動かさない");
}

/// 辺のスコープの隣へ、番号の小さい側の窓（本体）を付けたときは、拡大率が
/// 変わってもどちらも動かさない（番号の小さい側は動かさない・辺の窓は辺のまま）。
#[test]
fn a_lower_scope_attached_to_a_side_edge_scope_is_not_moved() {
    let row: EdgeRow = ("kero.seriko.alignmenttodesktop", "right", 2, 1, 0, true);
    let shown = sizes(&SPAWN[..2]);
    let mut world = spawn_resolved(&[(row.0, row.1)], &shown, 2);
    finalize_chain_once_with(&shown, &mut world);
    let x = beside_edge(&world, row);
    put_x(&mut world, 0, x);

    let before = snapshot_char_rects(&world);
    let shown = rescale_keeping_bottom_center(&mut world, 2, 1);
    put_x(&mut world, 1, WA.right - 556);
    let placed = (x_of(&world, 0), x_of(&world, 1));
    arm_chain_realign(&mut world, before);
    realign_chain_once_with_source(&shown, &mut world);

    assert_eq!(
        (x_of(&world, 0), x_of(&world, 1)),
        placed,
        "詰め直しはどちらも動かさない"
    );
}

/// 本番の道で通す: `defaultx` ごとに、起動の確定（出ている幅が準備と違う）→
/// 並べ終えた時点の保存 → 拡大率の後の詰め直し → 覚えた印 `left` での次の起動の確定が、どれも
/// 鎖の前の窓の左に `defaultx` の隙間で置く。本体は右の辺から 1000 空けて置く（負の `defaultx` が
/// 作業領域の端で切られないように）。
#[test]
fn every_world_path_keeps_the_defaultx_gap() {
    let real = [(500, 687), (300, 357)];
    let seen = [0, 2, 36, -36, -500].map(|default_x: i32| {
        let value = default_x.to_string();
        let shell = [
            ("sakura.defaultx", "1000"),
            ("kero.defaultx", value.as_str()),
        ];
        let shown = sizes(&real);

        // 起動の確定（記憶なし）と、並べ終えた時点の保存。
        let mut world = spawn_resolved(&shell, &shown, 2);
        let store = PersistStore::fake();
        store.wire(&mut world);
        finalize_chain_once_with(&shown, &mut world);
        let booted = left_gap(&world, 0, 1);
        let mark = store
            .load()
            .get(&PersistKey::WindowAdjacent { scope: 1 })
            .cloned();
        // 拡大率の後の詰め直し。
        change_scale_and_realign(&mut world, 2, 1);
        let realigned = left_gap(&world, 0, 1);
        store.finish();

        // 次の起動: 記憶から戻した窓を、覚えた印 left で付け直す。
        let mut world = spawn_resolved(&shell, &shown, 2);
        {
            let mut gw = world.resource_mut::<GhostWindows>();
            assert!(gw.clear_default_char_pos(1));
            gw.set_remembered_adjacency(1, Some(AdjacencyMark::Left));
        }
        finalize_chain_once_with(&shown, &mut world);
        let remembered = left_gap(&world, 0, 1);

        (default_x, booted, mark, realigned, remembered)
    });
    let want = seen.clone().map(|(default_x, ..)| {
        (
            default_x,
            default_x,
            Some("left".to_owned()),
            default_x,
            default_x,
        )
    });
    assert_eq!(
        seen, want,
        "(defaultx, 起動の確定, 保存の印, 詰め直し, 覚えた印の起動) の隙間"
    );
}

/// 中のスコープが無い（0 と 2 だけ）とき、鎖の前の窓は下の一番近いスコープ。覚えた印を
/// 「辺のスコープを飛ばした組として読む」記録は出さない（飛ばしたときに出ることは
/// `chain_finalize_align_tests` の同じ組で見る）。
#[test]
fn a_missing_middle_scope_is_not_logged_as_a_skipped_side_edge_scope() {
    let shown = PerTargetSizes::new([(0, Some((500, 687))), (2, Some(SPAWN[2]))]);
    let mut world = spawn_resolved_scopes(&[], &shown, &[0, 2]);
    {
        let mut gw = world.resource_mut::<GhostWindows>();
        assert!(gw.clear_default_char_pos(2));
        gw.set_remembered_adjacency(2, Some(AdjacencyMark::Left));
    }
    let logs = capture_lines(LineFormat::LevelTargetFields, || {
        finalize_chain_once_with(&shown, &mut world)
    })
    .1;
    assert_eq!(left_gap(&world, 0, 2), 0, "scope 2 は本体の左に付く");
    assert!(
        !logs.iter().any(|line| line.contains(READ_ACROSS)),
        "辺のスコープを飛ばしていないのに記録が出た: {logs:?}"
    );
}
