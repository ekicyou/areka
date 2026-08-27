//! **拡大率の往復**——遷移が元の水準へ戻ったとき、追従オフセットが基準から引き直されて
//! bit 同一で戻ることの檻（areka-P0-balloon-offset-dpi・task 6.1 の是正・design D4／D16・
//! 要件 3.3／7.8／9.4）。
//!
//! # ここが押さえる欠陥（2026-08-28 の実装時是正）
//!
//! 追随の適用相は恒等比の腕（`OffsetRescale::Unchanged`）で**現在値を据え置いて**いた。
//! 純関数は「比が恒等」としか言っておらず、「Component の現在値が既に正しい」とは
//! 言っていない——両者が一致するのは `offset == base.offset` のあいだだけである。
//! `apply_rescaled` は現在値だけを動かし基準対を意図的に残すので、一度追随した後に
//! 元の表示 DPI へ戻ると
//!
//! ```text
//! 144 で係留 → Anchored  offset=(-512,-48)  base=(-512,-48)@144
//! 192 へ     → Rescaled  offset=(-683,-64)  base=(-512,-48)@144
//! 144 へ戻る → Unchanged offset=(-683,-64)  ← 基準へ戻らない（要件 3.3 違反）
//! ```
//!
//! となり、往復した利用者のバルーンが高 DPI 時の相対位置に取り残される。
//! 是正は「恒等比でも**基準から引き直す**（`base.offset` を `apply_rescaled` で書く）」で
//! あり、判定語は**値が動いたかどうか**で選ぶ（動けば `rescaled`・動かなければ `unchanged`）。
//!
//! # なぜ既存の檻が見逃したか（本ファイルが埋める 2 つの穴）
//!
//! 1. **純関数側の往復の檻は判定語しか比べていない**
//!    （`follow_offset_space_tests.rs` の `roundtrip_*_is_bit_identical`）。純関数は
//!    Component の現在値を**一度も見ない**ので、「腕をどの書込口へ流すか」の取り違えは
//!    純関数の層では原理的に表現できない。ゆえに本ファイルは**適用相そのもの**を
//!    直接呼ぶ単体の檻（[`the_identity_ratio_rederives_from_the_base_after_a_return_transition`]）
//!    で、判定語ではなく **Component へ着地した値**を読む。
//! 2. **相の結合の行列に戻りの遷移が 1 本も無い**
//!    （`frame_balloon_offset_follow_tests.rs` の `TRANSITIONS` は 96→120・96→192・120→192＝
//!    すべて単調増加で、しかも各組が `FrameHarness::new()` を組み直して 1 遷移だけ走らせる）。
//!    ゆえに `Rescaled` の**後に** `Unchanged` が来る列が World の上で一度も起きない。
//!    本ファイルの [`the_display_dpi_round_trip_returns_the_offset_bit_identically`] と
//!    [`the_offset_is_bit_identical_at_every_revisit_of_a_dpi_chain`] がその列を作る。
//!
//! 置き場所を兄弟ファイルにしたのは、行列側が 948 行で 1,000 行制限（要件 9.6）に
//! 52 行しか残していないためである。

use wintf::ecs::window::SetWindowPosCommand;
use wintf::ecs::{DPI, Point, WindowPos};

use crate::placement::follow::{BalloonFollow, OffsetBase};
use crate::placement::resolver::PointPx;
use crate::placement::transition_diag::{OFFSET_VERDICT_RESCALED, OFFSET_VERDICT_UNCHANGED};

use super::balloon_offset_follow::{OffsetFollowOutcome, rescale_balloon_follow_offset};
use super::test_support::{FakeReports, FrameHarness, capture_logs};
use super::*;

// ---------------------------------------------------------------------------
// 1. 適用相を直接呼ぶ単体の檻（腕 → 書込口の対応を**着地値**で読む）
// ---------------------------------------------------------------------------

/// 単体の檻で使う基準オフセット（両軸とも非ゼロ＝往復の有無を区別できる）。
const UNIT_BASE_OFFSET: PointPx = PointPx { x: -512, y: -48 };

/// 単体の檻の低い水準（基準 DPI）。
const UNIT_LOW_DPI: u16 = 144;

/// 単体の檻の高い水準（比 4/3）。
const UNIT_HIGH_DPI: u16 = 192;

/// `144 → 192` の追随後の手計算値。
///
/// 丸めの単一権威 `ScaleRatio::scale_len`（`(2·len·num + den) ÷ (2·den)`）で
/// x: `(2·512·192 + 144) ÷ 288 = 196752 ÷ 288 = 683`、
/// y: `(2·48·192 + 144) ÷ 288 = 18576 ÷ 288 = 64`。符号は基準から引き継ぐ。
const UNIT_HIGH_OFFSET: PointPx = PointPx { x: -683, y: -64 };

fn dpi(v: u16) -> DPI {
    DPI::from_dpi(v, v)
}

/// 追従 Component と表示 DPI だけを持つ最小の World を組む（相の結合を通さない）。
fn unit_world(base: OffsetBase, current: u16) -> (World, Entity) {
    let mut world = World::new();
    let balloon = world.spawn_empty().id();
    let char_window = world
        .spawn((BalloonFollow::new(balloon, base), dpi(current)))
        .id();
    (world, char_window)
}

/// 当該 entity の追従 Component。
fn follow_at(world: &World, entity: Entity) -> BalloonFollow {
    world
        .get::<BalloonFollow>(entity)
        .copied()
        .expect("キャラ窓に BalloonFollow がある")
}

/// `kind=offset` の観測行だけを取り出す。
fn offset_lines(logs: &[String]) -> Vec<&String> {
    logs.iter().filter(|l| l.contains("kind=offset")).collect()
}

/// **是正の本体**: 一度追随した後に元の表示 DPI へ戻ると、恒等比の腕は
/// **基準から引き直して** bit 同一の値を Component へ書く（要件 3.3）。
///
/// 判定語も同時に固定する——値が動いた戻りの遷移で `unchanged` を記録すると、語が嘘になる。
#[test]
fn the_identity_ratio_rederives_from_the_base_after_a_return_transition() {
    let base = OffsetBase {
        offset: UNIT_BASE_OFFSET,
        dpi: Some(dpi(UNIT_LOW_DPI)),
    };
    let (mut world, char_window) = unit_world(base, UNIT_HIGH_DPI);

    // ⑴ 上りの遷移（144 → 192）——現在値だけが動き、基準対は残る。
    let up = rescale_balloon_follow_offset(&mut world, char_window, 0);
    assert_eq!(up, OffsetFollowOutcome::Changed);
    assert_eq!(
        follow_at(&world, char_window).offset(),
        UNIT_HIGH_OFFSET,
        "上りの遷移で追随していない（以降の主張が空虚になる）"
    );
    assert_eq!(
        follow_at(&world, char_window).base(),
        base,
        "追随が基準対を書き換えた（往復無誤差の前提が崩れる）"
    );

    // ⑵ 戻りの遷移（192 → 144）——比は恒等だが、現在値は基準から**離れている**。
    world.entity_mut(char_window).insert(dpi(UNIT_LOW_DPI));
    let logs = capture_logs(|| {
        let back = rescale_balloon_follow_offset(&mut world, char_window, 0);
        assert_eq!(
            back,
            OffsetFollowOutcome::Changed,
            "戻りの遷移で値が動いたのに Unchanged を返した（D16 の収束が発火せずバルーンが戻らない）"
        );
    });

    assert_eq!(
        follow_at(&world, char_window).offset(),
        UNIT_BASE_OFFSET,
        "戻りの遷移で基準値へ bit 同一で戻っていない（恒等比の腕が現在値を据え置いている）"
    );
    assert_eq!(
        follow_at(&world, char_window).base(),
        base,
        "戻りの遷移が基準対を書き換えた"
    );
    let lines = offset_lines(&logs);
    assert_eq!(lines.len(), 1, "戻りの遷移の観測行が 1 行でない: {logs:?}");
    assert!(
        lines[0].contains(&format!("verdict={OFFSET_VERDICT_RESCALED}")),
        "値が動いた戻りの遷移の判定語が rescaled でない（語が嘘になる）: {}",
        lines[0]
    );
}

/// 本当に何も動かない恒等比では `unchanged` の語が残る（語を嘘にしない対の側）。
#[test]
fn a_genuine_identity_transition_keeps_the_unchanged_word() {
    let base = OffsetBase {
        offset: UNIT_BASE_OFFSET,
        dpi: Some(dpi(UNIT_LOW_DPI)),
    };
    let (mut world, char_window) = unit_world(base, UNIT_LOW_DPI);

    let logs = capture_logs(|| {
        let outcome = rescale_balloon_follow_offset(&mut world, char_window, 0);
        assert_eq!(outcome, OffsetFollowOutcome::Unchanged);
    });

    assert_eq!(follow_at(&world, char_window).offset(), UNIT_BASE_OFFSET);
    assert_eq!(follow_at(&world, char_window).base(), base);
    let lines = offset_lines(&logs);
    assert_eq!(lines.len(), 1, "観測行が 1 行でない: {logs:?}");
    assert!(
        lines[0].contains(&format!("verdict={OFFSET_VERDICT_UNCHANGED}")),
        "値が 1 bit も動いていないのに unchanged 以外の語が出ている: {}",
        lines[0]
    );
}

/// `DPI{0,0}` の腕は是正後も**無遷移**のまま（既存裁定
/// `zero_on_both_sides_is_unchanged_not_unresolved` を動かさない）。
#[test]
fn the_zero_dpi_arm_stays_an_unchanged_no_op() {
    let base = OffsetBase {
        offset: UNIT_BASE_OFFSET,
        dpi: Some(DPI::from_dpi(0, 0)),
    };
    let (mut world, char_window) = unit_world(base, 0);

    let logs = capture_logs(|| {
        let outcome = rescale_balloon_follow_offset(&mut world, char_window, 0);
        assert_eq!(outcome, OffsetFollowOutcome::Unchanged);
    });

    assert_eq!(follow_at(&world, char_window).offset(), UNIT_BASE_OFFSET);
    let lines = offset_lines(&logs);
    assert_eq!(lines.len(), 1, "観測行が 1 行でない: {logs:?}");
    assert!(
        lines[0].contains(&format!("verdict={OFFSET_VERDICT_UNCHANGED}")),
        "DPI0 の腕の判定語が変わった: {}",
        lines[0]
    );
}

// ---------------------------------------------------------------------------
// 2. 相の結合で往復させる（World の上で `Rescaled` の後に `Unchanged` を起こす）
// ---------------------------------------------------------------------------

/// 相の結合で使う低い水準（作者基準と等倍）。
const LOW_DPI: u16 = 96;

/// 相の結合で使う高い水準（比 2）。
const HIGH_DPI: u16 = 192;

/// 基準オフセット（`frame_test_support::resnap_placements` の fixture 値・scope 昇順）。
const BASE_OFFSETS: [PointPx; 2] = [PointPx { x: -412, y: -25 }, PointPx { x: 285, y: -19 }];

/// 当該スコープの追従オフセット（現在値）。
fn offset_of(harness: &FrameHarness, scope: usize) -> PointPx {
    harness
        .world
        .get::<BalloonFollow>(harness.char_window(scope))
        .expect("キャラ窓に BalloonFollow がある")
        .offset()
}

/// 窓の位置（`WindowPos.position`）。
fn pos_of(harness: &FrameHarness, entity: Entity) -> Point {
    harness
        .world
        .get::<WindowPos>(entity)
        .and_then(|wp| wp.position)
        .expect("窓に位置がある")
}

/// 指定スコープ・指定種別の窓書込だけを取り出す。
fn writes_for(writes: &[SetWindowPosCommand], scope: u32, kind: &str) -> Vec<SetWindowPosCommand> {
    writes
        .iter()
        .filter(|cmd| cmd.tag.scope == Some(scope) && cmd.tag.kind == kind)
        .cloned()
        .collect()
}

/// 起動直後の整地——3 つの源と窓の拡大率を `level` へ揃え、初回全窓マッチを空回しで消費する。
///
/// この 1 巡で未係留の基準が `level` へ**係留**される（要件 5.2）。係留を自己検査するのは、
/// 済んでいないと以降の遷移が係留の腕へ落ち、追随が起きないまま緑になるからである。
fn settle_at(harness: &mut FrameHarness, source: &mut FakeReports, level: u16) {
    harness.set_monitor_sources_for_dpi(level);
    harness.set_monitor_table_for_dpi(level);
    harness.set_window_dpi(level);
    harness.advance_frame();
    harness.run_placement_phases(source);
    let _priming = harness.drain_writes();
    harness.reset_write_witness();
    for scope in harness.scopes().to_vec() {
        assert_eq!(
            harness
                .world
                .get::<BalloonFollow>(harness.char_window(scope))
                .expect("キャラ窓に BalloonFollow がある")
                .base(),
            OffsetBase {
                offset: BASE_OFFSETS[scope],
                dpi: Some(dpi(level)),
            },
            "scope={scope}: 整地で基準対が「fixture 値 × 係留済み {level}」になっていない"
        );
    }
}

/// 表示 DPI を `level` へ動かして 1 フレーム回す（待ち札の付かない通常の遷移）。
fn transition_to(harness: &mut FrameHarness, source: &mut FakeReports, level: u16) {
    harness.set_monitor_table_for_dpi(level);
    harness.set_window_dpi(level);
    harness.advance_frame();
    harness.run_placement_phases(source);
}

/// **戻りの遷移**（96 → 192 → 96）でオフセットが基準へ bit 同一で戻り、
/// バルーンが実際に元の位置へ帰る（要件 3.3・design D4／D16）。
#[test]
fn the_display_dpi_round_trip_returns_the_offset_bit_identically() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at(&mut harness, &mut source, LOW_DPI);

    let before: Vec<(usize, PointPx, Point)> = harness
        .scopes()
        .to_vec()
        .into_iter()
        .map(|scope| {
            (
                scope,
                offset_of(&harness, scope),
                pos_of(&harness, harness.balloon_window(scope)),
            )
        })
        .collect();

    // 上り（96 → 192）——比 2 ゆえ丸めなしで倍になる。
    transition_to(&mut harness, &mut source, HIGH_DPI);
    let _up_writes = harness.drain_writes();
    for (scope, old_offset, _) in &before {
        assert_eq!(*old_offset, BASE_OFFSETS[*scope]);
        assert_eq!(
            offset_of(&harness, *scope),
            PointPx {
                x: old_offset.x * 2,
                y: old_offset.y * 2
            },
            "scope={scope}: 上りの遷移で追随していない（往復の探針が退化している）"
        );
    }
    let high_balloons: Vec<Point> = before
        .iter()
        .map(|(scope, _, _)| pos_of(&harness, harness.balloon_window(*scope)))
        .collect();

    // 戻り（192 → 96）——恒等比の腕。基準から引き直せば bit 同一で戻る。
    let logs = capture_logs(|| {
        transition_to(&mut harness, &mut source, LOW_DPI);
    });
    let back_writes = harness.drain_writes();

    for (scope, old_offset, old_balloon) in &before {
        let scope32 = u32::try_from(*scope).expect("scope は u32 域");
        assert_eq!(
            offset_of(&harness, *scope),
            *old_offset,
            "scope={scope}: 往復したのにオフセットが往復前の値へ戻っていない"
        );
        // 空振り防止: 戻る前後でバルーンの位置が実際に違っていた。
        assert_ne!(
            high_balloons[*scope], *old_balloon,
            "scope={scope}: 高 DPI 側でバルーンが動いていない（往復を観測できない）"
        );
        // バルーンが**実際に**元の位置へ帰る（値だけ直って窓が古いまま、を塞ぐ）。
        assert!(
            !writes_for(&back_writes, scope32, "balloon").is_empty(),
            "scope={scope}: 戻りの遷移でバルーンへの窓書込が 1 件も無い: {back_writes:?}"
        );
        assert_eq!(
            pos_of(&harness, harness.balloon_window(*scope)),
            *old_balloon,
            "scope={scope}: 往復したのにバルーン窓が元の位置へ帰っていない"
        );
    }

    // 判定語は値が動いた事実に一致する（`unchanged` は嘘になる）。
    let lines = offset_lines(&logs);
    assert_eq!(
        lines.len(),
        harness.scopes().len(),
        "戻りの遷移の観測行がスコープ数と一致しない: {logs:?}"
    );
    for line in &lines {
        assert!(
            line.contains(&format!("verdict={OFFSET_VERDICT_RESCALED}")),
            "戻りの遷移で値が動いたのに判定語が rescaled でない: {line}"
        );
    }
}

/// 表示 DPI の列 `96 → 120 → 192 → 120 → 96` を巡り、**同じ水準へ戻るたび**に
/// 初回訪問と bit 同一（要件 3.3／7.8・誤差が往復で累積しない）。
#[test]
fn the_offset_is_bit_identical_at_every_revisit_of_a_dpi_chain() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at(&mut harness, &mut source, LOW_DPI);

    let mut first_seen: Vec<(u16, Vec<PointPx>)> = Vec::new();
    let mut visits = 0usize;
    for level in [96u16, 120, 192, 120, 96] {
        transition_to(&mut harness, &mut source, level);
        let _writes = harness.drain_writes();
        let offsets: Vec<PointPx> = harness
            .scopes()
            .to_vec()
            .into_iter()
            .map(|scope| offset_of(&harness, scope))
            .collect();
        match first_seen.iter().find(|(d, _)| *d == level) {
            Some((_, first)) => assert_eq!(
                &offsets, first,
                "表示 DPI {level} へ戻ったのに初回訪問と値が違う（往復で誤差が累積している）"
            ),
            None => first_seen.push((level, offsets)),
        }
        visits += 1;
    }
    assert_eq!(visits, 5, "列を最後まで巡っていない");

    // 逐語（比 2・比 5/4 の手計算）——列が緑でも値そのものが動いていないことを固定する。
    assert_eq!(
        first_seen,
        vec![
            (96u16, vec![BASE_OFFSETS[0], BASE_OFFSETS[1]]),
            (
                120u16,
                vec![PointPx { x: -515, y: -31 }, PointPx { x: 356, y: -24 }]
            ),
            (
                192u16,
                vec![PointPx { x: -824, y: -50 }, PointPx { x: 570, y: -38 }]
            ),
        ],
        "列の各水準の値が手計算と一致しない"
    );
}
