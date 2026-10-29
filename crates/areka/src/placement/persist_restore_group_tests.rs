//! 記憶から戻した配置に組の寄せを当てる檻（window-work-area-clamp・task 2.4・要件 4.1〜4.5・7.7）。
//!
//! 数値は開発者の机の形の合成: 主の画面の作業領域 `0..2880 × 0..1704`。記憶の位置は右に外した
//! モニタ（x=5000 から）で、今のどのモニタとも交わらない。窓は 640×840 の下端吸着
//! （当て直しで y=864）。記憶の x は下端中央の原点（左上 + 320）。

use super::*;
use crate::placement::follow::OffsetBase;

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
    let balloon_offset = PointPx { x: -200, y: 0 };
    ScopePlacement {
        scope,
        char_pos: DEFAULT,
        char_size: CSZ,
        balloon_pos: PointPx {
            x: DEFAULT.x + balloon_offset.x,
            y: DEFAULT.y,
        },
        balloon_size: BSZ,
        balloon_offset,
        balloon_offset_base: OffsetBase::unpinned(balloon_offset),
        balloon_limit: true,
        anchor: Anchor::Bottom,
        balloon_keyword_base: None,
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

/// 寄せた位置を記憶へ書き戻さない: 同じ記憶から何度戻しても同じ結果（純関数・要件 4.5）。
#[test]
fn group_shift_is_pure_and_repeatable() {
    let off = PointPx { x: -200, y: 0 };
    let mut entries = remembered(0, 5000, off, None);
    entries.extend(remembered(1, 5640, off, Some("right")));
    let before = entries.clone();
    let a = restore(&entries);
    assert_eq!(entries, before);
    assert_eq!(a, restore(&entries));
}
