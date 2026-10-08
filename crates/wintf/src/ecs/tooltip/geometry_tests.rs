//! `geometry` の決定論テスト（置き場所・安全地帯・折り返し・改行の揃え・最大の幅）。

use super::*;

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RectPx {
    RectPx {
        left,
        top,
        right,
        bottom,
    }
}

fn pt(x: i32, y: i32) -> PointPx {
    PointPx { x, y }
}

/// 1920x1080 の作業領域・100x40 のツールチップ・上へ 20・下へ 32 離す。
fn input(anchor: PointPx, work_area: RectPx, width: i32, height: i32) -> PlaceInput {
    PlaceInput {
        anchor,
        tip: SizePx { width, height },
        work_area,
        offset_above: 20,
        offset_below: 32,
    }
}

const FHD: RectPx = RectPx {
    left: 0,
    top: 0,
    right: 1920,
    bottom: 1080,
};

// ---- 置き場所 ----

/// 真上・左右の中央合わせ・カーソルから上へ離す量（3.1）。
#[test]
fn place_directly_above_centred_with_offset() {
    let p = place(&input(pt(500, 500), FHD, 100, 40));
    // 中央: 500 - 100/2 = 450。下端はカーソルから 20 上: 500 - 20 - 40 = 440。
    assert_eq!(p, pt(450, 440));
    assert_eq!(p.y + 40, 500 - 20);
}

/// 上に収まらなければ真下へ返す（カーソルの絵の高さぶん離す）（3.5）。
#[test]
fn place_flips_below_when_above_does_not_fit() {
    let p = place(&input(pt(500, 50), FHD, 100, 40));
    assert_eq!(p, pt(450, 50 + 32));
}

/// 左右の端では作業領域の中へ寄せる（3.5）。
#[test]
fn place_clamps_left_and_right_edges() {
    assert_eq!(place(&input(pt(10, 500), FHD, 100, 40)), pt(0, 440));
    assert_eq!(place(&input(pt(1910, 500), FHD, 100, 40)), pt(1820, 440));
}

/// 上にも下にも収まらなければ、広い側に置いて作業領域の中へ寄せる（3.5）。
#[test]
fn place_neither_side_fits_uses_wider_side() {
    let short = rect(0, 0, 1920, 100);
    // 上の空き 60-20=40・下の空き 100-(60+32)=8 → 上の側。上端へ寄せる。
    assert_eq!(place(&input(pt(500, 60), short, 100, 60)), pt(450, 0));
    // 上の空き 40-20=20・下の空き 100-(40+32)=28 → 下の側。下端へ寄せる。
    assert_eq!(place(&input(pt(500, 40), short, 100, 60)), pt(450, 40));
}

/// 境目: 上の空きがちょうど 0 なら真上に収まる。上下の空きが同じなら上の側。
#[test]
fn place_boundaries_prefer_above() {
    // 上端がちょうど作業領域の上端: 60 - 20 - 40 = 0。
    assert_eq!(place(&input(pt(500, 60), FHD, 100, 40)), pt(450, 0));
    // 上の空き 44-20-60=-36・下の空き 100-(44+32+60)=-36 → 同じなら上の側。上端へ寄せる。
    let short = rect(0, 0, 1920, 100);
    assert_eq!(place(&input(pt(500, 44), short, 100, 60)), pt(450, 0));
}

/// 作業領域より大きければ左上を作業領域の左上に合わせる（3.5）。
#[test]
fn place_too_large_aligns_top_left() {
    assert_eq!(place(&input(pt(500, 500), FHD, 2000, 1200)), pt(0, 0));
    // 幅だけ大きい: 左は合わせ、上下はふつうに決める。
    assert_eq!(place(&input(pt(500, 500), FHD, 2000, 40)), pt(0, 440));
    let left_screen = rect(-1920, -200, 0, 880);
    assert_eq!(
        place(&input(pt(-900, 300), left_screen, 2000, 1200)),
        pt(-1920, -200)
    );
}

/// 負の座標の画面（左や上の画面）でも収まる（3.5）。
#[test]
fn place_on_screen_with_negative_coordinates() {
    let left_screen = rect(-1920, -200, 0, 880);
    // 左端に寄せ、上に収まらないので下へ返す。
    assert_eq!(
        place(&input(pt(-1915, -150), left_screen, 100, 40)),
        pt(-1920, -150 + 32)
    );
    // 真ん中ならふつうに真上。
    assert_eq!(
        place(&input(pt(-960, 300), left_screen, 100, 40)),
        pt(-1010, 240)
    );
}

/// 作業領域より小さければ、どこにマウスがあっても（作業領域の外でも）必ず収まる。
#[test]
fn place_always_inside_work_area_when_smaller() {
    let wa = rect(-1920, -200, 0, 880);
    for x in (-2100..=200).step_by(37) {
        for y in (-400..=1100).step_by(29) {
            let p = place(&input(pt(x, y), wa, 300, 120));
            assert!(
                p.x >= wa.left && p.x + 300 <= wa.right && p.y >= wa.top && p.y + 120 <= wa.bottom,
                "anchor=({x},{y}) -> {p:?}"
            );
        }
    }
}

// ---- 安全地帯 ----

/// 範囲 (100,100)-(200,150) と、右上に出たツールチップ (300,20)-(400,60)。
#[test]
fn safe_zone_tip_to_upper_right() {
    let range = rect(100, 100, 200, 150);
    let tip = Some(rect(300, 20, 400, 60));
    assert!(in_safe_zone(pt(150, 125), range, tip), "範囲の中");
    assert!(in_safe_zone(pt(350, 40), range, tip), "ツールチップの中");
    assert!(in_safe_zone(pt(250, 80), range, tip), "通り道の中");
    // 通り道の上の縁は x=250 で y=40。その上は外。
    assert!(!in_safe_zone(pt(250, 30), range, tip), "通り道の上");
    // 2 つを囲む外枠の角のうち、通り道の外側の角は外。
    assert!(!in_safe_zone(pt(100, 20), range, tip), "左上の角");
    assert!(!in_safe_zone(pt(399, 149), range, tip), "右下の角");
    assert!(!in_safe_zone(pt(500, 500), range, tip), "遠く");
}

/// 左下に出たツールチップ（もう一方の向きの角を確かめる）。
#[test]
fn safe_zone_tip_to_lower_left() {
    let range = rect(100, 100, 200, 150);
    let tip = Some(rect(0, 200, 80, 240));
    assert!(in_safe_zone(pt(90, 170), range, tip), "通り道の中");
    assert!(in_safe_zone(pt(40, 220), range, tip), "ツールチップの中");
    assert!(!in_safe_zone(pt(0, 100), range, tip), "左上の角");
    assert!(!in_safe_zone(pt(199, 239), range, tip), "右下の角");
}

/// 右下に出たツールチップ（右上と左下の角を確かめる）。
#[test]
fn safe_zone_tip_to_lower_right() {
    let range = rect(100, 100, 200, 150);
    let tip = Some(rect(300, 200, 400, 240));
    assert!(in_safe_zone(pt(250, 175), range, tip), "通り道の中");
    assert!(!in_safe_zone(pt(399, 100), range, tip), "右上の角");
    assert!(!in_safe_zone(pt(100, 239), range, tip), "左下の角");
}

/// ツールチップが出ていなければ範囲の矩形だけ（右と下の端は含まない）。
#[test]
fn safe_zone_without_tip_is_range_only() {
    let range = rect(100, 100, 200, 150);
    assert!(in_safe_zone(pt(100, 100), range, None));
    assert!(in_safe_zone(pt(199, 149), range, None));
    assert!(!in_safe_zone(pt(200, 125), range, None));
    assert!(!in_safe_zone(pt(150, 150), range, None));
    assert!(!in_safe_zone(pt(250, 80), range, None), "通り道は無い");
}

// ---- 折り返し ----

/// 入る文字数ごとに割る。
#[test]
fn force_break_splits_by_fitting_count() {
    let fit = |_: &str| 5;
    assert_eq!(force_break("abcdefghijkl", &fit), "abcde\r\nfghij\r\nkl");
    // 入る行は触らない。
    assert_eq!(force_break("abc", &fit), "abc");
}

/// 入る文字数は割った残りの先頭から問い直す。
#[test]
fn force_break_asks_fit_from_each_chunk_start() {
    // 幅 6 まで入る。半角は 1・それ以外は 2。
    let fit = |s: &str| {
        let mut width = 0;
        s.chars()
            .take_while(|c| {
                width += if c.is_ascii() { 1 } else { 2 };
                width <= 6
            })
            .count()
    };
    assert_eq!(
        force_break("abあいうcdefgh", &fit),
        "abあい\r\nうcdef\r\ngh"
    );
}

/// 既にある改行（空行・末尾の改行も）を保つ。
#[test]
fn force_break_keeps_existing_newlines() {
    let fit = |_: &str| 5;
    assert_eq!(
        force_break("abcdefg\r\nxy\r\n\r\n1234567\r\n", &fit),
        "abcde\r\nfg\r\nxy\r\n\r\n12345\r\n67\r\n"
    );
    // CR だけ・LF だけの改行もそのまま保つ。
    assert_eq!(
        force_break("abcdefg\rxy\nabcdefg", &fit),
        "abcde\r\nfg\rxy\nabcde\r\nfg"
    );
}

/// 多バイトの文字の途中で切らない。
#[test]
fn force_break_does_not_cut_multibyte_chars() {
    let fit = |_: &str| 4;
    assert_eq!(
        force_break("あいうえおかきくけこさ", &fit),
        "あいうえ\r\nおかきく\r\nけこさ"
    );
    assert_eq!(
        force_break("😀😀😀😀😀", &|_: &str| 2),
        "😀😀\r\n😀😀\r\n😀"
    );
}

/// 入る文字数が 0 と返っても 1 文字ずつ進む（止まらない）。
#[test]
fn force_break_zero_fit_still_advances() {
    assert_eq!(force_break("abc", &|_: &str| 0), "a\r\nb\r\nc");
}

// ---- 改行の揃え ----

/// LF だけ・CR LF・CR だけが混ざっても CR LF に揃う（3.3）。
#[test]
fn normalize_newlines_to_crlf() {
    assert_eq!(normalize_newlines("a\nb\r\nc\rd"), "a\r\nb\r\nc\r\nd");
    assert_eq!(normalize_newlines("\r\r\n"), "\r\n\r\n");
    assert_eq!(normalize_newlines("abc"), "abc");
}

// ---- 最大の幅 ----

/// 320 を DPI で換算し、作業領域の幅を越えない（3.4）。
#[test]
fn max_tip_width_scales_with_dpi_and_caps_at_work_area() {
    assert_eq!(max_tip_width(96, FHD), 320);
    assert_eq!(max_tip_width(144, FHD), 480);
    assert_eq!(max_tip_width(96, rect(0, 0, 300, 1080)), 300);
    assert_eq!(max_tip_width(144, rect(-1920, 0, -1500, 1080)), 420);
}

/// 版 6 には、窓の DPI の倍率を掛け戻すと物理の最大の幅に収まる幅を渡す（6.3）。
#[test]
fn logical_max_width_divides_back_without_exceeding_physical() {
    assert_eq!(logical_max_width(640, 192), 320);
    assert_eq!(logical_max_width(480, 144), 320);
    assert_eq!(logical_max_width(320, 96), 320);
    // 作業領域で頭打ちにした割り切れない幅は切り捨てる（掛け戻して 419.x ≦ 420）。
    assert_eq!(logical_max_width(420, 144), 280);
    assert_eq!(logical_max_width(421, 144), 280);
    for (physical, dpi) in [(421, 144), (999, 120), (1001, 168), (333, 240)] {
        // OS が四捨五入で掛け戻しても越えない。
        let back = (i64::from(logical_max_width(physical, dpi)) * i64::from(dpi) + 48) / 96;
        assert!(
            back <= i64::from(physical),
            "{physical}@{dpi}: 掛け戻して {back}"
        );
    }
}
