
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use log_capture_kit::count_levels;

use super::{ImagePx, PhysicalPx, ScaleContract, TextRegion};
use crate::writing::WritingMode;

/// fixture 実測のバルーン画像原寸（balloons0.png・image px）。
const FIXTURE_IMAGE_SIZE: (u32, u32) = (400, 224);

/// テスト用 BalloonModel 生成ヘルパ（幾何成分だけ指定・font/windowposition は未指定）。
fn model(
    origin: (Option<i32>, Option<i32>),
    wordwrap: (Option<i32>, Option<i32>),
    validrect: (Option<i32>, Option<i32>, Option<i32>, Option<i32>),
) -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(origin.0, origin.1),
        WordWrapPoint::new(wordwrap.0, wordwrap.1),
        ValidRect::new(validrect.0, validrect.1, validrect.2, validrect.3),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    )
}

/// fixture 実測値（2層マージ後）の BalloonModel:
/// **origin は宣言しない**・wordwrappoint.x,-49（balloons0s.txt 上書き）・
/// validrect top,46／bottom,-56／left,36／right,-44。
///
/// origin の宣言が無いのは実フィクスチャに追随した結果である——`emo2-vertical`／
/// `emo2-kakukaku` の `descript.txt` はかつて validrect 外の `origin.x,0`／`origin.y,0`
/// を宣言していたが、spec `areka-P0-balloon-vertical-canon` の要件 10.9 で正典推奨形
/// （「通常は指定せず validrect の定義に任せる」）へ是正され、宣言そのものが消えた。
/// 未宣言時の書字開始角への縮退（要件 3.11）は不変なので、本ヘルパを使う檻の
/// 開始点期待値は是正の前後で変わらない。
fn fixture_model() -> BalloonModel {
    model(
        (None, None),
        (Some(-49), Some(0)),
        (Some(46), Some(-56), Some(36), Some(-44)),
    )
}

/// クロージャを共有のログ捕捉窓の中で実行し（戻り値, WARN 件数）を返す。
///
/// 件数の集計は硬化機構の唯一の定義元 `log-capture-kit` の [`count_levels`] に委ねる。
/// 戻り値の組は移行前と同一で、呼出側の判定内容は変わらない。
fn count_warns<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let (value, counts) = count_levels(f);
    (value, counts.warn)
}

// ── R4.3/R4.4: 負値=反対辺基準の解決（fixture 実測値で非退化矩形） ──

/// fixture 実測 validrect（top46/bottom-56/left36/right-44・画像 400×224）が
/// 画像座標空間の絶対矩形 (36,46)-(356,168) へ解決され、非退化である。
#[test]
fn fixture_validrect_resolves_to_nondegenerate_absolute_rect() {
    let region = TextRegion::resolve(
        &fixture_model(),
        FIXTURE_IMAGE_SIZE,
        WritingMode::HorizontalTb,
    );
    assert_eq!(region.left(), 36.0);
    assert_eq!(region.top(), 46.0);
    assert_eq!(region.right(), 356.0); // 400 + (-44)
    assert_eq!(region.bottom(), 168.0); // 224 + (-56)
    assert!(region.right() > region.left(), "非退化（幅 > 0）");
    assert!(region.bottom() > region.top(), "非退化（高さ > 0）");
}

/// 非負値は絶対座標として素通し（resolve(v, extent) = v for v >= 0）。
#[test]
fn nonnegative_validrect_passes_through_as_absolute() {
    let region = TextRegion::resolve(
        &model(
            (None, None),
            (None, None),
            (Some(10), Some(200), Some(20), Some(300)),
        ),
        FIXTURE_IMAGE_SIZE,
        WritingMode::HorizontalTb,
    );
    assert_eq!(region.left(), 20.0);
    assert_eq!(region.top(), 10.0);
    assert_eq!(region.right(), 300.0);
    assert_eq!(region.bottom(), 200.0);
}

/// validrect 成分 None は画像全域の辺へ縮退（left/top→0・right→幅・bottom→高さ）。
#[test]
fn missing_validrect_components_fall_back_to_image_edges() {
    let region = TextRegion::resolve(
        &model((None, None), (None, None), (None, None, None, None)),
        FIXTURE_IMAGE_SIZE,
        WritingMode::HorizontalTb,
    );
    assert_eq!(region.left(), 0.0);
    assert_eq!(region.top(), 0.0);
    assert_eq!(region.right(), 400.0);
    assert_eq!(region.bottom(), 224.0);
}

/// fixture の descript 基層のみ（validrect 全 0）は退化矩形＝warn を記録しつつ返す
/// （log-first・縮退継続）。2層マージ後のみ非退化になる fixture 実態の再現。
#[test]
fn base_layer_only_validrect_is_degenerate_and_warns() {
    // descript.txt 基層実測: origin 0,0・wordwrappoint.x,-34・validrect 全 0。
    let base = model(
        (Some(0), Some(0)),
        (Some(-34), Some(0)),
        (Some(0), Some(0), Some(0), Some(0)),
    );
    let (region, warns) = count_warns(|| {
        TextRegion::resolve(&base, FIXTURE_IMAGE_SIZE, WritingMode::HorizontalTb)
    });
    assert_eq!(region.left(), 0.0);
    assert_eq!(region.right(), 0.0);
    assert!(warns >= 1, "退化矩形は warn を記録する");
}

/// 2層マージ（balloon-parse 実機構）を通した fixture 再現:
/// 基層（退化）＋画像別上書きの合成後のみ非退化になる。
#[test]
fn two_layer_merged_fixture_yields_nondegenerate_region() {
    use areka_parsers::balloon::parse;
    use std::collections::BTreeMap;

    // fixture 実測の関連キー subset（descript.txt 基層／balloons0s.txt 上書き層）。
    // origin は実フィクスチャと同じく宣言しない（要件 10.9 の是正後の姿）——本檻の
    // 関心は「2 層マージ後にのみ非退化領域が成立する」ことであり、開始点は
    // 未宣言→書字開始角の縮退（要件 3.11）で (36,46) になる。
    let descript: BTreeMap<String, String> = [
        ("wordwrappoint.x", "-34"),
        ("wordwrappoint.y", "0"),
        ("validrect.top", "0"),
        ("validrect.bottom", "0"),
        ("validrect.left", "0"),
        ("validrect.right", "0"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .collect();
    let image: BTreeMap<String, String> = [
        ("wordwrappoint.x", "-49"),
        ("validrect.top", "46"),
        ("validrect.bottom", "-56"),
        ("validrect.left", "36"),
        ("validrect.right", "-44"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .collect();
    let merged = parse(&descript, Some(&image));
    let (region, warns) = count_warns(|| {
        TextRegion::resolve(&merged, FIXTURE_IMAGE_SIZE, WritingMode::HorizontalTb)
    });
    assert_eq!(
        (region.left(), region.top(), region.right(), region.bottom()),
        (36.0, 46.0, 356.0, 168.0)
    );
    assert_eq!(warns, 0, "非退化矩形は warn を記録しない");
    assert_eq!(region.start(), (36.0, 46.0));
    assert_eq!(region.wrap_threshold(), 351.0); // 400 + (-49)
}

// ── 描画開始点の解決（範囲内の宣言は宣言どおり・範囲外と未宣言は書字開始角） ──

/// origin を宣言しない fixture は書字開始角 (left,top)=(36,46) から書き始める
/// （要件 3.11 の縮退・SSP 表示実態と整合する期待座標）。
#[test]
fn fixture_without_origin_declaration_starts_at_writing_corner() {
    let region = TextRegion::resolve(
        &fixture_model(),
        FIXTURE_IMAGE_SIZE,
        WritingMode::HorizontalTb,
    );
    assert_eq!(region.start(), (36.0, 46.0));
}

/// validrect 内の origin はそのまま描画開始点になる。
#[test]
fn in_range_origin_is_kept_as_start_point() {
    let m = model(
        (Some(100), Some(50)),
        (None, None),
        (Some(46), Some(-56), Some(36), Some(-44)),
    );
    let region = TextRegion::resolve(&m, FIXTURE_IMAGE_SIZE, WritingMode::HorizontalTb);
    assert_eq!(region.start(), (100.0, 50.0));
}

/// origin 成分 None は書字開始角へ寄る（成分独立）。
#[test]
fn missing_origin_components_fall_back_to_start_corner() {
    let m = model(
        (None, None),
        (None, None),
        (Some(46), Some(-56), Some(36), Some(-44)),
    );
    let region = TextRegion::resolve(&m, FIXTURE_IMAGE_SIZE, WritingMode::HorizontalTb);
    assert_eq!(region.start(), (36.0, 46.0));
}

/// 範囲外の成分だけが書字開始角へ落ち、他方の成分は宣言どおり残る
/// （spec `areka-P0-balloon-origin-outside-validrect` の要件 1.1・1.2）。
#[test]
fn out_of_range_origin_component_falls_back_to_start_corner_independently() {
    let m = model(
        (Some(100), Some(0)), // x は範囲内・y(0) は top(46) より上＝範囲外
        (None, None),
        (Some(46), Some(-56), Some(36), Some(-44)),
    );
    let region = TextRegion::resolve(&m, FIXTURE_IMAGE_SIZE, WritingMode::HorizontalTb);
    // y だけが validrect.top(46)＝書字開始角の y へ落ち、x は宣言どおり 100 のまま。
    assert_eq!(region.start(), (100.0, 46.0));
}

/// origin の負値は反対辺基準で解決してから範囲の内外を判定する（要件 3.7）。
/// 本例の解決値は範囲内なので、そのまま描画開始点になる。
#[test]
fn negative_origin_resolves_from_opposite_edge() {
    // origin.x,-100 → 400-100=300・origin.y,-100 → 224-100=124。
    let m = model(
        (Some(-100), Some(-100)),
        (None, None),
        (Some(46), Some(-56), Some(36), Some(-44)),
    );
    let region = TextRegion::resolve(&m, FIXTURE_IMAGE_SIZE, WritingMode::HorizontalTb);
    assert_eq!(region.start(), (300.0, 124.0));
}

/// vertical_rl の書字開始角は validrect 右上＝origin 未宣言なら x は右端側になる。
#[test]
fn vertical_rl_start_corner_is_top_right() {
    let region = TextRegion::resolve(
        &fixture_model(),
        FIXTURE_IMAGE_SIZE,
        WritingMode::VerticalRl,
    );
    assert_eq!(region.start(), (356.0, 46.0));
}

/// vertical_lr の書字開始角は validrect 左上（horizontal_tb と同じ角・origin 未宣言時）。
#[test]
fn vertical_lr_start_corner_is_top_left() {
    let region = TextRegion::resolve(
        &fixture_model(),
        FIXTURE_IMAGE_SIZE,
        WritingMode::VerticalLr,
    );
    assert_eq!(region.start(), (36.0, 46.0));
}

// ── 折返し閾値（軸解釈は WritingMode 依存・負値=反対辺基準） ──

/// 横書き: wordwrappoint.x（fixture -49・負値=右辺基準）→ 400-49=351。
#[test]
fn horizontal_wrap_threshold_resolves_wordwrappoint_x() {
    let region = TextRegion::resolve(
        &fixture_model(),
        FIXTURE_IMAGE_SIZE,
        WritingMode::HorizontalTb,
    );
    assert_eq!(region.wrap_threshold(), 351.0);
}

/// 縦書き（vertical_rl／vertical_lr）: wordwrappoint.y（負値=下辺基準）。
#[test]
fn vertical_wrap_threshold_resolves_wordwrappoint_y() {
    let m = model(
        (None, None),
        (Some(-49), Some(-30)),
        (Some(46), Some(-56), Some(36), Some(-44)),
    );
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        let region = TextRegion::resolve(&m, FIXTURE_IMAGE_SIZE, mode);
        assert_eq!(region.wrap_threshold(), 194.0, "224 + (-30) for {mode:?}");
    }
}

/// fixture 実測 wordwrappoint.y,0 は 0 のまま忠実に解決される（縦書き折返しの
/// 退化は design 織り込み済み・退化補正はレイアウト層の領分でなく本層は転記解決に徹する）。
#[test]
fn degenerate_wordwrappoint_y_zero_is_resolved_faithfully() {
    let region = TextRegion::resolve(
        &fixture_model(),
        FIXTURE_IMAGE_SIZE,
        WritingMode::VerticalRl,
    );
    assert_eq!(region.wrap_threshold(), 0.0);
}

/// 折返し点の成分 None は行内軸の validrect 遠辺へ縮退
/// （横書き→right・縦書き→bottom＝領域端での自然折返し）。
#[test]
fn missing_wordwrappoint_falls_back_to_validrect_far_edge() {
    let m = model(
        (None, None),
        (None, None),
        (Some(46), Some(-56), Some(36), Some(-44)),
    );
    let horizontal = TextRegion::resolve(&m, FIXTURE_IMAGE_SIZE, WritingMode::HorizontalTb);
    assert_eq!(horizontal.wrap_threshold(), 356.0);
    let vertical = TextRegion::resolve(&m, FIXTURE_IMAGE_SIZE, WritingMode::VerticalRl);
    assert_eq!(vertical.wrap_threshold(), 168.0);
}

// ── R4.6/R10.4: DPI/スケール契約（画像座標空間と物理座標空間の 2 空間のみ） ──

/// author_dpi 未指定（fixture 実態: dpi キー無し）は 96 へ既定化する（ukadoc 正典）。
#[test]
fn author_dpi_defaults_to_96() {
    assert_eq!(ScaleContract::new(1.0, None).author_dpi, 96);
    assert_eq!(ScaleContract::new(1.0, Some(144)).author_dpi, 144);
}

/// 現行契約 k=1.0（物理 1:1 表示）: 変換は恒等。
#[test]
fn scale_one_maps_identically() {
    let contract = ScaleContract::new(1.0, None);
    assert_eq!(contract.to_physical(ImagePx(36.0)), PhysicalPx(36.0));
    assert_eq!(contract.to_image(PhysicalPx(168.0)), ImagePx(168.0));
    assert_eq!(contract.physical_extent(ImagePx(320.0)), 320);
}

/// k=1.25／2.0 の写像: 物理=画像×k・画像=物理/k・物理寸=ceil(寸×k)。
///
/// image px 原寸の導出はここには無い（2026-07-30 撤去）——原寸は presenter の native を
/// `TextSlotBinding::from_view` が透過するのみで、この契約型は逆写像を持たない。
#[test]
fn nonunit_scale_maps_between_image_and_physical() {
    let contract = ScaleContract::new(1.25, None);
    assert_eq!(contract.to_physical(ImagePx(320.0)), PhysicalPx(400.0));
    assert_eq!(contract.to_image(PhysicalPx(400.0)), ImagePx(320.0));
    // 物理寸 = ceil(image 寸 × k)（validrect 幅 320 → 400・端数は切上げ）。
    assert_eq!(contract.physical_extent(ImagePx(320.0)), 400);
    assert_eq!(contract.physical_extent(ImagePx(321.0)), 402); // 401.25 → 402

    let doubled = ScaleContract::new(2.0, None);
    assert_eq!(doubled.to_physical(ImagePx(36.0)), PhysicalPx(72.0));
    assert_eq!(doubled.physical_extent(ImagePx(399.0)), 798);
}

/// 画像→物理→画像の往復が原値へ戻る（k≠1 含む）。
#[test]
fn image_physical_roundtrip_returns_original() {
    for k in [1.0f32, 1.25, 2.0] {
        let contract = ScaleContract::new(k, None);
        for v in [0.0f32, 36.0, 168.0, 351.0] {
            let roundtrip = contract.to_image(contract.to_physical(ImagePx(v)));
            assert!(
                (roundtrip.0 - v).abs() < 1e-4,
                "k={k}: {v} → 往復 {} は原値へ戻る",
                roundtrip.0
            );
        }
    }
}

/// 不正スケール（0 以下・非有限）は warn を記録して 1.0 へ縮退する（log-first）。
#[test]
fn invalid_scale_falls_back_to_one_with_warn() {
    for bad in [0.0f32, -2.0, f32::NAN, f32::INFINITY] {
        let (contract, warns) = count_warns(|| ScaleContract::new(bad, None));
        assert_eq!(contract.scale, 1.0, "scale {bad} は 1.0 へ縮退する");
        assert_eq!(warns, 1, "scale {bad} はちょうど 1 回 warn を記録する");
    }
}

/// TextRegion の全値は image px（k 非依存）——resolve は ScaleContract を受けない
/// シグネチャで構造的に担保されるが、値レベルでも fixture 座標がスケール概念と
/// 無関係に一致することを固定する（レイアウト決定のスケール非依存・R4.6 前半）。
#[test]
fn text_region_values_are_image_px_independent_of_scale() {
    let region = TextRegion::resolve(
        &fixture_model(),
        FIXTURE_IMAGE_SIZE,
        WritingMode::HorizontalTb,
    );
    // image_size が同じである限り、どの k を仮定しても TextRegion は同一。
    // （物理への写像は ScaleContract 経由の一点のみ）
    assert_eq!(
        (region.left(), region.top(), region.right(), region.bottom()),
        (36.0, 46.0, 356.0, 168.0)
    );
    let contract = ScaleContract::new(2.0, None);
    // 物理寸への写像は消費側の一点適用（例: validrect 幅 320 → 物理 640）。
    assert_eq!(
        contract.physical_extent(ImagePx(region.right() - region.left())),
        640
    );
}

// ── areka-P0-cursor-tag-canon R4.3: バルーン画像原寸の保持（`\_l` の centerx／centery の基準） ──

/// 檻の独立入力に使う原寸。`FIXTURE_IMAGE_SIZE` とは**別の値**であり、幅 ≠ 高さで、
/// 下の各檻が宣言する validrect の 4 辺・幅・高さ・`start` のいずれとも一致しない。
/// 「実装の値を読み戻すだけ」にならないよう、期待値はこの定数から直に書く。
const ALT_IMAGE_SIZE: (u32, u32) = (531, 289);

/// `image_size()` は `resolve` に渡した原寸をそのまま f32 で返し、3 書字方向で同一である
/// （`centerx`／`centery` は書字方向に依らない——要件 4.4 の前提になる値）。
#[test]
fn image_size_returns_resolve_input_verbatim_in_every_writing_mode() {
    for mode in [
        WritingMode::HorizontalTb,
        WritingMode::VerticalRl,
        WritingMode::VerticalLr,
    ] {
        let alt = TextRegion::resolve(&fixture_model(), ALT_IMAGE_SIZE, mode);
        assert_eq!(alt.image_size(), (531.0, 289.0), "{mode:?}");
        let fixture = TextRegion::resolve(&fixture_model(), FIXTURE_IMAGE_SIZE, mode);
        assert_eq!(fixture.image_size(), (400.0, 224.0), "{mode:?}");
    }
}

/// 基準は**バルーン画像そのもの**であって validrect でも描画開始点でもない（要件 4.3）。
/// validrect と origin を明示宣言し、それらが実際に効いていること（対照）を見たうえで、
/// `image_size()` が validrect の 4 辺・幅高さ・`start` のどれとも一致しないことを固定する。
#[test]
fn image_size_is_the_balloon_image_not_the_validrect_or_origin() {
    // validrect (30,50)-(330,200)＝幅 300×高さ 150・origin (120,70)。
    // いずれの数も ALT_IMAGE_SIZE の 531／289 とは重ならない。
    let m = model(
        (Some(120), Some(70)),
        (None, None),
        (Some(50), Some(200), Some(30), Some(330)),
    );
    let region = TextRegion::resolve(&m, ALT_IMAGE_SIZE, WritingMode::HorizontalTb);
    // 対照: 宣言は確かに効いている（この檻は恒真ではない）。
    assert_eq!(
        (region.left(), region.top(), region.right(), region.bottom()),
        (30.0, 50.0, 330.0, 200.0)
    );
    assert_eq!(region.start(), (120.0, 70.0));
    // 本題: 画像原寸は渡された値のまま。
    assert_eq!(region.image_size(), (531.0, 289.0));
    assert_ne!(
        region.image_size(),
        (
            region.right() - region.left(),
            region.bottom() - region.top()
        ),
        "validrect の幅・高さとの取り違え"
    );
    assert_ne!(
        region.image_size(),
        (region.right(), region.bottom()),
        "validrect の右辺・下辺との取り違え"
    );
    assert_ne!(region.image_size(), region.start(), "start との取り違え");
}

/// validrect／origin／wordwrappoint を宣言してもしなくても `image_size()` は変わらない
/// （画像原寸は宣言から導かれる値ではない）。
#[test]
fn image_size_is_unchanged_by_validrect_and_origin_declarations() {
    let declared = model(
        (Some(120), Some(70)),
        (Some(-10), Some(-10)),
        (Some(50), Some(200), Some(30), Some(330)),
    );
    let bare = model((None, None), (None, None), (None, None, None, None));
    for mode in [
        WritingMode::HorizontalTb,
        WritingMode::VerticalRl,
        WritingMode::VerticalLr,
    ] {
        let declared = TextRegion::resolve(&declared, ALT_IMAGE_SIZE, mode);
        let bare = TextRegion::resolve(&bare, ALT_IMAGE_SIZE, mode);
        assert_eq!(declared.image_size(), (531.0, 289.0), "{mode:?}");
        assert_eq!(bare.image_size(), declared.image_size(), "{mode:?}");
    }
}
