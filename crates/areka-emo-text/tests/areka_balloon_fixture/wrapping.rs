//! 折り返しと、描画範囲への内包と、物理の寸法への変換を固定する。
//!
//! 出典 spec: `default-balloon-selfmade`（要件 **12.5**・設計「Testing Strategy」の `wrapping.rs`・
//! 「描画範囲の解け方」の表・「文のファイル」の書体）。
//!
//! ## ここで固定するもの
//!
//! 本体側の面 0（`balloons0.png` 326×169）を本番の読み込み（面の系列解決→`descript.txt` に
//! `balloons0s.txt` を重ねたモデル）で解いた範囲と書体（`ＭＳ ゴシック` 12）で、本番と同じ
//! 折り返しの入口（[`LayoutEngine::layout`]・`staysee_balloon_fixture/wrapping.rs` と同じ）へ
//! 3 種類の本文を流し、行数・各行の字数・各行の上端と、すべてのグリフが範囲の内側に収まることを
//! 固定する。
//!
//! | 本文 | 字種 | 字数 | 期待する行の字数 | 1 行目の右端 |
//! |---|---|---|---|---|
//! | 全角だけ | 全角 12 px | 60 | 23・23・14 | 290 |
//! | 半角だけ | 半角 6 px | 120 | 47・47・26 | 296 |
//! | 混ぜた本文 | 全角と半角の交互（全角から） | 90 | 31・31・28 | 296 |
//!
//! ## 導き方（手で書いた値・道具のコードは参照しない）
//!
//! 範囲は左 0・右 326（`validrect.left,0`・`validrect.right,326`）、書き始めは origin の
//! x 14（`balloons0s.txt` の `origin.x,14`）、折り返し基準は 326 − 30 ＝ **296**
//! （`wordwrappoint.x,-30`）。折り返しは「行内位置＋次の字の送り幅 > 296」で起きるので、
//! 1 行に詰められる幅は 296 − 14 ＝ **282** である（遠辺 326 は基準より外なので効かない）。
//!
//! - 全角: 14 + 23×12 ＝ 290 で、次の 1 字が 302 > 296 → 1 行 **23** 字（60 ＝ 23＋23＋14）。
//! - 半角: 14 + 47×6 ＝ 296 で基準**ちょうど**（超えないので入る）、次が 302 > 296 → **47** 字
//!   （120 ＝ 47＋47＋26）。
//! - 混ぜた本文: 全角と半角の 1 組が 18。全角から始まる行は 15 組（270）＋全角 1 字で 282
//!   ちょうど → 31 字で右端 296、次の半角が 302。半角から始まる 2 行目は 15 組＋半角 1 字で
//!   276 → 31 字で右端 290、次の全角が 302。残りが 28 字（90 ＝ 31＋31＋28）。
//!
//! 行の上端は origin の y 11 から行送り 14（`font.height` 12 ＋行間 2）の等差で 11・25・39、
//! 行ボックスの丈は 12（`ＭＳ ゴシック` は `ascent + descent` がちょうど 1em）。3 行目の下端 51 は
//! 範囲の下辺 149（169 − 20）の内である。
//!
//! 相方側（`balloonk0.png` 326×96・範囲 0,0,326,82・origin (14,14)・基準 296）は高さだけが違う。
//! 混ぜた本文 1 本を流すと行の字数は本体側と同じ 31・31・28 で、上端 14・28・42、最後の下端 54 は
//! 下辺 82 の内——下の辺で止まらない（あふれの送りが起きない）ことまで見る。
//!
//! ## 拡大率（`staysee_balloon_fixture/scale.rs` と同じ形）
//!
//! 行の並びは画像の寸法で解いた画像の座標のまま固定し、物理の寸法への変換（[`ScaleContract`]）
//! が k 倍になることを k ＝ 1.25 の手計算の絶対値で言う: 書き始め 14 → 17.5・半角の行の右端
//! 296 → 370・2 行目の上端 25 → 31.25・範囲の下辺 149 → 186.25・行ボックスの丈 12 →
//! `ceil(15.0)` ＝ 15。k ごとに行を解き直して比べる形は置かない（テストが自分で画像の寸法を
//! 渡すと k は入力に入らず、同じ純関数を同じ値で呼び直すだけになる）。
//!
//! ## 空振りで緑にならないための備え
//!
//! 内包はグリフが 0 でも真になるので、行数とグリフの総数を先に固定してから 4 辺を言う。
//! 貪欲に詰めたこと（次の行の先頭を足すと基準を超える）は最終行で空回りするので、発火の回数
//! （本文 3 本 × 各 2 回 ＝ 6 回）を数えて固定する。
//!
//! ## 決定論
//!
//! ファイル読み込み・純粋層の解決・DirectWrite の計測だけ。実 GPU・実窓は使わない。

use areka_emo_text::actor::ResolvedBalloonText;
use areka_emo_text::layout::{LayoutEngine, PositionedLine};
use areka_emo_text::region::{ImagePx, ScaleContract, TextRegion};
use areka_emo_text::writing::WritingMode;

use crate::support::fixture::{
    FULL_WIDTH_BODY, HALF_WIDTH_BODY, MIXED_BODY, assert_rect_inside, glyph_items, lay_out,
    load_face0,
};

use super::test_support::{
    balloon_root, expected_region, expected_wrap_threshold, scope_image_size,
};

// ── 期待値（設計「描画範囲の解け方」の表と書体から手で書いた値）──────────────────

/// scope ごとの面 0 の書き始め（寸法・範囲・折り返し基準は `test_support.rs` の表から引く）。
struct ScopeExpectation {
    scope: u32,
    /// 書き始め（origin の宣言どおり）。
    start: (f32, f32),
}

const SAKURA: ScopeExpectation = ScopeExpectation {
    scope: 0,
    start: (14.0, 11.0),
};

const KERO: ScopeExpectation = ScopeExpectation {
    scope: 1,
    start: (14.0, 14.0),
};

/// 文字の高さ（`font.height,12`）。
const FONT_HEIGHT: f32 = 12.0;
/// 半角・全角の送り幅（`ＭＳ ゴシック` 12 の 0.5em・1em）。
const ADVANCE_HALF: f32 = 6.0;
const ADVANCE_FULL: f32 = 12.0;
/// 行ボックスの丈と行送り（12 ＋行間 2）。
const LINE_BOX: f32 = 12.0;
const LINE_PITCH: f32 = 14.0;

/// 本文・各行の字数・各行の右端（導き方はモジュールの説明）。
const BODIES: [(&str, &str, &[usize], &[f32]); 3] = [
    (
        "全角だけ",
        FULL_WIDTH_BODY,
        &[23, 23, 14],
        &[290.0, 290.0, 182.0],
    ),
    (
        "半角だけ",
        HALF_WIDTH_BODY,
        &[47, 47, 26],
        &[296.0, 296.0, 170.0],
    ),
    (
        "混ぜた本文",
        MIXED_BODY,
        &[31, 31, 28],
        &[296.0, 290.0, 266.0],
    ),
];

// ── 本番と同じ読み込みと折り返しの経路 ──────────────────────────────────────────

/// 本番の読み込み（面の系列解決→面 0 に `descript.txt` と面別の設定を重ねたモデル）で
/// 面 0 の画像の寸法から範囲と書体を解く。
fn resolve_scope(exp: &ScopeExpectation) -> ResolvedBalloonText {
    let (_, model) = load_face0(&balloon_root(), exp.scope);
    ResolvedBalloonText::resolve(&model, scope_image_size(exp.scope))
}

/// 範囲が表のとおりであることを確かめてから、本文を全文可視で流す。
fn layout_body(exp: &ScopeExpectation, body: &str) -> (TextRegion, Vec<PositionedLine>) {
    let resolved = resolve_scope(exp);
    let region = resolved.region;
    let scope = exp.scope;
    assert_eq!(
        (region.left(), region.top(), region.right(), region.bottom()),
        expected_region(scope),
        "scope {scope}: 範囲が設計の表と違う"
    );
    assert_eq!(region.start(), exp.start, "scope {scope}: 書き始め");
    assert_eq!(
        region.wrap_threshold(),
        expected_wrap_threshold(scope),
        "scope {scope}: 折り返し基準"
    );
    assert_eq!(
        resolved.font.height, FONT_HEIGHT,
        "scope {scope}: 文字の高さ（以下の期待値はこの高さが前提）"
    );
    let lines = lay_out(&resolved, &glyph_items(body));
    (region, lines)
}

/// 行数とグリフの総数を先に固定し、すべてのグリフの矩形が範囲の内側にあることを言う。
fn assert_contained(label: &str, region: &TextRegion, lines: &[PositionedLine], total: usize) {
    let placed: usize = lines.iter().map(|l| l.glyphs.len()).sum();
    assert_eq!(
        placed, total,
        "{label}: 置かれたグリフが {placed} 個（流したのは {total} 個・内包は 0 個でも真になる）"
    );
    for (n, line) in lines.iter().enumerate() {
        for (i, g) in line.glyphs.iter().enumerate() {
            let at = format!("{label}: {} 行目 {} 字目 `{}`", n + 1, i + 1, g.text);
            assert_rect_inside(
                &at,
                (
                    g.inline_pos,
                    line.rect.top,
                    g.inline_pos + g.advance,
                    line.rect.bottom,
                ),
                (region.left(), region.top(), region.right(), region.bottom()),
            );
        }
    }
}

// ── 本体側: 行数・字数・上端・右端と内包 ─────────────────────────────────────────

/// 3 本文の行の字数・上端・右端が導いた値どおりで、すべてのグリフが範囲の内側にある。
#[test]
fn sakura_bodies_wrap_into_the_derived_lines_and_stay_inside() {
    for (label, body, lens, ends) in BODIES {
        let (region, lines) = layout_body(&SAKURA, body);
        let actual: Vec<usize> = lines.iter().map(|l| l.glyphs.len()).collect();
        assert_eq!(actual, lens, "{label}: 各行の字数");
        assert_contained(label, &region, &lines, lens.iter().sum());

        for (n, line) in lines.iter().enumerate() {
            let top = SAKURA.start.1 + LINE_PITCH * n as f32;
            assert_eq!(line.rect.top, top, "{label}: {} 行目の上端", n + 1);
            assert_eq!(
                line.rect.bottom - line.rect.top,
                LINE_BOX,
                "{label}: {} 行目の行ボックスの丈",
                n + 1
            );
            let head = &line.glyphs[0];
            assert_eq!(
                head.inline_pos,
                SAKURA.start.0,
                "{label}: {} 行目の書き始め",
                n + 1
            );
            let tail = line.glyphs.last().expect("空の行は無い");
            assert_eq!(
                tail.inline_pos + tail.advance,
                ends[n],
                "{label}: {} 行目の右端",
                n + 1
            );
        }
    }
}

/// 折り返しが貪欲であること——次の行の先頭の 1 字を足すと基準 296 を超える。
///
/// 送り幅も字種ごとに 6／12 であることを同じ走査で確かめる（字数の導出の前提）。
#[test]
fn sakura_wrapping_fills_each_line_up_to_the_threshold() {
    let mut checks = 0usize;
    for (label, body, _, _) in BODIES {
        let (_, lines) = layout_body(&SAKURA, body);
        for (n, line) in lines.iter().enumerate() {
            for g in &line.glyphs {
                let want = if g.text.is_ascii() {
                    ADVANCE_HALF
                } else {
                    ADVANCE_FULL
                };
                assert_eq!(g.advance, want, "{label}: `{}` の送り幅", g.text);
            }
            let Some(next) = lines.get(n + 1).and_then(|l| l.glyphs.first()) else {
                continue;
            };
            let tail = line.glyphs.last().expect("空の行は無い");
            let end = tail.inline_pos + tail.advance;
            assert!(
                end + next.advance > expected_wrap_threshold(SAKURA.scope),
                "{label}: {} 行目は右端 {end} で、次の `{}` を足しても基準 {} を超えない（早すぎる折り返し）",
                n + 1,
                next.text,
                expected_wrap_threshold(SAKURA.scope)
            );
            checks += 1;
        }
    }
    assert_eq!(checks, 6, "貪欲の検査の発火の回数（本文 3 本 × 各 2 回）");
}

// ── 拡大率: 物理の寸法は k 倍 ────────────────────────────────────────────────────

/// 拡大率 1.25 で、画像の座標の行の並びから物理の寸法への変換が k 倍になる。
#[test]
fn scale_multiplies_physical_sizes() {
    let contract = ScaleContract::new(1.25, None);

    for (label, body, _, _) in BODIES {
        let (region, lines) = layout_body(&SAKURA, body);
        assert_eq!(
            contract.to_physical(ImagePx(region.bottom())).0,
            186.25,
            "{label}: 範囲の下辺 149 の物理座標"
        );
        for line in &lines {
            for g in &line.glyphs {
                let l = contract.to_physical(ImagePx(g.inline_pos)).0;
                assert_eq!(l, g.inline_pos * 1.25, "{label}: `{}` の物理の左端", g.text);
            }
        }
    }

    // 手計算の値（モジュールの説明）。
    let (_, half) = layout_body(&SAKURA, HALF_WIDTH_BODY);
    let tail = half[0].glyphs.last().expect("空の行は無い");
    assert_eq!(
        contract
            .to_physical(ImagePx(half[0].glyphs[0].inline_pos))
            .0,
        17.5,
        "書き始め 14 の物理座標"
    );
    assert_eq!(
        contract
            .to_physical(ImagePx(tail.inline_pos + tail.advance))
            .0,
        370.0,
        "半角の行の右端 296 の物理座標"
    );
    assert_eq!(
        contract.to_physical(ImagePx(half[1].rect.top)).0,
        31.25,
        "2 行目の上端 25 の物理座標"
    );
    assert_eq!(
        contract.physical_extent(ImagePx(half[0].rect.bottom - half[0].rect.top)),
        15,
        "行ボックスの丈 12 の物理寸法"
    );
}

// ── 相方側: 下の辺で止まらない長さの本文 1 本 ────────────────────────────────────

/// 相方側へ混ぜた本文 1 本を流すと、行の字数は本体側と同じで、上端は 14 から 14 刻み、
/// すべてのグリフが範囲の内側にあり、あふれの送りは起きない。
#[test]
fn kero_body_stays_inside_without_reaching_the_bottom() {
    let label = "相方側";
    let (region, lines) = layout_body(&KERO, MIXED_BODY);
    let actual: Vec<usize> = lines.iter().map(|l| l.glyphs.len()).collect();
    assert_eq!(actual, [31, 31, 28], "{label}: 各行の字数");
    let tops: Vec<f32> = lines.iter().map(|l| l.rect.top).collect();
    assert_eq!(tops, [14.0, 28.0, 42.0], "{label}: 各行の上端");
    assert_contained(label, &region, &lines, 90);

    let window = LayoutEngine::visible_window(&lines, &region, WritingMode::HorizontalTb);
    assert_eq!(
        (window.first_visible_line, window.block_offset),
        (0, 0.0),
        "{label}: 下の辺で止まった（あふれの送りが起きた）"
    );
}
