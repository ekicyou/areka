// =============================================================================
// task 7.5 — move 決定論檻 全網羅 監査（audit + gap-fill）
//
// design.md Testing Strategy の 2 項目を既存檻へ 1:1 で写像し、全項目が具体的
// （fail-if-broken）に固定済みであることを監査した結果、追加檻は不要（gap ゼロ）と確定。
// 監査時点の写像（項目 → 檻）:
//
// Unit Tests item 4「parse_move_directive 檻（R5.2/5.4）」（下記 `mod tests`）:
//   - 正典省略既定〔fix/fix/0/screen/left.top〕 → `canon_omission_defaults`
//     （空 positional／空トークン埋めの両形で MoveDirective 構造体を厳密一致 assert）
//   - 裸 base≡base.base                        → `bare_base_equals_base_base`
//   - time>0 縮退                              → `timed_move_kept_and_recorded`
//   - 名前付き形縮退                            → `named_form_is_degraded_err`（純名前形＋混在検出）
//   - 基準語彙**全種**の受理/縮退分類          → `base_vocab_acceptance_and_classification`
//     数値スコープ 0/1/2＝受理（is_m1_derived∧UnsupportedBase 記録なし）／
//     screen・primaryscreen・me・global の 4 語＝各 variant 受理＋UnsupportedBase 記録／
//     未知語＝Err(UnknownBase)。MoveBase の全 5 variant＋未知を網羅。
//   - （fixture parse 検算）                    → `fixture_move_353_scope1`
//   - （防御的 Err）                            → `unparsable_axis_is_err`
//
// Integration Tests item 4「move 経路檻（R5.1/5.3/5.5/R6/9.5）」（`mod apply_move_tests`）:
//   - fixture 検算物理座標                      → `apply_moves_target_to_fixture_position`（Point{697,800}）
//   - バルーン随伴 offset 維持                  → `apply_keeps_balloon_offset`
//   - 対象不在 warn+false                       → `apply_target_absent_returns_false_without_mutation`
//   - Anchored 不変（対象**と**基準の両窓）     → `apply_leaves_anchored_bit_identical`
//   - （非スコープ基準 warn+false）             → `apply_non_scope_base_returns_false`
//
// 監査結論: 両項目とも欠落・弱檻（トートロジー/部分網羅）なし。全 26 檻が具体 assert で緑。
// =============================================================================

use areka_emo_compose::ScaleRatio;
use super::*;

fn toks(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// 正典省略時既定（fix/fix/0/screen/left.top）。空トークン列でも決定論的に既定へ落ちる。
#[test]
fn canon_omission_defaults() {
    let d = parse_move_directive(0, &[]).expect("空 positional は既定へ落ちて Ok");
    assert_eq!(
        d,
        MoveDirective {
            scope: 0,
            x: AxisSpec::Fix,
            y: AxisSpec::Fix,
            duration_ms: 0,
            base: MoveBase::Screen,
            base_offset: RefPoint::LEFT_TOP,
            move_offset: RefPoint::LEFT_TOP,
        }
    );
    // 空文字トークンで明示的に埋めても同じ既定（省略と空は同義・R4.2 の空トークン意味論）。
    let d2 = parse_move_directive(0, &toks(&["", "", "", "", "", ""]))
        .expect("空トークン埋めも既定へ落ちる");
    assert_eq!(d, d2);
}

/// 裸 `base`（ドット無し）≡ `base.base`（正典形式 `X.Y` の de-facto・R5.2 対応表）。
#[test]
fn bare_base_equals_base_base() {
    let bare = parse_move_directive(0, &toks(&["", "", "", "0", "base", "base"]))
        .expect("裸 base は Ok");
    let dotted = parse_move_directive(0, &toks(&["", "", "", "0", "base.base", "base.base"]))
        .expect("base.base は Ok");
    assert_eq!(bare.base_offset, RefPoint::BASE_BASE);
    assert_eq!(bare.move_offset, RefPoint::BASE_BASE);
    assert_eq!(bare, dotted, "裸 base と base.base は完全等価");
}

/// time>0 は `Ok` のまま `duration_ms` を保持し、記録として縮退が surface する（R5.4）。
#[test]
fn timed_move_kept_and_recorded() {
    let d = parse_move_directive(0, &toks(&["100", "", "2500", "0", "base", "base"]))
        .expect("time>0 は縮退記録付きでも Ok");
    assert_eq!(d.duration_ms, 2500);
    assert!(
        d.m1_degradations()
            .contains(&MoveDegradation::TimedMoveImmediate { duration_ms: 2500 }),
        "time>0 は m1_degradations に TimedMoveImmediate として記録される"
    );
}

/// 名前付き `--key=value` 形は M1 縮退＝`Err(NamedForm)`（記録付きスキップ・語彙は将来 additive）。
#[test]
fn named_form_is_degraded_err() {
    let err = parse_move_directive(
        0,
        &toks(&["--X=80", "--Y=-400", "--time=2500", "--base=screen"]),
    )
    .expect_err("名前付き形は Err へ縮退");
    assert_eq!(err, MoveDegradation::NamedForm("--X=80".to_string()));

    // positional に 1 トークンだけ名前付きが混入しても検出する。
    let err2 = parse_move_directive(0, &toks(&["-353", "", "", "--base=screen"]))
        .expect_err("混在でも名前付きを検出");
    assert!(matches!(err2, MoveDegradation::NamedForm(_)));
}

/// 基準語彙の受理と縮退分類（数値スコープ＝実導出／screen 等＝語彙保持＋UnsupportedBase 記録）。
#[test]
fn base_vocab_acceptance_and_classification() {
    // 数値スコープ＝M1 実導出（m1_degradations に基準縮退なし）。
    for scope_str in ["0", "1", "2"] {
        let d = parse_move_directive(0, &toks(&["", "", "", scope_str, "base", "base"]))
            .expect("数値スコープ基準は Ok");
        let n: u32 = scope_str.parse().unwrap();
        assert_eq!(d.base, MoveBase::Scope(n));
        assert!(d.base.is_m1_derived());
        assert!(
            !d.m1_degradations()
                .iter()
                .any(|g| matches!(g, MoveDegradation::UnsupportedBase(_))),
            "数値スコープ基準は縮退記録を持たない"
        );
    }

    // 非スコープ語＝語彙保持（Ok）＋UnsupportedBase 記録（M1 非実導出）。
    for (word, expected) in [
        ("screen", MoveBase::Screen),
        ("primaryscreen", MoveBase::PrimaryScreen),
        ("me", MoveBase::Me),
        ("global", MoveBase::Global),
    ] {
        let d = parse_move_directive(0, &toks(&["", "", "", word, "base", "base"]))
            .unwrap_or_else(|_| panic!("基準語 {word} は語彙保持で Ok"));
        assert_eq!(d.base, expected);
        assert!(!d.base.is_m1_derived());
        assert!(
            d.m1_degradations()
                .contains(&MoveDegradation::UnsupportedBase(expected)),
            "非スコープ基準 {word} は UnsupportedBase として記録される"
        );
    }

    // 未知の基準語は防御的に Err（非 panic）。
    let err = parse_move_directive(0, &toks(&["", "", "", "nonsense"]))
        .expect_err("未知基準は Err");
    assert_eq!(err, MoveDegradation::UnknownBase("nonsense".to_string()));
}

/// fixture `\1\![move,-353,,,0,base,base]`（scope 1）の完全一致（R9.3 の直入力檻の parse 部）。
#[test]
fn fixture_move_353_scope1() {
    let d = parse_move_directive(1, &toks(&["-353", "", "", "0", "base", "base"]))
        .expect("fixture move は Ok");
    assert_eq!(
        d,
        MoveDirective {
            scope: 1,
            x: AxisSpec::Px(-353),
            y: AxisSpec::Fix,
            duration_ms: 0,
            base: MoveBase::Scope(0),
            base_offset: RefPoint::BASE_BASE,
            move_offset: RefPoint::BASE_BASE,
        }
    );
    // fixture は数値スコープ基準＋time=0 ゆえ M1 縮退記録は空（実導出の正規経路）。
    assert!(d.m1_degradations().is_empty());
}

/// 軸トークンが fix でも i32 でもない場合は防御的に Err（非 panic）。
#[test]
fn unparsable_axis_is_err() {
    let err = parse_move_directive(0, &toks(&["abc"]))
        .expect_err("非数値・非 fix の軸は Err");
    assert_eq!(
        err,
        MoveDegradation::UnparsableAxis {
            axis: Axis::X,
            token: "abc".to_string(),
        }
    );
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
    let directive = parse_move_directive(1, &toks(&["-353", "", "", "0", "base", "base"]))
        .expect("fixture move は Ok");
    let base = win(1000, 500, 400, 687);
    let target = win(1200, 800, 300, 434);

    let pos = resolve_move_target_position(&CanonDefaultBasepos, &base, &target, &directive, ScaleRatio::ONE)
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
    let directive = parse_move_directive(1, &toks(&["-353", "", "", "0", "base", "base"]))
        .expect("fixture move は Ok");
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
    let directive = parse_move_directive(1, &toks(&["100", "50", "", "0", "base", "base"]))
        .expect("両軸 Px は Ok");
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
    let directive = parse_move_directive(1, &toks(&["", "50", "", "0", "base", "base"]))
        .expect("y=Px は Ok");
    assert_eq!(directive.x, AxisSpec::Fix);
    assert_eq!(directive.y, AxisSpec::Px(50));

    let base = win(1000, 500, 400, 687);
    let target = win(1200, 800, 300, 434);
    let pos = resolve_move_target_position(&CanonDefaultBasepos, &base, &target, &directive, ScaleRatio::ONE)
        .expect("算出できる");

    // X=Fix → 現状維持
    assert_eq!(pos.x, 1200, "X=Fix は対象窓の現在 X を現状維持");
    // y' = 500 + 687 + 50 − 434 = 803
    assert_eq!(pos.y, 803, "y' = pos0.y + h0 + dy − h1（下端 basepos で対称）");
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

    let directive = parse_move_directive(1, &toks(&["-353", "", "", "0", "base", "base"]))
        .expect("fixture move は Ok");
    let base = win(1000, 500, 400, 687);
    let target = win(1200, 800, 300, 434);

    let pos = resolve_move_target_position(&FullSizeBasepos, &base, &target, &directive, ScaleRatio::ONE)
        .expect("算出できる");
    // x' = pos0.x + w0 − 353 − w1 = 1000 + 400 − 353 − 300 = 747（Canon の 697 とは別）
    assert_eq!(pos.x, 747, "resolver の basepos 出力（幅そのもの）が使われる");
    assert_ne!(pos.x, 697, "正典既定（中央）とは異なる＝シームが効いている");
}

/// 両軸 Fix（現状維持のみ）は基準窓の寸法欠落でも対象窓の現在位置を返す（no-op 移動）。
#[test]
fn both_fix_returns_current_position() {
    let directive = parse_move_directive(1, &toks(&["", "", "", "0", "base", "base"]))
        .expect("両軸省略は Ok");
    assert_eq!(directive.x, AxisSpec::Fix);
    assert_eq!(directive.y, AxisSpec::Fix);

    // 基準窓は寸法なし（現状維持のみゆえ basepos 不要）。
    let base = WindowPos::default();
    let target = win(1200, 800, 300, 434);
    let pos = resolve_move_target_position(&CanonDefaultBasepos, &base, &target, &directive, ScaleRatio::ONE)
        .expect("両軸 Fix は現状維持で算出できる");
    assert_eq!(pos, PointPx { x: 1200, y: 800 });
}

/// 位置・寸法欠落（窓生成前等）で Px 軸を含むと算出不能＝`None`（呼び出し側が warn＋継続・R5.5）。
#[test]
fn missing_geometry_with_px_axis_is_none() {
    let directive = parse_move_directive(1, &toks(&["-353", "", "", "0", "base", "base"]))
        .expect("fixture move は Ok");
    // 対象窓に寸法が無い（basepos 算出不能）。
    let base = win(1000, 500, 400, 687);
    let target = WindowPos {
        position: Some(Point { x: 1200, y: 800 }),
        size: None,
        ..Default::default()
    };
    assert!(
        resolve_move_target_position(&CanonDefaultBasepos, &base, &target, &directive, ScaleRatio::ONE).is_none(),
        "Px 軸で寸法欠落は算出不能＝None"
    );
}
