// =============================================================================
// move_cue.rs の座標算出の決定論檻（basepos 型シーム＋`resolve_move_target_position`）。
//
// 引数の読みの檻（両方の形・既定・置き換えの記録）は `move_cue_args_tests.rs` へ移した
// （areka-P0-move-canon 1.1）。旧い「名前付きの形は Err」「非スコープ基準は縮退記録」の檻は、
// 読みが常に指令を返す形へ改めたので陳腐化として外し、置き換えの檻として新しい兄弟で書き改めた。
// =============================================================================

use super::*;
use areka_emo_compose::ScaleRatio;

fn toks(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

// -------------------------------------------------------------------------
// basepos 型シーム＋座標算出（task 7.2・R5.2・全て物理 px・R-6 対策）
// -------------------------------------------------------------------------

use wintf::ecs::{Point, SizeI, WindowPos};

/// 位置＋寸法を持つ WindowPos（物理 px・follow.rs のヘルパ流儀）。
fn win(x: i32, y: i32, w: i32, h: i32) -> WindowPos {
    WindowPos {
        position: Some(Point { x, y }),
        size: Some(SizeI::new(w, h)),
        ..Default::default()
    }
}

/// 正典既定 basepos は (幅÷2, 高さ＝下端)（R5.2・A-1）。奇数幅は整数切り捨て。
#[test]
fn canon_default_basepos_is_half_width_and_bottom() {
    assert_eq!(
        CanonDefaultBasepos.basepos(SizeI::new(400, 687)),
        PointPx { x: 200, y: 687 },
        "x=幅÷2・y=下端（＝height・窓左上原点相対）"
    );
    // 奇数幅は整数除算で切り捨て（435÷2=217）。
    assert_eq!(
        CanonDefaultBasepos.basepos(SizeI::new(435, 100)),
        PointPx { x: 217, y: 100 }
    );
}

/// fixture `\1\![move,-353,,,0,base,base]` の X 検算＝`pos0.x + w0/2 − 353 − w1/2`・Y 現状維持。
/// base=scope0=むらさき窓 pos0=(1000,500) size0=(400,687)、target=scope1=エモ窓
/// pos1=(1200,800) size1=(300,434) の具体値で完全一致を固定する。
#[test]
fn fixture_move_353_x_and_y_unchanged() {
    let directive =
        parse_move_directive(1, &toks(&["-353", "", "", "0", "base", "base"])).directive;
    let base = win(1000, 500, 400, 687);
    let target = win(1200, 800, 300, 434);

    let pos = resolve_move_target_position(
        &CanonDefaultBasepos,
        &base,
        &target,
        &directive,
        ScaleRatio::ONE,
    )
    .expect("位置・寸法が揃うので算出できる");

    // x' = 1000 + 200 − 353 − 150 = 697
    assert_eq!(pos.x, 697, "x' = pos0.x + w0/2 − 353 − w1/2");
    // Y は Fix ゆえ target の現在 Y を現状維持
    assert_eq!(pos.y, 800, "Y=Fix は対象窓の現在 Y を現状維持");
}

/// 台本オフセットは k 倍される（作者基準 px → 物理 px・`windowposition.x/y` と同じ写像）。
///
/// 同じ fixture を k=2/1 で解くと、`base_pos`／`basepos` は既に物理 px の入力ゆえそのまま、
/// **`dx` だけが −353 → −706** になる。素通し実装（k 非適用）なら k=1 と同じ 697 が返るため、
/// 本檻は「k を掛け忘れる退行」を厳密に判別する。
///
/// 実機の裏づけ（emo2・拡大率 200%）: k を掛けないと二体が 365px 重なり、過剰分 353px は
/// スケールし損ねた `dx` そのものだった。
#[test]
fn script_offset_is_scaled_by_k_not_passed_through() {
    let directive =
        parse_move_directive(1, &toks(&["-353", "", "", "0", "base", "base"])).directive;
    // 物理 px の入力（k=2 の実機で観測される寸法相当）。
    let base = win(1000, 500, 400, 687);
    let target = win(1200, 800, 300, 434);
    let k2 = ScaleRatio::new(2, 1).expect("2/1 は正当な比");

    let pos = resolve_move_target_position(&CanonDefaultBasepos, &base, &target, &directive, k2)
        .expect("位置・寸法が揃うので算出できる");

    // x' = 1000 + 200 + (2 × −353) − 150 = 344
    assert_eq!(
        pos.x, 344,
        "x' = pos0.x + w0/2 + k·(−353) − w1/2（dx が k 倍される）"
    );
    // 素通し（k 非適用）の値へ戻ってはならない＝退行の否定 assert。
    assert_ne!(
        pos.x, 697,
        "台本オフセットを素通しにしてはならない（k=1 の値と一致するのは退行）"
    );
    // Y は Fix ゆえ k に依らず現状維持（k 倍が Fix 軸へ漏れない檻）。
    assert_eq!(pos.y, 800, "Y=Fix は k に依らず現状維持");
}

/// k 倍は符号を保存する（正のオフセットが負へ化けない・`scale_signed` の符号規約）。
#[test]
fn script_offset_scaling_preserves_sign_on_both_axes() {
    let directive =
        parse_move_directive(1, &toks(&["100", "50", "", "0", "base", "base"])).directive;
    let base = win(1000, 500, 400, 687);
    let target = win(1200, 800, 300, 434);
    let k2 = ScaleRatio::new(2, 1).expect("2/1 は正当な比");

    let pos = resolve_move_target_position(&CanonDefaultBasepos, &base, &target, &directive, k2)
        .expect("算出できる");

    // x' = 1000 + 200 + 200 − 150 = 1250（+100 が +200 へ）
    assert_eq!(pos.x, 1250, "正の dx は正のまま k 倍される");
    // y' = 500 + 687 + 100 − 434 = 853（+50 が +100 へ）
    assert_eq!(pos.y, 853, "正の dy は正のまま k 倍される");
}

/// Y=Px 経路も対称に効く（Y が「常に現状維持」へ hardcode されていないことの檻）。
/// x=Fix・y=Px(50) → x' は target.x 現状維持、y' = pos0.y + h0 + 50 − h1。
#[test]
fn y_px_axis_is_symmetric_not_hardcoded() {
    let directive = parse_move_directive(1, &toks(&["", "50", "", "0", "base", "base"])).directive;
    assert_eq!(directive.x, AxisSpec::Fix);
    assert_eq!(directive.y, AxisSpec::Px(50));

    let base = win(1000, 500, 400, 687);
    let target = win(1200, 800, 300, 434);
    let pos = resolve_move_target_position(
        &CanonDefaultBasepos,
        &base,
        &target,
        &directive,
        ScaleRatio::ONE,
    )
    .expect("算出できる");

    // X=Fix → 現状維持
    assert_eq!(pos.x, 1200, "X=Fix は対象窓の現在 X を現状維持");
    // y' = 500 + 687 + 50 − 434 = 803
    assert_eq!(
        pos.y, 803,
        "y' = pos0.y + h0 + dy − h1（下端 basepos で対称）"
    );
}

/// BaseposResolver はトレイト＝差替シーム。テストダブルが別 basepos を供給すると
/// 算出結果がその出力を用いて変わる（宣言 point.basepos の追跡 spec 差替点の証明）。
#[test]
fn computation_honors_resolver_seam() {
    /// basepos を「幅・高さそのもの（右下端）」とする差替ダブル（正典既定＝中央下端とは別物）。
    struct FullSizeBasepos;
    impl BaseposResolver for FullSizeBasepos {
        fn basepos(&self, window_size: SizeI) -> PointPx {
            PointPx {
                x: window_size.width,
                y: window_size.height,
            }
        }
    }

    let directive =
        parse_move_directive(1, &toks(&["-353", "", "", "0", "base", "base"])).directive;
    let base = win(1000, 500, 400, 687);
    let target = win(1200, 800, 300, 434);

    let pos = resolve_move_target_position(
        &FullSizeBasepos,
        &base,
        &target,
        &directive,
        ScaleRatio::ONE,
    )
    .expect("算出できる");
    // x' = pos0.x + w0 − 353 − w1 = 1000 + 400 − 353 − 300 = 747（Canon の 697 とは別）
    assert_eq!(
        pos.x, 747,
        "resolver の basepos 出力（幅そのもの）が使われる"
    );
    assert_ne!(pos.x, 697, "正典既定（中央）とは異なる＝シームが効いている");
}

/// 両軸 Fix（現状維持のみ）は基準窓の寸法欠落でも対象窓の現在位置を返す（no-op 移動）。
#[test]
fn both_fix_returns_current_position() {
    let directive = parse_move_directive(1, &toks(&["", "", "", "0", "base", "base"])).directive;
    assert_eq!(directive.x, AxisSpec::Fix);
    assert_eq!(directive.y, AxisSpec::Fix);

    // 基準窓は寸法なし（現状維持のみゆえ basepos 不要）。
    let base = WindowPos::default();
    let target = win(1200, 800, 300, 434);
    let pos = resolve_move_target_position(
        &CanonDefaultBasepos,
        &base,
        &target,
        &directive,
        ScaleRatio::ONE,
    )
    .expect("両軸 Fix は現状維持で算出できる");
    assert_eq!(pos, PointPx { x: 1200, y: 800 });
}

/// 位置・寸法欠落（窓生成前等）で Px 軸を含むと算出不能＝`None`（呼び出し側が warn＋継続・R5.5）。
#[test]
fn missing_geometry_with_px_axis_is_none() {
    let directive =
        parse_move_directive(1, &toks(&["-353", "", "", "0", "base", "base"])).directive;
    // 対象窓に寸法が無い（basepos 算出不能）。
    let base = win(1000, 500, 400, 687);
    let target = WindowPos {
        position: Some(Point { x: 1200, y: 800 }),
        size: None,
        ..Default::default()
    };
    assert!(
        resolve_move_target_position(
            &CanonDefaultBasepos,
            &base,
            &target,
            &directive,
            ScaleRatio::ONE
        )
        .is_none(),
        "Px 軸で寸法欠落は算出不能＝None"
    );
}
