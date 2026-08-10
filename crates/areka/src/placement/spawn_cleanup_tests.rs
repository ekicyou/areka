use bevy_ecs::prelude::*;
use wintf::ecs::{Point, SizeI, WindowPos};

use super::test_support::{titles, two_scope_placements};
use super::{GhostWindowMarker, GhostWindows, ScopeWindows, spawn_ghost_windows};
use crate::placement::follow::{Anchored, BalloonFollow};
use crate::placement::resolver::{Anchor, PointPx};

// -------------------------------------------------------------------------
// T-V1: despawn 掃除（areka-P0-dpi-window-vanish 要件 6.1/6.4・D8）
//
// 檻は「判断が下される場所」＝本ファイル（`remove_entry_of` と on_remove hook）
// に置く（記憶 test-only-decision-branches-not-proven-wiring・タスク 1.4 の教訓）。
// -------------------------------------------------------------------------

/// 生存 scope の「動いていないこと」を判定するための可観測状態の束。
///
/// 位置・寸法（`WindowPos`）と追従関係（`BalloonFollow` の相手 entity と offset）＋
/// アンカーを 1 個の比較可能な値へまとめる（要件 6.4 の「位置・寸法・追従関係」）。
#[derive(Debug, Clone, PartialEq, Eq)]
struct ScopeState {
    char_pos: Option<Point>,
    char_size: Option<SizeI>,
    balloon_pos: Option<Point>,
    balloon_size: Option<SizeI>,
    follow_target: Option<Entity>,
    follow_offset: Option<PointPx>,
    anchor: Option<Anchor>,
}

/// `GhostWindows` の引き当てから scope の可観測状態を採取する。
fn scope_state(world: &World, gw: &GhostWindows, scope: usize) -> ScopeState {
    let char_e = gw.char_window(scope).expect("char window entity");
    let balloon_e = gw.balloon_window(scope).expect("balloon window entity");
    let char_wp = world.get::<WindowPos>(char_e);
    let balloon_wp = world.get::<WindowPos>(balloon_e);
    let follow = world.get::<BalloonFollow>(char_e);
    ScopeState {
        char_pos: char_wp.and_then(|w| w.position),
        char_size: char_wp.and_then(|w| w.size),
        balloon_pos: balloon_wp.and_then(|w| w.position),
        balloon_size: balloon_wp.and_then(|w| w.size),
        follow_target: follow.map(|f| f.balloon),
        follow_offset: follow.map(|f| f.offset),
        anchor: world.get::<Anchored>(char_e).map(|a| a.0),
    }
}

/// T-V1: キャラ窓を despawn すると当該 scope が `GhostWindows` から消える（6.1）。
///
/// 呼出点結合なしで拾えること（＝`world.despawn` を素で叩くだけで発火すること）が
/// 要件の核心ゆえ、掃除関数を明示的に呼ぶ経路は檻に持ち込まない。
#[test]
fn t_v1_despawn_char_window_removes_scope_entry() {
    let mut world = World::new();
    let placements = two_scope_placements();
    let gw = spawn_ghost_windows(&mut world, &placements, &titles());
    let char0 = gw.char_window(0).unwrap();

    world.despawn(char0);

    let reg = world.resource::<GhostWindows>();
    assert_eq!(reg.scopes().collect::<Vec<_>>(), vec![1]);
    assert_eq!(reg.char_window(0), None);
    // scope 粒度＝対の相方も同時に引き当て不能になる（D8）
    assert_eq!(reg.balloon_window(0), None);
}

/// T-V1: バルーン窓側の despawn でも同じ scope エントリが消える（片割れ対称性・D8）。
#[test]
fn t_v1_despawn_balloon_window_removes_same_scope_entry() {
    let mut world = World::new();
    let placements = two_scope_placements();
    let gw = spawn_ghost_windows(&mut world, &placements, &titles());
    let balloon1 = gw.balloon_window(1).unwrap();

    world.despawn(balloon1);

    let reg = world.resource::<GhostWindows>();
    assert_eq!(reg.scopes().collect::<Vec<_>>(), vec![0]);
    assert_eq!(reg.char_window(1), None);
    assert_eq!(reg.balloon_window(1), None);
}

/// T-V1: 対の**後追い** despawn は no-op（panic せず・他 scope を巻き込まない・6.1）。
///
/// 終了処理（`despawn_smoke_targets`）は同一 World 変異内で対を一括 despawn する＝
/// 2 個目の hook は必ず「既に除去済みの scope」を引く。ここが panic すると終了経路が
/// 落ちるため、良性であることを構造で固定する。
#[test]
fn t_v1_paired_followup_despawn_is_noop() {
    let mut world = World::new();
    let placements = two_scope_placements();
    let gw = spawn_ghost_windows(&mut world, &placements, &titles());
    let char0 = gw.char_window(0).unwrap();
    let balloon0 = gw.balloon_window(0).unwrap();

    world.despawn(char0);
    let after_first: Vec<usize> = world.resource::<GhostWindows>().scopes().collect();

    // 後追いの片割れ——panic しない・レジストリは 1 ミリも動かない
    world.despawn(balloon0);

    let reg = world.resource::<GhostWindows>();
    assert_eq!(after_first, vec![1]);
    assert_eq!(reg.scopes().collect::<Vec<_>>(), after_first);
    assert_eq!(reg.char_window(1), gw.char_window(1));
    assert_eq!(reg.balloon_window(1), gw.balloon_window(1));
}

/// T-V1: 未登録 entity の除去は no-op（不一致・二重除去・空レジストリ・6.1）。
///
/// `remove_entry_of` の全域性を直接叩いて固定する（hook はこの関数しか呼ばない）。
#[test]
fn t_v1_remove_entry_of_unregistered_entity_is_noop() {
    let mut world = World::new();
    let placements = two_scope_placements();
    let gw = spawn_ghost_windows(&mut world, &placements, &titles());
    let stranger = world.spawn_empty().id();

    let mut reg = world.resource_mut::<GhostWindows>();
    // 非ゴースト entity
    assert_eq!(reg.remove_entry_of(stranger), None);
    assert_eq!(reg.scopes().collect::<Vec<_>>(), vec![0, 1]);

    // 一致 → 二重除去は None
    let char1 = gw.char_window(1).unwrap();
    let (scope, removed) = reg.remove_entry_of(char1).expect("初回は除去成立");
    assert_eq!(scope, 1);
    assert_eq!(removed.char_window, char1);
    assert_eq!(removed.balloon_window, gw.balloon_window(1).unwrap());
    assert_eq!(reg.remove_entry_of(char1), None);
    assert_eq!(reg.remove_entry_of(removed.balloon_window), None);

    // 空になっても no-op（panic しない）
    let expected0 = ScopeWindows {
        char_window: gw.char_window(0).unwrap(),
        balloon_window: gw.balloon_window(0).unwrap(),
    };
    assert_eq!(
        reg.remove_entry_of(expected0.balloon_window),
        Some((0, expected0))
    );
    assert_eq!(reg.scopes().collect::<Vec<_>>(), Vec::<usize>::new());
    assert_eq!(reg.remove_entry_of(char1), None);
}

/// T-V1: 掃除の前後で**生存 scope** の位置・寸法・追従関係が変化しない（6.4）。
///
/// 空虚性回避（記憶 2.2 の教訓・タスク 2.2 Implementation Notes）:
/// ①探針は 2 scope で、位置・寸法・offset・追従先がすべて**互いに異なる**ことを
/// 事前に `assert_ne!` で自己検査する（＝「全部同じ値」なら不変主張が不動点に落ちて
/// 何も検出できない）。②「レジストリに scope 1 が残っている」も同時に主張する
/// （＝除去が広すぎる変異——例えば scope 粒度でなく全消し——を捕まえる）。
#[test]
fn t_v1_surviving_scope_state_unchanged_by_cleanup() {
    let mut world = World::new();
    let placements = two_scope_placements();
    let gw = spawn_ghost_windows(&mut world, &placements, &titles());

    let before0 = scope_state(&world, &gw, 0);
    let before1 = scope_state(&world, &gw, 1);

    // 探針の自己検査: 2 scope の可観測状態は互いに異なる（不動点でない）。
    assert_ne!(before0.char_pos, before1.char_pos);
    assert_ne!(before0.char_size, before1.char_size);
    assert_ne!(before0.balloon_pos, before1.balloon_pos);
    assert_ne!(before0.follow_offset, before1.follow_offset);
    assert_ne!(before0.follow_target, before1.follow_target);
    assert!(before1.char_pos.is_some() && before1.follow_offset.is_some());

    // scope 0 の対を一括 despawn（終了処理と同じ形）。
    world.despawn(gw.char_window(0).unwrap());
    world.despawn(gw.balloon_window(0).unwrap());

    // 生存 scope 1: 位置・寸法・追従関係・アンカーが 1 ビットも動かない（6.4）。
    assert_eq!(scope_state(&world, &gw, 1), before1);

    // 除去は scope 粒度で止まる（生存 scope の引き当ては健在）。
    let reg = world.resource::<GhostWindows>();
    assert_eq!(reg.scopes().collect::<Vec<_>>(), vec![1]);
    assert_eq!(reg.char_window(1), gw.char_window(1));
    assert_eq!(reg.balloon_window(1), gw.balloon_window(1));
}

/// T-V1: `GhostWindows` Resource 未挿入の World で despawn しても no-op（hook が
/// Resource 不在で panic しない・design「Resource 未挿入は no-op」）。
#[test]
fn t_v1_despawn_without_registry_resource_is_noop() {
    let mut world = World::new();
    let e = world.spawn(GhostWindowMarker).id();

    world.despawn(e); // panic しなければ合格

    assert!(world.get_resource::<GhostWindows>().is_none());
}

/// T-V1: 掃除は Resource のみを触る＝**生存 entity の component へ一切書かない**
/// ことをログ側からも固定する（6.4 の構造的保証）。除去成立・no-op のいずれも
/// `debug!` 止まりで、warn 以上を出さない（要件 6.2 の前提となる静穏性）。
#[test]
fn t_v1_cleanup_logs_are_debug_only_and_name_the_scope() {
    let (_, events) = crate::placement::test_support::capture_logs(|| {
        let mut world = World::new();
        let placements = two_scope_placements();
        let gw = spawn_ghost_windows(&mut world, &placements, &titles());
        let char0 = gw.char_window(0).unwrap();
        let balloon0 = gw.balloon_window(0).unwrap();
        world.despawn(char0);
        world.despawn(balloon0);
    });

    // 除去成立 1 件（最初の片割れ）＋ no-op 1 件（後追いの片割れ）
    let removed = crate::placement::test_support::expect_one(
        &events,
        "placement: ゴースト窓レジストリから scope エントリを除去",
    );
    assert_eq!(removed.level, tracing::Level::DEBUG);
    assert_eq!(removed.field("scope"), "0");

    let noop = crate::placement::test_support::expect_one(
        &events,
        "placement: ゴースト窓 despawn だがレジストリに該当 scope なし",
    );
    assert_eq!(noop.level, tracing::Level::DEBUG);

    // info 以上（＝warn/error を含む）は 1 行も出さない（良性ノイズを作らない）。
    // `tracing::Level` の Ord は ERROR < WARN < INFO < DEBUG < TRACE ゆえ
    // 「INFO より verbose」＝ debug/trace のみ、が静穏性の表現になる。
    assert!(
        events.iter().all(|e| e.level > tracing::Level::INFO),
        "掃除経路が info 以上のログを出している: {events:?}"
    );
}
