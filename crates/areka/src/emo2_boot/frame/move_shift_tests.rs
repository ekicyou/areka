//! ずれの相（[`run_move_shift_with`]）の檻（seriko-pattern-calls task 6.3・要件 3.8・7.1・7.3・7.6）。
//!
//! 窓の一式は再スナップの檻と同じ 2 スコープ（下揃え・scope0 のキャラ窓は (1483, 757)・
//! バルーンはその (−412, −25)）。下揃えなので x のずれだけを見る。

use std::collections::BTreeMap;

use areka_emo_compose::ScaleRatio;
use areka_emo_present::TargetId;
use bevy_ecs::prelude::*;
use wintf::ecs::Point;

use super::super::super::target_map::{balloon_target, shell_target};
use super::super::test_support::{
    WRITER_WITNESS, arrangement_offset_of, assert_no_write, pos_of, reset_write_witness,
    resnap_world,
};
use super::{MoveShiftSource, run_move_shift_with};
use crate::placement::dpi_sync::DpiSyncHold;
use crate::placement::spawn::GhostWindows;

/// 対象ごとのずれと拡大率を持つ偽の提示。
#[derive(Default)]
struct FakeShifts {
    shifts: BTreeMap<u32, (i64, i64)>,
    ratios: BTreeMap<u32, ScaleRatio>,
}

impl FakeShifts {
    /// 全対象（2 スコープのシェルとバルーン）が拡大率 `k` で表示済み。
    fn shown(k: ScaleRatio) -> Self {
        FakeShifts {
            shifts: BTreeMap::new(),
            ratios: (0..4).map(|t| (t, k)).collect(),
        }
    }
}

impl MoveShiftSource for FakeShifts {
    fn move_shift(&self, target: TargetId) -> (i64, i64) {
        self.shifts.get(&target.0).copied().unwrap_or((0, 0))
    }
    fn applied_ratio(&self, target: TargetId) -> Option<ScaleRatio> {
        self.ratios.get(&target.0).copied()
    }
}

fn x_of(world: &World, e: Entity) -> i32 {
    pos_of(world, e).expect("位置がある").x
}

fn windows(gw: &GhostWindows) -> (Entity, Entity, Entity) {
    (
        gw.char_window(0).unwrap(),
        gw.balloon_window(0).unwrap(),
        gw.char_window(1).unwrap(),
    )
}

/// 提示のずれがキャラ窓とバルーン窓へ当たる。ずれの無いスコープへは書かない。
#[test]
fn presenter_shift_reaches_the_windows() {
    let (mut world, gw) = resnap_world();
    let (char0, balloon0, char1) = windows(&gw);
    reset_write_witness(&mut world, &gw);
    let mut src = FakeShifts::shown(ScaleRatio::ONE);
    src.shifts.insert(shell_target(0).0, (10, 0));
    src.shifts.insert(balloon_target(0).0, (5, 0));

    run_move_shift_with(&src, &mut world);

    assert_eq!(pos_of(&world, char0), Some(Point { x: 1493, y: 757 }));
    assert_eq!(
        pos_of(&world, balloon0),
        Some(Point {
            x: 1493 - 412 + 5,
            y: 732
        }),
        "バルーンは動いたキャラ窓に付き、自分のずれを足す"
    );
    assert_eq!(x_of(&world, char1), 1049);
    assert_no_write(&world, char1, "ずれの無いスコープ");
}

/// 値か拡大率が変わったフレームだけ書く。
#[test]
fn writes_only_when_the_shift_or_the_scale_changes() {
    let (mut world, gw) = resnap_world();
    let (char0, _, _) = windows(&gw);
    let mut src = FakeShifts::shown(ScaleRatio::ONE);
    src.shifts.insert(shell_target(0).0, (10, 0));
    run_move_shift_with(&src, &mut world);
    assert_eq!(x_of(&world, char0), 1493);

    reset_write_witness(&mut world, &gw);
    run_move_shift_with(&src, &mut world);
    assert_no_write(&world, char0, "同じずれ・同じ拡大率");

    src.ratios
        .insert(shell_target(0).0, ScaleRatio::new(3, 2).unwrap());
    run_move_shift_with(&src, &mut world);
    assert_eq!(x_of(&world, char0), 1483 + 15, "拡大率が変われば掛け直す");
    assert_ne!(arrangement_offset_of(&world, char0), WRITER_WITNESS);

    reset_write_witness(&mut world, &gw);
    src.shifts.insert(shell_target(0).0, (0, 0));
    run_move_shift_with(&src, &mut world);
    assert_eq!(x_of(&world, char0), 1483, "0 で元へ戻る");
    assert_ne!(arrangement_offset_of(&world, char0), WRITER_WITNESS);
}

/// 拡大率がまだ無い（最初の面を出していない）窓へは何もしない。
#[test]
fn nothing_is_written_before_the_scale_is_known() {
    let (mut world, gw) = resnap_world();
    let (char0, _, _) = windows(&gw);
    reset_write_witness(&mut world, &gw);
    let mut src = FakeShifts::default();
    src.shifts.insert(shell_target(0).0, (10, 0));

    run_move_shift_with(&src, &mut world);

    assert_eq!(x_of(&world, char0), 1483);
    assert_no_write(&world, char0, "拡大率が無い");
}

/// 窓の書き込みを見送る間は書かず、門が開いたフレームで当てる。
#[test]
fn deferred_windows_are_shifted_after_the_hold() {
    let (mut world, gw) = resnap_world();
    let (char0, _, _) = windows(&gw);
    reset_write_witness(&mut world, &gw);
    world
        .entity_mut(char0)
        .insert(DpiSyncHold { since_frame: 0 });
    let mut src = FakeShifts::shown(ScaleRatio::ONE);
    src.shifts.insert(shell_target(0).0, (10, 0));

    run_move_shift_with(&src, &mut world);
    assert_eq!(x_of(&world, char0), 1483);
    assert_no_write(&world, char0, "見送りの間");

    world.entity_mut(char0).remove::<DpiSyncHold>();
    run_move_shift_with(&src, &mut world);
    assert_eq!(x_of(&world, char0), 1493, "門が開いたフレームで当たる");
}
