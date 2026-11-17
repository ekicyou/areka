//! 記憶から戻した配置に組の寄せを当てる檻（window-work-area-clamp・task 2.4・要件 4.1〜4.5・7.7）。
//!
//! 数値は開発者の机の形の合成: 主の画面の作業領域 `0..2880 × 0..1704`。記憶の位置は右に外した
//! モニタ（x=5000 から）で、今のどのモニタとも交わらない。窓は 640×840 の下端吸着
//! （当て直しで y=864）。記憶の x は下端中央の原点（左上 + 320）。

use super::*;
use crate::shared_test_support::placement_sized;

const CSZ: SizePx = SizePx { w: 640, h: 840 };
const BSZ: SizePx = SizePx { w: 200, h: 300 };
/// 既定の位置（記憶があれば使われない）。
const DEFAULT: PointPx = PointPx { x: 100, y: 864 };

fn desk() -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![RectPx {
            left: 0,
            top: 0,
            right: 2880,
            bottom: 1704,
        }],
    }
}

fn placement(scope: usize) -> ScopePlacement {
    ScopePlacement {
        anchor: Anchor::Bottom,
        ..placement_sized(scope, DEFAULT, PointPx { x: -200, y: 0 }, true, CSZ, BSZ)
    }
}

/// 左上 x が `left` の記憶（下端中央の原点で書く）・バルーンのずらし・印。
fn remembered(
    scope: u32,
    left: i32,
    offset: PointPx,
    mark: Option<&str>,
) -> Vec<(PersistKey, String)> {
    let mut e = char_pos_entries(
        scope,
        PointPx {
            x: left + 320,
            y: 864,
        },
    );
    e.extend(balloon_offset_entries(scope, offset));
    if let Some(m) = mark {
        e.push((PersistKey::WindowAdjacent { scope }, m.to_string()));
    }
    e
}

fn restore(entries: &[(PersistKey, String)]) -> Vec<ScopePlacement> {
    apply_restored_placements(vec![placement(0), placement(1)], entries, &desk())
}

fn xs(out: &[ScopePlacement]) -> Vec<i32> {
    out.iter().map(|p| p.char_pos.x).collect()
}

/// ⑷ 全員が外の隣り合っていた組は、互いの差（640）を保って作業領域へ戻る。バルーンの位置は
/// 窓と同じ量だけ動き、ずらし（`balloon_offset`）は記憶の値のまま（要件 4.1・4.2・7.7）。
#[test]
fn all_outside_group_keeps_spacing_and_balloon_follows() {
    let off0 = PointPx { x: -300, y: 50 };
    let off1 = PointPx { x: 640, y: 50 };
    let mut entries = remembered(0, 5000, off0, None);
    entries.extend(remembered(1, 5640, off1, Some("right")));
    let out = restore(&entries);
    assert_eq!(xs(&out), vec![1600, 2240], "組の差 640 が保たれていない");
    for (p, off) in out.iter().zip([off0, off1]) {
        assert_eq!(p.char_pos.y, 864);
        assert_eq!(p.balloon_offset, off, "ずらしが変わった");
        assert_eq!(
            p.balloon_pos,
            PointPx {
                x: p.char_pos.x + off.x,
                y: p.char_pos.y + off.y
            },
            "バルーンが窓と同じ量だけ動いていない"
        );
    }
}

/// 印が `none` でも記憶の矩形が重なっていた組は一緒に動く（要件 4.1）。
#[test]
fn overlapped_group_without_mark_moves_together() {
    let off = PointPx { x: -200, y: 0 };
    let mut entries = remembered(0, 5000, off, None);
    entries.extend(remembered(1, 5300, off, Some("none")));
    assert_eq!(xs(&restore(&entries)), vec![1940, 2240]);
}

/// ⑸ どれも見えている組は動かない＝記憶の位置のまま（要件 4.3）。
#[test]
fn all_visible_group_stays_at_remembered_positions() {
    let off = PointPx { x: -200, y: 0 };
    let mut entries = remembered(0, 1000, off, None);
    entries.extend(remembered(1, 1640, off, Some("right")));
    assert_eq!(xs(&restore(&entries)), vec![1000, 1640]);
}

/// ⑹ 離れていた組は窓ごと: どちらも窓ごとの判定のまま右端へ寄る（要件 4.4）。
#[test]
fn apart_windows_are_clamped_one_by_one() {
    let off = PointPx { x: -200, y: 0 };
    let mut entries = remembered(0, 5000, off, None);
    entries.extend(remembered(1, 7000, off, Some("none")));
    assert_eq!(xs(&restore(&entries)), vec![2240, 2240]);
}

/// 見えている窓のある組では、見えている窓は記憶の位置のまま・外の窓は窓ごとの結果（要件 4.1 の改め）。
#[test]
fn partner_visible_group_keeps_per_window_results() {
    let off = PointPx { x: -200, y: 0 };
    let mut entries = remembered(0, 2000, off, None);
    entries.extend(remembered(1, 5000, off, Some("right")));
    assert_eq!(xs(&restore(&entries)), vec![2000, 2240]);
}

/// 揃え方つきの配置（スコープ番号が飛んでもよい）。
fn placement_at(scope: usize, anchor: Anchor) -> ScopePlacement {
    ScopePlacement {
        anchor,
        ..placement(scope)
    }
}

/// T8（audit-placement-follow-residue 要件 1.3）: 中のスコープが無い（0 と 2 だけ）起動でも、
/// 2 の印 `right` は鎖の前の窓 0 との組の印で、組の差 640 が保たれる（(n−1, n) の線では組にならず落ちる）。
#[test]
fn missing_middle_scope_keeps_the_pair_spacing() {
    let off = PointPx { x: -200, y: 0 };
    let mut entries = remembered(0, 5000, off, None);
    entries.extend(remembered(2, 5640, off, Some("right")));
    let out = apply_restored_placements(
        vec![
            placement_at(0, Anchor::Bottom),
            placement_at(2, Anchor::Bottom),
        ],
        &entries,
        &desk(),
    );
    assert_eq!(xs(&out), vec![1600, 2240], "組の差 640 が保たれていない");
}

/// T6 ⑴（要件 1.2）: 0 下端・1 右の辺・2 下端で 2 の印が `right` なら、辺を飛ばして 0 と 2 が組になり
/// 差 640 が保たれる（(n−1, n) の線では辺の 1 と組んで落ちる）。
#[test]
fn edge_scope_between_keeps_the_pair_spacing() {
    let off = PointPx { x: -200, y: 0 };
    let mut entries = remembered(0, 5000, off, None);
    entries.extend(remembered(1, 9000, off, Some("separate")));
    entries.extend(remembered(2, 5640, off, Some("right")));
    let out = apply_restored_placements(
        vec![
            placement_at(0, Anchor::Bottom),
            placement_at(1, Anchor::Right),
            placement_at(2, Anchor::Bottom),
        ],
        &entries,
        &desk(),
    );
    let (a, c) = (out[0].char_pos.x, out[2].char_pos.x);
    assert_eq!((a, c), (1600, 2240), "0 と 2 の差 640 が保たれていない");
}

/// 判定は組の寄せの後の配置で読む: 全員が外の組は寄せたので真・どれも見えている組は偽（要件 4.6）。
#[test]
fn judgement_reads_the_layout_after_the_group_shift() {
    let off = PointPx { x: -200, y: 0 };
    let judge = |entries: &[(PersistKey, String)]| {
        restore_and_judge(vec![placement(0), placement(1)], entries, &desk())
    };
    let mut outside = remembered(0, 5000, off, None);
    outside.extend(remembered(1, 5640, off, Some("right")));
    let (out, clamped) = judge(&outside);
    assert_eq!(xs(&out), vec![1600, 2240], "組の寄せの後の配置を返す");
    assert!(clamped, "寄せて表示したのに偽");
    let mut visible = remembered(0, 1000, off, None);
    visible.extend(remembered(1, 1640, off, Some("right")));
    assert!(!judge(&visible).1, "記憶の位置のまま表示したのに真");
    // 記憶の無い起動は寄せていない。
    assert!(!judge(&[]).1);
}

mod clamped_flag_at_finalize {
    use super::super::finalize_save_tests::{find, old_memory_rig};
    use super::super::{RestoredPositionsClamped, persist_unremembered_char_positions};
    use areka_sylphya::PersistKey;
    use log_capture_kit::capture;

    const SKIP: &str = "記憶の位置を寄せて表示した起動なので、古い記憶へ隣り合いの印を書き足さない";
    const ADDED: &str = "古い記憶に隣り合いの印だけを書き足した";

    /// 印の無い古い記憶のスコープ 1（スコープ 0 の左に隣り合う）を持つ rig（[`old_memory_rig`]）に
    /// `flag` を据えて保存を呼ぶ。
    fn run(flag: Option<bool>) -> (bool, Vec<log_capture_kit::CapturedEvent>) {
        let (mut rig, before) = old_memory_rig();
        if let Some(f) = flag {
            rig.world.insert_resource(RestoredPositionsClamped(f));
        }
        let ((), events) = capture(|| persist_unremembered_char_positions(&rig.world));
        let mut after = rig.load();
        let added = after
            .remove(&PersistKey::WindowAdjacent { scope: 1 })
            .is_some();
        assert_eq!(after, before, "印のほかは前後で同じ");
        rig.finish();
        (added, events)
    }

    /// 寄せた起動（資源が真）は古い記憶へ印を書き足さず、そのことを info で 1 行残す（要件 4.6）。
    #[test]
    fn true_skips_the_mark_and_logs_info() {
        let (added, events) = run(Some(true));
        assert!(!added, "寄せた起動で印が書き足された");
        let skips: Vec<_> = events
            .iter()
            .filter(|e| e.message().contains(SKIP))
            .collect();
        assert_eq!(skips.len(), 1, "見送りの info が 1 行: {events:?}");
        assert_eq!(skips[0].level, tracing::Level::INFO);
        assert_eq!(skips[0].target, "areka::persist::save");
        assert_eq!(skips[0].field("scopes"), Some("{1}"));
        assert!(find(&events, ADDED, 1).is_empty());
    }

    /// 資源が偽の起動は今どおり書き足す（資源が無い起動は同じ rig の
    /// `finalize_save_tests::old_memory_gets_only_the_adjacency_mark_added` が持つ）。
    #[test]
    fn false_adds_the_mark() {
        let (added, events) = run(Some(false));
        assert!(added, "偽で印が書き足されない");
        assert!(!events.iter().any(|e| e.message().contains(SKIP)));
    }
}
