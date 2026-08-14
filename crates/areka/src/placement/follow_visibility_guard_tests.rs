//! 可視性遷移ガード [`guard_visibility`] の純関数檻。
//!
//! # 前提（windowposition-limit 7.3・要件 2.1）
//!
//! 本檻が固定するのは**ガード自体の規則**（完全不可視への遷移だけを X clamp で救い、
//! 部分的なはみ出しには手を出さない）であり、`windowposition.limit` の導入でこの規則は
//! 1 bit も変わっていない。ただしバルーン窓については、limit が有効なときに**部分的な
//! はみ出しを別途作業領域内へ補正する**のは下流の関門
//! （`follow::window_move::enqueue_window_set_pos` の runtime 関門・起動時
//! `balloon_limit::apply_balloon_limit`）である。すなわち limit 有効時の最終表示位置は
//! 本檻の期待値と一致しないことがあるが、それはガードの退行ではなく関門の仕事であり、
//! `follow_balloon_limit_tests.rs`／`balloon_limit_gate_tests.rs` が所有する。
//! 本檻は [`guard_visibility`] を矩形で直接呼ぶ純関数檻であり、関門が読む
//! `BalloonLimit` Component も ECS World もここには登場しない＝関門は走らない。

use super::test_support::{
    DPIS, balloon_size, char_size, grounded_y, left_wa, mixed_layout, overlaps, point, px,
    right_wa, win,
};
use super::{MonitorSnapshot, VisibilityVerdict, guard_visibility};
use crate::placement::resolver::{SizePx};

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
//
// 前提（windowposition-limit 7.3）: 以下 3 檻の「部分的なはみ出しには触らない」
// 「画面外へ留置されたものは引き戻さない」はガードの規則としていまも真である。
// `windowposition.limit` が有効なバルーンについては、この後に走る関門が別途
// 作業領域内へ補正する——ガード自体は不変で、限界の面倒を見るのは関門である。

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
