//! # 縦書き座標意味論の檻（純粋層・兄弟テストファイル）
//!
//! 守っているもの——縦書きバルーンの座標解決が SSP 正典（2.8.83）と**既に一致している**
//! という事実を、「たまたま合っている」から「検証された一致」へ変える。対象は
//! [`TextRegion::resolve`] の 1 関数だけであり、**本番コードは 1 行も変えない**
//! （本仕様 `areka-P0-balloon-vertical-canon` の設計 C4「コード変更は 0」）。
//!
//! 逐語で固定するのは次の 4 点である。
//!
//! 1. **`wordwrappoint.y` の既定＝`validrect.bottom`**——縦書きで `wordwrappoint.y` が
//!    宣言されていないとき、折返し閾値は解決後の `validrect.bottom` と一致する
//!    （本仕様の要件 3.4）。`wordwrappoint.x` が宣言されていても既定は変わらない。
//! 2. **`wordwrappoint.y` の負値＝ベース画像の下辺基準**——負値は
//!    `resolve(v, extent) = extent + v` の既存規約（ukadoc 脚注 *1）で解決され、
//!    その規約は横書きの `wordwrappoint.x` と同一である（本仕様の要件 3.4・3.7）。
//! 3. **`wordwrappoint.x` は縦書きで参照されない**（本仕様の要件 3.5・C4 の主眼）——
//!    縦書きの折返し軸選択は `wordwrappoint.y` のみを読む網羅 match であり、
//!    `wordwrappoint.x` の**不参照は型で保証されている**。その保証は読んでも見えないため、
//!    「同じバルーン定義から `wordwrappoint.x` だけを変えた複数のモデルが、縦書き 2 モードで
//!    `TextRegion` の全成分において逐語一致する」という**差分不変の檻**へ翻訳する。
//!    差分が本物であること（檻が恒真でないこと）は、同じ変化が横書きでは
//!    `wrap_threshold` を実際に動かすことを対照として示す。
//! 4. **`validrect` 4 辺の意味は横書きと同一**（本仕様の要件 3.6）——同一モデルに対し
//!    `HorizontalTb`／`VerticalRl`／`VerticalLr` の 3 モードで `left`／`top`／`right`／
//!    `bottom` が完全に一致する。負値宣言・未宣言（画像端フォールバック）・非負宣言の
//!    各形について見る。`start` と `wrap_threshold` はモードで変わってよい——そこは
//!    書字方向という別の関心事であり、本檻はそれを固定しない。
//!
//! ## SC5（列が並ぶ範囲の上限＝`validrect.left`）は既存の檻が守っている
//!
//! `vertical_rl` で列が左へ進める限界が `validrect.left` であることは**既存実装であり、
//! 既に決定論テストで固定されている**。出所は
//! `crates/areka-emo-text/src/layout_visible_window_tests.rs:57-79` の
//! `vertical_rl_overflow_scrolls_content_rightward`（validrect `left,360`／`right,400` で
//! 4 列目の左端 351 が 360 を下回った時点であふれが発火することを逐語固定する）。
//! **本ファイルではこの件について新しい檻を作らない**（設計 C4「新規の檻は作らず、
//! COMPAT §8 で『既に実装され固定されている挙動』として登記する」）。
//!
//! ## origin の解決 4 分岐と「validrect は返値に影響しない」不変条件
//!
//! 上の 4 点（要件 3.4〜3.6）を見る檻は `origin` の宣言を **validrect の内側**に置くか
//! **未宣言**にするかのどちらかに限っている——そこは書字方向とは別の関心事であり、
//! origin の規約が変わっても偽の赤を出さないためである。
//!
//! それとは別に、本ファイルは `origin` 解決そのものの判断分岐も固定する
//! （本仕様の要件 3.10／3.11・設計 C3 の Validation）——
//!
//! 5. **validrect 内の宣言**＝字義位置・記録 0 件。
//! 6. **validrect 外の宣言**＝**字義位置**（寄せない）・成分ごとに `debug!` ちょうど 1 件。
//! 7. **未宣言**＝書字開始角へ縮退・成分ごとに `debug!` ちょうど 1 件（要件 3.11・不変）。
//! 8. **負値宣言**＝反対端基準で絶対値化してから字義（要件 3.7）。
//! 9. **不変条件**——宣言された `origin` の解決結果は `validrect` を変えても動かない。
//!    これが areka 独自の「origin クランプ正準」（`clamp(resolve(origin), validrect)`）が
//!    もう残っていないことの、読み手向けの証拠である。クランプを戻すと 6. と 9. が赤になる。
//!
//! ## 0 件主張の規律（恒真の禁止）
//!
//! 「`debug` が 0 件」という主張は、捕捉窓が死んでいても成立してしまう。そこで記録件数を
//! 見る檻は捕捉窓の**内側**で対照イベント（`error!` 1 件）を発行し、その 1 件が数えられて
//! いることを件数主張と必ず同時に確認する（[`assert_capture_alive`]）。件数の集計は硬化
//! 機構の唯一の定義元 `log-capture-kit` の [`count_levels`] に委ね、`warn` と `debug` を
//! **別々に**数える（`writing_decision_tests.rs` と同じ流儀）。
//!
//! ## 実行条件
//!
//! 実 DPI モニタ・実 GPU・実ゴースト・実窓を一切要さない純粋層の檻であり、同一入力に
//! 対して常に同一の結果を返す（本仕様の要件 10.6）。`windows` 系 crate を import しない
//! （本ファイル自身が `lib.rs` の構造檻 `pure_layer_modules_have_no_windows_imports` の
//! 走査対象に列挙されている）。

use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use log_capture_kit::{LevelCounts, count_levels};

use super::TextRegion;
use crate::writing::WritingMode;

/// fixture 実測のバルーン画像原寸（balloons0.png・image px）。
///
/// `region.rs` のインラインテストの `FIXTURE_IMAGE_SIZE` と同値。負値解決の基準寸が
/// この値であることが、下の `-56 → 168` などの期待値の根拠になる。
const IMAGE: (u32, u32) = (400, 224);

/// 本檻の基準 validrect（`top,46`／`bottom,-56`／`left,36`／`right,-44`）。
///
/// 画像 400×224 に対し `(left, top, right, bottom) = (36, 46, 356, 168)` へ解決される。
/// 4 辺・画像高さ 224・画像幅 400 はいずれも相異なる値であり、折返し既定が
/// `validrect.bottom` **以外**の辺へ差し替わった変異は必ず赤になる。
const RECT: (Option<i32>, Option<i32>, Option<i32>, Option<i32>) =
    (Some(46), Some(-56), Some(36), Some(-44));
const RECT_LEFT: f32 = 36.0;
const RECT_TOP: f32 = 46.0;
const RECT_RIGHT: f32 = 356.0;
const RECT_BOTTOM: f32 = 168.0;

/// 縦書き 2 モード（列の送り方向だけが異なる）。
const VERTICAL_MODES: [WritingMode; 2] = [WritingMode::VerticalRl, WritingMode::VerticalLr];

/// 3 モード全部。
const ALL_MODES: [WritingMode; 3] = [
    WritingMode::HorizontalTb,
    WritingMode::VerticalRl,
    WritingMode::VerticalLr,
];

/// テスト用 `BalloonModel` 生成ヘルパ（幾何成分だけ指定・font/windowposition は未指定）。
///
/// 引数の並びは `region.rs` の既存インラインテストの `model` と同一にそろえてある
/// （`validrect` は `ValidRect::new` に合わせて **top／bottom／left／right** の順・
/// `wordwrap` は **x／y** の順）。取り違えを避けるため、呼出側では実値をコメントで
/// 添えること。
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

/// `TextRegion` の全 6 成分を組にして取り出す（逐語比較用・差分の所在を読める形にする）。
fn components(region: &TextRegion) -> (f32, f32, f32, f32, (f32, f32), f32) {
    (
        region.left(),
        region.top(),
        region.right(),
        region.bottom(),
        region.start(),
        region.wrap_threshold(),
    )
}

/// `validrect` 4 辺だけを取り出す（モード横断比較用）。
fn edges(region: &TextRegion) -> (f32, f32, f32, f32) {
    (region.left(), region.top(), region.right(), region.bottom())
}

// ── 要件 3.4: `wordwrappoint.y` の既定＝`validrect.bottom` ──

/// 縦書きで `wordwrappoint.y` が未宣言なら、折返し閾値は解決後の `validrect.bottom`
/// （＝168）である。`wordwrappoint.x` が宣言されていても既定は変わらない
/// （＝既定の出所が y 軸側であることの檻）。
#[test]
fn vertical_wordwrap_y_defaults_to_validrect_bottom() {
    // wordwrappoint.x の宣言有無を変えても、縦書きの既定は validrect.bottom のまま。
    let cases = [
        ("x 未宣言", (None, None)),
        ("x のみ宣言（100）", (Some(100), None)),
        ("x のみ宣言（負値 -49）", (Some(-49), None)),
    ];
    for (label, wordwrap) in cases {
        for mode in VERTICAL_MODES {
            let region = TextRegion::resolve(&model((None, None), wordwrap, RECT), IMAGE, mode);
            // 前提の確認: 基準 validrect が想定どおり解決されている。
            assert_eq!(
                edges(&region),
                (RECT_LEFT, RECT_TOP, RECT_RIGHT, RECT_BOTTOM),
                "{label} / {mode:?}: 基準 validrect の解決が想定と異なる"
            );
            assert_eq!(
                region.wrap_threshold(),
                region.bottom(),
                "{label} / {mode:?}: 縦書きの折返し既定は validrect.bottom でなければならない"
            );
            assert_eq!(
                region.wrap_threshold(),
                RECT_BOTTOM,
                "{label} / {mode:?}: 折返し既定の実値が 168（=224-56）と異なる"
            );
        }
    }
}

/// 対照——横書きの既定は `validrect.right`（356）であり、縦書きの既定（168）とは
/// 別の値である。上の檻が「どの辺でも通る恒真」ではないことを示す。
#[test]
fn horizontal_wordwrap_default_is_validrect_right_not_bottom() {
    let region = TextRegion::resolve(
        &model((None, None), (None, None), RECT),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    assert_eq!(region.wrap_threshold(), RECT_RIGHT);
    assert_ne!(
        RECT_RIGHT, RECT_BOTTOM,
        "基準 validrect の right と bottom が同値では対照にならない"
    );
}

// ── 要件 3.4／3.7: `wordwrappoint.y` の負値＝ベース画像の下辺基準 ──

/// 縦書きの `wordwrappoint.y` の負値は、ベース画像の下辺（高さ 224）からの相対として
/// 解決される（`resolve(v, extent) = extent + v`・ukadoc 脚注 *1 の既存規約）。
/// 規約は横書きの `wordwrappoint.x`（幅 400 基準）と同一の形である。
#[test]
fn vertical_negative_wordwrap_y_resolves_from_image_bottom_edge() {
    // (宣言値, 期待値) — 224 + v。
    let cases = [(-56, 168.0), (-24, 200.0), (-224, 0.0)];
    for (declared, expected) in cases {
        for mode in VERTICAL_MODES {
            let region = TextRegion::resolve(
                // wordwrappoint (x=None, y=declared)
                &model((None, None), (None, Some(declared)), RECT),
                IMAGE,
                mode,
            );
            assert_eq!(
                region.wrap_threshold(),
                expected,
                "{mode:?}: wordwrappoint.y,{declared} は画像高さ 224 の下辺基準で \
                 {expected} へ解決される"
            );
        }
    }
    // 非負値は絶対座標として素通し（負値規約が非負まで巻き込んでいないことの境界檻）。
    for mode in VERTICAL_MODES {
        let region =
            TextRegion::resolve(&model((None, None), (None, Some(120)), RECT), IMAGE, mode);
        assert_eq!(region.wrap_threshold(), 120.0, "{mode:?}: 非負値は素通し");
    }
    // 横書きの wordwrappoint.x も同一の規約（画像幅 400 基準）——負値解決は
    // 書字方向で分岐しない（要件 3.7）。
    let horizontal = TextRegion::resolve(
        &model((None, None), (Some(-44), None), RECT),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    assert_eq!(horizontal.wrap_threshold(), 356.0, "400 + (-44)");
}

// ── 要件 3.5: `wordwrappoint.x` 不参照の差分不変檻（C4 の主眼） ──

/// **縦書きの `TextRegion` は `wordwrappoint.x` の値に一切依存しない。**
///
/// 同一のバルーン定義から `wordwrappoint.x` **だけ**を変えた 5 つのモデルが、縦書き 2 モードの
/// それぞれで `TextRegion` の全 6 成分（`left`／`top`／`right`／`bottom`／`start`／
/// `wrap_threshold`）において逐語一致することを固定する。これは
/// 「縦書きの折返し軸選択が `wordwrappoint.y` のみを読む網羅 match である」という
/// **型による保証**を、人間が読める形へ翻訳したものである（設計 C4）。
///
/// `origin` は未宣言形と validrect 内側の宣言形の両方で見る（撤去された旧「origin クランプ正準」
/// の発火に依存しない形で書かれており、撤去後もそのまま成立する）。
/// `wordwrappoint.y` も未宣言形と宣言形の両方で見る（既定経路と宣言経路の双方で不変）。
#[test]
fn vertical_region_is_invariant_to_wordwrappoint_x() {
    // 変えるのはこの 1 成分だけ。
    let x_variants = [None, Some(0), Some(100), Some(-49), Some(390)];
    // origin: 未宣言／validrect (36..356, 46..168) の内側の宣言。
    let origin_variants = [(None, None), (Some(200), Some(60))];
    let y_variants = [None, Some(120), Some(-56)];

    for origin in origin_variants {
        for y in y_variants {
            for mode in VERTICAL_MODES {
                let baseline =
                    TextRegion::resolve(&model(origin, (x_variants[0], y), RECT), IMAGE, mode);
                for x in &x_variants[1..] {
                    let varied = TextRegion::resolve(&model(origin, (*x, y), RECT), IMAGE, mode);
                    assert_eq!(
                        components(&varied),
                        components(&baseline),
                        "{mode:?} / origin {origin:?} / wordwrappoint.y {y:?}: \
                         wordwrappoint.x を {:?} から {x:?} へ変えたら TextRegion が動いた\
                         （縦書きは wordwrappoint.x を参照してはならない）",
                        x_variants[0]
                    );
                    // 型の等価も同時に見る（成分取り出しの取りこぼし防止）。
                    assert_eq!(varied, baseline, "{mode:?}: TextRegion 全体の等価");
                }
            }
        }
    }
}

/// 対照——上の差分不変が恒真ではないこと。**同じ** `wordwrappoint.x` の変化は、
/// 横書きでは `wrap_threshold` を実際に動かす。
#[test]
fn horizontal_region_does_depend_on_wordwrappoint_x() {
    let mut seen = Vec::new();
    for x in [None, Some(0), Some(100), Some(-49), Some(390)] {
        let region = TextRegion::resolve(
            &model((None, None), (x, None), RECT),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        seen.push(region.wrap_threshold());
    }
    // None→356（validrect.right へ縮退）／0→0／100→100／-49→351／390→390。
    assert_eq!(seen, vec![356.0, 0.0, 100.0, 351.0, 390.0]);
}

// ── 要件 3.6: `validrect` 4 辺は横書きと同一に解決される ──

/// `validrect` の 4 辺の意味は書字方向で変わらない。負値宣言・未宣言（画像端への
/// フォールバック）・非負宣言・混在の各形について、3 モードで
/// `left`／`top`／`right`／`bottom` が完全に一致する。
///
/// `start` と `wrap_threshold` はモードで変わってよい（書字方向という別の関心事）——
/// 本檻はそれらを固定しない。
#[test]
fn validrect_edges_resolve_identically_across_writing_modes() {
    // (ラベル, validrect(top,bottom,left,right), 期待 (left,top,right,bottom))
    let cases = [
        (
            "負値混じり（fixture 実測形）",
            RECT,
            (RECT_LEFT, RECT_TOP, RECT_RIGHT, RECT_BOTTOM),
        ),
        (
            "全未宣言（画像端へフォールバック）",
            (None, None, None, None),
            (0.0, 0.0, 400.0, 224.0),
        ),
        (
            "全非負（素通し）",
            (Some(10), Some(200), Some(20), Some(300)),
            (20.0, 10.0, 300.0, 200.0),
        ),
        (
            "一部未宣言・一部負値の混在",
            (None, Some(-24), Some(-360), None),
            (40.0, 0.0, 400.0, 200.0),
        ),
    ];
    for (label, rect, expected) in cases {
        let resolved: Vec<(f32, f32, f32, f32)> = ALL_MODES
            .iter()
            .map(|mode| {
                // origin は未宣言（撤去された旧クランプ正準に非依存・要件 3.11 の
                // 縮退のみに触れる）。
                edges(&TextRegion::resolve(
                    &model((None, None), (None, None), rect),
                    IMAGE,
                    *mode,
                ))
            })
            .collect();
        for (mode, actual) in ALL_MODES.iter().zip(&resolved) {
            assert_eq!(
                *actual, expected,
                "{label} / {mode:?}: validrect 4 辺の解決が期待と異なる"
            );
        }
        assert!(
            resolved.windows(2).all(|w| w[0] == w[1]),
            "{label}: 3 モードで validrect 4 辺が一致しない: {resolved:?}"
        );
    }
}

/// 対照——4 辺が一致することは「モードが区別されていない」ことを意味しない。
/// 同一モデルでも書字開始角は `HorizontalTb`／`VerticalLr`＝validrect 左上・
/// `VerticalRl`＝validrect 右上へ分かれる（未宣言時の縮退・要件 3.11）。
#[test]
fn writing_mode_still_separates_start_corner_while_edges_agree() {
    let resolve = |mode| TextRegion::resolve(&model((None, None), (None, None), RECT), IMAGE, mode);
    let horizontal = resolve(WritingMode::HorizontalTb);
    let vertical_rl = resolve(WritingMode::VerticalRl);
    let vertical_lr = resolve(WritingMode::VerticalLr);

    assert_eq!(edges(&horizontal), edges(&vertical_rl));
    assert_eq!(edges(&horizontal), edges(&vertical_lr));

    assert_eq!(horizontal.start(), (RECT_LEFT, RECT_TOP));
    assert_eq!(vertical_lr.start(), (RECT_LEFT, RECT_TOP));
    assert_eq!(vertical_rl.start(), (RECT_RIGHT, RECT_TOP));
    assert_ne!(
        vertical_rl.start(),
        horizontal.start(),
        "vertical_rl の書字開始角が横書きと同じでは、モードが区別されていない"
    );
}

// ── 要件 3.10／3.11: origin 解決の 4 分岐と validrect 非依存の不変条件（設計 C3） ──

/// 対照イベント専用の宛先（本番コードがここへ発火することは無い）。
const CONTROL_TARGET: &str = "areka_emo_text::region_vertical_canon_tests::control";

/// `origin` 由来の記録だけが数えられる条件で解決し、（`TextRegion`, レベル別件数）を返す。
///
/// `wordwrappoint` は x・y の**両方**を宣言してある——未宣言成分は `resolve_or` が
/// `debug!` を出すため、宣言しておかないと origin の件数主張に他所の記録が混ざる。
/// `validrect` は呼出側が渡す（4 辺すべてを宣言した形を渡せば同様に混入しない）。
/// 窓の内側で対照イベントを 1 件発行し、[`assert_capture_alive`] で捕捉が生きていることを
/// 示せるようにする。
fn resolve_counting(
    origin: (Option<i32>, Option<i32>),
    validrect: (Option<i32>, Option<i32>, Option<i32>, Option<i32>),
    mode: WritingMode,
) -> (TextRegion, LevelCounts) {
    count_levels(|| {
        tracing::error!(
            target: CONTROL_TARGET,
            "捕捉窓の対照イベント（この 1 件が数えられないなら同窓の 0 件主張は無効）"
        );
        TextRegion::resolve(
            // wordwrappoint は x,-49／y,-56 を宣言（未宣言由来の debug を封じる）。
            &model(origin, (Some(-49), Some(-56)), validrect),
            IMAGE,
            mode,
        )
    })
}

/// 捕捉窓が生きていたことを対照イベントの件数で示す（件数主張の前提条件）。
fn assert_capture_alive(counts: &LevelCounts) {
    assert_eq!(
        counts.error, 1,
        "捕捉窓の対照イベントが数えられていない。この窓の debug 件数の主張は\
         「出た／出なかった」ことの証拠にならない"
    );
}

/// 分岐 1——**validrect の内側**に宣言された `origin` は字義どおり用いられ、記録は出ない
/// （要件 3.10 前半・正常系）。書字方向 3 モードで同一（開始角の選択は使われない）。
#[test]
fn declared_origin_inside_validrect_is_literal_and_unrecorded() {
    // (200, 60) は解決後 validrect (36,46)-(356,168) の内側。
    for mode in ALL_MODES {
        let (region, counts) = resolve_counting((Some(200), Some(60)), RECT, mode);
        assert_capture_alive(&counts);
        assert_eq!(
            region.start(),
            (200.0, 60.0),
            "{mode:?}: 範囲内の宣言は字義どおり用いる"
        );
        assert_eq!(counts.debug, 0, "{mode:?}: 範囲内の宣言は記録しない");
        assert_eq!(counts.warn, 0, "{mode:?}: warn も出さない");
    }
}

/// 分岐 2——**validrect の外**に宣言された `origin` も**字義どおり**用いられ、
/// 成分ごとに `debug!` ちょうど 1 件を記録する（要件 3.10 後半）。
///
/// **これが撤去の本体である**。旧「origin クランプ正準」が残っていれば期待値は
/// 書字開始角（横書き (36,46)／`vertical_rl` (356,46)）になり、本檻は赤くなる。
#[test]
fn declared_origin_outside_validrect_is_literal_with_one_debug_per_component() {
    // (ラベル, origin, mode, 期待 start, 期待 debug 件数)
    let cases = [
        // y(0) だけが top(46) より上＝範囲外。x(200) は範囲内なので記録は 1 件だけ。
        (
            "y のみ範囲外",
            (Some(200), Some(0)),
            WritingMode::HorizontalTb,
            (200.0, 0.0),
            1,
        ),
        // x(400) だけが right(356) より右＝範囲外。
        (
            "x のみ範囲外",
            (Some(400), Some(60)),
            WritingMode::HorizontalTb,
            (400.0, 60.0),
            1,
        ),
        // 両成分とも範囲外（フィクスチャがかつて宣言していた形）。
        (
            "両成分が範囲外・横書き",
            (Some(0), Some(0)),
            WritingMode::HorizontalTb,
            (0.0, 0.0),
            2,
        ),
        // 縦書きでも寄せない——クランプが残っていれば (356,46) になる。
        (
            "両成分が範囲外・vertical_rl",
            (Some(0), Some(0)),
            WritingMode::VerticalRl,
            (0.0, 0.0),
            2,
        ),
        (
            "両成分が範囲外・vertical_lr",
            (Some(0), Some(0)),
            WritingMode::VerticalLr,
            (0.0, 0.0),
            2,
        ),
    ];
    for (label, origin, mode, expected_start, expected_debug) in cases {
        let (region, counts) = resolve_counting(origin, RECT, mode);
        assert_capture_alive(&counts);
        assert_eq!(
            region.start(),
            expected_start,
            "{label} / {mode:?}: validrect 外の宣言は寄せずに字義どおり用いる\
             （書字開始角へ寄っていたら旧クランプ正準が残っている）"
        );
        assert_eq!(
            counts.debug, expected_debug,
            "{label} / {mode:?}: 範囲外の成分 1 つにつき debug ちょうど 1 件"
        );
        assert_eq!(
            counts.warn, 0,
            "{label} / {mode:?}: 範囲外宣言は warn ではない（記録水準表・要件 3.10）"
        );
    }
}

/// 分岐 3——**未宣言**の成分は書字開始角へ縮退し、成分ごとに `debug!` ちょうど 1 件を
/// 記録する（要件 3.11・クランプ撤去の前後で完全に同一の挙動）。
#[test]
fn undeclared_origin_falls_back_to_writing_corner_with_one_debug_per_component() {
    // (ラベル, origin, mode, 期待 start, 期待 debug 件数)
    let cases = [
        (
            "y のみ未宣言",
            (Some(200), None),
            WritingMode::HorizontalTb,
            (200.0, RECT_TOP),
            1,
        ),
        (
            "x のみ未宣言・横書き",
            (None, Some(60)),
            WritingMode::HorizontalTb,
            (RECT_LEFT, 60.0),
            1,
        ),
        (
            "x のみ未宣言・vertical_rl",
            (None, Some(60)),
            WritingMode::VerticalRl,
            (RECT_RIGHT, 60.0),
            1,
        ),
        (
            "全未宣言・横書き",
            (None, None),
            WritingMode::HorizontalTb,
            (RECT_LEFT, RECT_TOP),
            2,
        ),
        (
            "全未宣言・vertical_rl",
            (None, None),
            WritingMode::VerticalRl,
            (RECT_RIGHT, RECT_TOP),
            2,
        ),
        (
            "全未宣言・vertical_lr",
            (None, None),
            WritingMode::VerticalLr,
            (RECT_LEFT, RECT_TOP),
            2,
        ),
    ];
    for (label, origin, mode, expected_start, expected_debug) in cases {
        let (region, counts) = resolve_counting(origin, RECT, mode);
        assert_capture_alive(&counts);
        assert_eq!(
            region.start(),
            expected_start,
            "{label} / {mode:?}: 未宣言成分は書字開始角へ縮退する（要件 3.11）"
        );
        assert_eq!(
            counts.debug, expected_debug,
            "{label} / {mode:?}: 未宣言の成分 1 つにつき debug ちょうど 1 件"
        );
    }
}

/// 分岐 4——**負値の宣言**は反対端基準（`extent + v`）で絶対値化してから字義どおり
/// 用いられる（要件 3.7）。範囲内なら記録 0 件・範囲外なら成分ごとに debug 1 件。
#[test]
fn negative_origin_resolves_from_opposite_edge_then_is_used_literally() {
    // (ラベル, origin, 期待 start, 期待 debug 件数)。画像は 400×224。
    let cases = [
        // 400-100=300 ∈ [36,356]・224-100=124 ∈ [46,168] ＝ 両成分とも範囲内。
        ("範囲内へ解決", (Some(-100), Some(-100)), (300.0, 124.0), 0),
        // 400-380=20 < 36・224-200=24 < 46 ＝ 両成分とも範囲外でも寄せない。
        ("範囲外へ解決", (Some(-380), Some(-200)), (20.0, 24.0), 2),
        // 反対端ちょうど（0）——負値規約の境界。両成分とも範囲外。
        ("反対端ちょうど", (Some(-400), Some(-224)), (0.0, 0.0), 2),
    ];
    for (label, origin, expected_start, expected_debug) in cases {
        for mode in ALL_MODES {
            let (region, counts) = resolve_counting(origin, RECT, mode);
            assert_capture_alive(&counts);
            assert_eq!(
                region.start(),
                expected_start,
                "{label} / {mode:?}: 負値は反対端基準で解決してから字義どおり用いる"
            );
            assert_eq!(
                counts.debug, expected_debug,
                "{label} / {mode:?}: debug 件数"
            );
        }
    }
    // 対照——負値解決の結果は、同じ位置を非負で宣言したものと逐語一致する
    // （負値規約が「別の経路」になっていないことの証拠）。
    for mode in ALL_MODES {
        let (negative, _) = resolve_counting((Some(-100), Some(-100)), RECT, mode);
        let (nonnegative, _) = resolve_counting((Some(300), Some(124)), RECT, mode);
        assert_eq!(negative, nonnegative, "{mode:?}: 負値解決は非負宣言と同値");
    }
}

/// 不変条件——**宣言された `origin` の解決結果は `validrect` を変えても動かない**。
///
/// `resolve_origin_component` の `range` 引数は記録の判定にのみ使われ、返す値には影響
/// しない（設計 C3 の Invariants）。これが「クランプが残っていない」ことの読み手向けの
/// 証拠である。**宣言が在る場合のみの主張**であることに注意——未宣言のときは書字開始角
/// そのものが `validrect` から決まるため、下の対照テストのとおり当然に動く。
#[test]
fn declared_origin_resolution_is_independent_of_validrect() {
    // 宣言は固定。validrect だけを差し替える（origin が内側になる形・外側になる形・
    // 画像全域へ縮退する形・全 0 の退化形——退化形は warn を出すが値は動かない）。
    let variants: [(&str, (Option<i32>, Option<i32>, Option<i32>, Option<i32>)); 4] = [
        ("基準（origin は内側）", RECT),
        // top100／bottom 224-10=214／left210／right 400-10=390 ＝ origin (200,60) は外側。
        (
            "origin が外側になる矩形",
            (Some(100), Some(-10), Some(210), Some(-10)),
        ),
        ("全未宣言（画像端へ縮退）", (None, None, None, None)),
        ("全 0（退化矩形）", (Some(0), Some(0), Some(0), Some(0))),
    ];
    let declared = (Some(200), Some(60));

    let mut seen_edges = Vec::new();
    for (label, rect) in variants {
        for mode in ALL_MODES {
            let (region, _) = resolve_counting(declared, rect, mode);
            assert_eq!(
                region.start(),
                (200.0, 60.0),
                "{label} / {mode:?}: validrect を変えても宣言 origin の解決結果は動かない\
                 （動いたなら validrect が返値に効いている＝クランプが残っている）"
            );
        }
        seen_edges.push(edges(
            &resolve_counting(declared, rect, WritingMode::HorizontalTb).0,
        ));
    }
    // 前提の確認——validrect は本当に変わっている（不変条件が恒真ではないことの証拠）。
    assert_eq!(
        seen_edges,
        vec![
            (RECT_LEFT, RECT_TOP, RECT_RIGHT, RECT_BOTTOM),
            (210.0, 100.0, 390.0, 214.0),
            (0.0, 0.0, 400.0, 224.0),
            (0.0, 0.0, 0.0, 0.0),
        ],
        "差し替えた validrect が解決後も同一だと、上の不変条件は何も主張しない"
    );
}

/// 対照——上の不変条件は**宣言が在る場合のみ**の主張である。`origin` が未宣言なら、
/// 同じ `validrect` の差し替えで開始点は実際に動く（書字開始角が validrect から決まるため）。
#[test]
fn undeclared_origin_does_move_when_validrect_changes() {
    let (base, _) = resolve_counting((None, None), RECT, WritingMode::HorizontalTb);
    let (moved, _) = resolve_counting(
        (None, None),
        (Some(100), Some(-10), Some(210), Some(-10)),
        WritingMode::HorizontalTb,
    );
    assert_eq!(base.start(), (RECT_LEFT, RECT_TOP));
    assert_eq!(moved.start(), (210.0, 100.0));
    assert_ne!(
        base.start(),
        moved.start(),
        "未宣言時まで validrect 非依存になっていたら、書字開始角の縮退（要件 3.11）が壊れている"
    );
}
