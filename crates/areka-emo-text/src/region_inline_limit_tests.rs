//! # region_inline_limit_tests — 描画範囲の行内軸の遠辺と、粗いバルーンの警告（純粋層・兄弟テスト）
//!
//! 出典 spec: `areka-P0-emo-text-line-height-canon`（要件 **6.2**／**6.3**／**6.7**・
//! design.md §4.3「折返し基準と描画範囲の二段構え」）。
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
//! 3. 折返し基準が遠辺の外に解決されたバルーンで**警告がちょうど 1 件**記録され、欄に
//!    バルーン名・軸・両方の値が載ること、
//! 4. 折返し基準が遠辺の内（および遠辺と同値）のバルーンでは**警告が出ない**こと。
//!
//! 実際に無条件折返しを行う配置側の判定は別ファイル（`layout_hard_limit_tests.rs`）の担当で、
//! 本ファイルは触れない。
//!
//! ## 相方側と本体側の実データを並べる理由
//!
//! 警告の 1 件は「粗いバルーン定義が実在する」ことに根ざしている。出荷 fixture
//! `emo2-kakukaku` の相方側（`balloonk0s.txt`）は `wordwrappoint.x` を上書きせず、共通
//! `descript.txt` の `-34` を継ぐ。画像 288×203 では 288−34＝**254** に解決され、描画範囲の
//! 右辺 288−48＝**240** の外へ出る。本体側（`balloons0s.txt`）は `wordwrappoint.x,-49` を
//! 自ら上書きしており、400−49＝**351** は右辺 400−44＝**356** の内に収まる。この 2 面を
//! 並べることで、警告が「どのバルーンでも出る」ものでないことが示せる（fixture は
//! 改変しない＝要件 6.7）。数値の出所は `tests/shipped_fixture_region_test.rs` 冒頭の
//! 解決結果の表と同一である。
//!
//! ## 0 件の主張が恒真にならないようにする
//!
//! 「警告が 0 件」という主張は、ログの捕捉そのものが死んでいても成立してしまう。そこで
//! 件数を見るテストは捕捉窓の内側で対照の `error!` を 1 件発行し、その 1 件が数えられて
//! いることを件数の主張と同時に確かめる（`region_vertical_canon_tests.rs` の
//! `assert_capture_alive` と同じ流儀）。
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

/// 数値欄を f32 として読む（`{:?}` 表現の細部に依存しないよう、解析してから比べる）。
fn number_field(event: &CapturedEvent, name: &str) -> f32 {
    let raw = event
        .field(name)
        .unwrap_or_else(|| panic!("欄 {name} が警告に載っていない"));
    raw.parse::<f32>()
        .unwrap_or_else(|_| panic!("欄 {name} の値 {raw} を数値として読めない"))
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

// ── 要件 6.7: 折返し基準が遠辺の外のとき、読み込み 1 回につき警告 1 件 ──

/// 相方側 fixture 相当（折返し基準 254 > 遠辺 240）は警告をちょうど 1 件記録し、
/// 欄にバルーン名・軸・折返し基準・遠辺の 4 つを載せる。
#[test]
fn coarse_balloon_warns_once_with_balloon_axis_and_both_values() {
    let (region, warns, errors) =
        resolve_capturing(&merged(KERO_OVERLAY), KERO_IMAGE, WritingMode::HorizontalTb);
    assert_eq!(
        errors, 1,
        "捕捉窓の対照イベントが数えられていない。この窓の件数の主張は証拠にならない"
    );
    assert_eq!(
        warns.len(),
        1,
        "折返し基準が描画範囲の外のバルーンは警告をちょうど 1 件記録する"
    );

    let warn = &warns[0];
    assert_eq!(warn.field_str("axis"), Some("x"), "横書きの行内軸は x");
    assert_eq!(number_field(warn, "wrap_threshold"), KERO_WRAP_X);
    assert_eq!(number_field(warn, "inline_limit"), KERO_RIGHT);
    let balloon = warn
        .field_str("balloon")
        .expect("欄 balloon が警告に載っていない");
    assert!(
        !balloon.is_empty(),
        "バルーン名の欄を空にしてはならない（名前が無いときもプレースホルダで記録する）"
    );

    // 領域そのものの値も併せて固定する（ログだけが正しくても意味がない）。
    assert_eq!(region.wrap_threshold(), KERO_WRAP_X);
    assert_eq!(region.inline_limit(), KERO_RIGHT);
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

/// 縦書きでは行内軸が y になり、警告の軸欄も遠辺の値も y 側へ切り替わる。
#[test]
fn vertical_balloon_warns_with_the_inline_axis_of_the_block_direction() {
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
            1,
            "{mode:?}: 折返し基準が描画範囲の外のバルーンは警告をちょうど 1 件記録する"
        );

        let warn = &warns[0];
        assert_eq!(
            warn.field_str("axis"),
            Some("y"),
            "{mode:?}: 縦書きの行内軸は y"
        );
        assert_eq!(number_field(warn, "wrap_threshold"), 193.0, "{mode:?}");
        assert_eq!(number_field(warn, "inline_limit"), KERO_BOTTOM, "{mode:?}");
        assert_eq!(region.inline_limit(), KERO_BOTTOM, "{mode:?}");
        assert_eq!(region.wrap_threshold(), 193.0, "{mode:?}");
    }
}
