
use areka_parsers::balloon::BalloonModel;
use log_capture_kit::count_levels;

use super::{ImagePx, PhysicalPx, ScaleContract, TextRegion};
// `IMAGE` は fixture 実測のバルーン画像原寸（balloons0.png・400×224・image px）。
use crate::layout::test_support::{IMAGE, model_full};
use crate::writing::WritingMode;

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
    model_full(
        (None, None),
        (Some(-49), Some(0)),
        (Some(46), Some(-56), Some(36), Some(-44)),
    )
}

/// クロージャを共有のログ捕捉窓の中で実行し（戻り値, WARN 件数）を返す。
///
/// 件数の集計は硬化機構の唯一の定義元 `log-capture-kit` の [`count_levels`] に委ねる。
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
        IMAGE,
        WritingMode::HorizontalTb,
    );
    assert_eq!(region.left(), 36.0);
    assert_eq!(region.top(), 46.0);
    assert_eq!(region.right(), 356.0); // 400 + (-44)
    assert_eq!(region.bottom(), 168.0); // 224 + (-56)
    assert!(region.right() > region.left(), "非退化（幅 > 0）");
    assert!(region.bottom() > region.top(), "非退化（高さ > 0）");
}

/// fixture の descript 基層のみ（validrect 全 0）は画像全体 (0, 0, 400, 224) に解け、潰れで
/// なく、解決は警告を 1 行も出さない。右辺・下辺の 0 は画像の右端・下端と読む——要件 6
/// （requirements.md の正典の段）。かつてはこの基層を潰れとして固定していたが、読みの変更で
/// 期待値が変わった。「解決は潰れでも警告を書かない」（spec `areka-P0-balloon-text-bugs`
/// 件 2・要件 2.2）の主張は、0 でない値で潰す `is_collapsed_counts_zero_negative_and_nan_extent`
/// が持つ。解決は装着と入力の変わり目に呼ばれ、潰れの記録は actor の登録口が状態の変わり目に
/// 1 回だけ書く。
#[test]
fn base_layer_only_validrect_resolves_to_whole_image_and_resolve_does_not_warn() {
    // descript.txt 基層実測: origin 0,0・wordwrappoint.x,-34・validrect 全 0。
    let base = model_full(
        (Some(0), Some(0)),
        (Some(-34), Some(0)),
        (Some(0), Some(0), Some(0), Some(0)),
    );
    let (region, warns) = count_warns(|| {
        TextRegion::resolve(&base, IMAGE, WritingMode::HorizontalTb)
    });
    assert_eq!(
        (region.left(), region.top(), region.right(), region.bottom()),
        (0.0, 0.0, 400.0, 224.0)
    );
    assert!(!region.is_collapsed(), "全 0 は画像全体で潰れでない");
    assert_eq!(warns, 0, "解決は警告を書かない");
}

/// 要件 6（requirements.md の正典の段）: `validrect.right`・`validrect.bottom` の 0 は画像の
/// 右端・下端と読む。正典（ukadoc の descript.txt（バルーン）の脚注 *1「マイナス座標は
/// 右下からの相対」）は 0 の意味を書かず、既定値は「不明」。同じ頁の記述例が
/// `validrect.right,0` を書き、SSP の測り（2026-10-29・`bottle`）が右辺 0 を「左端から 0」と
/// 読まないことを示した。下辺の 0 は SSP で測っておらず、右辺と脚注の対称で読む。
/// 左辺・上辺の 0・正の値・負の値の読みは今のまま。どの行も解決は `warn` を書かない。
#[test]
fn validrect_far_edge_zero_reads_as_image_edge() {
    // (説明, validrect (top, bottom, left, right), 期待 (left, top, right, bottom))。
    type Rect = (Option<i32>, Option<i32>, Option<i32>, Option<i32>);
    type Row = (&'static str, Rect, (f32, f32, f32, f32));
    let cases: [Row; 6] = [
        ("右辺 0 → 画像の右端", (Some(10), Some(100), Some(20), Some(0)), (20.0, 10.0, 400.0, 100.0)),
        ("下辺 0 → 画像の下端", (Some(10), Some(0), Some(20), Some(300)), (20.0, 10.0, 300.0, 224.0)),
        ("左辺・上辺の 0 → 0（今のまま）", (Some(0), Some(100), Some(0), Some(300)), (0.0, 0.0, 300.0, 100.0)),
        ("正の値（今のまま）", (Some(10), Some(200), Some(20), Some(300)), (20.0, 10.0, 300.0, 200.0)),
        ("負の値（今のまま）", (Some(10), Some(-56), Some(20), Some(-44)), (20.0, 10.0, 356.0, 168.0)),
        ("4 辺とも 0 → 画像全体", (Some(0), Some(0), Some(0), Some(0)), (0.0, 0.0, 400.0, 224.0)),
    ];
    for (label, vr, expected) in cases {
        let (region, warns) = count_warns(|| {
            TextRegion::resolve(&model_full((None, None), (None, None), vr), IMAGE, WritingMode::HorizontalTb)
        });
        assert_eq!((region.left(), region.top(), region.right(), region.bottom()), expected, "{label}");
        assert!(!region.is_collapsed(), "{label}: 潰れでない");
        assert_eq!(warns, 0, "{label}: 解決は warn を書かない");
    }

    // vertical_rl で右辺 0・origin 未指定 → 書き始めは (画像の右端, top)。
    let (region, warns) = count_warns(|| {
        TextRegion::resolve(
            &model_full((None, None), (None, None), (Some(10), Some(100), Some(20), Some(0))),
            IMAGE,
            WritingMode::VerticalRl,
        )
    });
    assert_eq!(region.start(), (400.0, 10.0), "vertical_rl の書き始めは右辺 0＝画像の右端");
    assert_eq!(warns, 0, "vertical_rl: 解決は warn を書かない");
}

/// 潰れの判定（要件 2.1）: 幅か高さが正でなければ潰れ。幅 0・高さ 0・負の幅・負の高さ・
/// 非数を潰れに数え、幅・高さとも正なら潰れでない。どの解決も警告を出さない（要件 2.2）。
#[test]
fn is_collapsed_counts_zero_negative_and_nan_extent() {
    let resolve = |vr: (i32, i32, i32, i32)| {
        let m = model_full(
            (None, None),
            (None, None),
            (Some(vr.0), Some(vr.1), Some(vr.2), Some(vr.3)),
        );
        TextRegion::resolve(&m, IMAGE, WritingMode::HorizontalTb)
    };
    // (top, bottom, left, right) の順（ValidRect::new の引数順）。
    let cases: [(&str, (i32, i32, i32, i32), bool); 5] = [
        ("幅 0", (10, 100, 50, 50), true),
        ("高さ 0", (60, 60, 10, 200), true),
        ("負の幅", (10, 100, 200, 50), true),
        ("負の高さ", (100, 10, 10, 200), true),
        ("幅・高さとも正", (10, 100, 10, 200), false),
    ];
    for (label, vr, expected) in cases {
        let (region, warns) = count_warns(|| resolve(vr));
        assert_eq!(region.is_collapsed(), expected, "{label}");
        assert_eq!(warns, 0, "{label}: 解決は警告を書かない");
    }

    // 非数は解決の入口（i32・u32）からは作れないので、解決済みの値を直に差し替えて問う。
    let healthy = resolve((10, 100, 10, 200));
    for (label, region) in [
        ("left が非数", TextRegion { left: f32::NAN, ..healthy }),
        ("right が非数", TextRegion { right: f32::NAN, ..healthy }),
        ("top が非数", TextRegion { top: f32::NAN, ..healthy }),
        ("bottom が非数", TextRegion { bottom: f32::NAN, ..healthy }),
    ] {
        assert!(region.is_collapsed(), "{label} は潰れに数える");
    }
}

/// 2層マージ（balloon-parse 実機構）を通した fixture 再現: 基層（validrect 全 0）だけでも
/// 画像全体に解けるが、画像別上書きとの合成で (36, 46, 356, 168) になる。基層は右辺・下辺の
/// 0 を画像の端と読むので退化しない——要件 6（requirements.md の正典の段）。
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
        TextRegion::resolve(&merged, IMAGE, WritingMode::HorizontalTb)
    });
    assert_eq!(
        (region.left(), region.top(), region.right(), region.bottom()),
        (36.0, 46.0, 356.0, 168.0)
    );
    assert_eq!(warns, 0, "非退化矩形は warn を記録しない");
    assert_eq!(region.start(), (36.0, 46.0));
    assert_eq!(region.wrap_threshold(), 351.0); // 400 + (-49)
}

// 描画開始点の解決（範囲内・範囲外・未宣言・負値・書字開始角）と、validrect の素通し・
// 画像端への縮退・折返し点の既定は `region_vertical_canon_tests.rs` の表が 3 方向で持つ。

// ── 折返し閾値（軸解釈は WritingMode 依存・負値=反対辺基準） ──

/// 横書き: wordwrappoint.x（fixture -49・負値=右辺基準）→ 400-49=351。
#[test]
fn horizontal_wrap_threshold_resolves_wordwrappoint_x() {
    let region = TextRegion::resolve(
        &fixture_model(),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    assert_eq!(region.wrap_threshold(), 351.0);
}

/// 縦書き（vertical_rl／vertical_lr）: wordwrappoint.y（負値=下辺基準）。
#[test]
fn vertical_wrap_threshold_resolves_wordwrappoint_y() {
    let m = model_full(
        (None, None),
        (Some(-49), Some(-30)),
        (Some(46), Some(-56), Some(36), Some(-44)),
    );
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        let region = TextRegion::resolve(&m, IMAGE, mode);
        assert_eq!(region.wrap_threshold(), 194.0, "224 + (-30) for {mode:?}");
    }
}

/// fixture 実測 wordwrappoint.y,0 は 0 のまま忠実に解決される（縦書き折返しの
/// 退化は design 織り込み済み・退化補正はレイアウト層の領分でなく本層は転記解決に徹する）。
#[test]
fn degenerate_wordwrappoint_y_zero_is_resolved_faithfully() {
    let region = TextRegion::resolve(
        &fixture_model(),
        IMAGE,
        WritingMode::VerticalRl,
    );
    assert_eq!(region.wrap_threshold(), 0.0);
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

// ── areka-P0-cursor-tag-canon R4.3: バルーン画像原寸の保持（`\_l` の centerx／centery の基準） ──

/// 檻の独立入力に使う原寸。`IMAGE` とは**別の値**であり、幅 ≠ 高さで、
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
        let fixture = TextRegion::resolve(&fixture_model(), IMAGE, mode);
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
    let m = model_full(
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
    let declared = model_full(
        (Some(120), Some(70)),
        (Some(-10), Some(-10)),
        (Some(50), Some(200), Some(30), Some(330)),
    );
    let bare = model_full((None, None), (None, None), (None, None, None, None));
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
