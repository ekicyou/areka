// =============================================================================
// 初期配置の確定（finalize_chain_once）の結線檻（scg 要件 7.1/7.3/7.4・design C6）
//
// 判定そのものは placement::chain_finalize の純関数檻が全網羅する。ここで固定するのは
// **結線の振る舞い**——いつ駆動し、いつ見送り、二度目に駆動しないこと。
//
// 偽装境界: resnap 檻と同じ headless World（`resnap_world`＝実 spawn_ghost_windows で
// 2 スコープの char/balloon 窓＋偽 WindowHandle＋MonitorSnapshot）に、scope ごとに寸を
// 作り分ける `PhysicalSizeSource` の fake を組み合わせる。
// =============================================================================

use std::collections::BTreeMap;

use super::*;
use super::test_support::{pos_of, resnap_world, size_of};

use crate::placement::chain_finalize::ChainFinalized;

use wintf::ecs::{Point, SizeI};

/// scope ごとに物理寸を作り分ける fake（`FakeSizes` は shell/balloon の 2 値しか返せず、
/// 「全スコープの寸が窓と一致しているか」を扱う本檻には粒度が足りない）。
///
/// 値 `None` は「表示未成立（初回 ShowSurface 前）」を表す。
struct PerTargetSizes(BTreeMap<u32, Option<(u32, u32)>>);

impl PerTargetSizes {
    /// scope → shell 物理寸（`None` は未表示）を与える。
    fn new<I: IntoIterator<Item = (usize, Option<(u32, u32)>)>>(entries: I) -> Self {
        Self(
            entries
                .into_iter()
                .map(|(scope, size)| (shell_target(scope as u32).0, size))
                .collect(),
        )
    }
}

impl PhysicalSizeSource for PerTargetSizes {
    fn physical_size(&self, target: TargetId) -> Option<(u32, u32)> {
        self.0.get(&target.0).copied().flatten()
    }
}

/// spawn 時の実寸（`resnap_placements` と一致）。窓の `WindowPos.size` と揃える基準値。
const SPAWN_SIZE_0: (u32, u32) = (434, 687);
const SPAWN_SIZE_1: (u32, u32) = (278, 357);

/// 全スコープが表示成立し、寸が窓と一致している状態の fake。
fn settled_sizes() -> PerTargetSizes {
    PerTargetSizes::new([(0, Some(SPAWN_SIZE_0)), (1, Some(SPAWN_SIZE_1))])
}

// -----------------------------------------------------------------------------
// 駆動して隣接を回復する
// -----------------------------------------------------------------------------

/// 実表示寸が確定したフレームで連鎖を解き直し、後続スコープだけを動かして隙間 0 にする。
///
/// フィクスチャの spawn 位置は scope0 `[1483, 1917]`・scope1 `[1049, 1327]` で 156px 空いて
/// いる。確定後は scope1 の右端が scope0 の左端に一致する。
#[test]
fn finalize_closes_the_gap_by_moving_the_follower_only() {
    let (mut world, gw) = resnap_world();
    let char0 = gw.char_window(0).unwrap();
    let char1 = gw.char_window(1).unwrap();
    assert_eq!(
        pos_of(&world, char0).map(|p| p.x),
        Some(1483),
        "前提: scope0 の spawn 位置"
    );
    assert_eq!(
        pos_of(&world, char1).map(|p| p.x),
        Some(1049),
        "前提: scope1 の spawn 位置（隣接していない）"
    );
    let y1_before = pos_of(&world, char1).map(|p| p.y);

    finalize_chain_once_with(&settled_sizes(), &mut world);

    assert_eq!(
        pos_of(&world, char0).map(|p| p.x),
        Some(1483),
        "起点スコープは動かさない（接地点は不変・7.2）"
    );
    assert_eq!(
        pos_of(&world, char1).map(|p| p.x),
        Some(1205),
        "new_x = 1483 − 278（前スコープ左端 − 自スコープ幅）"
    );
    assert_eq!(
        pos_of(&world, char1).map(|p| p.y),
        y1_before,
        "Y は動かさない（下端吸着は再アンカーが保つ・7.2）"
    );
    // 隣接不変量: scope1 の右端＝scope0 の左端。
    assert_eq!(
        1483 - (1205 + 278),
        0,
        "確定後は隙間 0（scg 7.1）"
    );
    assert!(
        world.get_resource::<ChainFinalized>().is_some(),
        "確定標識が立つ（7.4）"
    );
}

/// 寸は変えない（確定は位置だけを直す）。
#[test]
fn finalize_does_not_touch_window_sizes() {
    let (mut world, gw) = resnap_world();
    let char1 = gw.char_window(1).unwrap();

    finalize_chain_once_with(&settled_sizes(), &mut world);

    assert_eq!(
        size_of(&world, char1),
        Some(SizeI::new(278, 357)),
        "寸は resnap の領分ゆえ確定は触らない"
    );
}

// -----------------------------------------------------------------------------
// 一度きり（7.4）
// -----------------------------------------------------------------------------

/// 二度目以降は駆動しない。確定後にサーフェス寸が変わっても位置は動かない
/// （会話中の表情差替でキャラが横滑りしない）。
#[test]
fn finalize_runs_only_once_even_if_sizes_change_again() {
    let (mut world, gw) = resnap_world();
    let char1 = gw.char_window(1).unwrap();

    finalize_chain_once_with(&settled_sizes(), &mut world);
    let after_first = pos_of(&world, char1);
    assert_eq!(after_first.map(|p| p.x), Some(1205), "一度目は動く");

    // 二度目: 同じ入力でも動かない（べき等以前に駆動自体がない）。
    finalize_chain_once_with(&settled_sizes(), &mut world);
    assert_eq!(pos_of(&world, char1), after_first, "二度目は駆動しない");

    // 確定後に scope0 の窓を別寸へ変えても連鎖は解き直されない（7.4）。
    resnap_from_sizes(
        &mut world,
        [(0usize, crate::placement::resolver::SizePx { w: 500, h: 687 })].into_iter(),
    );
    let moved_origin_x = pos_of(&world, gw.char_window(0).unwrap()).map(|p| p.x);
    finalize_chain_once_with(
        &PerTargetSizes::new([(0, Some((500, 687))), (1, Some(SPAWN_SIZE_1))]),
        &mut world,
    );
    assert_eq!(
        pos_of(&world, char1),
        after_first,
        "確定後のサーフェス切替では後続スコープを動かさない（7.4）"
    );
    assert!(
        moved_origin_x.is_some(),
        "前提: 起点スコープ側は resnap で再アンカーされている（観測の退化防止）"
    );
}

// -----------------------------------------------------------------------------
// 見送り条件（部分適用しない）
// -----------------------------------------------------------------------------

/// 表示未成立のスコープが 1 つでもあれば確定しない（次フレームへ送る）。
#[test]
fn finalize_defers_while_any_scope_has_not_shown_yet() {
    let (mut world, gw) = resnap_world();
    let char1 = gw.char_window(1).unwrap();
    let before = pos_of(&world, char1);

    finalize_chain_once_with(
        &PerTargetSizes::new([(0, Some(SPAWN_SIZE_0)), (1, None)]),
        &mut world,
    );

    assert_eq!(pos_of(&world, char1), before, "未表示があれば動かさない");
    assert!(
        world.get_resource::<ChainFinalized>().is_none(),
        "確定させない（次フレームで再挑戦する）"
    );

    // 表示が成立したら確定する。
    finalize_chain_once_with(&settled_sizes(), &mut world);
    assert_eq!(
        pos_of(&world, char1).map(|p| p.x),
        Some(1205),
        "全スコープが揃った時点で確定する"
    );
}

/// 実表示寸と窓の寸が食い違う間は確定しない（再アンカーが未 landing＝位置が未確定）。
#[test]
fn finalize_defers_until_resnap_has_landed() {
    let (mut world, gw) = resnap_world();
    let char1 = gw.char_window(1).unwrap();
    let before = pos_of(&world, char1);

    // scope0 の実表示寸だけが窓の寸（434x687）と食い違う状態。
    finalize_chain_once_with(
        &PerTargetSizes::new([(0, Some((500, 687))), (1, Some(SPAWN_SIZE_1))]),
        &mut world,
    );

    assert_eq!(pos_of(&world, char1), before, "寸が揃うまで動かさない");
    assert!(
        world.get_resource::<ChainFinalized>().is_none(),
        "確定させない（古い位置で連鎖を解く取り違えを構造的に避ける）"
    );
}

// -----------------------------------------------------------------------------
// 明示的な再配置の尊重（7.3）
// -----------------------------------------------------------------------------

/// 台本の移動指令などで既定位置から動かされたスコープは引き戻さない。
#[test]
fn finalize_does_not_pull_back_an_explicitly_moved_scope() {
    let (mut world, gw) = resnap_world();
    let char1 = gw.char_window(1).unwrap();

    // 明示的な再配置（唯一の位置ライター経由）。以後 current_x != default_x になる。
    crate::placement::follow::move_window_to(&mut world, char1, 800, 1087);
    assert_eq!(pos_of(&world, char1), Some(Point { x: 800, y: 1087 }));

    finalize_chain_once_with(&settled_sizes(), &mut world);

    assert_eq!(
        pos_of(&world, char1).map(|p| p.x),
        Some(800),
        "明示的に動かされたスコープは引き戻さない（7.3）"
    );
}
