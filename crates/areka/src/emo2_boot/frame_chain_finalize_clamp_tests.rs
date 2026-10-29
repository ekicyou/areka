//! 起動の並べ直しが寄せを通してから x と y を書く檻（window-work-area-clamp タスク 2.1・
//! 要件 1.1〜1.5・5.1・5.2・7.5・design Testing Strategy ⑴⑻⑼）。
//!
//! 本体が主の画面の左端に立ち、相方を左へ並べると左隣の画面へ渡る並び（x=−1108 の形）。
//! 並べ直しは相方を本体の右の隣へ並べ替え、本体は動かさない。並べ終えた時点の保存には
//! 寄せた後の位置（下端中央の x）が書かれ、相方のバルーンはずらしどおりに立つ。

use super::chain_finalize_persist_test_support::{PersistStore, spawn_shown};
use super::test_support::{PerTargetSizes, capture_logs, pos_of};
use super::*;

use crate::placement::chain_finalize::ChainFinalized;
use crate::placement::follow::{MonitorSnapshot, OffsetBase};
use crate::placement::resolver::{Anchor, PointPx, RectPx, ScopePlacement, SizePx};
use areka_sylphya::{Axis, PersistKey};
use wintf::ecs::window::drain_window_pos_commands;

/// 主の画面（200%）の作業領域。
const PRIMARY: RectPx = RectPx {
    left: 0,
    top: 0,
    right: 2880,
    bottom: 1704,
};
/// 左隣の画面（150%）の作業領域。
const LEFT: RectPx = RectPx {
    left: -2560,
    top: 0,
    right: 0,
    bottom: 1400,
};
/// 本体・相方の絵の大きさ（準備と同じ幅で出す）。
const SIZES: [(u32, u32); 2] = [(764, 1094), (1108, 1368)];
/// 相方のバルーンのずらし。
const OFFSET: PointPx = PointPx { x: -230, y: 0 };

fn shown() -> PerTargetSizes {
    PerTargetSizes::new([(0, Some(SIZES[0])), (1, Some(SIZES[1]))])
}

/// 本体は主の画面の左端・相方は本体から離して置く（記憶なし＝既定の位置のまま・並べ直しで動く）。
fn placements() -> Vec<ScopePlacement> {
    [(0usize, 0), (1, 1500)]
        .into_iter()
        .map(|(scope, x)| {
            let char_size = SizePx {
                w: SIZES[scope].0 as i32,
                h: SIZES[scope].1 as i32,
            };
            let char_pos = PointPx {
                x,
                y: PRIMARY.bottom - char_size.h,
            };
            ScopePlacement {
                scope,
                char_pos,
                char_size,
                balloon_pos: PointPx {
                    x: char_pos.x + OFFSET.x,
                    y: char_pos.y + OFFSET.y,
                },
                balloon_size: SizePx { w: 223, h: 158 },
                balloon_offset: OFFSET,
                balloon_offset_base: OffsetBase::unpinned(OFFSET),
                // バルーンの引き戻しを効かせ、キャラ窓を収めれば起きないことを見る（要件 5.2）。
                balloon_limit: true,
                anchor: Anchor::Bottom,
                balloon_keyword_base: None,
            }
        })
        .collect()
}

fn world() -> World {
    let mut world = World::new();
    world.insert_resource(MonitorSnapshot {
        work_areas: vec![PRIMARY, LEFT],
    });
    world
}

/// ⑴⑼ 相方は本体の右の隣へ並べ替えられ、本体は動かない。相方のバルーンはずらしどおりに立ち、
/// `[balloon-limit] Clamp` は出ない。寄せの行（場面・並べ替え）が 1 つ出る。
#[test]
fn partner_flips_to_right_of_body_on_left_edge() {
    let mut world = world();
    let gw = spawn_shown(&mut world, &placements(), &shown());
    let _resnap = drain_window_pos_commands();
    let body_before = pos_of(&world, gw.char_window(0).unwrap()).unwrap();
    assert_eq!(body_before.x, PRIMARY.left, "前提: 本体は主の画面の左端");

    let logs = capture_logs(|| finalize_chain_once_with(&shown(), &mut world));
    assert!(
        world.contains_resource::<ChainFinalized>(),
        "前提: 並べ終えた"
    );

    let body = pos_of(&world, gw.char_window(0).unwrap()).unwrap();
    let partner = pos_of(&world, gw.char_window(1).unwrap()).unwrap();
    assert_eq!(body, body_before, "本体は動かない");
    assert_eq!(
        (partner.x, partner.y),
        (
            body.x + SIZES[0].0 as i32,
            PRIMARY.bottom - SIZES[1].1 as i32
        ),
        "相方は本体の右の隣（左へ並べると x=−1108 で主の画面の外）"
    );
    let balloon = pos_of(&world, gw.balloon_window(1).unwrap()).unwrap();
    assert_eq!(
        (balloon.x, balloon.y),
        (partner.x + OFFSET.x, partner.y + OFFSET.y),
        "相方のバルーンはずらしどおり"
    );
    let clamp: Vec<&String> = logs
        .iter()
        .filter(|l| l.contains("[work-area-clamp] Clamp"))
        .collect();
    assert_eq!(clamp.len(), 1, "寄せの行は 1 つ: {logs:?}");
    for field in ["scene=\"boot-repack\"", "how=\"flipped\"", "scope=1"] {
        assert!(clamp[0].contains(field), "{field} が無い: {}", clamp[0]);
    }
    assert!(
        !logs.iter().any(|l| l.contains("[balloon-limit] Clamp")),
        "バルーンだけの寄せは起きない: {logs:?}"
    );
    let _residue = drain_window_pos_commands();
}

/// ⑻ 並べ終えた時点の保存に、相方の寄せた後の位置（下端中央の x・上端の y）が書かれる（要件 7.5）。
#[test]
fn save_at_finalize_writes_clamped_partner_position() {
    let mut world = world();
    let store = PersistStore::fake();
    store.wire(&mut world);
    let gw = spawn_shown(&mut world, &placements(), &shown());

    finalize_chain_once_with(&shown(), &mut world);

    let partner = pos_of(&world, gw.char_window(1).unwrap()).unwrap();
    assert_eq!(
        partner.x, SIZES[0].0 as i32,
        "前提: 相方は右の隣へ寄せられた"
    );
    let saved = store.load();
    let get = |axis| {
        saved
            .get(&PersistKey::WindowPos { scope: 1, axis })
            .cloned()
    };
    assert_eq!(
        (get(Axis::X), get(Axis::Y)),
        (
            Some((partner.x + SIZES[1].0 as i32 / 2).to_string()),
            Some(partner.y.to_string())
        ),
        "寄せた後の位置が書かれる: {saved:?}"
    );
    let _residue = drain_window_pos_commands();
}
