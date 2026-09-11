//! # region_inline_limit_tests — 描画範囲の行内軸の遠辺（純粋層・兄弟テスト）
//!
//! 出典 spec: `areka-P0-emo-text-line-height-canon`（要件 **6.2**／**6.3**／**6.7**・
//! design.md §4.3「折返し基準と描画範囲の二段構え」）。粗いバルーンの警告についての
//! 主張は spec `areka-P0-emo2-conformance-e2e` の要件 **14.1**／**14.4** で引き直した
//! （2026-09-06 第 2 回改訂・下の「警告の件数はここでは主張しない」）。
//!
//! ## 何を固定するか
//!
//! 開発者裁定（2026-09-05）で、行内軸には**別々の意味を持つ 2 つの値**が立った。
//!
//! - **折返し基準**（`wordwrappoint`・[`TextRegion::wrap_threshold`]）＝「ここを超えたら
//!   折り返す」。行末の禁則文字はここを超えてぶら下がってよい（折返しの遅延・本仕様では未実装）。
//! - **描画範囲の行内軸の遠辺**（`validrect` の当該辺・[`TextRegion::inline_limit`]）＝
//!   「ここを超えてはならない」絶対上限。超えそうなら折返し基準に関わらず無条件に折り返す。
//!
//! 本ファイルが固定するのは**領域を解決する側**だけである。すなわち
//!
//! 1. 遠辺の軸解決が書字方向 3 方向で正しいこと（横書き＝`right`・縦書き 2 方向＝`bottom`）、
//! 2. 遠辺が折返し基準へ丸め込まれず、2 つの値が独立に読めること、
//! 3. 解決そのものが**粗さの警告を書かない**こと——粗いバルーン（折返し基準が遠辺の外）
//!    でも遠辺の内のバルーンでも WARN は 0 件であり、毎フレーム呼ばれても記録が増えない
//!    （要件 14.1）。
//!
//! 3 が言うのは**粗さの記録**についてであって、[`TextRegion::resolve`] が一切ログを持たない
//! という意味ではない——退化した validrect（幅/高さ ≤ 0）の `warn!` と、未指定成分の縮退の
//! `debug!` は別の症状の記録であり、本改訂では 1 行も動かしていない。
//!
//! ## 警告の件数はここでは主張しない（2026-09-06 第 2 回改訂）
//!
//! 粗いバルーン定義を知らせる警告は、かつて [`TextRegion::resolve`] の中にあった。しかし
//! この関数は再追従の判定キーを得るために**毎フレーム**呼ばれるため、実機の一周走行では
//! 生ログ 30,837 行のうち 27,908 行が同じ警告になった（spec
//! `areka-P0-emo2-conformance-e2e` の要件 14・走行 A の実測）。「読み込み 1 回につき 1 件」
//! という意味を持つのは actor の登録口の側なので、警告はそちらへ移した。件数と 4 つの欄の
//! 主張は兄弟テスト `actor_region_warn_tests.rs` が持つ——本ファイルは「解決は書かない」
//! 側だけを固定する。
//!
//! 実際に無条件折返しを行う配置側の判定は別ファイル（`layout_hard_limit_tests.rs`）の担当で、
//! 本ファイルは触れない。
//!
//! ## 相方側と本体側の実データを並べる理由
//!
//! 「粗いバルーン定義」は実在する。出荷 fixture
//! `emo2-kakukaku` の相方側（`balloonk0s.txt`）は `wordwrappoint.x` を上書きせず、共通
//! `descript.txt` の `-34` を継ぐ。画像 288×203 では 288−34＝**254** に解決され、描画範囲の
//! 右辺 288−48＝**240** の外へ出る。本体側（`balloons0s.txt`）は `wordwrappoint.x,-49` を
//! 自ら上書きしており、400−49＝**351** は右辺 400−44＝**356** の内に収まる。この 2 面を
//! 並べることで、粗さが「どのバルーンにもある」ものでないことが示せる（fixture は
//! 改変しない＝要件 6.7）。数値の出所は `tests/shipped_fixture_region_test.rs` 冒頭の
//! 解決結果の表と同一である。同じ 2 面を `actor_region_warn_tests.rs` も使う——警告の
//! 件数の主張と、解決結果の主張が同じ実データの上に載る。
//!
//! ## 0 件の主張が恒真にならないようにする
//!
//! 本ファイルの主張は**すべて「0 件」**であり、この形はログの捕捉そのものが死んでいても
//! 成立してしまう。そこで件数を見るテストは捕捉窓の内側で対照の `error!` を 1 件発行し、
//! その 1 件が数えられていることを件数の主張と同時に確かめる
//! （`region_vertical_canon_tests.rs` の `assert_capture_alive` と同じ流儀）。
//!
//! ## 決定論
//!
//! 実 DPI モニタ・実 GPU・実フォント・実窓を一切要さない。文字列 2 層の写像と純粋層の
//! 解決だけで完結し、同一入力に対して常に同一の結果を返す。`windows` 系 crate を
//! import しない（純粋層の規律）。

use areka_parsers::balloon::{BalloonModel, parse_str};
use log_capture_kit::{CapturedEvent, capture};

use super::TextRegion;
use crate::writing::WritingMode;

/// 相方側 `balloonk0.png` の原寸（image px）。
const KERO_IMAGE: (u32, u32) = (288, 203);
/// 相方側の描画範囲の右辺＝`validrect.right,-48` → 288−48。
const KERO_RIGHT: f32 = 240.0;
/// 相方側の描画範囲の下辺＝`validrect.bottom,-70` → 203−70。
const KERO_BOTTOM: f32 = 133.0;
/// 相方側の折返し基準＝共通 `descript.txt` の `wordwrappoint.x,-34` を継ぐ → 288−34。
const KERO_WRAP_X: f32 = 254.0;

/// 本体側 `balloons0.png` の原寸（image px）。
const SAKURA_IMAGE: (u32, u32) = (400, 224);
/// 本体側の描画範囲の右辺＝`validrect.right,-44` → 400−44。
const SAKURA_RIGHT: f32 = 356.0;
/// 本体側の折返し基準＝`balloons0s.txt` の `wordwrappoint.x,-49` → 400−49。
const SAKURA_WRAP_X: f32 = 351.0;

/// 出荷 fixture `emo2-kakukaku` の共通 `descript.txt` から関連キーだけを写した基層。
///
/// 行の連結を `concat!` で書くのは、本ファイルの改行が CRLF であっても文字列リテラルに
/// 復帰文字が紛れ込まないようにするためである。
const DESCRIPT: &str = concat!(
    "wordwrappoint.x,-34\n",
    "wordwrappoint.y,0\n",
    "validrect.top,0\n",
    "validrect.bottom,0\n",
    "validrect.left,0\n",
    "validrect.right,0\n",
);

/// 相方側の面別上書き層（`balloonk0s.txt`・`wordwrappoint` を上書き**しない**）。
const KERO_OVERLAY: &str = concat!(
    "validrect.top,40\n",
    "validrect.bottom,-70\n",
    "validrect.left,24\n",
    "validrect.right,-48\n",
);

/// 本体側の面別上書き層（`balloons0s.txt`・`wordwrappoint.x` を自ら上書きする）。
const SAKURA_OVERLAY: &str = concat!(
    "wordwrappoint.x,-49\n",
    "validrect.top,46\n",
    "validrect.bottom,-56\n",
    "validrect.left,36\n",
    "validrect.right,-44\n",
);

/// 2 層マージ（共通基層＋面別上書き層）を本番と同じ写像経路で通した `BalloonModel`。
fn merged(overlay: &str) -> BalloonModel {
    parse_str(DESCRIPT, Some(overlay))
}

/// 単層のバルーン定義（軸ごとの分岐を作るための最小の入力）。
fn single_layer(source: &str) -> BalloonModel {
    parse_str(source, None)
}

/// 捕捉窓の中で領域を解決し、`(領域, WARN イベント一覧, ERROR 件数)` を返す。
///
/// 対照の `error!` を窓の内側で 1 件発行するのは、「WARN が 0 件」という主張が捕捉窓の
/// 死によって恒真になるのを防ぐためである（呼出側は ERROR 件数 1 を必ず併せて確認する）。
fn resolve_capturing(
    model: &BalloonModel,
    image_size: (u32, u32),
    mode: WritingMode,
) -> (TextRegion, Vec<CapturedEvent>, usize) {
    let (region, events) = capture(|| {
        tracing::error!("捕捉窓が生きていることの対照イベント");
        TextRegion::resolve(model, image_size, mode)
    });
    let warns: Vec<CapturedEvent> = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .cloned()
        .collect();
    let errors = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .count();
    (region, warns, errors)
}

// ── 要件 6.2: 描画範囲の行内軸の遠辺を軸解決して保持する ──

/// 遠辺は書字方向で軸が切り替わる——横書きは `validrect.right`・縦書き 2 方向は
/// `validrect.bottom`。対照として右辺と下辺が別値であることを併せて示す
/// （同値なら軸の取り違えを見分けられない）。
#[test]
fn inline_limit_is_the_validrect_far_edge_in_every_writing_mode() {
    let kero = merged(KERO_OVERLAY);

    let horizontal = TextRegion::resolve(&kero, KERO_IMAGE, WritingMode::HorizontalTb);
    assert_eq!(horizontal.inline_limit(), KERO_RIGHT);
    assert_eq!(
        horizontal.inline_limit(),
        horizontal.right(),
        "横書きの遠辺は解決後の validrect.right と同じ値でなければならない"
    );

    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        let vertical = TextRegion::resolve(&kero, KERO_IMAGE, mode);
        assert_eq!(vertical.inline_limit(), KERO_BOTTOM, "{mode:?}");
        assert_eq!(
            vertical.inline_limit(),
            vertical.bottom(),
            "{mode:?}: 縦書きの遠辺は解決後の validrect.bottom と同じ値でなければならない"
        );
    }

    assert_ne!(
        KERO_RIGHT, KERO_BOTTOM,
        "右辺と下辺が同値では軸の取り違えを見分けられない"
    );
}

/// 遠辺は折返し基準へ丸め込まれない——粗いバルーンでは 2 つの値が食い違ったまま
/// 独立に読める（design §4.3 の「丸め込み案は採らない」・要件 6.3）。
#[test]
fn inline_limit_is_not_rounded_toward_the_wrap_threshold() {
    let region = TextRegion::resolve(&merged(KERO_OVERLAY), KERO_IMAGE, WritingMode::HorizontalTb);
    assert_eq!(region.wrap_threshold(), KERO_WRAP_X);
    assert_eq!(region.inline_limit(), KERO_RIGHT);
    assert!(
        region.wrap_threshold() > region.inline_limit(),
        "本 fixture は折返し基準（254）が遠辺（240）の外にあるという前提で書かれている"
    );
}

// ── 要件 14.1（2026-09-06 第 2 回改訂）: 解決はログを書かない ──

/// 相方側 fixture 相当（折返し基準 254 > 遠辺 240）を解決しても警告は 1 件も出ない——
/// 粗さを知らせる警告は actor の登録口が持つ（`actor_region_warn_tests.rs`）。値そのものは
/// 従来どおり 2 つとも保持される。
#[test]
fn resolving_a_coarse_balloon_writes_no_warning() {
    let (region, warns, errors) =
        resolve_capturing(&merged(KERO_OVERLAY), KERO_IMAGE, WritingMode::HorizontalTb);
    assert_eq!(
        errors, 1,
        "捕捉窓の対照イベントが数えられていない。この窓の 0 件の主張は証拠にならない"
    );
    assert_eq!(
        warns.len(),
        0,
        "解決は純粋であり、粗いバルーンでもログを書かない（要件 14.1）: {warns:?}"
    );

    // 領域そのものの値は従来どおり（ログを外したことで解決結果が動いていないことの証拠）。
    assert_eq!(region.wrap_threshold(), KERO_WRAP_X);
    assert_eq!(region.inline_limit(), KERO_RIGHT);
    assert!(
        region.wrap_threshold() > region.inline_limit(),
        "本テストは折返し基準（254）が遠辺（240）の外にあるという前提で書かれている"
    );
}

/// 本体側 fixture 相当（折返し基準 351 ≤ 遠辺 356）は警告を記録しない。
#[test]
fn balloon_with_wrap_threshold_inside_the_range_does_not_warn() {
    let (region, warns, errors) = resolve_capturing(
        &merged(SAKURA_OVERLAY),
        SAKURA_IMAGE,
        WritingMode::HorizontalTb,
    );
    assert_eq!(
        errors, 1,
        "捕捉窓の対照イベントが数えられていない。この窓の 0 件の主張は証拠にならない"
    );
    assert_eq!(
        warns.len(),
        0,
        "折返し基準が描画範囲の内にあるバルーンは警告を記録しない"
    );
    assert_eq!(region.wrap_threshold(), SAKURA_WRAP_X);
    assert_eq!(region.inline_limit(), SAKURA_RIGHT);
    assert!(region.wrap_threshold() < region.inline_limit());
}

/// 折返し基準が遠辺と**同値**のときは「外」ではない——警告は出ない（境界の向きの固定）。
#[test]
fn wrap_threshold_equal_to_the_far_edge_does_not_warn() {
    // 画像 288×203 に対し validrect.right,240 と wordwrappoint.x,240 で両者を同値にする。
    let source = concat!(
        "wordwrappoint.x,240\n",
        "validrect.top,40\n",
        "validrect.bottom,-70\n",
        "validrect.left,24\n",
        "validrect.right,240\n",
    );
    let (region, warns, errors) =
        resolve_capturing(&single_layer(source), KERO_IMAGE, WritingMode::HorizontalTb);
    assert_eq!(
        errors, 1,
        "捕捉窓の対照イベントが数えられていない。この窓の 0 件の主張は証拠にならない"
    );
    assert_eq!(
        region.wrap_threshold(),
        region.inline_limit(),
        "本テストは 2 値が同値であるという前提で書かれている"
    );
    assert_eq!(
        warns.len(),
        0,
        "遠辺と同値は「外」ではない（超えたときだけ警告する）"
    );
}

/// 縦書きでも解決はログを書かない。行内軸が y へ切り替わったことは、警告の欄ではなく
/// 解決結果の 2 値（折返し基準 193・遠辺 133＝`validrect.bottom`）で示す
/// （軸欄の主張は `actor_region_warn_tests.rs` が持つ）。
#[test]
fn resolving_a_vertical_coarse_balloon_writes_no_warning() {
    // 画像高さ 203 に対し wordwrappoint.y,-10 → 193 が描画範囲の下辺 133 の外。
    let source = concat!(
        "wordwrappoint.x,-34\n",
        "wordwrappoint.y,-10\n",
        "validrect.top,40\n",
        "validrect.bottom,-70\n",
        "validrect.left,24\n",
        "validrect.right,-48\n",
    );
    let model = single_layer(source);
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        let (region, warns, errors) = resolve_capturing(&model, KERO_IMAGE, mode);
        assert_eq!(
            errors, 1,
            "{mode:?}: 捕捉窓の対照イベントが数えられていない"
        );
        assert_eq!(
            warns.len(),
            0,
            "{mode:?}: 解決は純粋であり、粗いバルーンでもログを書かない（要件 14.1）: {warns:?}"
        );

        assert_eq!(region.inline_limit(), KERO_BOTTOM, "{mode:?}");
        assert_eq!(region.wrap_threshold(), 193.0, "{mode:?}");
    }
}
