//! 可視性遷移ガード [`guard_visibility`] の純関数檻。
//!
//! 固定するのはガード自体の規則（完全不可視への遷移だけを X clamp で救い、
//! 部分的なはみ出しには手を出さない）。`windowposition.limit` が有効なバルーン窓の
//! 部分的なはみ出しを作業領域内へ補正するのは下流の関門
//! （`follow::window_move::enqueue_window_set_pos`・起動時 `balloon_limit::apply_balloon_limit`）で、
//! limit 有効時の最終表示位置が本檻の期待値と違ってもガードの退行ではない
//! （`follow_balloon_limit_tests.rs`／`balloon_limit_gate_tests.rs` が所有）。
//! 本檻は矩形で直接呼ぶので `BalloonLimit` も ECS World も登場せず、関門は走らない。

use super::MonitorSnapshot;
use super::test_support::{
    DPIS, balloon_size, char_size, grounded_y, left_wa, mixed_layout, overlaps, point, px,
    right_wa, win,
};
use super::visibility::{VisibilityVerdict, guard_visibility};
use crate::resolver::SizePx;

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

        let proposed = point(px(1200, dpi), grounded_y(wa, size));
        assert_eq!(
            guard_visibility(Some(old), proposed, size, wa, &snapshot),
            VisibilityVerdict::Keep(proposed),
            "dpi={dpi}: 交差維持は素通し"
        );

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

// --- guard_visibility: バルーン矩形 -----------------------
//
// バルーンは別規則を持たず、キャラ窓と同じ純関数・同じ遷移規則へバルーン矩形
// （`char_pos + offset` とバルーン寸）を渡すだけ。寸の違いは下の横断檻が
// キャラ窓の寸と並べて全部の腕へ通す。
// `windowposition.limit` が有効なバルーンの補正は、この後に走る関門の仕事。

/// Y 不変の横断檻: 全分岐・キャラ／バルーン両寸で `position().y == proposed.y`
/// （Y は射影 T の所有）。分岐の識別（Keep か ClampX か）も同時に固定する。
///
/// 提案 Y が射影 T 由来の接地値（`wa.bottom − h`）だけだと、その Y は work area の
/// Y clamp の不動点なので、「ガードが Y も clamp する」退行
/// （`y: proposed.y.min(wa.bottom − h).max(wa.top)`）と正しい実装を区別できず檻が空虚になる。
/// そのため各分岐へ `[clamp_wa.top, clamp_wa.bottom − h]` の範囲外の Y を必ず通す。
/// `guard_visibility` の前提は正寸のみで、Y の値域は射影 T の関心なので、範囲外 Y の投入は契約上も正当。
#[test]
fn guard_never_modifies_y_in_any_branch() {
    for dpi in DPIS {
        let snapshot = mixed_layout(dpi);
        let wa = right_wa(dpi);
        for size in [char_size(dpi), balloon_size(dpi)] {
            // Y clamp の不動点（射影 T が出す接地 Y）
            let y_fixed = grounded_y(wa, size);
            // Y clamp の不動点ではない Y（clamp が入れば必ず動く）
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
