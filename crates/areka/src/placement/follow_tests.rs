use bevy_ecs::prelude::*;
use wintf::ecs::SizeI;
use wintf::ecs::pointer::Phase;
use wintf::ecs::{Point, WindowHandle, WindowPos};

use super::test_support::{
    drag_end_event_at, drag_event_at, dragging_state, fake_handle, odd_edge_snapshot, position_of,
    rect, single_monitor_snapshot, window_pos_at, window_pos_sized,
};
use super::{
    Anchored, BalloonFollow, move_window_to, on_char_drag, on_char_drag_end, project_anchor,
};
use crate::placement::resolver::Anchor;
use crate::placement::resolver::PointPx;
use crate::placement::resolver::SizePx;

// -------------------------------------------------------------------------
// move_window_to（R7 公開 API・7.1/7.2/7.3・U4）
// -------------------------------------------------------------------------

/// 観測可能な完了状態: headless World 上で move_window_to を呼ぶと
/// 対象窓の WindowPos が期待座標へ更新される（物理 px 素通し・U4）。
/// 座標は 96 の倍数を避けた値を使い、隠れた dpi/96 再スケールがあれば
/// 完全一致が崩れる檻とする（07-05 欠陥の再発防止・3.2/3.3）。
#[test]
fn move_window_to_updates_window_pos_physical_px() {
    let mut world = World::new();
    let window = world
        .spawn((fake_handle(0x1234), window_pos_at(10, 20)))
        .id();

    assert!(move_window_to(&mut world, window, 1531, 883));
    assert_eq!(position_of(&world, window), Point { x: 1531, y: 883 });
}

/// WindowHandle 未付与（窓生成前）は false を返し、位置も変更しない。
#[test]
fn move_window_to_without_handle_returns_false() {
    let mut world = World::new();
    let window = world.spawn(window_pos_at(10, 20)).id();

    assert!(!move_window_to(&mut world, window, 500, 600));
    assert_eq!(position_of(&world, window), Point { x: 10, y: 20 });
}

/// despawn 済み（対象不在）の entity も false（silent no-op にしない・panic しない）。
#[test]
fn move_window_to_on_despawned_entity_returns_false() {
    let mut world = World::new();
    let window = world
        .spawn((fake_handle(0x1234), window_pos_at(0, 0)))
        .id();
    world.despawn(window);

    assert!(!move_window_to(&mut world, window, 100, 200));
}

/// BalloonFollow を持つ対象の移動はバルーンも offset 維持で随伴移動する
/// （T-I4: 移動後も balloon_pos − char_pos ≡ offset が保存される）。
#[test]
fn move_window_to_moves_balloon_with_offset_preserved() {
    let mut world = World::new();
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
        .id();
    let offset = PointPx { x: -412, y: -25 };
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_at(50, 60),
            BalloonFollow { balloon, offset },
        ))
        .id();

    assert!(move_window_to(&mut world, window, 907, 1201));

    let char_pos = position_of(&world, window);
    let balloon_pos = position_of(&world, balloon);
    assert_eq!(char_pos, Point { x: 907, y: 1201 });
    assert_eq!(
        balloon_pos,
        Point {
            x: 907 + offset.x,
            y: 1201 + offset.y
        }
    );
    // offset 保存則（balloon_pos − char_pos ≡ offset）
    assert_eq!(balloon_pos.x - char_pos.x, offset.x);
    assert_eq!(balloon_pos.y - char_pos.y, offset.y);
}

/// 対象自身に WindowHandle が無ければ false で、バルーンも動かさない。
#[test]
fn move_window_to_target_without_handle_does_not_move_balloon() {
    let mut world = World::new();
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(70, 80)))
        .id();
    let window = world
        .spawn((
            window_pos_at(50, 60),
            BalloonFollow {
                balloon,
                offset: PointPx { x: 11, y: 22 },
            },
        ))
        .id();

    assert!(!move_window_to(&mut world, window, 907, 1201));
    assert_eq!(position_of(&world, window), Point { x: 50, y: 60 });
    assert_eq!(position_of(&world, balloon), Point { x: 70, y: 80 });
}

/// バルーン側に WindowHandle が無い場合: 対象の移動自体は成功（true）し、
/// バルーンは動かない（warn ログ・silent failure ではない）。
#[test]
fn move_window_to_balloon_without_handle_still_moves_target() {
    let mut world = World::new();
    let balloon = world.spawn(window_pos_at(70, 80)).id();
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_at(50, 60),
            BalloonFollow {
                balloon,
                offset: PointPx { x: 11, y: 22 },
            },
        ))
        .id();

    assert!(move_window_to(&mut world, window, 907, 1201));
    assert_eq!(position_of(&world, window), Point { x: 907, y: 1201 });
    assert_eq!(position_of(&world, balloon), Point { x: 70, y: 80 });
}

// -------------------------------------------------------------------------
// MonitorSnapshot / work_area_for_window（task 8.1・DD15 基盤・4.7）
// -------------------------------------------------------------------------

use super::{MonitorSnapshot, work_area_for_window};
use crate::placement::resolver::RectPx;

/// 複数モニタ: 窓中心が属するモニタの work area が返る（縦位置・寸法の異なる
/// 2 面で中心帰属を固定する）。
#[test]
fn work_area_for_window_picks_monitor_containing_center() {
    let snapshot = MonitorSnapshot {
        work_areas: vec![
            rect(0, 0, 1920, 1040),       // primary
            rect(1920, -213, 4480, 1227), // 右の高解像度モニタ（負 top）
        ],
    };
    // 中心 (2500, 500) → 右モニタ
    let window = rect(2100, 100, 2900, 900);
    assert_eq!(
        work_area_for_window(&snapshot, window),
        Some(rect(1920, -213, 4480, 1227))
    );
    // 中心 (960, 520) → primary
    let window = rect(660, 220, 1260, 820);
    assert_eq!(
        work_area_for_window(&snapshot, window),
        Some(rect(0, 0, 1920, 1040))
    );
}

/// 負座標モニタ（プライマリの左）でも中心帰属が成立する。
#[test]
fn work_area_for_window_handles_negative_coords() {
    let snapshot = MonitorSnapshot {
        work_areas: vec![rect(0, 0, 1920, 1040), rect(-1920, -40, 0, 1000)],
    };
    let window = rect(-1500, 100, -700, 700); // 中心 (-1100, 400)
    assert_eq!(
        work_area_for_window(&snapshot, window),
        Some(rect(-1920, -40, 0, 1000))
    );
}

/// 境界中心の決定論: 帰属判定は half-open（right/bottom 排他）＝共有辺上の中心は
/// 右隣モニタへ属する。複数矩形が同一中心を含む（重複）場合は昇順 index 先勝ち。
#[test]
fn work_area_for_window_boundary_center_is_half_open_and_first_match_wins() {
    let a = rect(0, 0, 1920, 1040);
    let b = rect(1920, 0, 3840, 1040);
    let snapshot = MonitorSnapshot {
        work_areas: vec![a, b],
    };
    // 中心 x=1920 ちょうど（共有辺）→ a の right は排他ゆえ b
    let window = rect(1520, 220, 2320, 820); // 中心 (1920, 520)
    assert_eq!(work_area_for_window(&snapshot, window), Some(b));

    // 重複 2 面が同一中心を含む → 先勝ち（昇順 index）
    let overlap = MonitorSnapshot {
        work_areas: vec![a, rect(-10, -10, 2000, 1100)],
    };
    let window = rect(700, 300, 1300, 700); // 中心 (1000, 500) は両方に属す
    assert_eq!(work_area_for_window(&overlap, window), Some(a));
}

/// どのモニタにも属さない中心 → 最近傍（中心→矩形 clamp 点の自乗距離最小・
/// 等距離は昇順 index 先勝ち）。
#[test]
fn work_area_for_window_off_all_monitors_returns_nearest() {
    let a = rect(0, 0, 1920, 1040);
    let b = rect(1920, 0, 3840, 1040);
    let snapshot = MonitorSnapshot {
        work_areas: vec![a, b],
    };
    // 中心 (4340, 500): b の右外 500px・a の右外 2420px → b
    let window = rect(4040, 200, 4640, 800);
    assert_eq!(work_area_for_window(&snapshot, window), Some(b));
    // 中心 (-1000, 2000): a の clamp 点 (0,1040) が b の (1920,1040) より近い → a
    let window = rect(-1300, 1700, -700, 2300);
    assert_eq!(work_area_for_window(&snapshot, window), Some(a));
    // 等距離: 中心 (1920, 2000) は a clamp (1920,1040)・b clamp (1920,1040) と
    // 同距離 → 先勝ちで a
    let window = rect(1620, 1700, 2220, 2300);
    assert_eq!(work_area_for_window(&snapshot, window), Some(a));
}

/// 空 snapshot → `None`（架空の既定矩形を発明しない）。
#[test]
fn work_area_for_window_empty_snapshot_is_none() {
    let snapshot = MonitorSnapshot { work_areas: vec![] };
    assert_eq!(work_area_for_window(&snapshot, rect(0, 0, 100, 100)), None);
}

// -------------------------------------------------------------------------
// work_area_for_window_with_origin ／ guard_visibility
// （task 2.2・D6/S3・S3′・Req 3.1/3.2/5.1/5.3/5.6）
//
// 共通規約: 判定は絶対 px の固定値ではなく**交差・不変条件**で書く（Req 5.6）。
// 座標は 96/120/192 の各水準へスケールした合成レイアウト上で構築し、96 の
// 自己整合（k=1 で恒等写像に退化して欠陥を隠す性質・Req 5.1）に依存しない。
// -------------------------------------------------------------------------

use super::{
    VisibilityVerdict, WorkAreaResolution, guard_visibility, work_area_for_window_with_origin,
};

/// DPI 水準（Req 5.1: 96 のほかに 120・192 を必ず含む）。
const DPIS: [i32; 3] = [96, 120, 192];

/// 論理基準値 → 各 DPI の物理 px（整数演算のみ・厳密整除を強制。
/// `resolver.rs` の `px()` が donor・Req 5.6）。
fn px(logical: i32, dpi: i32) -> i32 {
    assert_eq!(
        (logical * dpi) % 96,
        0,
        "テスト入力は厳密整除になる論理値（4 の倍数）で構築する"
    );
    logical * dpi / 96
}

/// 混在 DPI マルチモニタの合成レイアウト（Req 5.1/5.3）。
///
/// - index 0: 96 水準の左モニタ。**負座標**（`-1920..0`）・上端 40px の
///   非対称 work area（`top = -40`）
/// - index 1: `dpi` 水準の右モニタ。左端に 64 論理 px のタスクバー＝
///   **非対称 work area**（`left = px(64)`）。192 では右端 3840＝**3200 超座標**
///
/// 2 面のあいだ（`0 ..= px(64)`）はどの work area にも属さない帯であり、
/// 最近傍フォールバックの発火面として使う。
fn mixed_layout(dpi: i32) -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![left_wa(), right_wa(dpi)],
    }
}

/// 左モニタ（96 水準・負座標）の work area。
fn left_wa() -> RectPx {
    rect(-1920, -40, 0, 1000)
}

/// 右モニタ（`dpi` 水準・非対称）の work area。192 で right=3840（>3200）。
fn right_wa(dpi: i32) -> RectPx {
    rect(px(64, dpi), 0, px(1920, dpi), px(1040, dpi))
}

/// キャラ窓の寸（論理 300x400）。
fn char_size(dpi: i32) -> SizePx {
    SizePx {
        w: px(300, dpi),
        h: px(400, dpi),
    }
}

/// バルーン窓の寸（論理 500x300）。
fn balloon_size(dpi: i32) -> SizePx {
    SizePx {
        w: px(500, dpi),
        h: px(300, dpi),
    }
}

fn point(x: i32, y: i32) -> PointPx {
    PointPx { x, y }
}

/// 位置＋寸 → 窓矩形（テスト側の独立実装＝実装の `rect_at` を再利用しない）。
fn win(pos: PointPx, size: SizePx) -> RectPx {
    rect(pos.x, pos.y, pos.x + size.w, pos.y + size.h)
}

/// 面積を持つ重なりの独立実装（実装の `rects_intersect` とは別式で書く）。
fn overlaps(a: RectPx, b: RectPx) -> bool {
    a.left.max(b.left) < a.right.min(b.right) && a.top.max(b.top) < a.bottom.min(b.bottom)
}

/// キャラ窓の Bottom 接地位置（射影 T が出す Y＝`wa.bottom − h`）。
fn grounded_y(wa: RectPx, size: SizePx) -> i32 {
    wa.bottom - size.h
}

// --- work_area_for_window_with_origin -------------------------------------

/// 中心が帰属するときは `Contains` を返す（左右どちらのモニタでも・全水準）。
#[test]
fn with_origin_reports_contains_when_center_belongs() {
    for dpi in DPIS {
        let snapshot = mixed_layout(dpi);
        let size = char_size(dpi);

        // 右モニタの中央付近
        let pos = point(px(800, dpi), grounded_y(right_wa(dpi), size));
        assert_eq!(
            work_area_for_window_with_origin(&snapshot, win(pos, size)),
            Some((right_wa(dpi), WorkAreaResolution::Contains)),
            "dpi={dpi}: 右モニタ内の窓は Contains"
        );

        // 左モニタ（負座標）の中央付近
        let pos = point(-1200, grounded_y(left_wa(), size));
        assert_eq!(
            work_area_for_window_with_origin(&snapshot, win(pos, size)),
            Some((left_wa(), WorkAreaResolution::Contains)),
            "dpi={dpi}: 左モニタ（負座標）内の窓は Contains"
        );
    }
}

/// どのモニタにも属さない中心は `NearestFallback` として判別される
/// （S3 後半＝最近傍フォールバックが異常を無観測で吸収する性質の是正・Req 3.2）。
#[test]
fn with_origin_reports_nearest_fallback_when_center_belongs_nowhere() {
    for dpi in DPIS {
        let snapshot = mixed_layout(dpi);
        let size = char_size(dpi);

        // ① 右モニタの右外（192 では 3200 超座標）
        let far_right = point(
            px(1920, dpi) + px(400, dpi),
            grounded_y(right_wa(dpi), size),
        );
        let (wa, origin) = work_area_for_window_with_origin(&snapshot, win(far_right, size))
            .expect("非空 snapshot ゆえ Some");
        assert_eq!(
            origin,
            WorkAreaResolution::NearestFallback,
            "dpi={dpi}: 右外の窓は最近傍フォールバック"
        );
        assert_eq!(wa, right_wa(dpi), "dpi={dpi}: 最近傍は右モニタ");

        // ② 左モニタの左外（負座標側）
        let far_left = point(-4000, 400);
        let (wa, origin) = work_area_for_window_with_origin(&snapshot, win(far_left, size))
            .expect("非空 snapshot ゆえ Some");
        assert_eq!(
            origin,
            WorkAreaResolution::NearestFallback,
            "dpi={dpi}: 左外の窓は最近傍フォールバック"
        );
        assert_eq!(wa, left_wa(), "dpi={dpi}: 最近傍は左モニタ");

        // ③ 2 面のあいだの帯（右モニタのタスクバー上・非対称 work area 由来）
        //    幅 px(60) の窓を帯へ完全に収め、中心を帯の中へ落とす
        let strip_size = SizePx {
            w: px(40, dpi),
            h: px(40, dpi),
        };
        let strip = point(px(12, dpi), px(400, dpi));
        let (_, origin) = work_area_for_window_with_origin(&snapshot, win(strip, strip_size))
            .expect("非空 snapshot ゆえ Some");
        assert_eq!(
            origin,
            WorkAreaResolution::NearestFallback,
            "dpi={dpi}: 非対称 work area の帯（タスクバー上）は帰属なし"
        );
    }
}

/// 空 snapshot は判別付き版でも `None`（架空の既定矩形を発明しない）。
#[test]
fn with_origin_empty_snapshot_is_none() {
    let snapshot = MonitorSnapshot { work_areas: vec![] };
    assert_eq!(
        work_area_for_window_with_origin(&snapshot, rect(0, 0, 100, 100)),
        None
    );
}

/// **委譲の等価性**（task 2.2 完了条件）: 既存 `work_area_for_window` の戻り値は
/// 判別付き版の第 1 要素と常に一致する＝既存呼出元の挙動が 1 bit も変わらない。
///
/// 帰属・最近傍・境界・重複・空 snapshot の全経路を同一の probe 集合で走らせる。
#[test]
fn work_area_for_window_delegates_to_with_origin() {
    for dpi in DPIS {
        let size = char_size(dpi);
        let snapshots = [
            mixed_layout(dpi),
            // 重複（先勝ち）と共有辺（half-open）を含む合成
            MonitorSnapshot {
                work_areas: vec![
                    rect(0, 0, px(1920, dpi), px(1040, dpi)),
                    rect(px(1920, dpi), 0, px(3840, dpi), px(1040, dpi)),
                    rect(-40, -40, px(2000, dpi), px(1100, dpi)),
                ],
            },
            MonitorSnapshot { work_areas: vec![] },
        ];
        let probes = [
            point(px(800, dpi), grounded_y(right_wa(dpi), size)),
            point(-1200, 400),
            point(px(1920, dpi) + px(400, dpi), 100),
            point(-4000, 2000),
            point(px(12, dpi), px(400, dpi)),
            // 共有辺ちょうどに中心が来る位置（half-open の分岐点）
            point(px(1920, dpi) - size.w / 2, px(500, dpi)),
        ];
        for snapshot in &snapshots {
            for pos in probes {
                let window = win(pos, size);
                assert_eq!(
                    work_area_for_window(snapshot, window),
                    work_area_for_window_with_origin(snapshot, window).map(|(wa, _)| wa),
                    "dpi={dpi}: 委譲の等価性が崩れた（pos={pos:?}）"
                );
            }
        }
    }
}

// --- guard_visibility: キャラ矩形 -----------------------------------------

/// 提案矩形がいずれかの work area と交差していれば素通し（`Keep`）。
/// clamp 先 work area の水平範囲外であっても、交差している限り触らない。
#[test]
fn guard_keeps_position_while_still_intersecting() {
    for dpi in DPIS {
        let snapshot = mixed_layout(dpi);
        let size = char_size(dpi);
        let wa = right_wa(dpi);
        let old = win(point(px(800, dpi), grounded_y(wa, size)), size);

        // 右モニタ内の別位置（交差維持）
        let proposed = point(px(1200, dpi), grounded_y(wa, size));
        assert_eq!(
            guard_visibility(Some(old), proposed, size, wa, &snapshot),
            VisibilityVerdict::Keep(proposed),
            "dpi={dpi}: 交差維持は素通し"
        );

        // 右端から半分はみ出した位置（部分可視＝交差あり）でも素通し
        let half_out = point(wa.right - size.w / 2, grounded_y(wa, size));
        assert!(overlaps(win(half_out, size), wa), "前提: 部分可視である");
        assert_eq!(
            guard_visibility(Some(old), half_out, size, wa, &snapshot),
            VisibilityVerdict::Keep(half_out),
            "dpi={dpi}: 部分可視は clamp しない（美観政策は本 spec 非所有）"
        );
    }
}

/// 交差→非交差の**遷移**は X のみ clamp（Y は射影の所有＝不変）。
/// clamp 後は clamp 先 work area と交差する＝完全不可視が消える。
#[test]
fn guard_clamps_x_on_transition_to_invisible() {
    for dpi in DPIS {
        let snapshot = mixed_layout(dpi);
        let size = char_size(dpi);
        let wa = right_wa(dpi);
        let y = grounded_y(wa, size);
        let old = win(point(px(800, dpi), y), size);
        assert!(overlaps(old, wa), "前提: 旧矩形は可視だった");

        // ① 右外へ吹き飛んだ提案（192 では 4000 超＝3200 超座標）
        let proposed = point(wa.right + px(600, dpi), y);
        assert!(
            !overlaps(win(proposed, size), wa) && !overlaps(win(proposed, size), left_wa()),
            "前提: 提案矩形はどの work area とも交差しない"
        );
        let verdict = guard_visibility(Some(old), proposed, size, wa, &snapshot);
        let VisibilityVerdict::ClampX(got) = verdict else {
            panic!("dpi={dpi}: 交差→非交差の遷移は ClampX（got {verdict:?}）");
        };
        assert_eq!(got.y, proposed.y, "dpi={dpi}: Y は一切変更しない");
        assert!(
            got.x >= wa.left && got.x <= wa.right - size.w,
            "dpi={dpi}: X は clamp_wa の水平範囲内（got.x={}）",
            got.x
        );
        assert!(
            overlaps(win(got, size), wa),
            "dpi={dpi}: clamp 後は clamp 先 work area と交差する"
        );

        // ② 左外（負座標側）へ吹き飛んだ提案でも同じ規則
        let proposed = point(left_wa().left - px(2000, dpi), y);
        assert!(
            !overlaps(win(proposed, size), wa) && !overlaps(win(proposed, size), left_wa()),
            "前提: 提案矩形はどの work area とも交差しない"
        );
        let verdict = guard_visibility(Some(old), proposed, size, wa, &snapshot);
        let VisibilityVerdict::ClampX(got) = verdict else {
            panic!("dpi={dpi}: 左外への遷移も ClampX（got {verdict:?}）");
        };
        assert_eq!(got.y, proposed.y, "dpi={dpi}: Y は一切変更しない");
        assert_eq!(
            got.x, wa.left,
            "dpi={dpi}: 左方向の逸脱は clamp_wa.left へ引き戻す"
        );
        assert!(overlaps(win(got, size), wa), "dpi={dpi}: 交差が回復する");
    }
}

/// 旧矩形も非交差だった（ユーザーが自ら画面外へ留置した窓）＝尊重して素通し。
/// 本 spec の Out of scope「明示ドラッグでの画面外運搬」を型で守る腕。
#[test]
fn guard_respects_window_already_parked_off_screen() {
    for dpi in DPIS {
        let snapshot = mixed_layout(dpi);
        let size = char_size(dpi);
        let wa = right_wa(dpi);
        let y = grounded_y(wa, size);

        let old = win(point(wa.right + px(400, dpi), y), size);
        assert!(
            !overlaps(old, wa) && !overlaps(old, left_wa()),
            "前提: 旧矩形は既に全 work area と非交差（ユーザー留置）"
        );
        let proposed = point(wa.right + px(800, dpi), y);
        assert_eq!(
            guard_visibility(Some(old), proposed, size, wa, &snapshot),
            VisibilityVerdict::Keep(proposed),
            "dpi={dpi}: 既に非交差なら引き戻さない"
        );
    }
}

/// 旧矩形が不明（`None`＝窓生成直後等）は安全側で clamp する。
#[test]
fn guard_clamps_when_old_rect_is_unknown() {
    for dpi in DPIS {
        let snapshot = mixed_layout(dpi);
        let size = char_size(dpi);
        let wa = right_wa(dpi);
        let y = grounded_y(wa, size);
        let proposed = point(wa.right + px(600, dpi), y);

        let verdict = guard_visibility(None, proposed, size, wa, &snapshot);
        let VisibilityVerdict::ClampX(got) = verdict else {
            panic!("dpi={dpi}: 旧矩形不明は安全側 clamp（got {verdict:?}）");
        };
        assert_eq!(got.y, proposed.y, "dpi={dpi}: Y は一切変更しない");
        assert!(
            overlaps(win(got, size), wa),
            "dpi={dpi}: clamp 後は clamp 先 work area と交差する"
        );

        // 旧矩形不明でも、提案が交差しているなら素通し（clamp は遷移時のみ）
        let inside = point(px(800, dpi), y);
        assert_eq!(
            guard_visibility(None, inside, size, wa, &snapshot),
            VisibilityVerdict::Keep(inside),
            "dpi={dpi}: 交差している提案は old 不明でも素通し"
        );
    }
}

/// 窓幅が clamp 先 work area より広い退化ケース: 左端合わせで必ず水平に重なる
/// （`i32::clamp` の逆転区間 panic を踏まない・非 panic 契約）。
#[test]
fn guard_clamp_handles_window_wider_than_work_area() {
    for dpi in DPIS {
        let snapshot = mixed_layout(dpi);
        let wa = right_wa(dpi);
        let size = SizePx {
            w: (wa.right - wa.left) + px(400, dpi),
            h: px(400, dpi),
        };
        let y = grounded_y(wa, size);
        let old = win(point(wa.left, y), size);
        let proposed = point(wa.right + px(1200, dpi), y);

        let verdict = guard_visibility(Some(old), proposed, size, wa, &snapshot);
        let VisibilityVerdict::ClampX(got) = verdict else {
            panic!("dpi={dpi}: 遷移は ClampX（got {verdict:?}）");
        };
        assert_eq!(got.x, wa.left, "dpi={dpi}: 幅超過は left 合わせ");
        assert!(overlaps(win(got, size), wa), "dpi={dpi}: 交差が回復する");
    }
}

/// 空 snapshot（縮退）: 何も交差しないため、旧矩形が読めるなら現状維持。
/// 架空の可視領域を発明しない。
#[test]
fn guard_empty_snapshot_keeps_position() {
    for dpi in DPIS {
        let snapshot = MonitorSnapshot { work_areas: vec![] };
        let size = char_size(dpi);
        let wa = right_wa(dpi);
        let proposed = point(px(800, dpi), px(600, dpi));
        let old = win(point(px(700, dpi), px(600, dpi)), size);
        assert_eq!(
            guard_visibility(Some(old), proposed, size, wa, &snapshot),
            VisibilityVerdict::Keep(proposed),
            "dpi={dpi}: 空 snapshot は現状維持"
        );
    }
}

// --- guard_visibility: バルーン矩形（S3′・Req 3.4） -----------------------
//
// バルーンは**別規則を持たない**——キャラ窓とまったく同一の純関数・同一の
// 遷移規則へ、バルーン矩形（`char_pos + offset` と バルーン寸）を渡すだけ。

/// キャラ窓が右端で clamp された合成で、offset 恒等式が出したバルーン提案位置
/// だけが全 work area と非交差になるケース → バルーン矩形も ClampX で救われる。
#[test]
fn guard_clamps_balloon_rect_that_alone_becomes_invisible() {
    for dpi in DPIS {
        let snapshot = mixed_layout(dpi);
        let wa = right_wa(dpi);
        let c_size = char_size(dpi);
        let b_size = balloon_size(dpi);

        // キャラ窓は右端ぎりぎりに clamp 済み（可視）
        let char_pos = point(wa.right - c_size.w, grounded_y(wa, c_size));
        assert!(overlaps(win(char_pos, c_size), wa), "前提: キャラは可視");

        // offset 恒等式（キャラの右上へ出す）が work area の外を指す
        let offset = point(px(320, dpi), -px(200, dpi));
        let proposed = point(char_pos.x + offset.x, char_pos.y + offset.y);
        let old_balloon = win(point(px(800, dpi), proposed.y), b_size);
        assert!(overlaps(old_balloon, wa), "前提: 旧バルーンは可視だった");
        assert!(
            !overlaps(win(proposed, b_size), wa) && !overlaps(win(proposed, b_size), left_wa()),
            "前提: 提案バルーン矩形はどの work area とも交差しない"
        );

        let verdict = guard_visibility(Some(old_balloon), proposed, b_size, wa, &snapshot);
        let VisibilityVerdict::ClampX(got) = verdict else {
            panic!("dpi={dpi}: バルーンも同一規則で ClampX（got {verdict:?}）");
        };
        assert_eq!(got.y, proposed.y, "dpi={dpi}: バルーンの Y も変更しない");
        assert!(
            got.x >= wa.left && got.x <= wa.right - b_size.w,
            "dpi={dpi}: バルーン X も clamp_wa の水平範囲内"
        );
        assert!(
            overlaps(win(got, b_size), wa),
            "dpi={dpi}: clamp 後のバルーン矩形は work area と交差する（Req 3.4）"
        );
        // clamp によりキャラと部分的に重なり得る＝許容（見えない会話より重なった会話）
    }
}

/// バルーンが交差を保っているあいだは素通し（キャラと同一規則）。
#[test]
fn guard_keeps_balloon_rect_while_intersecting() {
    for dpi in DPIS {
        let snapshot = mixed_layout(dpi);
        let wa = right_wa(dpi);
        let b_size = balloon_size(dpi);
        let proposed = point(px(600, dpi), px(200, dpi));
        let old = win(point(px(500, dpi), px(200, dpi)), b_size);
        assert_eq!(
            guard_visibility(Some(old), proposed, b_size, wa, &snapshot),
            VisibilityVerdict::Keep(proposed),
            "dpi={dpi}: 交差維持のバルーンは素通し"
        );
    }
}

/// ユーザーが画面外へ留置したバルーンは引き戻さない（キャラと同一規則）。
#[test]
fn guard_respects_balloon_parked_off_screen() {
    for dpi in DPIS {
        let snapshot = mixed_layout(dpi);
        let wa = right_wa(dpi);
        let b_size = balloon_size(dpi);
        let old = win(point(wa.right + px(200, dpi), px(200, dpi)), b_size);
        assert!(
            !overlaps(old, wa) && !overlaps(old, left_wa()),
            "前提: 旧バルーンは既に非交差（ユーザー留置）"
        );
        let proposed = point(wa.right + px(600, dpi), px(200, dpi));
        assert_eq!(
            guard_visibility(Some(old), proposed, b_size, wa, &snapshot),
            VisibilityVerdict::Keep(proposed),
            "dpi={dpi}: 留置バルーンは尊重する"
        );
    }
}

/// Y 不変の横断檻: 全分岐・キャラ／バルーン両寸で `position().y == proposed.y`
/// （Y は射影 T の所有・D6）。分岐の識別（Keep か ClampX か）も同時に固定する。
///
/// # 檻の非空虚性の要（レビュー #1・2026-07-31 の指摘に対する是正）
///
/// 提案 Y に射影 T 由来の接地値（`wa.bottom − h`）だけを与えると、その Y は
/// **work area の Y clamp の不動点**であるため「ガードが Y も clamp する」という
/// 実在しやすい退行（`y: proposed.y.min(wa.bottom − h).max(wa.top)`）と正しい実装が
/// 区別できず、檻が空虚になる。よって各分岐へ
/// `[clamp_wa.top, clamp_wa.bottom − h]` の**範囲外**の Y を必ず通す。
///
/// 範囲外 Y の投入は契約上も正当である——`guard_visibility` の前提条件は正寸のみ
/// であり（design.md:425）、Y の値域は射影 T の関心であってガードの前提ではない。
#[test]
fn guard_never_modifies_y_in_any_branch() {
    for dpi in DPIS {
        let snapshot = mixed_layout(dpi);
        let wa = right_wa(dpi);
        for size in [char_size(dpi), balloon_size(dpi)] {
            // Y clamp の**不動点**（射影 T が出す接地 Y）＝従来の網羅を維持する側
            let y_fixed = grounded_y(wa, size);
            // Y clamp の不動点**ではない** Y ＝ clamp が入れば必ず動く側
            let y_above = wa.top - px(300, dpi); // 上端より上
            let y_below = wa.bottom + px(200, dpi); // 下端より下
            let y_partial = wa.top - size.h / 2; // 上端を跨ぐ（水平内なら交差は保つ）
            for y in [y_above, y_below, y_partial] {
                assert!(
                    y < wa.top || y > wa.bottom - size.h,
                    "前提: {y} は work area Y clamp の不動点であってはならない\
                     （dpi={dpi} size={size:?}）"
                );
            }

            let x_in = px(800, dpi);
            let x_far = wa.right + px(900, dpi);
            let old_visible = Some(win(point(px(700, dpi), y_fixed), size));
            let old_parked = Some(win(point(wa.right + px(500, dpi), y_fixed), size));
            let in_partial = point(x_in, y_partial);
            let far_above = point(x_far, y_above);
            let far_below = point(x_far, y_below);
            let in_fixed = point(x_in, y_fixed);
            let far_fixed = point(x_far, y_fixed);

            for (label, old, proposed, expect_clamped) in [
                // --- 範囲外 Y（Y clamp 退行を必ず捕まえる側）---
                ("Keep 交差維持", old_visible, in_partial, false),
                ("ClampX 遷移", old_visible, far_above, true),
                ("Keep 留置尊重", old_parked, far_below, false),
                ("ClampX 安全側", None, far_below, true),
                // --- 不動点 Y（射影 T の実出力に相当する正常系）---
                ("Keep 交差維持@接地Y", old_visible, in_fixed, false),
                ("ClampX 遷移@接地Y", old_visible, far_fixed, true),
                ("Keep 留置尊重@接地Y", old_parked, far_fixed, false),
                ("ClampX 安全側@接地Y", None, far_fixed, true),
            ] {
                let verdict = guard_visibility(old, proposed, size, wa, &snapshot);
                assert_eq!(
                    matches!(verdict, VisibilityVerdict::ClampX(_)),
                    expect_clamped,
                    "dpi={dpi} {label}: 分岐の識別が想定と違う\
                     （size={size:?} proposed={proposed:?} verdict={verdict:?}）"
                );
                assert_eq!(
                    verdict.position().y,
                    proposed.y,
                    "dpi={dpi} {label}: Y は全分岐で不変\
                     （size={size:?} proposed={proposed:?}）"
                );
            }
        }
    }
}

// -------------------------------------------------------------------------
// Arrangement.offset 同期（task 8.3-fix・4.8 実機ブロッカ）
//
// enqueue_window_set_pos は WindowPos を bypass_change_detection() で書くため
// Changed<WindowPos> が発火せず、wintf の
// sync_window_arrangement_from_window_pos は走らない。同期を怠ると
// GlobalArrangement（αマスクヒットテストの境界）が spawn 位置に取り残され、
// 移動後のバルーンがクリック死する（実機で確認された 4.8 ブロッカ）。
// 実 pipeline では window entity に Arrangement が付く（Visual::on_add）が、
// bare World には無いので spawn 時 offset 付きで手動挿入して檻にする。
// 期待値は wintf DragEnd 直接同期と同じ `as f32` 転写の完全一致。
// -------------------------------------------------------------------------

use wintf::ecs::layout::{Arrangement, Offset};

/// spawn 時 offset 付きの Arrangement（実 pipeline の spawn 位置を模す）。
fn arrangement_at(x: f32, y: f32) -> Arrangement {
    Arrangement {
        offset: Offset { x, y },
        ..Default::default()
    }
}

/// entity の Arrangement.offset を読む（未付与は panic で検出）。
fn arrangement_offset_of(world: &World, entity: Entity) -> Offset {
    world
        .get::<Arrangement>(entity)
        .expect("Arrangement があるはず")
        .offset
}

/// (a) 実 on_char_drag（Bubble DragEvent＋DraggingState・8.2R 単一ライター）:
/// 移動後、キャラ窓・随伴バルーンとも Arrangement.offset が
/// WindowPos.position と一致する（GA ヒットテスト境界の追従・4.8）。
#[test]
fn on_char_drag_syncs_arrangement_offset_of_char_and_balloon() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot()); // 下端 1043・釘付け Y=356
    let balloon = world
        .spawn((
            fake_handle(0x2000),
            window_pos_at(795, 331),
            arrangement_at(795.0, 331.0),
        ))
        .id();
    let offset = PointPx { x: -412, y: -25 };
    let start = (1400, 600);
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(1207, 356, 434, 687),
            arrangement_at(1207.0, 356.0),
            Anchored(Anchor::Bottom),
            BalloonFollow { balloon, offset },
            dragging_state((1207, 356), start),
        ))
        .id();

    let ev = Phase::Bubble(drag_event_at(window, start, (1450, 350)));
    assert!(!on_char_drag(&mut world, window, window, &ev));

    // 適用後キャラ窓 (1257, 356)・バルーン (1257−412, 356−25)
    let char_pos = position_of(&world, window);
    assert_eq!(char_pos, Point { x: 1257, y: 356 });
    assert_eq!(
        arrangement_offset_of(&world, window),
        Offset {
            x: char_pos.x as f32,
            y: char_pos.y as f32
        },
        "キャラ窓の Arrangement.offset が WindowPos に追従する"
    );
    let balloon_pos = position_of(&world, balloon);
    assert_eq!(balloon_pos, Point { x: 845, y: 331 });
    assert_eq!(
        arrangement_offset_of(&world, balloon),
        Offset {
            x: balloon_pos.x as f32,
            y: balloon_pos.y as f32
        },
        "バルーンの Arrangement.offset が WindowPos に追従する（クリック死の檻）"
    );
}

/// (b) move_window_to: 対象キャラ窓・随伴バルーンとも Arrangement.offset が
/// 移動後の WindowPos.position と一致する。
#[test]
fn move_window_to_syncs_arrangement_offset_of_target_and_balloon() {
    let mut world = World::new();
    let balloon = world
        .spawn((
            fake_handle(0x2000),
            window_pos_at(0, 0),
            arrangement_at(0.0, 0.0),
        ))
        .id();
    let offset = PointPx { x: -412, y: -25 };
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_at(50, 60),
            arrangement_at(50.0, 60.0),
            BalloonFollow { balloon, offset },
        ))
        .id();

    assert!(move_window_to(&mut world, window, 907, 1201));

    assert_eq!(
        arrangement_offset_of(&world, window),
        Offset { x: 907.0, y: 1201.0 }
    );
    assert_eq!(
        arrangement_offset_of(&world, balloon),
        Offset {
            x: (907 + offset.x) as f32,
            y: (1201 + offset.y) as f32
        }
    );
}

/// (c) move_window_to（BalloonFollow なしの単独窓）: 自身の Arrangement.offset
/// が同期される（バルーン単独移動＝enqueue 共通経路の檻）。
#[test]
fn move_window_to_syncs_arrangement_offset_of_single_window() {
    let mut world = World::new();
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_at(10, 20),
            arrangement_at(10.0, 20.0),
        ))
        .id();

    assert!(move_window_to(&mut world, window, 1531, 883));
    assert_eq!(
        arrangement_offset_of(&world, window),
        Offset { x: 1531.0, y: 883.0 }
    );
}

// -------------------------------------------------------------------------
// enqueue_window_set_pos（size 対応一般化・task 2.3・Req1.5/3.3・
// design Testing Strategy > Integration Tests #5）
//
// 既存 move 専用発行口の一般化。`None` は移動専用の後方互換（position のみ
// ミラー・size 不変・SWP_NOSIZE 継続）、`Some` は位置＋寸を一度に反映
// （WindowPos.size も bypass ミラー）。観測境界は `WindowPos.position`／
// `WindowPos.size` のミラー——`SetWindowPosCommand` キューは private TLS で
// flush せず flags/width/height を覗けないため（design Validation の指定）。
// 座標・寸法は 96 の非倍数を使い、隠れた dpi/96 再スケールの檻とする。
// -------------------------------------------------------------------------

use super::enqueue_window_set_pos;

/// entity の WindowPos.size を読む（未設定は panic で検出）。
fn size_of(world: &World, entity: Entity) -> SizeI {
    world
        .get::<WindowPos>(entity)
        .expect("WindowPos があるはず")
        .size
        .expect("size があるはず")
}

/// `None`（後方互換・移動専用）: position のみ更新し size は触らない
/// （既存移動専用挙動＝SWP_NOSIZE 継続の観測境界）。
#[test]
fn enqueue_window_set_pos_none_updates_position_leaves_size() {
    let mut world = World::new();
    let window = world
        .spawn((fake_handle(0x1234), window_pos_sized(10, 20, 434, 687)))
        .id();

    assert!(enqueue_window_set_pos(
        &mut world, window, 1531, 883, None, None
    ));
    assert_eq!(position_of(&world, window), Point { x: 1531, y: 883 });
    // size は不変（移動専用＝寸法を書かない）
    assert_eq!(size_of(&world, window), SizeI::new(434, 687));
}

/// `Some`: 位置と寸法の**双方**が更新される（WindowPos.size = SizeI::new(w,h)）。
#[test]
fn enqueue_window_set_pos_some_updates_position_and_size() {
    let mut world = World::new();
    let window = world
        .spawn((fake_handle(0x1234), window_pos_sized(10, 20, 434, 687)))
        .id();

    assert!(enqueue_window_set_pos(
        &mut world,
        window,
        907,
        1201,
        Some(SizePx { w: 517, h: 823 }),
        None,
    ));
    assert_eq!(position_of(&world, window), Point { x: 907, y: 1201 });
    assert_eq!(size_of(&world, window), SizeI::new(517, 823));
}

/// 不在/未付与（Req3.3）: `WindowHandle` 無し entity は `false`＋位置/寸法不変
/// （warn no-op・`Some` 経路でも既存 warn 経路を継承）。
#[test]
fn enqueue_window_set_pos_without_handle_returns_false_and_leaves_state() {
    let mut world = World::new();
    let window = world.spawn(window_pos_sized(10, 20, 434, 687)).id();

    assert!(!enqueue_window_set_pos(
        &mut world,
        window,
        907,
        1201,
        Some(SizePx { w: 517, h: 823 }),
        None,
    ));
    assert_eq!(position_of(&world, window), Point { x: 10, y: 20 });
    assert_eq!(size_of(&world, window), SizeI::new(434, 687));
}

// -------------------------------------------------------------------------
// resize_window_to（単一ライター反映口・task 2.4・
// Req1.1/1.3/1.7/3.1/3.4＋2.6/3.3・design Integration Tests #1・#4 一部）
//
// 新しい表示寸法へアンカー射影 T を再適用し、確定 position＋size を単一ライター
// 経路で一度だけ書く（bottom は wa.bottom−h' 再計算）。観測境界は headless World
// （偽 HWND）の WindowPos.position／WindowPos.size ミラー——SetWindowPosCommand
// キューは private TLS で flush せず flags/width/height を覗けないため。縮退
// （べき等・非正寸・不在・Anchored 欠落）は false＋状態不変で固定する。座標・
// 寸法は 96 の非倍数を使い、隠れた dpi/96 再スケールの檻とする。
// -------------------------------------------------------------------------

use super::{PlacementRoute, resize_window_to};

/// #1 一度書き＋re-snap（Req1.1/1.3/1.7/2.1）: `Anchored(Bottom)` の char 窓を
/// 新寸へ resize すると、`WindowPos.size` が新寸・`position.y` が `wa.bottom − h'`
/// へ更新され `true`。**原点＝下端中央**ゆえ x は「中央を保つ」よう付け替わる
/// （伺かの立ち絵は足元中央が接地点＝寸法が変わっても原点は動かない）。
/// 下端・寸法とも 96 非倍数で dpi/96 再スケール混入の檻。
#[test]
fn resize_window_to_bottom_resnaps_size_and_position_once() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot()); // 下端 1043
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687), // 旧寸で下端釘付け済み
            Anchored(Anchor::Bottom),
        ))
        .id();

    // 新寸 (517×823・いずれも 96 非倍数): Y=1043−823=220。
    // X は下端中央保持: 旧中央 731+434/2=948 → 新 x = 948−517/2 = 690。
    assert!(resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    assert_eq!(
        position_of(&world, window),
        Point {
            x: 690,
            y: 1043 - 823
        },
        "下端中央保持（旧中央 948 を維持）・Y=wa.bottom−h'（bottom 再計算）"
    );
    assert_eq!(size_of(&world, window), SizeI::new(517, 823));
}

/// #4 べき等 skip（Req3.1）: 既に射影済み位置＋同寸の窓へ同寸 resize すると、
/// 書込なし・`false`・状態不変（冗長な再配置を避ける）。
#[test]
fn resize_window_to_is_idempotent_on_same_size_and_position() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot()); // 下端 1043・Y=1043−687=356
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687), // 既に bottom 射影済み
            Anchored(Anchor::Bottom),
        ))
        .id();

    // 同寸 → 導出 (731,356)＋(434,687) は現在値と同一 → 書込なし・false
    assert!(!resize_window_to(
        &mut world,
        window,
        SizePx { w: 434, h: 687 },
        PlacementRoute::Resnap
    ));
    assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
    assert_eq!(size_of(&world, window), SizeI::new(434, 687));
}

/// #4 非正寸縮退（Req3.4）: w≤0 or h≤0 は T 再適用せず `false`・位置/寸不変
/// （warn・`BottomSnapPolicy` の非正寸縮退と整合）。
#[test]
fn resize_window_to_nonpositive_size_holds_state() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687),
            Anchored(Anchor::Bottom),
        ))
        .id();

    for bad in [
        SizePx { w: 0, h: 823 },
        SizePx { w: 517, h: 0 },
        SizePx { w: -517, h: -823 },
    ] {
        assert!(
            !resize_window_to(&mut world, window, bad, PlacementRoute::Resnap),
            "{bad:?}: 非正寸は false"
        );
        assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
        assert_eq!(size_of(&world, window), SizeI::new(434, 687));
    }
}

/// #4 不在/未付与（Req3.3）: `WindowHandle` 未付与の char 窓は `false`・状態不変
/// （`enqueue_window_set_pos` の warn no-op を継承・随伴バルーンも動かさない）。
#[test]
fn resize_window_to_without_handle_returns_false_and_leaves_state() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let window = world
        .spawn((
            // WindowHandle なし（窓生成前）
            window_pos_sized(731, 356, 434, 687),
            Anchored(Anchor::Bottom),
        ))
        .id();

    assert!(!resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
    assert_eq!(size_of(&world, window), SizeI::new(434, 687));
}

/// #4 Anchored 欠落: 単一真実源 `Anchored` 未付与の窓は `false`・状態不変
/// （char 窓は spawn で必ず付与＝異常系・warn no-op）。
#[test]
fn resize_window_to_without_anchored_returns_false_and_leaves_state() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687),
            // Anchored なし
        ))
        .id();

    assert!(!resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
    assert_eq!(size_of(&world, window), SizeI::new(434, 687));
}

/// #1 随伴バルーン維持（Req2.6）＝**窓相対 offset 不変**（Bottom）:
/// `BalloonFollow` 付き Bottom char 窓を resize しても `BalloonFollow.offset` は
/// 書き換わらず、バルーンは `new_char_pos + offset` へ随伴して恒等式
/// `balloon_pos − char_pos ≡ offset` を保つ。
///
/// キャラ窓自身の原点は下端中央（`char_pos` は中央 x を保って再導出される）が、
/// **バルーンの追従は原点基準ではなく窓（左上）相対**である——受理オラクルは
/// 参照実装 SSP の実測で、SSP のバルーンは観測時つねに現在表示中のキャラ窓に対して
/// 窓相対にある（2026-07-31 実機裁定）。以前の「下端中央基準の offset 補正」は Bottom だけを
/// 窓相対から外し、実機でバルーンを旧絶対位置に置き去りにしていた（本檻はその反転）。
/// 実寸オラクルは `resize_window_to_bottom_keeps_ssp_window_relative_balloon_offset`。
#[test]
fn resize_window_to_bottom_preserves_balloon_follow_offset() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot()); // 下端 1043
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
        .id();
    let offset = PointPx { x: -412, y: -25 };
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687),
            Anchored(Anchor::Bottom),
            BalloonFollow { balloon, offset },
        ))
        .id();

    // 旧原点（下端中央）: x=731+434/2=948・y=356+687=1043。
    // 旧バルーン絶対位置: (731−412, 356−25)=(319, 331)。
    let old_origin = (731 + 434 / 2, 356 + 687);
    let old_balloon = Point {
        x: 731 + offset.x,
        y: 356 + offset.y,
    };

    // 新寸 (517×823): char は下端中央保持で x=948−517/2=690・y=1043−823=220。
    assert!(resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    let char_pos = position_of(&world, window);
    let balloon_pos = position_of(&world, balloon);
    assert_eq!(char_pos, Point { x: 690, y: 1043 - 823 });

    // キャラ窓の原点（下端中央）は寸法変動で動かない（step 3b の契約・無改変）。
    let new_origin = (char_pos.x + 517 / 2, char_pos.y + 823);
    assert_eq!(
        new_origin, old_origin,
        "原点（下端中央）は寸法変動で動かない"
    );

    // バルーンは**窓相対**: 新 char 左上 + 不変 offset。
    assert_eq!(
        balloon_pos,
        Point {
            x: char_pos.x + offset.x,
            y: char_pos.y + offset.y
        },
        "バルーンは窓（左上）相対 offset で追随する"
    );
    // offset 恒等式（balloon_pos − char_pos ≡ offset）の維持。
    assert_eq!(balloon_pos.x - char_pos.x, offset.x);
    assert_eq!(balloon_pos.y - char_pos.y, offset.y);
    assert_eq!(
        world.get::<BalloonFollow>(window).unwrap().offset,
        offset,
        "BalloonFollow.offset は resize で補正されない"
    );
    // 旧「下端中央基準」実装は原点不動ゆえバルーン絶対位置も不動にしていた——
    // 窓上端が 136px 上がった本ケースでは窓相対と弁別できる（反転の証明）。
    assert_ne!(
        balloon_pos, old_balloon,
        "下端中央基準補正の復活検出: 窓が動いた以上バルーンも動く"
    );
}

/// SSP オラクル檻（2026-07-31 実機裁定・実 DPI 120／k=1.25 のむらさき実寸）:
/// talk 中のサーフェス切替で Bottom キャラ窓が 543×859 → 478×684（下端 2100 固定）へ
/// 縮んでも、バルーンは**窓相対 offset (−167,−161) を保ったまま**追随する。
///
/// 参照実装 SSP は同時点で char 477×683@(3363,1417)／balloon (3195,1256)＝offset
/// (−168,−161) を保っており、本檻の (−167,−161) とは x が 1px だけ違う。この 1px は
/// サーフェス寸の丸め権威（SSP と areka のスケール丸め）由来であって、追従セマンティクス
/// とは無関係——本変更の受理判定には影響しない。
///
/// 欠陥（削除した step 6＝Bottom 限定の下端中央基準 offset 補正）が残っていると、
/// offset は (−167+(478/2−543/2), −161+(684−859)) = (−199,−336) へ書き換わり、
/// バルーンは旧絶対位置 (3130,1080) に貼り付いたまま新窓上端の 336px 上空へ浮く
/// ——実機で観測された症状そのもの。本檻はその恒久回帰檻。
#[test]
fn resize_window_to_bottom_keeps_ssp_window_relative_balloon_offset() {
    let mut world = World::new();
    // 実機 4K 縦 2100 の work area（下端 2100・むらさきが載っていたモニタ）。
    world.insert_resource(MonitorSnapshot {
        work_areas: vec![rect(2560, 0, 3840, 2100)],
    });
    // boot 直後の実測: char 543×859 @ (3297,1241)／balloon (3130,1080)。
    let offset = PointPx { x: -167, y: -161 };
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(3130, 1080)))
        .id();
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(3297, 1241, 543, 859),
            Anchored(Anchor::Bottom),
            BalloonFollow { balloon, offset },
        ))
        .id();

    // サーフェス切替後の実測寸 478×684 へ resize。
    // route は `dpi-window-vanish` の D11 配管でシグネチャに入った引数。本檻の主題は
    // 追従セマンティクス（窓相対）ゆえ、遷移ガードが発火する配置系 route の代表値
    // （`Resnap`）を渡す——ガードが働く経路でも offset が補正されないことを見る。
    assert!(resize_window_to(
        &mut world,
        window,
        SizePx { w: 478, h: 684 },
        PlacementRoute::Resnap
    ));
    let char_pos = position_of(&world, window);
    let balloon_pos = position_of(&world, balloon);

    // char は下端中央原点を保つ: 中央 x = 3297+271 = 3568 → 左上 x = 3568−239 = 3329・
    // y = wa.bottom − h' = 2100−684 = 1416（実機実測 (3329,1416) と一致）。
    assert_eq!(
        char_pos,
        Point { x: 3329, y: 1416 },
        "char は下端中央原点維持（実機実測 (3329,1416)）"
    );
    // バルーンは**窓相対**: 新窓左上 + offset = (3329−167, 1416−161) = (3162,1255)。
    assert_eq!(
        balloon_pos,
        Point { x: 3329 - 167, y: 1416 - 161 },
        "バルーンは窓相対 offset で追随する（SSP と同セマンティクス）"
    );
    // 恒等式 balloon_pos − char_pos ≡ offset（resize で補正しない）。
    assert_eq!(balloon_pos.x - char_pos.x, offset.x);
    assert_eq!(balloon_pos.y - char_pos.y, offset.y);
    assert_eq!(
        world.get::<BalloonFollow>(window).unwrap().offset,
        offset,
        "BalloonFollow.offset は resize で書き換わらない"
    );
    // 欠陥時の値（旧絶対位置に貼り付く）を明示的に排除する。
    assert_ne!(
        balloon_pos,
        Point { x: 3130, y: 1080 },
        "step 6 復活の検出: 旧絶対位置に貼り付いてはならない"
    );
}

// -------------------------------------------------------------------------
// resize_window_to 5 アンカー統合網羅（task 2.5・テスト固定タスク・
// Req1.1/2.1-2.6/3.1/3.3/3.4・design Integration Tests #2・#3・#4）
//
// task 2.4 が Bottom で押さえた「一度書き＋re-snap／べき等／非正寸／不在／
// Anchored 欠落／随伴バルーン維持」を、残る Top/Left/Right/Free へ拡張する。
// resize_window_to 本体は 2.4 で完成済み＝本群は「既存配線が 5 アンカーで
// 正しく `Anchored.0` を転送している（非 Bottom を `Anchor::Bottom` へ
// ハードコードしていない）」ことを固定する回帰檻（非 Bottom 配線バグ＝
// 2.4 エスケープの捕捉）。
//
// 全辺 96 非倍数の odd_edge_snapshot（rect(53,37,1877,1043)）で各アンカー辺の
// 再計算を dpi/96 再スケール混入の檻とし、各アンカーで「固定辺の座標」と
// 「非アンカー軸の保持」を両方 assert する（Top↔Bottom は Y・Left↔Right は X が
// 合わず落ちる取り違え耐性）。
// -------------------------------------------------------------------------

/// #2 Top resize（Req2.2）: `Anchored(Top)` を新寸へ resize すると `WindowPos.size`
/// 新寸・`position.y = wa.top`（上端固定）・`position.x` 保持で `true`。
/// Bottom と取り違えれば Y が `wa.bottom−h'` になって落ちる（辺取り違え耐性）。
#[test]
fn resize_window_to_top_pins_top_edge_and_keeps_x() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot()); // rect(53, 37, 1877, 1043)
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 500, 434, 687),
            Anchored(Anchor::Top),
        ))
        .id();

    // 新寸 (517×823・いずれも 96 非倍数): Y=wa.top=37・X=731 保持
    assert!(resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    assert_eq!(
        position_of(&world, window),
        Point { x: 731, y: 37 },
        "X 保持・Y=wa.top（上端固定・Bottom と取り違えたら 1043−823 で落ちる）"
    );
    assert_eq!(size_of(&world, window), SizeI::new(517, 823));
}

/// #2 Left resize（Req2.3）: `Anchored(Left)` を新寸へ resize すると `WindowPos.size`
/// 新寸・`position.x = wa.left`（左端固定）・`position.y` 保持で `true`。
/// Right と取り違えれば X が `wa.right−w'` になって落ちる（辺取り違え耐性）。
#[test]
fn resize_window_to_left_pins_left_edge_and_keeps_y() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot()); // rect(53, 37, 1877, 1043)
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 500, 434, 687),
            Anchored(Anchor::Left),
        ))
        .id();

    // 新寸 (517×823): X=wa.left=53・Y=500 保持
    assert!(resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    assert_eq!(
        position_of(&world, window),
        Point { x: 53, y: 500 },
        "X=wa.left（左端固定・Right と取り違えたら 1877−517 で落ちる）・Y 保持"
    );
    assert_eq!(size_of(&world, window), SizeI::new(517, 823));
}

/// #2 Right resize（Req2.4）: `Anchored(Right)` を新寸へ resize すると `WindowPos.size`
/// 新寸・`position.x = wa.right − w'`（右端固定）・`position.y` 保持で `true`。
/// Left と取り違えれば X が `wa.left` になって落ちる（辺取り違え耐性）。
#[test]
fn resize_window_to_right_pins_right_edge_and_keeps_y() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot()); // rect(53, 37, 1877, 1043)
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 500, 434, 687),
            Anchored(Anchor::Right),
        ))
        .id();

    // 新寸 (517×823): X = wa.right − w' = 1877 − 517 = 1360・Y=500 保持
    assert!(resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    assert_eq!(
        position_of(&world, window),
        Point { x: 1877 - 517, y: 500 },
        "X=wa.right−w'（右端固定・Left と取り違えたら 53 で落ちる）・Y 保持"
    );
    assert_eq!(size_of(&world, window), SizeI::new(517, 823));
}

/// #2 Free resize（Req2.5）: `Anchored(Free)` はアンカー辺を持たず position を
/// 保持し、`WindowPos.size` のみ新寸へ反映する。size が変わるので冗長でなく
/// `true`（書込あり）。Bottom へ取り違えれば position.y が動いて落ちる
/// （射影なし・寸法反映のみの区別）。
#[test]
fn resize_window_to_free_keeps_position_and_updates_size_only() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot());
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 500, 434, 687),
            Anchored(Anchor::Free),
        ))
        .id();

    // Free: 射影なし＝position 不変・size のみ新寸（size 変化ゆえ冗長でなく true）
    assert!(resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    assert_eq!(
        position_of(&world, window),
        Point { x: 731, y: 500 },
        "Free は position 再計算なし（現在位置保持・Bottom 取り違えなら Y が動く）"
    );
    assert_eq!(size_of(&world, window), SizeI::new(517, 823));
}

/// #3 随伴バルーン維持（Left・Req2.6）: `Anchored(Left)`＋`BalloonFollow` の
/// char 窓を resize すると、char は左端固定（Y 保持）へ移り、バルーンは
/// `new_char_pos + offset` へ随伴し `balloon_pos − char_pos ≡ offset` を維持する。
///
/// 本檻はかつて「非 Bottom だけの例外」を主張していたが、2026-07-31 実機裁定で
/// Bottom の下端中央基準補正が撤去され、窓相対追従が**全アンカー共通の規範**になった
/// ——Bottom 版は `resize_window_to_bottom_preserves_balloon_follow_offset`。
/// 本檻はその規範をアンカー辺 x 固定（Left）側で固定する。
#[test]
fn resize_window_to_left_preserves_balloon_follow_offset() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot()); // 左端 53
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
        .id();
    let offset = PointPx { x: -412, y: -25 };
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 500, 434, 687),
            Anchored(Anchor::Left),
            BalloonFollow { balloon, offset },
        ))
        .id();

    // 新寸 (517×823) → char 左端固定 (53, 500)・balloon (53−412, 500−25)
    assert!(resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    let char_pos = position_of(&world, window);
    let balloon_pos = position_of(&world, balloon);
    assert_eq!(char_pos, Point { x: 53, y: 500 }, "左端固定・Y 保持");
    assert_eq!(
        balloon_pos,
        Point {
            x: 53 + offset.x,
            y: 500 + offset.y
        }
    );
    // offset 恒等式（balloon_pos − char_pos ≡ offset）の維持
    assert_eq!(balloon_pos.x - char_pos.x, offset.x);
    assert_eq!(balloon_pos.y - char_pos.y, offset.y);
}

/// #4 べき等（非 Bottom・Req3.1）: 既に左端一致（x=wa.left）の位置＋同寸へ
/// `Anchored(Left)` を resize すると、導出 (position, size) が現在値と同一ゆえ
/// 書込なし・`false`・状態不変（Bottom 版 idempotent の非 Bottom 対応・
/// 同一寸法/同一アンカーの再適用が窓状態を変更しない＝冗長書込をしない）。
#[test]
fn resize_window_to_left_is_idempotent_on_same_size_and_position() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot()); // 左端 53
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(53, 500, 517, 823), // 既に左端射影済み・同寸
            Anchored(Anchor::Left),
        ))
        .id();

    // 同寸・既に左端一致 → 導出 (53,500)＋(517,823) は現在値と同一 → 書込なし・false
    assert!(!resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    assert_eq!(position_of(&world, window), Point { x: 53, y: 500 });
    assert_eq!(size_of(&world, window), SizeI::new(517, 823));
}

/// #4 非 Bottom 縮退（Req3.3/3.4）: 縮退経路がアンカー非依存（Bottom 特化でない）
/// ことを代表として Top で固定する。task 2.4 が Bottom で押さえた縮退を、
/// 別アンカーでも配線が同一であることの確認（過剰重複を避け 1 件へ集約）。
/// - 非正寸（w≤0 or h≤0）: project_anchor 前に弾かれ `false`・位置/寸不変。
/// - `WindowHandle` 未付与: 射影は走るが enqueue が warn no-op＝`false`・位置/寸不変。
#[test]
fn resize_window_to_non_bottom_degrades_on_nonpositive_and_missing_handle() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot());

    // (a) Top＋非正寸: project_anchor 前に弾かれ false・状態不変（Bottom と同一縮退）
    let with_handle = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 500, 434, 687),
            Anchored(Anchor::Top),
        ))
        .id();
    for bad in [
        SizePx { w: 0, h: 823 },
        SizePx { w: 517, h: 0 },
        SizePx { w: -517, h: -823 },
    ] {
        assert!(
            !resize_window_to(&mut world, with_handle, bad, PlacementRoute::Resnap),
            "{bad:?}: 非正寸は false（Top でも Bottom と同一縮退）"
        );
        assert_eq!(position_of(&world, with_handle), Point { x: 731, y: 500 });
        assert_eq!(size_of(&world, with_handle), SizeI::new(434, 687));
    }

    // (b) Top＋WindowHandle 未付与: 射影は走るが enqueue が warn no-op＝false・状態不変
    let no_handle = world
        .spawn((
            // WindowHandle なし（窓生成前）
            window_pos_sized(731, 500, 434, 687),
            Anchored(Anchor::Top),
        ))
        .id();
    assert!(!resize_window_to(
        &mut world,
        no_handle,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    assert_eq!(position_of(&world, no_handle), Point { x: 731, y: 500 });
    assert_eq!(size_of(&world, no_handle), SizeI::new(434, 687));
}

// -------------------------------------------------------------------------
// anchor_changed_system（アンカー変化トリガ・task 2.6・Req1.4・
// design「Anchored（Component）/ anchor_changed_system」「System Flows >
// アンカー変化トリガ」「File Structure Plan > follow.rs」）
//
// producer（seriko の `\![set,alignmenttodesktop]` routing）は本 spec 非所有＝
// 本群は `Changed<Anchored>` に反応する **consumer** のみを固定し、テストは
// `Anchored` を直接書き換えて駆動する。change tick を正しく管理するため system は
// `Schedule` に登録して run し（同一 Schedule インスタンスを使い回すことで
// 永続 `SystemState` の `last_run` を run 跨ぎで効かせる）、初回 run の全マッチは
// resize_window_to のべき等 skip で吸収する。全辺 96 非倍数の odd_edge_snapshot
// （rect(53,37,1877,1043)）で dpi/96 再スケール混入の檻とする。
// -------------------------------------------------------------------------

use super::anchor_changed_system;

/// #1 アンカー変化で再射影（Req1.4 の核）: `Anchored(Bottom)` の釘付け済み char 窓を
/// spawn し、初回 run はべき等 skip（初回 Changed 付与を resize が同寸・同位置で吸収
/// ＝位置不変）。次に `Anchored` を Top へ**直接書換**→再 run で「現在の表示寸法の
/// まま」新アンカー辺（y=wa.top）へ再配置され、X 保持・size 不変（新寸を与えない
/// ので size は変わらない）。
#[test]
fn anchor_changed_system_reprojects_to_new_anchor_edge_at_current_size() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot()); // rect(53, 37, 1877, 1043)
    // Bottom 釘付け済み: y = wa.bottom − h = 1043 − 687 = 356・x=731（96 非倍数）
    let e = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687),
            Anchored(Anchor::Bottom),
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems(anchor_changed_system);

    // 初回 run: 初回 Changed 付与で発火し得るが、Bottom は現寸で y=356 のまま
    // ＝べき等 skip で吸収（位置・寸法不変）。
    schedule.run(&mut world);
    assert_eq!(
        position_of(&world, e),
        Point { x: 731, y: 356 },
        "初回 run はべき等 skip（位置不変）"
    );
    assert_eq!(size_of(&world, e), SizeI::new(434, 687), "初回 run: size 不変");

    // Anchored を Top へ直接書換（producer=seriko の代替＝consumer 駆動の檻）。
    world.get_mut::<Anchored>(e).unwrap().0 = Anchor::Top;

    // 再 run: 現在の表示寸法(434×687)のまま新アンカー辺 y=wa.top=37 へ再射影。
    schedule.run(&mut world);
    assert_eq!(
        position_of(&world, e),
        Point { x: 731, y: 37 },
        "新アンカー辺 y=wa.top へ再配置・X=731 保持（Bottom のままなら y=356 で落ちる）"
    );
    assert_eq!(
        size_of(&world, e),
        SizeI::new(434, 687),
        "現在の表示寸法のまま（新寸を与えないので size は不変）"
    );
}

/// #2 Anchored 未変化では発火しない（変更検知の正しさの檻・最重要）: 初回 run で
/// 初回 Changed を消費した後、`Anchored` を触らずに `WindowPos.position` を故意に
/// アンカー辺から外して再 run しても**再スナップされない**（system は `Anchored`
/// 変化にのみ反応し `WindowPos` 変化には反応しない）。毎 run 全マッチ実装
/// （fresh QueryState の last_run=0）ならここで y=356 へ戻り落ちる。
#[test]
fn anchor_changed_system_does_not_fire_when_anchor_unchanged() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot()); // 下端 1043
    let e = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687), // Bottom 釘付け済み（y=1043−687）
            Anchored(Anchor::Bottom),
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems(anchor_changed_system);

    // 初回 run で初回 Changed<Anchored> を消費（べき等 skip・位置不変）。
    schedule.run(&mut world);
    assert_eq!(position_of(&world, e), Point { x: 731, y: 356 });

    // Anchored は触らず、WindowPos.position をアンカー辺から外れた位置へ手動移動。
    world.get_mut::<WindowPos>(e).unwrap().position = Some(Point { x: 731, y: 900 });

    // 再 run: Anchored 未変化ゆえ Changed にマッチせず再スナップしない。
    schedule.run(&mut world);
    assert_eq!(
        position_of(&world, e),
        Point { x: 731, y: 900 },
        "Anchored 未変化では再スナップしない（毎 run 全マッチ実装ならここで y=356 へ戻り落ちる）"
    );
}

/// #3 別遷移（Bottom→Left）: `Anchored` を Left へ直接書換すると、現在の表示寸法の
/// まま左端固定（x=wa.left=53）へ再射影され Y 保持（Top 以外の辺でも配線が
/// `Anchored.0` を正しく転送していることの補強）。
#[test]
fn anchor_changed_system_reprojects_bottom_to_left() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot()); // 左端 53・下端 1043
    let e = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687), // Bottom 釘付け済み
            Anchored(Anchor::Bottom),
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems(anchor_changed_system);
    schedule.run(&mut world); // 初回 Changed 消費（べき等・位置不変）
    assert_eq!(position_of(&world, e), Point { x: 731, y: 356 });

    world.get_mut::<Anchored>(e).unwrap().0 = Anchor::Left;
    schedule.run(&mut world);
    // Left: x=wa.left=53・Y=356 保持・size 不変
    assert_eq!(
        position_of(&world, e),
        Point { x: 53, y: 356 },
        "x=wa.left=53（左端固定）・Y=356 保持"
    );
    assert_eq!(size_of(&world, e), SizeI::new(434, 687));
}

// -------------------------------------------------------------------------
// resize_window_keep_position（balloon 窓の位置維持リサイズ・
// areka-P0-emo-dpi-scaling task 2.2・R3.1/R4.2・
// design「areka / placement > follow.rs（additive・balloon 窓の k 追従）」・D8）
//
// 「書込ゼロ」の観測境界について: `SetWindowPosCommand` の TLS キューは
// wintf 私有（`WINDOW_POS_COMMANDS`）で件数を覗く公開 API が無く、`flush()` は
// 偽 HWND に対し実 `SetWindowPos` を撃ってしまうため使えない（既存
// enqueue_window_set_pos 群と同じ制約）。代わりに **`Arrangement.offset` 同期**
// を witness に使う——この同期は `enqueue_window_set_pos` 内で enqueue と
// 不可分に対で走るため、「stale な sentinel offset が据え置かれたまま」＝
// 単一ライター経路を一度も通っていない＝enqueue 件数 0 の決定論的証拠になる
// （逆に通れば offset は必ず `WindowPos.position` の `as f32` 転写になる）。
// 寸法・座標は 96 の非倍数を使い、隠れた dpi/96 再スケールの檻とする。
// -------------------------------------------------------------------------

use super::resize_window_keep_position;

/// 単一ライター経路を通ったか否かの witness 用 sentinel（実位置と重ならない値）。
const WRITER_WITNESS: Offset = Offset { x: -1.0, y: -1.0 };

/// 経路を通っていない＝書込ゼロ（sentinel が据え置かれている）。
fn assert_no_write(world: &World, entity: Entity) {
    assert_eq!(
        arrangement_offset_of(world, entity),
        WRITER_WITNESS,
        "単一ライター経路を通った痕跡がある（書込ゼロのはず）"
    );
}

/// べき等 skip（R4.2・D8「同寸なら書込ゼロで振動しない」）: 現寸と同じ寸を
/// 渡すと単一ライター経路を**一度も通らず** `false` を返し、位置・寸法とも不変。
#[test]
fn resize_window_keep_position_same_size_writes_nothing() {
    let mut world = World::new();
    let window = world
        .spawn((
            fake_handle(0x3000),
            window_pos_sized(731, 356, 434, 687),
            arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
        ))
        .id();

    assert!(
        !resize_window_keep_position(&mut world, window, SizePx { w: 434, h: 687 }),
        "同寸はべき等 skip ゆえ false"
    );
    assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
    assert_eq!(size_of(&world, window), SizeI::new(434, 687));
    assert_no_write(&world, window);
}

/// 異寸（R3.1/R4.2）: 位置は**現在位置のまま**・寸法だけが新寸へ更新され `true`。
/// `resize_window_to` と違いアンカー射影 T を再適用しない（balloon は char 窓
/// 追従で位置が決まるため、DPI 追従では寸だけを差し替える）。
#[test]
fn resize_window_keep_position_new_size_keeps_position_and_writes_once() {
    let mut world = World::new();
    let window = world
        .spawn((
            fake_handle(0x3000),
            window_pos_sized(731, 356, 434, 687),
            arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
        ))
        .id();

    assert!(resize_window_keep_position(
        &mut world,
        window,
        SizePx { w: 517, h: 823 }
    ));
    assert_eq!(
        position_of(&world, window),
        Point { x: 731, y: 356 },
        "位置は維持される（再射影しない）"
    );
    assert_eq!(size_of(&world, window), SizeI::new(517, 823));
    // 単一ライター経路を通った証拠＝Arrangement.offset が現在位置の as f32 転写
    assert_eq!(
        arrangement_offset_of(&world, window),
        Offset { x: 731.0, y: 356.0 }
    );
}

/// 現寸不明（`WindowPos.size` が `None`＝窓生成直後）はべき等判定が成立しない
/// ため書込へ進む（位置維持・新寸反映）。
#[test]
fn resize_window_keep_position_with_unknown_current_size_writes() {
    let mut world = World::new();
    let window = world
        .spawn((
            fake_handle(0x3000),
            window_pos_at(731, 356),
            arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
        ))
        .id();

    assert!(resize_window_keep_position(
        &mut world,
        window,
        SizePx { w: 517, h: 823 }
    ));
    assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
    assert_eq!(size_of(&world, window), SizeI::new(517, 823));
}

/// `WindowPos` 未付与（窓生成前の異常系）: warn＋`false`＋書込ゼロ
/// （silent no-op にしない）。
#[test]
fn resize_window_keep_position_without_window_pos_returns_false() {
    let mut world = World::new();
    let window = world
        .spawn((
            fake_handle(0x3000),
            arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
        ))
        .id();

    assert!(!resize_window_keep_position(
        &mut world,
        window,
        SizePx { w: 517, h: 823 }
    ));
    assert_no_write(&world, window);
}

/// `WindowPos.position` 不在（窓生成前）: 現在位置を読めないため warn＋`false`＋
/// 書込ゼロ。`size` も書き換えない。
#[test]
fn resize_window_keep_position_without_position_returns_false() {
    let mut world = World::new();
    let window = world
        .spawn((
            fake_handle(0x3000),
            WindowPos {
                position: None,
                size: Some(SizeI::new(434, 687)),
                ..Default::default()
            },
            arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
        ))
        .id();

    assert!(!resize_window_keep_position(
        &mut world,
        window,
        SizePx { w: 517, h: 823 }
    ));
    assert!(
        world
            .get::<WindowPos>(window)
            .expect("WindowPos があるはず")
            .position
            .is_none(),
        "position は復活しない"
    );
    assert_eq!(size_of(&world, window), SizeI::new(434, 687));
    assert_no_write(&world, window);
}

/// 非正寸（0・負）: warn＋`false`＋書込ゼロ（`resize_window_to` の非正寸縮退と
/// 同一流儀・`wa.right−w` 系の暴走を先に弾く）。
#[test]
fn resize_window_keep_position_nonpositive_size_holds_state() {
    for bad in [
        SizePx { w: 0, h: 687 },
        SizePx { w: 434, h: 0 },
        SizePx { w: 0, h: 0 },
        SizePx { w: -517, h: 823 },
        SizePx { w: 517, h: -823 },
    ] {
        let mut world = World::new();
        let window = world
            .spawn((
                fake_handle(0x3000),
                window_pos_sized(731, 356, 434, 687),
                arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
            ))
            .id();

        assert!(
            !resize_window_keep_position(&mut world, window, bad),
            "非正寸 {bad:?} は false"
        );
        assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
        assert_eq!(size_of(&world, window), SizeI::new(434, 687));
        assert_no_write(&world, window);
    }
}

/// `WindowHandle` 未付与（窓生成前）: 判定を二重化せず `enqueue_window_set_pos`
/// の既存 warn 経路へ委譲し `false`＋状態不変（単一ライター規律の継承）。
#[test]
fn resize_window_keep_position_without_handle_returns_false() {
    let mut world = World::new();
    let window = world
        .spawn((
            window_pos_sized(731, 356, 434, 687),
            arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
        ))
        .id();

    assert!(!resize_window_keep_position(
        &mut world,
        window,
        SizePx { w: 517, h: 823 }
    ));
    assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
    assert_eq!(size_of(&world, window), SizeI::new(434, 687));
    assert_no_write(&world, window);
}

/// despawn 済み（対象不在）でも panic せず `false`。
#[test]
fn resize_window_keep_position_on_despawned_entity_returns_false() {
    let mut world = World::new();
    let window = world
        .spawn((fake_handle(0x3000), window_pos_sized(731, 356, 434, 687)))
        .id();
    world.despawn(window);

    assert!(!resize_window_keep_position(
        &mut world,
        window,
        SizePx { w: 517, h: 823 }
    ));
}

// -------------------------------------------------------------------------
// task 3.2: 消費側の存在確認と警告水準の区別（Req 6.2/6.3・design D8 消費側・
// design「guard_visibility > Implementation Notes > 消費側の区別」）
//
// 追従層の消費入口（[`resize_window_to`]／[`resize_window_keep_position`]）は
// **2 つの事象を混ぜてはならない**:
//   (a) entity 不在（既に despawn 済み）＝終了処理の正常系 → `debug!` で打ち切り
//   (b) entity は実在するが接地点規約の component（`Anchored`）が欠落＝真の異常 → `warn!`
// (a) を warn のままにすると終了時ログが良性ノイズで埋まり（Req 6.2 違反）、(b) を
// debug へ落とすと本物の結線バグが観測から消える。**同じ檻の中で両方**を見る。
// -------------------------------------------------------------------------

/// Req 6.2/6.3（追従層・キャラ窓入口）: despawn 済み entity への resize は正常終了系
/// として `debug!` 1 行で打ち切られ、**warn 以上を 1 行も出さない**。
#[test]
fn resize_window_to_on_despawned_entity_is_debug_only_normal_termination() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687),
            Anchored(Anchor::Bottom),
        ))
        .id();
    world.despawn(window);

    let (ok, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            window,
            SizePx { w: 517, h: 823 },
            PlacementRoute::Resnap,
        )
    });

    assert!(!ok, "破棄済み窓へは書けない（false・panic しない）");
    // `tracing::Level` の Ord は ERROR < WARN < INFO < DEBUG < TRACE ゆえ
    // 「INFO より verbose」＝ debug/trace のみ、が静穏性の表現になる（spawn.rs T-V1 と同型）。
    assert!(
        events.iter().all(|e| e.level > tracing::Level::INFO),
        "破棄済み窓に対して警告以上のログが出ている（Req 6.2 違反）: {events:?}"
    );
    let skipped = expect_one(&events, DESPAWNED_SKIP_TAG);
    assert_eq!(
        skipped.level,
        tracing::Level::DEBUG,
        "破棄済みの打ち切りは debug 水準（正常終了系）"
    );
}

/// Req 6.2 の裏面（真の異常を殺さない）: **生存している** entity の接地点規約 component
/// （`Anchored`）欠落は従来どおり `warn!`。存在確認の導入でこちらまで静穏化してはならない。
#[test]
fn resize_window_to_missing_anchored_on_living_entity_still_warns() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687),
            // Anchored なし（entity は実在する）
        ))
        .id();

    let (ok, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            window,
            SizePx { w: 517, h: 823 },
            PlacementRoute::Resnap,
        )
    });

    assert!(!ok, "Anchored 欠落は書かない（false）");
    let warned = expect_one(&events, "Anchored 未付与");
    assert_eq!(
        warned.level,
        tracing::Level::WARN,
        "実在 entity の Anchored 欠落は真の異常＝warn のまま（Req 6.2 の区別）"
    );
    assert!(
        !events.iter().any(|e| e.message().contains(DESPAWNED_SKIP_TAG)),
        "実在 entity を『破棄済み』と誤判定している: {events:?}"
    );
}

/// Req 6.2/6.3（追従層・バルーン窓入口）: despawn 済み entity への位置据置きリサイズも
/// 正常終了系（`debug!`）として打ち切られ、warn 以上を出さない。
#[test]
fn resize_window_keep_position_on_despawned_entity_is_debug_only_normal_termination() {
    let mut world = World::new();
    let window = world
        .spawn((fake_handle(0x3000), window_pos_sized(731, 356, 434, 687)))
        .id();
    world.despawn(window);

    let (ok, events) =
        capture_logs(|| resize_window_keep_position(&mut world, window, SizePx { w: 517, h: 823 }));

    assert!(!ok, "破棄済み窓へは書けない（false・panic しない）");
    assert!(
        events.iter().all(|e| e.level > tracing::Level::INFO),
        "破棄済み窓に対して警告以上のログが出ている（Req 6.2 違反）: {events:?}"
    );
    let skipped = expect_one(&events, DESPAWNED_SKIP_TAG);
    assert_eq!(skipped.level, tracing::Level::DEBUG);
}

/// Req 6.2 の裏面（バルーン窓入口）: **生存している** entity の `WindowPos` 欠落
/// （窓生成前の異常系）は従来どおり `warn!`。
#[test]
fn resize_window_keep_position_missing_window_pos_on_living_entity_still_warns() {
    let mut world = World::new();
    let window = world.spawn(fake_handle(0x3000)).id(); // WindowPos なし・entity は実在

    let (ok, events) =
        capture_logs(|| resize_window_keep_position(&mut world, window, SizePx { w: 517, h: 823 }));

    assert!(!ok);
    let warned = expect_one(&events, "WindowPos 未付与");
    assert_eq!(
        warned.level,
        tracing::Level::WARN,
        "実在 entity の WindowPos 欠落は真の異常＝warn のまま"
    );
}

// -------------------------------------------------------------------------
// 窓移動レコード（Req 1.2／2.4・task 1.4・design「placement::diag > Invariants」
// ＋「PlacementRoute 配管＋guard_visibility > Integration」・D11）
//
// 単一ライター `enqueue_window_set_pos` の**書込成功時**に 1 レコードを専用 target
// （`areka::placement::diag`）へ出す。檻の要点:
//   (1) 経路名が呼出点と 1:1（route を取り違えたら赤）
//   (2) route・entity・種別・scope・位置・寸・DPI の**全フィールド**が揃う
//       （entity は wintf 側ログとの結合キーゆえ必ず入る＝Req 1.9 の 2 段 grep 条件）
//   (3) 書込が起きない経路（べき等 skip・`WindowHandle` 未付与）ではレコードが出ない
//   (4) 既定 `RUST_LOG=info` では 1 行も出ない（Req 1.7）
//
// 観測境界は tracing イベント本体（`test_support::capture_logs`）——本レコードは
// `WindowPos` ミラーと違い「書込が起きた事実」そのものの証跡だからである。
// 座標・寸・DPI は 96 の非倍数／非既定値を使い、取り違えを差で炙り出す。
// -------------------------------------------------------------------------

use std::sync::{Arc, Mutex};

use tracing_subscriber::EnvFilter;
use wintf::ecs::DPI;

use super::super::diag::{DESPAWNED_SKIP_TAG, WINDOW_MOVE_RECORD_TAG};
use super::super::spawn::{BalloonWindowMarker, CharWindowMarker};
use super::super::test_support::{LogEvent, capture_logs, ensure_interest_probes, expect_one};

/// 捕捉イベントから窓移動レコード行だけを抜く（他の debug ログは無視）。
fn window_move_lines(events: &[LogEvent]) -> Vec<String> {
    events
        .iter()
        .map(|e| e.message().to_string())
        .filter(|m| m.starts_with(WINDOW_MOVE_RECORD_TAG))
        .collect()
}

/// ちょうど 1 行の窓移動レコードを取り出す（0 件・複数件は落とす）。
fn only_window_move_line(events: &[LogEvent]) -> String {
    let lines = window_move_lines(events);
    assert_eq!(
        lines.len(),
        1,
        "窓移動レコードがちょうど 1 行ではない: {lines:?} / all={events:?}"
    );
    lines.into_iter().next().expect("1 件あることは検査済み")
}

/// 釘付け済みキャラ窓（marker/DPI 付き）1 枚だけの World。
///
/// `DPI` は **`WindowHandle` 付与の後**に入れる——wintf の `WindowHandle` on_add フックが
/// `GetDpiForWindow` を引き（偽 HWND では失敗＝96）`DPI` を上書きするため、同一 spawn の
/// タプルへ混ぜると意図した DPI が 96 に潰れる（混在 DPI の檻が自己整合で無力化する罠）。
fn char_window_world(scope: usize, dpi: u16) -> (World, Entity) {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot()); // 下端 1043
    let e = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687),
            Anchored(Anchor::Bottom),
            CharWindowMarker { scope },
        ))
        .id();
    world.entity_mut(e).insert(DPI::from_dpi(dpi, dpi));
    (world, e)
}

/// (2) 全フィールドの檻: 書込成功で**ちょうど 1 行**、route・entity・kind・scope・
/// 物理位置・物理寸・DPI が揃う（1 つでも落ちたら赤）。
#[test]
fn window_move_record_carries_route_entity_kind_scope_position_size_and_dpi() {
    let (mut world, e) = char_window_world(1, 192);

    let (ok, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            e,
            SizePx { w: 517, h: 823 },
            PlacementRoute::DpiReproject,
        )
    });
    assert!(ok, "前提: 書込は成立する");

    // 期待値は resize_window_to の既存檻と同一の導出（下端中央保持 x=690・Y=1043−823）。
    assert_eq!(
        only_window_move_line(&events),
        format!(
            "[diag.window_move] route=DpiReproject entity={e:?} kind=char scope=1 \
             x=690 y=220 w=517 h=823 dpi=192"
        )
    );
}

/// (2) 結合キーの檻: entity は wintf 側ログ（`entity = ?e`＝`Debug` 表現・scope を
/// 持たない）と同一表現で出る——Req 1.9 の scope 別計数（2 段 grep）の成立条件。
#[test]
fn window_move_record_entity_matches_wintf_debug_rendering() {
    let (mut world, e) = char_window_world(0, 120);

    let (_, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            e,
            SizePx { w: 517, h: 823 },
            PlacementRoute::Resnap,
        )
    });
    let line = only_window_move_line(&events);
    assert!(
        line.contains(&format!("entity={e:?}")),
        "wintf 側ログと結合できる Debug 表現になっていない: {line}"
    );
    assert!(line.contains("scope=0") && line.contains("kind=char"));
}

/// (1) 経路名は**呼出側が渡した route と 1:1**（`resize_window_to` は 3 経路の共通
/// 反映口ゆえ、ここを取り違えると書き手の名指し＝Req 2.4 が丸ごと嘘になる）。
#[test]
fn window_move_record_route_follows_the_argument_of_the_shared_resize_entry() {
    for route in [
        PlacementRoute::AnchorChange,
        PlacementRoute::Resnap,
        PlacementRoute::DpiReproject,
    ] {
        let (mut world, e) = char_window_world(0, 96);
        let (ok, events) =
            capture_logs(|| resize_window_to(&mut world, e, SizePx { w: 517, h: 823 }, route));
        assert!(ok);
        let line = only_window_move_line(&events);
        assert!(
            line.contains(&format!("route={}", route.as_str())),
            "route={route} を渡したのにレコードが一致しない: {line}"
        );
        // 他 8 経路の語が混ざらない（取り違えの檻）。
        for other in PlacementRoute::ALL {
            if other == route {
                continue;
            }
            assert!(
                !line.contains(&format!("route={}", other.as_str())),
                "route={other} が混入: {line}"
            );
        }
    }
}

/// (1) 呼出点割当の檻: アンカー変化トリガ（`anchor_changed_system`）は
/// `AnchorChange` を渡す（system 側の割当ミスを検出する）。
#[test]
fn anchor_changed_system_records_the_anchor_change_route() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot()); // rect(53, 37, 1877, 1043)
    let e = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687),
            Anchored(Anchor::Bottom),
            CharWindowMarker { scope: 1 },
        ))
        .id();
    world.entity_mut(e).insert(DPI::from_dpi(120, 120)); // on_add フックの後に入れる
    let mut schedule = Schedule::default();
    schedule.add_systems(anchor_changed_system);
    // 初回 run はべき等 skip（＝レコードも出ない＝(3) の裏取りも兼ねる）。
    let (_, first) = capture_logs(|| schedule.run(&mut world));
    assert!(
        window_move_lines(&first).is_empty(),
        "べき等 skip でレコードが出た: {first:?}"
    );

    world.get_mut::<Anchored>(e).unwrap().0 = Anchor::Top;
    let (_, second) = capture_logs(|| schedule.run(&mut world));
    let line = only_window_move_line(&second);
    assert!(
        line.contains("route=AnchorChange"),
        "アンカー変化の書込が AnchorChange として記録されない: {line}"
    );
    assert!(line.contains("y=37") && line.contains("dpi=120"), "{line}");
}

/// (1) 呼出点割当の檻: バルーン窓の位置据置きリサイズは `KeepPositionResize`。
/// 種別・scope はバルーン marker から読む（キャラと取り違えない）。
#[test]
fn resize_window_keep_position_records_the_keep_position_route() {
    let mut world = World::new();
    let window = world
        .spawn((
            fake_handle(0x3000),
            window_pos_sized(731, 356, 434, 687),
            BalloonWindowMarker { scope: 1 },
        ))
        .id();
    world.entity_mut(window).insert(DPI::from_dpi(192, 192)); // on_add フックの後に入れる

    let (ok, events) = capture_logs(|| {
        resize_window_keep_position(&mut world, window, SizePx { w: 517, h: 823 })
    });
    assert!(ok);
    assert_eq!(
        only_window_move_line(&events),
        format!(
            "[diag.window_move] route=KeepPositionResize entity={window:?} kind=balloon \
             scope=1 x=731 y=356 w=517 h=823 dpi=192"
        )
    );
}

/// (1)(2) `\![move]` cue（[`move_window_to`]）は**対象窓を `MoveCue`**・**随伴バルーンを
/// `BalloonFollow`** として記録する（D13: スクリプト明示移動は固有の経路語を持つ＝Q3
/// 「ドラッグ以外の経路での消失」の観測穴を塞ぐ）。移動専用ゆえ寸は番兵（`w=-`／`h=-`）で
/// 欠落させない（フィールド語彙は経路によらず不変）。
#[test]
fn move_cue_write_is_recorded_as_move_cue_with_a_balloon_follow_companion() {
    let mut world = World::new();
    let balloon = world
        .spawn((
            fake_handle(0x2000),
            window_pos_at(180, 383),
            BalloonWindowMarker { scope: 0 },
        ))
        .id();
    // `DPI` 未付与の窓（component 欠落の防御経路）を作る——`WindowHandle` on_add フックが
    // 常に `DPI` を挿すため、番兵 `dpi=-` を単一ライター越しに固定するには外す必要がある。
    world.entity_mut(balloon).remove::<DPI>();
    let char_window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687),
            CharWindowMarker { scope: 0 },
            BalloonFollow {
                balloon,
                offset: PointPx { x: -551, y: 27 },
            },
        ))
        .id();
    // 96 非倍数の DPI を明示付与（on_add フックの後に入れる＝96 へ潰されない）。
    world
        .entity_mut(char_window)
        .insert(DPI::from_dpi(120, 120));

    let (ok, events) = capture_logs(|| move_window_to(&mut world, char_window, 999, 777));
    assert!(ok);
    // 対象窓＝MoveCue／随伴バルーン＝BalloonFollow の 2 行（発行順＝書込順）。
    assert_eq!(
        window_move_lines(&events),
        vec![
            format!(
                "[diag.window_move] route=MoveCue entity={char_window:?} kind=char scope=0 \
                 x=999 y=777 w=- h=- dpi=120"
            ),
            format!(
                "[diag.window_move] route=BalloonFollow entity={balloon:?} kind=balloon scope=0 \
                 x=448 y=804 w=- h=- dpi=-"
            ),
        ]
    );
    // 位置自体は従来どおり両方書かれている（挙動不変の裏取り）。
    assert_eq!(position_of(&world, char_window), Point { x: 999, y: 777 });
    assert_eq!(position_of(&world, balloon), Point { x: 448, y: 804 });
}

/// (1) ドラッグ経路（連続イベント）はキャラ窓の書込を記録しない一方、随伴バルーンは
/// `BalloonFollow` として記録される（Req 2.5「バルーン消失は追従の随伴か」の判別材料）。
#[test]
fn drag_path_records_only_the_balloon_follow_write() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot()); // 下端 1043
    let balloon = world
        .spawn((
            fake_handle(0x2000),
            window_pos_at(180, 383),
            BalloonWindowMarker { scope: 0 },
        ))
        .id();
    let char_window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(1207, 356, 434, 687),
            Anchored(Anchor::Bottom),
            CharWindowMarker { scope: 0 },
            BalloonFollow {
                balloon,
                offset: PointPx { x: -551, y: 27 },
            },
            dragging_state((1207, 356), (1300, 500)),
        ))
        .id();

    let ev = Phase::Bubble(drag_event_at(char_window, (1300, 500), (1450, 520)));
    let (_, events) = capture_logs(|| on_char_drag(&mut world, char_window, char_window, &ev));

    let lines = window_move_lines(&events);
    assert_eq!(
        lines.len(),
        1,
        "ドラッグ 1 イベントの記録は随伴 1 行: {lines:?}"
    );
    assert!(
        lines[0].contains("route=BalloonFollow")
            && lines[0].contains(&format!("entity={balloon:?}")),
        "{lines:?}"
    );
    assert!(
        !lines[0].contains(&format!("entity={char_window:?}")),
        "ドラッグ経路のキャラ窓書込は本 target を通らない（wintf `[drag]` の所有）: {lines:?}"
    );
}

/// (3) 書込が起きなければレコードも出ない: べき等 skip（同寸・同位置）と
/// `WindowHandle` 未付与（失敗）の双方で 0 行。
#[test]
fn no_window_move_record_when_nothing_is_written() {
    // べき等 skip（Req3.1）
    let (mut world, e) = char_window_world(0, 120);
    let (wrote, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            e,
            SizePx { w: 434, h: 687 },
            PlacementRoute::Resnap,
        )
    });
    assert!(!wrote, "前提: 同寸・同位置はべき等 skip");
    assert!(
        window_move_lines(&events).is_empty(),
        "書込ゼロなのにレコードが出た: {events:?}"
    );

    // WindowHandle 未付与（Req3.3・enqueue が warn＋false）
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let no_handle = world
        .spawn((
            window_pos_sized(731, 356, 434, 687),
            Anchored(Anchor::Bottom),
            CharWindowMarker { scope: 0 },
        ))
        .id();
    let (wrote, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            no_handle,
            SizePx { w: 517, h: 823 },
            PlacementRoute::Resnap,
        )
    });
    assert!(!wrote);
    assert!(
        window_move_lines(&events).is_empty(),
        "失敗経路でレコードが出た: {events:?}"
    );
}

/// 与えた `RUST_LOG` 相当 directive で実際に濾した出力を集める（diag.rs の
/// `emit_all_under_filter` と同型——こちらは**単一ライター経由**で点灯を確かめる）。
fn window_move_output_under_filter(directives: &str) -> String {
    ensure_interest_probes();

    #[derive(Clone)]
    struct VecWriter(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for VecWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .expect("捕捉バッファの毒化なし")
                .extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let sink = buf.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(directives))
        .with_ansi(false)
        .with_writer(move || VecWriter(sink.clone()))
        .finish();

    let (mut world, e) = char_window_world(1, 192);
    tracing::subscriber::with_default(subscriber, || {
        tracing::callsite::rebuild_interest_cache();
        assert!(resize_window_to(
            &mut world,
            e,
            SizePx { w: 517, h: 823 },
            PlacementRoute::DpiReproject
        ));
    });

    String::from_utf8(buf.lock().expect("捕捉バッファの毒化なし").clone()).expect("UTF-8")
}

/// (4) 既定 `RUST_LOG=info`（`main.rs` のフォールバック）では窓移動レコードが
/// **1 行も出ない**（Req 1.7・恒久計装の既定 OFF）。
#[test]
fn window_move_records_are_silent_under_default_info_filter() {
    let out = window_move_output_under_filter("info");
    assert!(
        !out.contains(WINDOW_MOVE_RECORD_TAG),
        "既定 RUST_LOG=info で窓移動レコードが漏れている（Req 1.7 違反）: {out}"
    );
}

/// (4) 手順書の directive（`areka::placement::diag=debug`）で点灯する
/// ＝単一ライター経由でも target が手順書と 1:1 で結ばれている（Req 1.5/1.7）。
#[test]
fn window_move_records_light_up_under_the_procedure_directive() {
    let out = window_move_output_under_filter("info,areka::placement::diag=debug");
    assert!(
        out.contains(WINDOW_MOVE_RECORD_TAG) && out.contains("route=DpiReproject"),
        "手順書の RUST_LOG で単一ライターのレコードが点灯しない: {out}"
    );
}

// -------------------------------------------------------------------------
// 遷移ガードの**配線**（task 6.1・S3 是正・Req 3.1/3.2/3.3・D5/D6/D13）
//
// task 2.2 は `guard_visibility`／`work_area_for_window_with_origin` を純関数として
// 用意したが**本番呼出はゼロ**だった（diagnosis-report.md §1.3「純関数が在ることは
// S3 の充足ではない」）。本節が檻に入れるのは純関数の判定規則ではなく、
// **`resize_window_to` の中でそれが実際に走るか・どの route で走るか**である。
//
// 檻の要点（空虚化を避けるための自己検査を各檻が持つ）:
//   (1) 探針の自己検査——ガード**無し**の提案が本当に全 work area 非交差であること
//       （交差する探針では ClampX 腕へ一度も入らず「緑」が何も意味しない・[[2.2 の教訓]]）
//   (2) 位置の不変条件——clamp 後の矩形がいずれかの work area と交差する（Req 3.1）
//   (3) route による発火条件——適用外 route（`MoveCue`／`Restore` 等）とドラッグ経路
//       では**位置が素の射影と 1 bit も違わない**こと。ログ側の否定 assert だけに
//       依存しない（[[5.2 の教訓＝空虚性 6 例目]]: 不変量がログ側にしか無いと
//       別ファイルの水準変更で守りが消える）
//   (4) 判定語のリテラル——手順書 §3.3 の grep 語を檻側にも literal で持つ
//       （[[5.1 → 7.2 の申し送り]]「判定語に使っているのに檻が無い」型の再発防止）
//
// 座標はすべて論理値 × DPI（96/120/192）で構築し、絶対 px の固定値を持たない（Req 5.6）。
// -------------------------------------------------------------------------

use super::route_applies_visibility_guard;

/// 手順書 §3.3 の grep 判定語（**本体の定数とは独立にここへ literal で置く**）。
const CLAMP_TAG: &str = "[visibility-guard] ClampX";
/// 同上（最近傍フォールバックの非ドラッグ経路 warn 昇格・Req 3.2）。
const NEAREST_TAG: &str = "[visibility-guard] NearestFallback";
/// 同上（work area を解決できず判定不能・Req 3.3）。
const UNRESOLVED_TAG: &str = "[visibility-guard] WorkAreaUnresolved";
/// 3 語に共通の接頭辞（「ガードが何かを言った」ことの一括検出）。
const GUARD_TAG_PREFIX: &str = "[visibility-guard]";

/// 幅広のキャラ窓寸（論理 320×400）。論理 320／32 はいずれも 8 の倍数ゆえ、
/// 96/120/192 のどの水準でも物理 px が偶数＝手順 3b の `w/2` が切り捨てで狂わない。
fn wide_char_size(dpi: i32) -> SizePx {
    SizePx {
        w: px(320, dpi),
        h: px(400, dpi),
    }
}

/// 「どの work area にも属さない帯」（`0 ..= px(64)`）より**狭い**新寸。
fn narrow_char_size(dpi: i32) -> SizePx {
    SizePx {
        w: px(32, dpi),
        h: px(400, dpi),
    }
}

/// 帯の中で**右モニタが一意に最近傍になる**中心 x（帯の中点 `px(32)` は左右等距離で
/// 先勝ちに依存するため使わない）。
fn gap_center_x(dpi: i32) -> i32 {
    px(40, dpi)
}

/// 「旧矩形は可視・新提案は全 work area 非交差」へ落ちるキャラ窓 World を組む。
///
/// 旧寸 [`wide_char_size`] の窓を、下端中央付替え（`resize_window_to` 手順 3b）後の
/// 中心が帯へ落ちる位置に置く。新寸 [`narrow_char_size`] は帯より狭いので、射影 T が
/// 出す提案矩形は帯へ収まり **どの work area とも交差しない**——S3 が言う
/// 「非ドラッグ要因で不可視へ遷移する」状態そのものを合成する。
fn gap_bound_char_world(dpi: i32) -> (World, Entity, PointPx) {
    let old = wide_char_size(dpi);
    let old_pos = PointPx {
        x: gap_center_x(dpi) - old.w / 2,
        y: left_wa().bottom - old.h,
    };
    let mut world = World::new();
    world.insert_resource(mixed_layout(dpi));
    let e = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(old_pos.x, old_pos.y, old.w, old.h),
            Anchored(Anchor::Bottom),
        ))
        .id();
    (world, e, old_pos)
}

/// ガードを通さない**素の**射影結果（＝本タスク以前の挙動）。手順 3b と
/// [`project_anchor`] を檻側で独立に再現し、本体の実装を呼び直さない。
fn unguarded_projection(dpi: i32, old_pos: PointPx, new: SizePx) -> PointPx {
    let old = wide_char_size(dpi);
    let raw = PointPx {
        x: old_pos.x + old.w / 2 - new.w / 2,
        y: old_pos.y,
    };
    project_anchor(Anchor::Bottom, raw, new, Some(&mixed_layout(dpi)))
}

/// 窓矩形がいずれかの work area と交差するか（檻側の独立実装 [`overlaps`] で判定）。
fn visible_in(layout: &MonitorSnapshot, pos: PointPx, size: SizePx) -> bool {
    layout
        .work_areas
        .iter()
        .any(|wa| overlaps(win(pos, size), *wa))
}

/// 現在位置を [`PointPx`] で読む（檻の比較単位を射影の単位へ揃える）。
fn point_of(world: &World, entity: Entity) -> PointPx {
    let p = position_of(world, entity);
    PointPx { x: p.x, y: p.y }
}

/// `[visibility-guard]` を名乗るイベントだけを抜く。
fn guard_events<'a>(events: &'a [LogEvent], needle: &str) -> Vec<&'a LogEvent> {
    events
        .iter()
        .filter(|e| e.message().contains(needle))
        .collect()
}

/// 発火条件の**表そのもの**を固定する（D13 帰結⑴⑵）。挙動側の檻（下 2 件）と
/// 二段構えにしてあるのは、語彙が 9 種あるのに `resize_window_to` を実際に通るのは
/// 現状 4 種だけで、残り 5 種の判定が挙動檻だけでは**合成でしか**検査できないため。
/// [`PlacementRoute::ALL`] を回すので、語彙が増えたら本檻も落ちる。
#[test]
fn visibility_guard_route_table_matches_the_d13_decision() {
    for route in PlacementRoute::ALL {
        let expected = matches!(
            route,
            PlacementRoute::AnchorChange
                | PlacementRoute::Resnap
                | PlacementRoute::DpiReproject
                | PlacementRoute::ReportedSizeReconcile
        );
        assert_eq!(
            route_applies_visibility_guard(route),
            expected,
            "route={route} の発火判定が D13 帰結⑴⑵ と食い違う"
        );
    }
    // 表が「全部真」「全部偽」へ潰れていないこと（自明な述語への退化の検出）。
    let fired = PlacementRoute::ALL
        .into_iter()
        .filter(|r| route_applies_visibility_guard(*r))
        .count();
    assert_eq!(fired, 4, "発火 route が 4 種でない（表が潰れている）");
}

/// **Req 3.1 の本体**: 非ドラッグの配置系 4 経路（D13 帰結⑴）では、全 work area
/// 非交差への遷移が X の clamp で阻止され、`warn!` が 1 行残る。
///
/// Y は射影 T の所有ゆえ 1 bit も動かない（[`guard_visibility`] の事後条件）。
#[test]
fn visibility_guard_clamps_x_on_non_drag_placement_routes() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let new = narrow_char_size(dpi);
        for route in [
            PlacementRoute::AnchorChange,
            PlacementRoute::Resnap,
            PlacementRoute::DpiReproject,
            PlacementRoute::ReportedSizeReconcile,
        ] {
            let (mut world, e, old_pos) = gap_bound_char_world(dpi);

            // (1) 探針の自己検査: 素の射影は**本当に**不可視へ落ちる／旧矩形は可視。
            //     どちらかが崩れると ClampX 腕に入らず、この檻は空虚になる。
            let bare = unguarded_projection(dpi, old_pos, new);
            assert!(
                !visible_in(&layout, bare, new),
                "dpi={dpi}: 探針が不動点——ガード無しの提案 {bare:?} が既に可視で ClampX 腕へ入らない"
            );
            assert!(
                visible_in(&layout, old_pos, wide_char_size(dpi)),
                "dpi={dpi}: 旧矩形が非交差では『遷移』でなく留置＝Keep が正解になってしまう"
            );

            let (ok, events) = capture_logs(|| resize_window_to(&mut world, e, new, route));
            assert!(ok, "dpi={dpi} route={route}: 書込は成立する前提");

            // (2) 位置の不変条件（Req 3.1）: 書かれた矩形はどこかの work area と交差する。
            let pos = point_of(&world, e);
            assert!(
                visible_in(&layout, pos, new),
                "dpi={dpi} route={route}: Req 3.1 違反——{pos:?} は全 work area と非交差"
            );
            assert_eq!(
                pos.y, bare.y,
                "dpi={dpi} route={route}: Y は射影 T の所有＝ガードが触ってはならない"
            );
            assert_ne!(
                pos.x, bare.x,
                "dpi={dpi} route={route}: X が引き戻されていない（ガード未発火）"
            );
            // clamp 先は射影が Y に用いた work area（右モニタ）の水平範囲内。
            let wa = right_wa(dpi);
            assert!(
                wa.left <= pos.x && pos.x <= wa.right - new.w,
                "dpi={dpi} route={route}: clamp 先が射影の work area {wa:?} の外: {pos:?}"
            );

            // (4) 判定語: ClampX の warn が 1 行・水準は WARN（Req 3.1/3.2 の観測）。
            let clamped = expect_one(&events, CLAMP_TAG);
            assert_eq!(
                clamped.level,
                tracing::Level::WARN,
                "dpi={dpi} route={route}: clamp の記録が warn 水準でない"
            );
        }
    }
}

/// **Req 3.1 の裏面（D13 帰結⑵）**: 明示操作系・非配置系の route では、位置が素の
/// 射影と 1 bit も違わず、ガードのログも 1 行も出ない。
///
/// `MoveCue`（`\![move]`）と `Restore`（位置復元）を引き戻すのは、スクリプト／
/// 永続化が決めた位置の否定であり本 spec の Out of scope である。**ここが緑のまま
/// 「常に発火」へ変異させられると S3 是正が明示操作の尊重を壊す**ため、位置側の
/// assert（ログではなく挙動）を第一の守りに置く。
#[test]
fn visibility_guard_does_not_fire_on_explicit_or_non_placement_routes() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let new = narrow_char_size(dpi);
        for route in [
            PlacementRoute::SpawnInitial,
            PlacementRoute::Restore,
            PlacementRoute::KeepPositionResize,
            PlacementRoute::BalloonFollow,
            PlacementRoute::MoveCue,
        ] {
            let (mut world, e, old_pos) = gap_bound_char_world(dpi);
            let bare = unguarded_projection(dpi, old_pos, new);
            // 探針の自己検査: ガードが**発火する条件は揃っている**（route だけが違う）。
            assert!(
                !visible_in(&layout, bare, new),
                "dpi={dpi}: 探針が不動点——発火条件が揃っていない"
            );

            let (ok, events) = capture_logs(|| resize_window_to(&mut world, e, new, route));
            assert!(ok, "dpi={dpi} route={route}: 書込は成立する前提");

            assert_eq!(
                point_of(&world, e),
                bare,
                "dpi={dpi} route={route}: 適用外 route で位置が動いた（明示操作の尊重が壊れている）"
            );
            assert!(
                guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
                "dpi={dpi} route={route}: 適用外 route でガードが喋っている: {events:?}"
            );
        }
    }
}

/// **ドラッグ経路は従来の水準のまま**（Req 3.3 の水準分岐・D5）: ユーザーが自分で
/// 帯へ運んだ窓は引き戻されず、毎イベント発火する経路に `warn!` を増やさない。
#[test]
fn drag_path_neither_clamps_nor_warns_when_leaving_every_work_area() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let size = narrow_char_size(dpi);
        // 開始位置は右モニタ上（可視）・接地済み。
        let start_pos = PointPx {
            x: px(200, dpi),
            y: right_wa(dpi).bottom - size.h,
        };
        assert!(
            visible_in(&layout, start_pos, size),
            "dpi={dpi}: 前提——ドラッグ開始位置は可視"
        );

        let mut world = World::new();
        world.insert_resource(mixed_layout(dpi));
        let cursor = (px(800, dpi), px(400, dpi));
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(start_pos.x, start_pos.y, size.w, size.h),
                Anchored(Anchor::Bottom),
                dragging_state((start_pos.x, start_pos.y), cursor),
            ))
            .id();

        // カーソルを帯へ運ぶ: 生ドラッグ x = px(24) ＝ 帯の内側。
        let moved = (cursor.0 - (px(200, dpi) - px(24, dpi)), cursor.1);
        let ev = Phase::Bubble(drag_event_at(window, cursor, moved));
        let (consumed, events) = capture_logs(|| on_char_drag(&mut world, window, window, &ev));
        assert!(!consumed);

        let pos = point_of(&world, window);
        // 自己検査: ドラッグは**実際に**窓を全 work area の外へ運んだ（＝ガードが
        // 配線されていれば必ず clamp する状況である）。
        assert!(
            !visible_in(&layout, pos, size),
            "dpi={dpi}: 探針が不動点——ドラッグ先が可視のままでは『引き戻さない』を検査していない"
        );
        assert_eq!(
            pos.x,
            px(24, dpi),
            "dpi={dpi}: ドラッグの X は素通し（明示操作の尊重）"
        );
        assert!(
            guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
            "dpi={dpi}: ドラッグ経路でガードが喋っている（spam・水準分岐の破壊）: {events:?}"
        );
    }
}

/// **Req 3.2**: 最近傍フォールバック（窓中心がどのモニタにも属さない＝モニタ構成
/// 情報と実画面の食い違いの兆候）は、非ドラッグ経路で `warn!` へ昇格する。
///
/// この探針は **clamp を伴わない**（提案矩形は work area と交差したまま）——
/// `NearestFallback` の観測が `ClampX` の副産物ではなく独立に成立することを示す。
#[test]
fn nearest_fallback_warns_on_non_drag_route_even_without_clamping() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let old = wide_char_size(dpi);
        // 幅は据置き・高さだけ変える＝手順 3b で x は動かず、中心は帯に留まる。
        let new = SizePx {
            w: old.w,
            h: px(200, dpi),
        };
        let (mut world, e, old_pos) = gap_bound_char_world(dpi);

        // 探針の自己検査: **決めた位置**の work area 解決が本当に最近傍へ落ちる
        // （`Contains` なら昇格の腕へ入らず空虚になる）。かつ提案矩形は交差したまま
        // ＝clamp しない（`NearestFallback` が `ClampX` の副産物でないことの担保）。
        let bare = unguarded_projection(dpi, old_pos, new);
        let (_, resolution) = work_area_for_window_with_origin(&layout, win(bare, new))
            .expect("合成レイアウトは空でない");
        assert_eq!(
            resolution,
            WorkAreaResolution::NearestFallback,
            "dpi={dpi}: 探針が `Contains` に落ちている＝昇格の腕を検査していない"
        );
        assert!(
            visible_in(&layout, bare, new),
            "dpi={dpi}: 探針が clamp を伴っている＝`NearestFallback` 単独の檻になっていない"
        );

        let (ok, events) =
            capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::Resnap));
        assert!(ok);
        assert_eq!(
            point_of(&world, e),
            bare,
            "dpi={dpi}: Keep 腕で位置が動いた"
        );

        let warned = expect_one(&events, NEAREST_TAG);
        assert_eq!(
            warned.level,
            tracing::Level::WARN,
            "dpi={dpi}: 最近傍フォールバックが非ドラッグ経路で warn へ昇格していない"
        );
        assert!(
            guard_events(&events, CLAMP_TAG).is_empty(),
            "dpi={dpi}: clamp していないのに ClampX が出ている: {events:?}"
        );
    }
}

/// **Req 3.3**: 位置決定に必要な入力（モニタ work area）が取得できない場合は、
/// 位置を変更せず現状のまま `warn!` を残す（架空の可視領域を発明しない）。
///
/// `MonitorSnapshot` 不在／空 snapshot のいずれでも、射影 T は identity へ縮退
/// 済みである＝ガードが位置へ手を入れないことが「現状維持」の内容になる。
#[test]
fn missing_work_area_holds_position_and_warns_on_non_drag_route() {
    for dpi in DPIS {
        for (label, snapshot) in [
            ("resource 不在", None),
            ("空 snapshot", Some(MonitorSnapshot { work_areas: vec![] })),
        ] {
            let new = narrow_char_size(dpi);
            let (mut world, e, old_pos) = gap_bound_char_world(dpi);
            world.remove_resource::<MonitorSnapshot>();
            if let Some(s) = snapshot {
                world.insert_resource(s);
            }
            // work area が無いときの射影は identity ＝ 手順 3b 後の raw そのもの。
            let old = wide_char_size(dpi);
            let identity = PointPx {
                x: old_pos.x + old.w / 2 - new.w / 2,
                y: old_pos.y,
            };

            let (ok, events) =
                capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::Resnap));
            assert!(ok, "dpi={dpi} {label}: 寸の反映自体は従来どおり成立する");
            assert_eq!(
                point_of(&world, e),
                identity,
                "dpi={dpi} {label}: ガードが位置を動かした（現状維持の違反）"
            );

            let warned = expect_one(&events, UNRESOLVED_TAG);
            assert_eq!(
                warned.level,
                tracing::Level::WARN,
                "dpi={dpi} {label}: 入力欠落が warn として残っていない（Req 3.3）"
            );
            assert!(
                guard_events(&events, CLAMP_TAG).is_empty(),
                "dpi={dpi} {label}: work area 不明なのに clamp している: {events:?}"
            );
        }
    }
}

/// 適用外 route では、work area 不明であってもガードは 1 行も喋らない
/// （警告の出所が route 条件の**内側**にあることの檻）。
#[test]
fn missing_work_area_stays_silent_on_guard_exempt_routes() {
    for dpi in DPIS {
        let (mut world, e, _) = gap_bound_char_world(dpi);
        world.remove_resource::<MonitorSnapshot>();
        let (_, events) = capture_logs(|| {
            resize_window_to(
                &mut world,
                e,
                narrow_char_size(dpi),
                PlacementRoute::MoveCue,
            )
        });
        assert!(
            guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
            "dpi={dpi}: 適用外 route でガードが喋っている: {events:?}"
        );
    }
}

/// **旧矩形『不明』は `Option::None` だけではない**（[[4.6 の教訓]]）: wintf の
/// [`WindowPos::default`] は寸を `Some(CW_USEDEFAULT)`（＝`i32::MIN` センチネル）で
/// 持つ。これを素の矩形として交差判定へ入れると退化矩形が「もともと画面外に
/// 留置されていた」と誤判定され、**安全側 clamp の腕が丸ごと死ぬ**。
#[test]
fn undetermined_old_size_is_treated_as_unknown_rect_and_clamps() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let new = narrow_char_size(dpi);
        // 手順 3b は旧寸が非正のとき付替えを行わない＝raw は現在位置そのもの。
        let raw = PointPx {
            x: gap_center_x(dpi) - new.w / 2,
            y: left_wa().bottom - new.h,
        };
        let mut world = World::new();
        world.insert_resource(mixed_layout(dpi));
        let e = world
            .spawn((
                fake_handle(0x1000),
                // 寸は `CW_USEDEFAULT` センチネルのまま（窓生成直後の実表現）。
                WindowPos {
                    position: Some(Point { x: raw.x, y: raw.y }),
                    ..Default::default()
                },
                Anchored(Anchor::Bottom),
            ))
            .id();

        // 探針の自己検査: 素の射影は不可視へ落ちる（＝安全側 clamp が要る状況）。
        let bare = project_anchor(Anchor::Bottom, raw, new, Some(&layout));
        assert!(
            !visible_in(&layout, bare, new),
            "dpi={dpi}: 探針が不動点——素の射影が既に可視"
        );

        let (ok, events) =
            capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::Resnap));
        assert!(ok);
        assert!(
            visible_in(&layout, point_of(&world, e), new),
            "dpi={dpi}: 寸未確定（センチネル）を『留置』と誤読して clamp を見送っている"
        );
        expect_one(&events, CLAMP_TAG);
    }
}

// -------------------------------------------------------------------------
// バルーン矩形への遷移ガード配線（task 6.2・S3′ 是正・Req 3.4・D6）
//
// task 2.2 は `guard_visibility` のバルーン矩形ケース（純関数）を、task 6.1 は
// キャラ窓経路の配線を固めた。本節が檻に入れるのは**バルーン随伴で実際に走るか・
// どの引き金で走るか**である（diagnosis-report.md §1.4「純関数が在ることは S3′ の
// 充足ではない」）。
//
// 檻の要点（空虚化を避けるための自己検査を各檻が持つ）:
//   (1) 探針の自己検査——ガード**無し**のバルーン提案が本当に全 work area 非交差で
//       あること／旧バルーン矩形は可視であること（どちらかが崩れると ClampX 腕へ
//       入らず「緑」が何も意味しない・[[2.2 の教訓]]）
//   (2) **キャラ窓は clamp されない**こと——キャラ側のガードが動かした結果を
//       バルーンの成果と読み違えない（S3 と S3′ の分離）
//   (3) 引き金による発火条件——**ドラッグ随伴では位置が素の恒等式と 1 bit も違わない**。
//       ログ側の否定 assert だけに依存しない（[[5.2 の教訓＝空虚性 6 例目]]:
//       不変量がログ側にしか無いと別ファイルの水準変更で守りが消える）
//   (4) 判定語のリテラル——`CLAMP_TAG`／`NEAREST_TAG`／`UNRESOLVED_TAG` を檻側にも持つ
//
// 座標はすべて論理値 × DPI（96/120/192）で構築し、絶対 px の固定値を持たない（Req 5.6）。
// -------------------------------------------------------------------------

use super::BalloonFollowTrigger;

/// キャラ窓の初期位置（**接地していない** Y）。同寸の [`resize_window_to`] でも
/// 射影 T が Y を `wa.bottom − h` へ動かす＝手順 4 のべき等 skip に落ちない。
fn char_start_pos(dpi: i32) -> PointPx {
    point(px(1500, dpi), px(100, dpi))
}

/// 射影 T 適用後のキャラ窓確定位置（右モニタへ接地・**可視のまま**）。
fn char_settled_pos(dpi: i32) -> PointPx {
    point(px(1500, dpi), grounded_y(right_wa(dpi), char_size(dpi)))
}

/// 全 work area の外を指す追従 offset（キャラの右上へ px(500)／−px(400)）。
///
/// キャラ窓（右端 `px(1800)`）は右モニタ内に留まる一方、バルーン（幅 `px(500)`）は
/// `px(2000)` 以降＝`right_wa.right = px(1920)` の外側へ丸ごと出る。左モニタは負座標
/// ゆえ交差し得ない＝**バルーンだけが完全不可視**になる S3′ の合成そのもの。
fn far_out_offset(dpi: i32) -> PointPx {
    point(px(500, dpi), -px(400, dpi))
}

/// 旧バルーン位置（右モニタ内＝**可視**。ゆえに「可視→不可視の遷移」になる）。
fn visible_balloon_pos(dpi: i32) -> PointPx {
    point(px(800, dpi), px(240, dpi))
}

/// 「キャラ窓は可視のまま・offset 恒等式の提案位置だけが全 work area 非交差」へ
/// 落ちる合成 World を組む（S3′＝*キャラは見えているのに会話が読めない*）。
fn char_with_far_balloon_world(
    dpi: i32,
    balloon_pos: PointPx,
    offset: PointPx,
) -> (World, Entity, Entity) {
    let c = char_size(dpi);
    let b = balloon_size(dpi);
    let start = char_start_pos(dpi);
    let mut world = World::new();
    world.insert_resource(mixed_layout(dpi));
    let balloon = world
        .spawn((
            fake_handle(0x2000),
            window_pos_sized(balloon_pos.x, balloon_pos.y, b.w, b.h),
        ))
        .id();
    let char_window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(start.x, start.y, c.w, c.h),
            Anchored(Anchor::Bottom),
            BalloonFollow { balloon, offset },
        ))
        .id();
    (world, char_window, balloon)
}

/// 引き金の表（D13 帰結⑴⑵ の**キャラ窓と同一の表**）を固定する。
///
/// バルーンは別規則を持たない——違うのは「何を入力に引くか」だけで、引くのは
/// キャラ窓と同じ [`route_applies_visibility_guard`] である。ドラッグ腕が真へ倒れる
/// 変異（＝明示操作の尊重の破壊）は挙動檻
/// [`balloon_drag_trigger_neither_clamps_nor_warns`] が第一の守りとして捕まえる。
#[test]
fn balloon_follow_trigger_table_mirrors_the_char_window_table() {
    assert!(
        !BalloonFollowTrigger::Drag.applies_visibility_guard(),
        "ドラッグ随伴でガードが発火する（明示操作の尊重が壊れている・Req 3.1）"
    );
    for route in PlacementRoute::ALL {
        assert_eq!(
            BalloonFollowTrigger::Placement(route).applies_visibility_guard(),
            route_applies_visibility_guard(route),
            "route={route} の引き金判定がキャラ窓の表と食い違う"
        );
    }
    // 表が「全部真」「全部偽」へ潰れていないこと（自明な述語への退化の検出）。
    let fired = PlacementRoute::ALL
        .into_iter()
        .filter(|r| BalloonFollowTrigger::Placement(*r).applies_visibility_guard())
        .count();
    assert_eq!(fired, 4, "発火する引き金が 4 種でない（表が潰れている）");
}

/// **Req 3.4 の本体**: 非ドラッグの配置系 4 経路が引き金のとき、offset 恒等式が出した
/// バルーン提案位置が全 work area 非交差へ落ちるなら、X の clamp で救われる。
///
/// キャラ窓は終始可視（clamp されない）＝救われたのは**バルーンだけ**である。
#[test]
fn balloon_visibility_guard_clamps_x_on_non_drag_placement_triggers() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let b_size = balloon_size(dpi);
        let offset = far_out_offset(dpi);
        let old_pos = visible_balloon_pos(dpi);
        for route in [
            PlacementRoute::AnchorChange,
            PlacementRoute::Resnap,
            PlacementRoute::DpiReproject,
            PlacementRoute::ReportedSizeReconcile,
        ] {
            let (mut world, char_window, balloon) =
                char_with_far_balloon_world(dpi, old_pos, offset);

            // (1) 探針の自己検査: 恒等式の素の提案は**本当に**全 work area 非交差／
            //     旧バルーン矩形は可視。どちらかが崩れると ClampX 腕へ入らず空虚になる。
            let settled = char_settled_pos(dpi);
            let bare = point(settled.x + offset.x, settled.y + offset.y);
            assert!(
                !visible_in(&layout, bare, b_size),
                "dpi={dpi}: 探針が不動点——素のバルーン提案 {bare:?} が既に可視"
            );
            assert!(
                visible_in(&layout, old_pos, b_size),
                "dpi={dpi}: 旧バルーンが非交差では『遷移』でなく留置＝Keep が正解になる"
            );

            let (ok, events) = capture_logs(|| {
                resize_window_to(&mut world, char_window, char_size(dpi), route)
            });
            assert!(ok, "dpi={dpi} route={route}: 書込は成立する前提");

            // (2) キャラ窓は clamp されていない＝救われたのはバルーンだけである。
            assert_eq!(
                point_of(&world, char_window),
                settled,
                "dpi={dpi} route={route}: キャラ窓が動いた＝S3′ ではなく S3 の檻になっている"
            );

            // Req 3.4: 書かれたバルーン矩形はいずれかの work area と交差する。
            let pos = point_of(&world, balloon);
            assert!(
                visible_in(&layout, pos, b_size),
                "dpi={dpi} route={route}: Req 3.4 違反——バルーン {pos:?} が全 work area と非交差"
            );
            assert_eq!(
                pos.y, bare.y,
                "dpi={dpi} route={route}: バルーンの Y は恒等式の所有＝ガードが触ってはならない"
            );
            assert_ne!(
                pos.x, bare.x,
                "dpi={dpi} route={route}: バルーンの X が引き戻されていない（ガード未発火）"
            );
            let wa = right_wa(dpi);
            assert!(
                wa.left <= pos.x && pos.x <= wa.right - b_size.w,
                "dpi={dpi} route={route}: clamp 先が work area {wa:?} の外: {pos:?}"
            );

            // (4) 判定語: ClampX の warn が 1 行・水準は WARN（縮退シームの記録）。
            let clamped = expect_one(&events, CLAMP_TAG);
            assert_eq!(
                clamped.level,
                tracing::Level::WARN,
                "dpi={dpi} route={route}: バルーンの clamp が warn 水準でない"
            );
            // 提案位置の中心はどの work area にも属さない＝食い違いの兆候も 1 行残る。
            assert_eq!(
                expect_one(&events, NEAREST_TAG).level,
                tracing::Level::WARN,
                "dpi={dpi} route={route}: 最近傍フォールバックが warn へ昇格していない"
            );
        }
    }
}

/// **Req 3.1 の裏面**: 明示操作系・非配置系の引き金では、バルーン位置が素の offset
/// 恒等式と 1 bit も違わず、ガードのログも 1 行も出ない。
#[test]
fn balloon_visibility_guard_does_not_fire_on_explicit_or_non_placement_triggers() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let b_size = balloon_size(dpi);
        let offset = far_out_offset(dpi);
        let old_pos = visible_balloon_pos(dpi);
        for route in [
            PlacementRoute::SpawnInitial,
            PlacementRoute::Restore,
            PlacementRoute::KeepPositionResize,
            PlacementRoute::BalloonFollow,
            PlacementRoute::MoveCue,
        ] {
            let (mut world, char_window, balloon) =
                char_with_far_balloon_world(dpi, old_pos, offset);
            let settled = char_settled_pos(dpi);
            let bare = point(settled.x + offset.x, settled.y + offset.y);
            // 探針の自己検査: 発火条件は揃っている（引き金だけが違う）。
            assert!(
                !visible_in(&layout, bare, b_size),
                "dpi={dpi}: 探針が不動点——発火条件が揃っていない"
            );

            let (ok, events) = capture_logs(|| {
                resize_window_to(&mut world, char_window, char_size(dpi), route)
            });
            assert!(ok, "dpi={dpi} route={route}: 書込は成立する前提");

            assert_eq!(
                point_of(&world, balloon),
                bare,
                "dpi={dpi} route={route}: 適用外の引き金でバルーンが動いた（明示操作の尊重が壊れている）"
            );
            assert!(
                guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
                "dpi={dpi} route={route}: 適用外の引き金でガードが喋っている: {events:?}"
            );
        }
    }
}

/// **本タスクの中核の守り（[[6.1 → 6.2 の申し送り]]）**: ドラッグ随伴では発火しない。
///
/// `follow_balloon` は配置系（[`resize_window_to`]）とドラッグ
/// （[`on_char_drag`]／[`on_char_drag_end`]）の**双方**から呼ばれる。無条件適用すると
/// ユーザーがキャラを画面端へ運んだときにバルーンだけが引き戻され、Req 3.1 の
/// 「明示操作の尊重」が壊れる——その変異を**位置 assert**で捕まえる（ログ側の否定
/// assert だけに依存しない・[[5.2 の教訓]]）。
#[test]
fn balloon_drag_trigger_neither_clamps_nor_warns() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let c_size = char_size(dpi);
        let b_size = balloon_size(dpi);
        let offset = far_out_offset(dpi);
        let old_pos = visible_balloon_pos(dpi);
        let start = char_start_pos(dpi);
        let cursor = (px(800, dpi), px(400, dpi));
        // カーソルを右へ px(100) 動かす＝生ドラッグ x は px(1600)。
        let moved = (cursor.0 + px(100, dpi), cursor.1);
        // 射影 T 適用後のキャラ確定位置（下端接地・X は素通し）。
        let settled = point(px(1600, dpi), grounded_y(right_wa(dpi), c_size));
        let bare = point(settled.x + offset.x, settled.y + offset.y);

        // 探針の自己検査: ドラッグ随伴の提案は**本当に**全 work area 非交差
        //（＝ガードが配線されていれば必ず clamp する状況である）。旧矩形は可視。
        assert!(
            !visible_in(&layout, bare, b_size),
            "dpi={dpi}: 探針が不動点——ドラッグ随伴の提案 {bare:?} が可視のまま"
        );
        assert!(
            visible_in(&layout, old_pos, b_size),
            "dpi={dpi}: 旧バルーンが非交差では『留置の尊重』と区別が付かない"
        );

        for entry in ["on_char_drag", "on_char_drag_end"] {
            let (mut world, char_window, balloon) =
                char_with_far_balloon_world(dpi, old_pos, offset);
            world
                .entity_mut(char_window)
                .insert(dragging_state((start.x, start.y), cursor));

            let (_, events) = capture_logs(|| match entry {
                "on_char_drag" => {
                    let ev = Phase::Bubble(drag_event_at(char_window, cursor, moved));
                    on_char_drag(&mut world, char_window, char_window, &ev)
                }
                _ => {
                    let ev = Phase::Bubble(drag_end_event_at(char_window, moved));
                    on_char_drag_end(&mut world, char_window, char_window, &ev)
                }
            });

            assert_eq!(
                point_of(&world, char_window),
                settled,
                "dpi={dpi} {entry}: 前提——ドラッグの確定位置が想定と違う"
            );
            assert_eq!(
                point_of(&world, balloon),
                bare,
                "dpi={dpi} {entry}: ドラッグ随伴でバルーンが引き戻された（Req 3.1 違反）"
            );
            assert!(
                guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
                "dpi={dpi} {entry}: ドラッグ随伴でガードが喋っている（spam・水準分岐の破壊）: {events:?}"
            );
        }
    }
}

/// ユーザーが画面外へ留置したバルーンは、配置系の引き金でも引き戻さない
/// （キャラ窓と完全に同一の規則＝`Keep` 腕・Req 3.1 の「明示操作の尊重」）。
#[test]
fn balloon_parked_off_screen_is_respected_on_placement_trigger() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let b_size = balloon_size(dpi);
        let offset = far_out_offset(dpi);
        // 旧バルーンは既に全 work area の外（ユーザー留置）。
        let parked = point(px(2400, dpi), px(240, dpi));
        assert!(
            !visible_in(&layout, parked, b_size),
            "dpi={dpi}: 前提——旧バルーンは既に非交差（留置）"
        );

        let (mut world, char_window, balloon) =
            char_with_far_balloon_world(dpi, parked, offset);
        let settled = char_settled_pos(dpi);
        let bare = point(settled.x + offset.x, settled.y + offset.y);
        assert!(
            !visible_in(&layout, bare, b_size),
            "dpi={dpi}: 前提——提案も非交差（`Keep` 腕を通る条件）"
        );

        let (ok, events) = capture_logs(|| {
            resize_window_to(
                &mut world,
                char_window,
                char_size(dpi),
                PlacementRoute::DpiReproject,
            )
        });
        assert!(ok);
        assert_eq!(
            point_of(&world, balloon),
            bare,
            "dpi={dpi}: 留置バルーンが引き戻された（Keep 腕が効いていない）"
        );
        assert!(
            guard_events(&events, CLAMP_TAG).is_empty(),
            "dpi={dpi}: 留置バルーンに ClampX が出ている: {events:?}"
        );
    }
}

/// 任意の `WindowPos` を持つバルーンで [`char_with_far_balloon_world`] 相当を組む
/// （未確定表現の探針用）。
fn char_with_balloon_window_pos(
    dpi: i32,
    balloon_pos: WindowPos,
    offset: PointPx,
) -> (World, Entity, Entity) {
    let c = char_size(dpi);
    let start = char_start_pos(dpi);
    let mut world = World::new();
    world.insert_resource(mixed_layout(dpi));
    let balloon = world.spawn((fake_handle(0x2000), balloon_pos)).id();
    let char_window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(start.x, start.y, c.w, c.h),
            Anchored(Anchor::Bottom),
            BalloonFollow { balloon, offset },
        ))
        .id();
    (world, char_window, balloon)
}

/// **バルーン寸の未確定は `Option::None` だけではない**（[[4.6 の教訓]]・6.1 の
/// `old_rect` 導出と同型の罠）: `WindowPos::default()` は position・size の**両方**を
/// `CW_USEDEFAULT`（`i32::MIN` センチネル）で持つ。
///
/// センチネルを素の矩形として交差判定へ入れると `saturating_add` で逆転矩形になり、
/// 判定そのものが意味を失う。是正版は**寸が未確定なら位置に一切手を入れず** `warn!` を残す。
///
/// # 檻の非空虚性（[[5.2 の教訓]]＝ログ側だけの守りにしない）
///
/// 寸フィルタを外す変異では、位置センチネルが `old_rect = None`（不明）へ落ちるため
/// 安全側 `ClampX` が走り、`clamp_x_into(x, i32::MIN, wa)` が `wa.left` を返す
/// ＝**提案位置と違う座標が書かれる**。提案 X を `left_wa().left` より左へ置いてあるのは
/// そのためで、位置 assert が第一の守りになる。
#[test]
fn balloon_undetermined_size_holds_proposed_position_and_warns() {
    for dpi in DPIS {
        // 提案 X は左モニタ work area の左端よりさらに左（センチネル素通し変異で
        // 必ず `left_wa().left` へ引き戻される位置）。
        let offset = point(-px(4500, dpi), -px(400, dpi));
        let settled = char_settled_pos(dpi);
        let bare = point(settled.x + offset.x, settled.y + offset.y);
        assert!(
            bare.x < left_wa().left,
            "dpi={dpi}: 探針が不動点——センチネル素通し変異でも X が動かない配置になっている"
        );

        // 窓生成直後の実表現（position・size ともに CW_USEDEFAULT センチネル）。
        let (mut world, char_window, balloon) =
            char_with_balloon_window_pos(dpi, WindowPos::default(), offset);

        let (ok, events) = capture_logs(|| {
            resize_window_to(
                &mut world,
                char_window,
                char_size(dpi),
                PlacementRoute::ReportedSizeReconcile,
            )
        });
        assert!(ok);
        assert_eq!(
            point_of(&world, balloon),
            bare,
            "dpi={dpi}: 寸未確定（センチネル）なのに位置へ手が入った"
        );
        let warned = expect_one(&events, UNRESOLVED_TAG);
        assert_eq!(
            warned.level,
            tracing::Level::WARN,
            "dpi={dpi}: 判定不能が warn として残っていない（Req 3.3）"
        );
        // **フィールド集合の固定**（`diagnosis-procedure.md` §3.1／§6.3 の振り分け規則が
        // これに依存する）: `route=BalloonFollow` で窓種別が引け、**`proposed` の有無**が
        // 本行（良性の判定不能）と装置異常（`MonitorSnapshot` 不在・モニタ 0 台）を分ける。
        // どちらを落としても実機判定が反転するので、literal で固定する
        // （[[5.1 → 7.2 の申し送り＝判定語に使っているのに檻が無い型]] の再発防止）。
        assert_eq!(
            warned.field("route"),
            "BalloonFollow",
            "dpi={dpi}: 判定不能行が窓種別を名乗っていない（§3.1 の振り分けが成立しない）"
        );
        assert_eq!(
            warned.field("proposed"),
            format!("{bare:?}"),
            "dpi={dpi}: 判定不能行の `proposed` が提案位置と違う（§6.3 の判別子）"
        );
        assert!(
            guard_events(&events, CLAMP_TAG).is_empty(),
            "dpi={dpi}: 寸が読めないのに clamp している: {events:?}"
        );
    }
}

/// **§6.3 の判別子の裏面**: 真の観測装置異常（`MonitorSnapshot` 不在）はキャラ窓・
/// バルーン窓の**双方**から `WorkAreaUnresolved` を出すが、いずれも **`proposed` を
/// 持たない**。
///
/// 手順書はこの 1 点で「良性の判定不能（バルーン寸未確定）」と「セッション全体を
/// 無効にする装置異常」を分ける。`route=` だけでは分けられない——装置異常も
/// バルーン随伴で起きれば `route=BalloonFollow` を名乗るからである。
#[test]
fn missing_monitor_snapshot_warns_for_both_windows_without_the_proposed_field() {
    for dpi in DPIS {
        let (mut world, char_window, _balloon) = char_with_far_balloon_world(
            dpi,
            visible_balloon_pos(dpi),
            far_out_offset(dpi),
        );
        world.remove_resource::<MonitorSnapshot>();
        // 射影が identity へ縮退しても書込が起きるよう、寸を変える（高さのみ＝
        // 手順 3b の x 付替えを避ける）。同寸だとべき等 skip で随伴まで届かない。
        let new = SizePx {
            w: char_size(dpi).w,
            h: px(200, dpi),
        };

        let (ok, events) =
            capture_logs(|| resize_window_to(&mut world, char_window, new, PlacementRoute::Resnap));
        assert!(ok, "dpi={dpi}: 寸の反映自体は従来どおり成立する");

        let warned = guard_events(&events, UNRESOLVED_TAG);
        assert_eq!(
            warned.len(),
            2,
            "dpi={dpi}: 装置異常はキャラ窓とバルーン窓の双方から出るはず: {events:?}"
        );
        let routes: Vec<&str> = warned.iter().map(|e| e.field("route")).collect();
        assert!(
            routes.contains(&"Resnap") && routes.contains(&"BalloonFollow"),
            "dpi={dpi}: 2 行の route が {routes:?}（キャラ窓＋バルーン窓の対になっていない）"
        );
        for e in &warned {
            assert_eq!(e.level, tracing::Level::WARN, "dpi={dpi}: 水準が warn でない");
            assert!(
                !e.fields.contains_key("proposed"),
                "dpi={dpi}: 装置異常の行が `proposed` を持っている＝§6.3 の判別子が壊れる: {:?}",
                e.fields
            );
        }
        assert!(
            guard_events(&events, CLAMP_TAG).is_empty(),
            "dpi={dpi}: work area 不明なのに clamp している: {events:?}"
        );
    }
}

/// **旧位置の未確定も `Option::None` だけではない**: 寸だけ確定して位置が
/// `CW_USEDEFAULT` のままの窓は、素通しすると矩形が `i32::MIN` 近傍へ落ちて
/// 「もともと画面外に留置されていた」と誤判定され、**安全側 clamp の腕が丸ごと死ぬ**
/// （6.1 が寸について踏んだのと同型の罠を、位置について踏まないための檻）。
///
/// 負座標そのものは正当（左モニタは `-1920..0`）ゆえ、判定は符号ではなく
/// wintf 正典のセンチネル一致で行う。
#[test]
fn balloon_undetermined_position_is_treated_as_unknown_rect_and_clamps() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let b_size = balloon_size(dpi);
        let offset = far_out_offset(dpi);
        let settled = char_settled_pos(dpi);
        let bare = point(settled.x + offset.x, settled.y + offset.y);
        assert!(
            !visible_in(&layout, bare, b_size),
            "dpi={dpi}: 探針が不動点——提案が既に可視で安全側 clamp の腕へ入らない"
        );

        // 寸は確定済み・位置だけ CW_USEDEFAULT（wintf 正典の未確定表現）。
        let window_pos = WindowPos {
            size: Some(SizeI::new(b_size.w, b_size.h)),
            ..Default::default()
        };
        let (mut world, char_window, balloon) =
            char_with_balloon_window_pos(dpi, window_pos, offset);

        let (ok, events) = capture_logs(|| {
            resize_window_to(
                &mut world,
                char_window,
                char_size(dpi),
                PlacementRoute::DpiReproject,
            )
        });
        assert!(ok);
        assert!(
            visible_in(&layout, point_of(&world, balloon), b_size),
            "dpi={dpi}: 位置未確定（センチネル）を『留置』と誤読して clamp を見送っている"
        );
        expect_one(&events, CLAMP_TAG);
    }
}

/// 破棄済みバルーンへの随伴は**正常終了系**として `debug!` で打ち切る（Req 6.2/6.3・
/// task 3.2 と同じ区別）。ここを `warn!` にすると終了時ログが良性ノイズで埋まり、
/// 本物の異常（実在窓の寸未確定）が読めなくなる。
#[test]
fn balloon_despawned_skips_guard_without_warning() {
    for dpi in DPIS {
        let (mut world, char_window, balloon) =
            char_with_far_balloon_world(dpi, visible_balloon_pos(dpi), far_out_offset(dpi));
        world.despawn(balloon);

        let (_, events) = capture_logs(|| {
            resize_window_to(
                &mut world,
                char_window,
                char_size(dpi),
                PlacementRoute::Resnap,
            )
        });

        assert!(
            guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
            "dpi={dpi}: 破棄済みバルーンに対してガードが喋っている（Req 6.2 違反）: {events:?}"
        );
        // **task 7.3 で強化**: 6.2 が固定していたのは「ガードが喋らない」だけで、
        // 随伴書込そのもの（`enqueue_window_set_pos`）が破棄済みバルーンに対して
        // `warn!` を出していた（6.2 → 7.3 の申し送り）。終了時静穏（Req 6.2）は
        // **経路全体**の主張ゆえ、ここで警告以上ゼロを丸ごと見る。
        assert!(
            events.iter().all(|e| e.level > tracing::Level::INFO),
            "dpi={dpi}: 破棄済みバルーンに対して警告以上のログが出ている（Req 6.2 違反）: {events:?}"
        );
        // **相ごとに数える**——総数で数えると、片方の打ち切りを外しても他方が同じ
        // 判定語を出して総数が偶然一致し、檻が空虚になる（3.2 の教訓と同型）。
        let skips = despawn_skip_lines(&events);
        assert!(
            skips.iter().all(|e| e.level == tracing::Level::DEBUG),
            "dpi={dpi}: 破棄済み打ち切りが debug 水準でない: {skips:?}"
        );
        assert_eq!(
            skips
                .iter()
                .filter(|e| e.message().contains("可視性の遷移ガード"))
                .count(),
            1,
            "dpi={dpi}: 遷移ガード相の打ち切りが 1 行でない: {events:?}"
        );
        assert_eq!(
            skips
                .iter()
                .filter(|e| e.message().contains("窓移動"))
                .count(),
            1,
            "dpi={dpi}: 随伴書込相の打ち切りが 1 行でない: {events:?}"
        );
    }
}

/// 破棄済み判定語（[`DESPAWNED_SKIP_TAG`]）を含む行を抜く（相ごとの計数用）。
fn despawn_skip_lines(events: &[LogEvent]) -> Vec<&LogEvent> {
    events
        .iter()
        .filter(|e| e.message().contains(DESPAWNED_SKIP_TAG))
        .collect()
}

/// Req 6.2 の裏面（真の異常を殺さない・随伴書込相）: **生存している** entity の
/// `WindowHandle` 欠落（窓生成前）は従来どおり `warn!`。存在確認の導入でこちらまで
/// 静穏化してはならない——「窓がまだ無い」は結線の異常であって終了系ではない。
#[test]
fn balloon_without_handle_on_living_entity_still_warns_on_follow_write() {
    let dpi = 96;
    let (mut world, char_window, balloon) =
        char_with_far_balloon_world(dpi, visible_balloon_pos(dpi), far_out_offset(dpi));
    // entity は実在させたまま `WindowHandle` だけを剥がす（窓生成前と同じ状態）。
    world.entity_mut(balloon).remove::<WindowHandle>();

    let (_, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            char_window,
            char_size(dpi),
            PlacementRoute::Resnap,
        )
    });

    let warned = expect_one(&events, "WindowHandle 未付与");
    assert_eq!(
        warned.level,
        tracing::Level::WARN,
        "実在 entity の WindowHandle 欠落は真の異常＝warn のまま（Req 6.2 の区別）"
    );
    assert!(
        !despawn_skip_lines(&events)
            .iter()
            .any(|e| e.message().contains("窓移動")),
        "実在 entity を『破棄済み』と誤判定している: {events:?}"
    );
}

// -------------------------------------------------------------------------
// 位置の未確定表現（`CW_USEDEFAULT`）をキャラ窓経路でも打ち切る
// （task 6.3・S3 補・D15・Req 3.1/3.3）
//
// `resize_window_to` 手順 3 は `WindowPos.position` の `Option::None` しか縮退させて
// おらず、wintf 正典の**もう一つの未確定表現**（`CW_USEDEFAULT` ＝ `i32::MIN`・
// `WindowPos::default()` が position に持つ）を素通ししていた。素通しすると
//   ① 手順 3a の `old_rect` が `i32::MIN` 近傍の全 work area 非交差矩形になり、
//      `guard_visibility` が「もともと留置されていた」と誤読して `Keep` へ落ちる
//      ＝**6.1 が敷いた安全側 clamp の腕が黙って死ぬ**
//   ② 手順 3b の中央付替えと射影 T の入力（raw）も同時に汚染される
// D15 は (b) **resize 打ち切り**を採る——位置未確定は「保存すべき接地点が存在しない」
// ゆえ、`Option::None` と同じ腕（`warn!`＋`false`）へ合流させて①②を一括で断つ。
//
// 檻の要点（空虚化を避けるための自己検査を各檻が持つ）:
//   (1) 打ち切り檻の自己検査——**位置だけを実値に替えた対照窓**が同じ route・同じ寸で
//       確実に書込まで進むこと（進まないなら「打ち切れた」は何も意味しない）
//   (2) 書込ゼロの直接観測——`WindowPos` が呼出前後で**完全一致**（`PartialEq`）
//   (3) `warn!` ちょうど 1 件——ログ側の守りを位置 assert と二段構えにする
//       （[[5.2 の教訓＝空虚性 6 例目]]／[[6.2 の教訓＝檻の空虚性]]）
//   (4) **符号判定への変異の検出**——左モニタは `-1920..0` ＝負座標そのものは正当。
//       実在する負座標の窓が打ち切られないことを独立の檻で固定する
//
// なお寸センチネルとの**非対称は意図的**（D15 帰結⑴）: 寸未確定は接地点（位置）が
// 実在するので resize に意味があり、`old_rect` 不明の安全側 clamp で扱う
// （既存檻 `undetermined_old_size_is_treated_as_unknown_rect_and_clamps` が無改変で
// 緑のまま＝その非対称の檻を兼ねる）。
// -------------------------------------------------------------------------

/// wintf 正典の未確定センチネル（`== i32::MIN`）。**本体の import とは独立に**
/// 定義元から直接引き、判定式が正典と同式であることを檻側でも固定する
/// （`window_pos.rs:41`／`monitor_systems.rs:408` と同じ値）。
use windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT as SENTINEL;

/// 手順 3 の位置センチネル打ち切りが名乗る語（**本体の文言とは独立に literal で置く**）。
const POSITION_SENTINEL_TAG: &str = "センチネル（位置未確定）";

/// 位置・寸を明示した単独キャラ窓の World（混在 DPI 合成レイアウト付き）。
fn char_world_with_window_pos(dpi: i32, position: Point, size: Option<SizeI>) -> (World, Entity) {
    let mut world = World::new();
    world.insert_resource(mixed_layout(dpi));
    let e = world
        .spawn((
            fake_handle(0x1000),
            WindowPos {
                position: Some(position),
                size,
                ..Default::default()
            },
            Anchored(Anchor::Bottom),
        ))
        .id();
    (world, e)
}

/// 旧寸（[`wide_char_size`] の `SizeI` 表現）。
fn old_size_i(dpi: i32) -> SizeI {
    let s = wide_char_size(dpi);
    SizeI::new(s.w, s.h)
}

/// 左モニタ（**負座標** `-1920..0`）内の**実在する**接地位置。
///
/// 符号（`x < 0`）や大きさの閾値で未確定判定をすると、この正当な位置が巻き添えで
/// 打ち切られる＝檻 [`negative_real_position_is_not_aborted_and_still_resizes`] の被検体。
fn negative_real_pos(dpi: i32) -> Point {
    Point {
        x: left_wa().left / 2,
        y: left_wa().bottom - old_size_i(dpi).height,
    }
}

/// **探針の自己検査**: 位置**だけ**を実値に替えた対照窓は、同じ route・同じ新寸で
/// 必ず書込まで進む。これが崩れていると打ち切り檻の「何も起きなかった」は
/// センチネルの成果ではなく入力の不備になる（不動点の検出）。
fn assert_control_position_writes(dpi: i32, new: SizePx) {
    let (mut world, e) =
        char_world_with_window_pos(dpi, negative_real_pos(dpi), Some(old_size_i(dpi)));
    let before = *world.get::<WindowPos>(e).expect("WindowPos があるはず");
    assert!(
        resize_window_to(&mut world, e, new, PlacementRoute::Resnap),
        "dpi={dpi}: 探針が不動点——位置が実値の対照でも resize が成立しない"
    );
    assert_ne!(
        *world.get::<WindowPos>(e).expect("WindowPos があるはず"),
        before,
        "dpi={dpi}: 探針が不動点——対照でも WindowPos が 1 bit も変わらない"
    );
}

/// **位置がセンチネルの窓は log-first で打ち切る**（D15 採用案 (b)）: 戻り値 `false`・
/// `WindowPos` 書込ゼロ・`warn!` ちょうど 1 件。
///
/// 是正前はここで安全側 `ClampX` が走り、`clamp_x_into(i32::MIN, .., wa)` が返す
/// `wa.left` が**位置権威の無い窓へ書き込まれて**いた（＝位置権威の僭称）。
#[test]
fn undetermined_position_aborts_resize_without_writing() {
    for dpi in DPIS {
        let new = narrow_char_size(dpi);
        assert_control_position_writes(dpi, new);

        for (label, size) in [
            // `on_window_add` が挿す実表現そのもの（位置・寸とも未確定）。
            ("窓生成直後（位置・寸ともセンチネル）", None),
            // 寸だけ確定した窓＝汚染されるのは位置の側だけ、という切り分け。
            ("寸のみ確定・位置センチネル", Some(old_size_i(dpi))),
        ] {
            let position = Point {
                x: SENTINEL,
                y: SENTINEL,
            };
            let (mut world, e) = char_world_with_window_pos(dpi, position, size);
            // 探針の前提: 被検体が本当にセンチネルを持っている。
            assert_eq!(
                world
                    .get::<WindowPos>(e)
                    .expect("WindowPos があるはず")
                    .position,
                Some(position),
                "dpi={dpi} {label}: 探針がセンチネルを持っていない"
            );
            let before = *world.get::<WindowPos>(e).expect("WindowPos があるはず");

            let (ok, events) =
                capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::Resnap));

            assert!(
                !ok,
                "dpi={dpi} {label}: 位置未確定（センチネル）なのに resize が成立している"
            );
            assert_eq!(
                *world.get::<WindowPos>(e).expect("WindowPos があるはず"),
                before,
                "dpi={dpi} {label}: 打ち切りのはずが WindowPos へ書き込まれている（Req 3.3 の現状維持違反）"
            );
            let warned = expect_one(&events, POSITION_SENTINEL_TAG);
            assert_eq!(
                warned.level,
                tracing::Level::WARN,
                "dpi={dpi} {label}: 打ち切りが warn として残っていない（log-first 違反）"
            );
            assert_eq!(
                warned.field("entity"),
                format!("{e:?}"),
                "dpi={dpi} {label}: 警告行が対象 entity を名乗っていない"
            );
            assert_eq!(
                warned.field("position"),
                format!("{position:?}"),
                "dpi={dpi} {label}: 警告行が問題の位置を載せていない"
            );
            assert!(
                guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
                "dpi={dpi} {label}: 打ち切ったのにガードが喋っている（射影 T の入力が汚染されている）: {events:?}"
            );
        }
    }
}

/// **負座標そのものは正当**（合成レイアウトの左モニタは `-1920..0`）。
///
/// 判定を符号（`x < 0`）や大きさの閾値へ変異させると、この実在位置の窓まで打ち切られる。
/// ゆえに本檻は「打ち切られない」ことを**位置の実値**で固定する（従来経路の非退行）。
#[test]
fn negative_real_position_is_not_aborted_and_still_resizes() {
    for dpi in DPIS {
        let start = negative_real_pos(dpi);
        let new = narrow_char_size(dpi);
        let layout = mixed_layout(dpi);
        // 探針の自己検査: ①本当に負座標であり ②センチネルではなく
        // ③旧矩形が実際に可視（＝「もともと留置」腕へ落ちない通常経路の入力）。
        assert!(start.x < 0, "dpi={dpi}: 探針が負座標になっていない");
        assert_ne!(start.x, SENTINEL, "dpi={dpi}: 探針がセンチネルと衝突している");
        assert!(
            visible_in(
                &layout,
                PointPx {
                    x: start.x,
                    y: start.y
                },
                wide_char_size(dpi)
            ),
            "dpi={dpi}: 探針の旧矩形が既に不可視——通常経路を通らない"
        );

        let (mut world, e) = char_world_with_window_pos(dpi, start, Some(old_size_i(dpi)));
        let (ok, events) =
            capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::Resnap));

        assert!(
            ok,
            "dpi={dpi}: 正当な負座標が打ち切られた（符号での未確定判定＝D15 が禁じた式）"
        );
        assert_eq!(
            point_of(&world, e),
            unguarded_projection(
                dpi,
                PointPx {
                    x: start.x,
                    y: start.y
                },
                new
            ),
            "dpi={dpi}: 負座標の従来経路（手順 3b＋射影 T）が退行している"
        );
        assert!(
            guard_events(&events, POSITION_SENTINEL_TAG).is_empty(),
            "dpi={dpi}: 正当な負座標に対してセンチネル警告が出ている: {events:?}"
        );
        assert!(
            guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
            "dpi={dpi}: 可視 → 可視の遷移でガードが喋っている: {events:?}"
        );
    }
}

/// **片軸だけ**のセンチネルも打ち切る（`pos.x == SENTINEL || pos.y == SENTINEL`）。
///
/// `&&` への変異（両軸そろったときだけ打ち切る）を検出する。y のみのセンチネルは
/// wintf 正典の `window_center` が見ていない軸であり、`||` にしてある理由が
/// 「接地点（下端中央）は x・y の**両方**が揃って初めて意味を持つ」ことである。
#[test]
fn single_axis_position_sentinel_also_aborts() {
    for dpi in DPIS {
        let new = narrow_char_size(dpi);
        let real = negative_real_pos(dpi);
        assert_control_position_writes(dpi, new);

        for (label, position) in [
            (
                "x のみセンチネル",
                Point {
                    x: SENTINEL,
                    y: real.y,
                },
            ),
            (
                "y のみセンチネル",
                Point {
                    x: real.x,
                    y: SENTINEL,
                },
            ),
        ] {
            let (mut world, e) =
                char_world_with_window_pos(dpi, position, Some(old_size_i(dpi)));
            let before = *world.get::<WindowPos>(e).expect("WindowPos があるはず");

            let (ok, events) =
                capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::Resnap));

            assert!(
                !ok,
                "dpi={dpi} {label}: 片軸センチネルが打ち切られていない"
            );
            assert_eq!(
                *world.get::<WindowPos>(e).expect("WindowPos があるはず"),
                before,
                "dpi={dpi} {label}: 打ち切りのはずが WindowPos へ書き込まれている"
            );
            let warned = expect_one(&events, POSITION_SENTINEL_TAG);
            assert_eq!(
                warned.level,
                tracing::Level::WARN,
                "dpi={dpi} {label}: 打ち切りが warn として残っていない"
            );
            assert!(
                guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
                "dpi={dpi} {label}: 打ち切ったのにガードが喋っている: {events:?}"
            );
        }
    }
}

// -------------------------------------------------------------------------
// 混在 DPI・複数モニタ回帰檻の拡充（task 7.2・Req 3.4/4.4/5.1/5.2/5.3/5.6）
//
// task 6.1 は**キャラ窓だけ**が不可視へ落ちる合成を、task 6.2 は**バルーンだけ**が
// 落ちる合成（キャラは終始可視だと明示的に assert する）を固めた。どちらの檻も
// 「もう一方の窓は自明に安全」な世界で 1 つの連言肢を証明しており、Req 3.4 が
// 要求する **連言**——「キャラ窓とバルーン窓の *どちらも* 不可視状態に遷移させない」
// ——を 1 回の書込の中で見た檻は存在しない。本節が足すのはその連言と、
// 2 つのガードが**互いの結果に依存する**接続点である。
//
//   (A) 1 回の [`resize_window_to`] で**両窓が同時に**全 work area 非交差へ落ちる
//       合成。しかも救出先の work area が**別々のモニタ**になる配置で組むので、
//       clamp 先の解決が窓ごとに独立であること（キャラの clamp_wa を流用していない
//       こと）まで座標で固定される。
//   (B) バルーンが追従するのは **ガード適用後**のキャラ位置であること。手順 7 が
//       `new_pos` ではなく素の射影（`raw`／ガード前）を渡す変異は、6.2 の檻では
//       **不動点**になる（あちらはキャラが clamp されない合成ゆえ両者が同値）。
//       ここでは clamp 前後で px(40) ずれるので、恒等式の主張が実際に効く。
//
// 座標はすべて論理値 × DPI（96/120/192）で構築し、絶対 px の固定値を持たない
// （Req 5.6）。実 GPU・実高 DPI モニタを要さず決定論（Req 5.2）。
// -------------------------------------------------------------------------

/// [`gap_bound_char_world`] に随伴バルーンを足した World。
///
/// `offset` は**窓（char 窓左上）相対**の追従 offset。[`resize_window_to`] は寸法変動で
/// これを**一切書き換えない**（2026-07-31 実機 SSP 裁定・恒等式
/// `balloon_pos − char_pos ≡ offset` が全アンカーで不変）ので、spawn 時点の値が
/// そのまま追従に使われる。
fn gap_bound_char_world_with_balloon(
    dpi: i32,
    balloon_size: SizePx,
    balloon_pos: PointPx,
    offset: PointPx,
) -> (World, Entity, Entity, PointPx) {
    let old = wide_char_size(dpi);
    let old_pos = PointPx {
        x: gap_center_x(dpi) - old.w / 2,
        y: left_wa().bottom - old.h,
    };
    let mut world = World::new();
    world.insert_resource(mixed_layout(dpi));
    let balloon = world
        .spawn((
            fake_handle(0x2000),
            window_pos_sized(
                balloon_pos.x,
                balloon_pos.y,
                balloon_size.w,
                balloon_size.h,
            ),
        ))
        .id();
    let char_window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(old_pos.x, old_pos.y, old.w, old.h),
            Anchored(Anchor::Bottom),
            BalloonFollow { balloon, offset },
        ))
        .id();
    (world, char_window, balloon, old_pos)
}

/// **Req 3.4／5.3 の連言**: 1 回の非ドラッグ配置書込で、キャラ窓とバルーン窓の
/// **どちらも**全 work area 非交差にならない。しかも救出先は**別々のモニタ**である。
///
/// 合成の骨格（混在 DPI・複数モニタ・負座標・192 で 3200 超座標）:
/// - キャラ窓は帯（`0 ..= px(64)`＝どの work area にも属さない）へ落ちる幅の新寸を
///   受け取り、**右モニタ**へ引き戻される（[`gap_bound_char_world`] と同じ機序）。
/// - 随伴 offset は救出後のキャラ位置から見て遥か左（`-px(2600)`）を指すので、
///   バルーン提案矩形は**左モニタよりさらに左**の完全不可視域へ出る。最近傍は
///   左モニタゆえ **`left_wa().left` へ**引き戻される。
///
/// ゆえに 2 つの clamp 先が別モニタになる——キャラの `clamp_wa` を流用する実装は
/// バルーンを右モニタへ引き戻してしまい、`balloon.x == left_wa().left` の assert が
/// 落ちる。6.1／6.2 の単窓檻はどちらもこの取り違えに対して不動点である
/// （両窓の clamp 先が同じ右モニタになる合成しか持っていない）。
#[test]
fn both_windows_survive_a_single_write_onto_different_monitors() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let new = narrow_char_size(dpi);
        let b_size = balloon_size(dpi);
        // 窓相対の追従 offset。リサイズで補正されないので spawn 時点＝追従時点。
        // 救出後のキャラ位置から見て遥か左（左モニタよりさらに外）を指す。
        let offset = point(-px(2600, dpi), -px(600, dpi));
        // 旧バルーンは**左モニタ内**で可視（＝「遷移」であって留置ではない）。
        // 座標は左モニタ左端からの論理オフセット×DPI で組む（絶対 px を置かない・Req 5.6）。
        let old_balloon = point(left_wa().left + px(360, dpi), px(200, dpi));

        for route in [
            PlacementRoute::AnchorChange,
            PlacementRoute::Resnap,
            PlacementRoute::DpiReproject,
            PlacementRoute::ReportedSizeReconcile,
        ] {
            let (mut world, char_window, balloon, old_pos) =
                gap_bound_char_world_with_balloon(dpi, b_size, old_balloon, offset);

            // --- (1) 探針の自己検査（[[2.2 の教訓]]）---
            let char_bare = unguarded_projection(dpi, old_pos, new);
            let char_saved = point(right_wa(dpi).left, char_bare.y);
            let balloon_bare = point(char_saved.x + offset.x, char_saved.y + offset.y);
            assert!(
                visible_in(&layout, old_pos, wide_char_size(dpi)),
                "dpi={dpi}: 旧キャラ矩形が非交差では『遷移』にならない"
            );
            assert!(
                visible_in(&layout, old_balloon, b_size),
                "dpi={dpi}: 旧バルーン矩形が非交差では『遷移』にならない"
            );
            assert!(
                !visible_in(&layout, char_bare, new),
                "dpi={dpi}: 探針が不動点——ガード無しのキャラ提案 {char_bare:?} が既に可視"
            );
            assert!(
                !visible_in(&layout, balloon_bare, b_size),
                "dpi={dpi}: 探針が不動点——ガード無しのバルーン提案 {balloon_bare:?} が既に可視"
            );

            let (ok, events) = capture_logs(|| {
                resize_window_to(&mut world, char_window, new, route)
            });
            assert!(ok, "dpi={dpi} route={route}: 書込は成立する前提");

            let char_pos = point_of(&world, char_window);
            let balloon_pos = point_of(&world, balloon);

            // --- (2) 連言そのもの（Req 3.4）: どちらも全 work area 非交差ではない ---
            assert!(
                visible_in(&layout, char_pos, new),
                "dpi={dpi} route={route}: キャラ窓 {char_pos:?} が全 work area と非交差"
            );
            assert!(
                visible_in(&layout, balloon_pos, b_size),
                "dpi={dpi} route={route}: バルーン窓 {balloon_pos:?} が全 work area と非交差"
            );

            // --- (3) 救出先は**別々のモニタ**（clamp 先の解決が窓ごとに独立）---
            assert_eq!(
                char_pos, char_saved,
                "dpi={dpi} route={route}: キャラは右モニタ左端へ引き戻されるはず"
            );
            assert_eq!(
                balloon_pos.x,
                left_wa().left,
                "dpi={dpi} route={route}: バルーンの clamp 先が左モニタでない\
                 （キャラの clamp_wa を流用している疑い）: {balloon_pos:?}"
            );

            // --- (4) Y は両窓とも射影／恒等式の所有＝ガードは触らない ---
            assert_eq!(
                char_pos.y, char_bare.y,
                "dpi={dpi} route={route}: キャラの Y が動いた"
            );
            assert_eq!(
                balloon_pos.y, balloon_bare.y,
                "dpi={dpi} route={route}: バルーンの Y が動いた"
            );

            // --- (5) 判定語: ClampX が**ちょうど 2 行**（両窓ぶん）・水準は WARN ---
            let clamps = guard_events(&events, CLAMP_TAG);
            assert_eq!(
                clamps.len(),
                2,
                "dpi={dpi} route={route}: ClampX が両窓ぶん 2 行でない: {events:?}"
            );
            for ev in clamps {
                assert_eq!(
                    ev.level,
                    tracing::Level::WARN,
                    "dpi={dpi} route={route}: clamp の記録が warn 水準でない"
                );
            }
        }
    }
}

/// **Req 4.4 の恒等式は「ガード適用後のキャラ位置」に対して成立する**。
///
/// [`resize_window_to`] 手順 7 は確定位置（`new_pos`＝遷移ガード適用**後**）で
/// [`follow_balloon`] を呼ぶ。ここを素の射影（ガード前）へ差し替える変異は、
/// 6.2 の檻ではキャラが clamp されない合成ゆえ**不動点**になる。
///
/// 本檻はキャラだけが clamp される合成（clamp 前後で X が `px(40)` ずれる）を組み、
/// バルーンの追従先が**ずれた後**の位置であることを座標で固定する。バルーン自身は
/// clamp されない（＝救われたのはキャラだけ・`ClampX` はちょうど 1 行）ので、
/// 「バルーンが偶然どこかへ clamp されて結果が一致した」逃げ道も塞がる。
#[test]
fn balloon_follows_the_guarded_char_position_not_the_raw_projection() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let new = narrow_char_size(dpi);
        // 帯（`0 ..= px(64)`）より**狭い**バルーン＝帯の中へ丸ごと収まり得る。
        let b_size = SizePx {
            w: px(48, dpi),
            h: px(300, dpi),
        };
        // 窓相対の追従 offset（リサイズで補正されない＝spawn 時点＝追従時点）。
        let offset = point(-px(12, dpi), -px(600, dpi));
        let old_balloon = visible_balloon_pos(dpi);

        for route in [
            PlacementRoute::AnchorChange,
            PlacementRoute::Resnap,
            PlacementRoute::DpiReproject,
            PlacementRoute::ReportedSizeReconcile,
        ] {
            let (mut world, char_window, balloon, old_pos) =
                gap_bound_char_world_with_balloon(dpi, b_size, old_balloon, offset);

            let char_bare = unguarded_projection(dpi, old_pos, new);
            let char_saved = point(right_wa(dpi).left, char_bare.y);
            let follows_guarded = point(char_saved.x + offset.x, char_saved.y + offset.y);
            let follows_raw = point(char_bare.x + offset.x, char_bare.y + offset.y);

            // --- 探針の自己検査: 2 つの追従先が**区別できる**こと ---
            assert_ne!(
                follows_guarded.x, follows_raw.x,
                "dpi={dpi}: 探針が不動点——ガード前後でキャラ X が動いていない"
            );
            assert!(
                !visible_in(&layout, char_bare, new),
                "dpi={dpi}: 探針が不動点——ガード無しのキャラ提案が既に可視"
            );
            assert!(
                visible_in(&layout, follows_guarded, b_size),
                "dpi={dpi}: 救出後のキャラに追従したバルーンは可視のはず（clamp 不要）"
            );
            assert!(
                !visible_in(&layout, follows_raw, b_size),
                "dpi={dpi}: 素の射影に追従したバルーン {follows_raw:?} が可視では変異を区別できない"
            );

            let (ok, events) = capture_logs(|| {
                resize_window_to(&mut world, char_window, new, route)
            });
            assert!(ok, "dpi={dpi} route={route}: 書込は成立する前提");

            assert_eq!(
                point_of(&world, char_window),
                char_saved,
                "dpi={dpi} route={route}: キャラが右モニタ左端へ救出されていない"
            );
            assert_eq!(
                point_of(&world, balloon),
                follows_guarded,
                "dpi={dpi} route={route}: バルーンが**ガード適用後**のキャラ位置に追従していない\
                 （素の射影に追従した場合は {follows_raw:?}）"
            );
            assert!(
                visible_in(&layout, point_of(&world, balloon), b_size),
                "dpi={dpi} route={route}: 追従先のバルーンが全 work area と非交差"
            );

            // 恒等式（Req 4.4）: `balloon − char ≡ BalloonFollow.offset`。
            // 比較相手は**書込前から不変の**窓相対 offset（テスト側の定数）であり、
            // world から読み直した値ではない——読み直すと「恒等式を、それを作った
            // 当人に問う」恒真形になる（[[7.2 の空虚性 8 例目]]）。
            let stored_offset = world
                .get::<BalloonFollow>(char_window)
                .expect("char 窓は BalloonFollow を持つ")
                .offset;
            assert_eq!(
                stored_offset, offset,
                "dpi={dpi} route={route}: BalloonFollow.offset が書き換わった\
                 （窓相対契約＝リサイズで offset を補正しない・2026-07-31 実機 SSP 裁定）"
            );
            let c = point_of(&world, char_window);
            let b = point_of(&world, balloon);
            assert_eq!(
                point(b.x - c.x, b.y - c.y),
                offset,
                "dpi={dpi} route={route}: 追従恒等式が崩れている"
            );

            // 救われたのは**キャラだけ**＝`ClampX` はちょうど 1 行。
            let clamps = guard_events(&events, CLAMP_TAG);
            assert_eq!(
                clamps.len(),
                1,
                "dpi={dpi} route={route}: ClampX がキャラぶん 1 行でない\
                 （バルーンまで clamp されているなら追従先が偶然一致しただけ）: {events:?}"
            );
            assert_eq!(
                clamps[0].level,
                tracing::Level::WARN,
                "dpi={dpi} route={route}: clamp の記録が warn 水準でない"
            );
        }
    }
}
