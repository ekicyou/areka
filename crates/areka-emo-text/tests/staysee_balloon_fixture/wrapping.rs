//! 折返しと、描画範囲への内包を固定する。
//!
//! 出典 spec: `areka-P0-default-balloon-bundle`（要件 **3.5**・設計 **C2** の G 行・
//! Data Models「Domain Model」）。
//!
//! ## ここで固定するもの
//!
//! 検体の描画範囲（本体側 `[22,309]×[20,158]`・相方側 `[22,309]×[20,88]`）と既定書体
//! （`ＭＳ ゴシック` 12）で、本番と同じ折返しエンジン
//! （[`LayoutEngine::layout`]）へ 3 種類の本文を流し、
//!
//! - 行数・各行の文字数・各行の上端、
//! - すべてのグリフ矩形が描画範囲の内側に収まること（1 画素も超えない）、
//! - 折返しが**貪欲**であること（行は入るだけ詰め、次の 1 文字を足すと遠辺を超える）、
//! - 最終行の下端が描画範囲の境界**ちょうど**のときはあふれ扱いにならないこと
//!
//! を固定する。
//!
//! | 本文 | 字種 | 字数 | 期待する行の文字数 |
//! |---|---|---|---|
//! | 全角のみ | 全角 12 px | 60 | 23・23・14 |
//! | 半角のみ | 半角 6 px | 120 | 47・47・26 |
//! | 混在 | 全角と半角の交互 | 90 | 31・31・28 |
//!
//! 1 行の字数は描画範囲の幅 287 px（309 − 22）からの導出である——全角は
//! 22 + 23×12 ＝ 298 で次の 1 字が 310 > 309、半角は 22 + 47×6 ＝ 304 で次が 310 > 309。
//!
//! ## 空振りで緑にならないための備え
//!
//! 「すべてのグリフ矩形が内側」はグリフが 1 つも無くても真になる。そこで内包を言う前に
//! **行数とグリフ総数**を先に固定する。同じ理由で、貪欲充填の検査は**実際に何回発火したか**
//! を数えて固定する（最終行には次の行が無いので、数えないと空回りに気付けない）。
//!
//! 内包の 4 辺のうち、左辺と上辺は先頭のグリフがちょうど接するので、その側を 1 画素狭めれば
//! 内包の検査が赤くなる。下辺は相方側の 5 行の観測で境界ちょうどに接する。**右辺だけは
//! 原理的に接しない**——幅 287 は送り幅 6 でも 12 でも割り切れず、詰められるのは 282 までである。
//! 右辺側の余りが広がっていないこと（＝早すぎる折返しをしていないこと）は貪欲充填の検査が
//! 受け持つ。
//!
//! あふれ判定も、判定そのものが常に非発火なら「境界ちょうどで非発火」は何も示さない。
//! そこで同じ行の並びを**下辺を 1 画素だけ狭めた**描画範囲へ通し、そちらでは発火することを
//! 対照として並べる（[`one_pixel_narrower_bottom_turns_the_same_lines_into_an_overflow`]）。
//! 狭めた描画範囲は検体の宣言 `validrect.bottom,-47` を `-48` に差し替えた定義から解く
//! ——検体のファイルは読むだけで、1 バイトも変えない（要件 2.2）。
//!
//! ## 決定論
//!
//! ファイル読み込み・純粋層の解決・DirectWrite の計測のみ。実 GPU・実窓・実ゴーストを
//! 要さず、同一入力に対して常に同一の結果を返す。既定書体が無い環境では
//! [`super::region`] の寸法の門が先に赤で止まる。

use areka_emo_text::layout::{LayoutEngine, PositionedLine, WrapPlan};
use areka_emo_text::region::TextRegion;
use areka_emo_text::state::TextItem;
use areka_emo_text::writing::WritingMode;
use areka_parsers::balloon::parse_str;

use super::test_support::{
    EXPECTED_ADVANCE_FULL, EXPECTED_ADVANCE_HALF, EXPECTED_FONT_HEIGHT, EXPECTED_LEFT,
    EXPECTED_LINE_BOX, EXPECTED_LINE_PITCH, EXPECTED_RIGHT, EXPECTED_TOP, expected_bottom,
    read_decoded, resolve_staysee, scope_image_size, staysee_metrics,
};

// ── 流し込む 3 本文（設計 C2 の G 行）────────────────────────────────────────

/// 全角のみの本文（60 字）。1 字 12 px。
const FULL_WIDTH_BODY: &str = "あいうえおかきくけこ\
                               あいうえおかきくけこ\
                               あいうえおかきくけこ\
                               あいうえおかきくけこ\
                               あいうえおかきくけこ\
                               あいうえおかきくけこ";
/// 半角のみの本文（120 字）。1 字 6 px。
const HALF_WIDTH_BODY: &str = "abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij\
                               abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij";
/// 全角と半角を交互に並べた本文（90 字）。偶数番目が全角。
const MIXED_BODY: &str = "あaいbうcえdおeかfきgくhけiこj\
                          あaいbうcえdおeかfきgくhけiこj\
                          あaいbうcえdおeかfきgくhけiこj\
                          あaいbうcえdおeかfきgくhけiこj\
                          あaいbうcえdおe";

/// 各本文の字数と、期待する行ごとの文字数（設計 C2 の G 行・Domain Model の導出）。
const EXPECTED_LINE_LENGTHS: [(&str, &str, &[usize]); 3] = [
    ("全角のみ", FULL_WIDTH_BODY, &[23, 23, 14]),
    ("半角のみ", HALF_WIDTH_BODY, &[47, 47, 26]),
    ("全角半角混在", MIXED_BODY, &[31, 31, 28]),
];

/// 相方側の低い枠（高さ 68）にちょうど収まる行数（5 行目の下端が境界 88 ちょうど）。
const KERO_EXACT_FIT_LINES: usize = 5;

// ── 本番と同じ折返し経路 ─────────────────────────────────────────────────────

/// 本文を 1 文字 1 グリフの追記列へ写す（`\n` は行区切りとして写す）。
fn glyph_items(body: &str) -> Vec<TextItem> {
    body.chars()
        .map(|ch| {
            if ch == '\n' {
                TextItem::LineBreak { ratio: 1.0 }
            } else {
                TextItem::Glyph { ch }
            }
        })
        .collect()
}

/// 追記列のうち可視グリフの数（非グリフのアイテムは序数を消費しない）。
fn glyph_count(items: &[TextItem]) -> usize {
    items
        .iter()
        .filter(|it| matches!(it, TextItem::Glyph { .. }))
        .count()
}

/// 当該 scope の描画範囲・既定書体で本文を配置する（全文可視）。
fn layout_in_scope(scope: u32, items: &[TextItem]) -> (TextRegion, Vec<PositionedLine>) {
    let resolved = resolve_staysee(scope);
    assert_eq!(
        resolved.font.height, EXPECTED_FONT_HEIGHT,
        "scope {scope}: 文字の高さが {EXPECTED_FONT_HEIGHT} ではなく {}（以下の期待値はこの高さが前提）",
        resolved.font.height
    );
    let metrics = staysee_metrics(&resolved);
    let lines = LayoutEngine::layout(
        items,
        glyph_count(items),
        &resolved.region,
        resolved.mode,
        resolved.font.height,
        &metrics,
        WrapPlan::CharByChar,
    );
    (resolved.region, lines)
}

/// 検体の `validrect.bottom` を 1 画素だけ狭めた描画範囲を解く（対照専用）。
///
/// 検体の `descript.txt` は読むだけで、差し替えるのは読み込んだ**文字列の写し**である。
/// 差し替えが当たらなければ対照は狭まらないまま緑に見えかねないので、当たったことを
/// その場で確かめる。
fn narrowed_bottom_region(scope: u32) -> TextRegion {
    const DECLARED: &str = "validrect.bottom,-47";
    const NARROWED: &str = "validrect.bottom,-48";
    let text = read_decoded("descript.txt");
    assert!(
        text.contains(DECLARED),
        "検体の `descript.txt` に `{DECLARED}` の行が無い（対照の前提が崩れている）"
    );
    let resolved = resolve_staysee(scope);
    let narrowed = parse_str(&text.replace(DECLARED, NARROWED), None);
    let region = TextRegion::resolve(&narrowed, scope_image_size(scope), resolved.mode);
    assert_eq!(
        region.bottom(),
        expected_bottom(scope) - 1.0,
        "scope {scope}: 対照の下辺が本来の {} より 1 画素狭い {} になっていない（実測 {}）",
        expected_bottom(scope),
        expected_bottom(scope) - 1.0,
        region.bottom()
    );
    region
}

/// 行の上端の期待値（n 行目＝`20 + 14(n − 1)`）。
fn expected_line_top(index: usize) -> f32 {
    EXPECTED_TOP + EXPECTED_LINE_PITCH * index as f32
}

// ── 行数・行の文字数・行の上端（設計 C2 の G 行）──────────────────────────────

/// 3 本文それぞれについて、行数・各行の文字数・各行の上端を逐語で固定する。
///
/// 文字数はグリフの実数であり、行の上端は行送り 14 の等差である。どちらも
/// 「全部で何字流したか」を先に突き合わせるので、配置が途中で落ちても気付ける。
#[test]
fn each_body_wraps_into_the_pinned_line_lengths() {
    for (label, body, expected) in EXPECTED_LINE_LENGTHS {
        let items = glyph_items(body);
        let total: usize = expected.iter().sum();
        assert_eq!(
            glyph_count(&items),
            total,
            "{label}: 流す本文の字数が期待値の合計 {total} ではなく {}",
            glyph_count(&items)
        );

        let (_, lines) = layout_in_scope(0, &items);
        let actual: Vec<usize> = lines.iter().map(|l| l.glyphs.len()).collect();
        assert_eq!(
            actual.len(),
            expected.len(),
            "{label}: 行数が {} ではなく {}（各行の文字数 実測 {:?}／期待 {:?}）",
            expected.len(),
            actual.len(),
            actual,
            expected
        );
        assert_eq!(
            actual, expected,
            "{label}: 各行の文字数が 期待 {expected:?} ではなく 実測 {actual:?}"
        );
        assert_eq!(
            actual.iter().sum::<usize>(),
            total,
            "{label}: 配置されたグリフの総数が {total} ではなく {}（途中で落ちている）",
            actual.iter().sum::<usize>()
        );

        for (n, line) in lines.iter().enumerate() {
            assert_eq!(
                line.rect.top,
                expected_line_top(n),
                "{label}: {} 行目の上端が {} ではなく {}（行送りは {}）",
                n + 1,
                expected_line_top(n),
                line.rect.top,
                EXPECTED_LINE_PITCH
            );
            assert_eq!(
                line.rect.bottom - line.rect.top,
                EXPECTED_LINE_BOX,
                "{label}: {} 行目の行ボックスの丈が {} ではなく {}",
                n + 1,
                EXPECTED_LINE_BOX,
                line.rect.bottom - line.rect.top
            );
        }
    }
}

// ── 描画範囲への内包（設計 C2 の G 行・要件 3.5）──────────────────────────────

/// 3 本文のすべてのグリフ矩形が描画範囲の内側に収まり、1 画素も超えない。
///
/// 内包は対象が 0 件でも真になるので、**行数とグリフ総数を先に固定**してから 4 辺を言う。
/// 左辺と上辺は先頭のグリフがちょうど接するので、描画範囲をその側へ 1 画素狭めれば
/// この検査は赤くなる（下辺が接する場合は
/// [`the_kero_side_fits_exactly_five_lines_without_overflowing`] が受け持つ）。
#[test]
fn every_glyph_stays_inside_the_drawing_range() {
    for (label, body, expected) in EXPECTED_LINE_LENGTHS {
        let items = glyph_items(body);
        let (region, lines) = layout_in_scope(0, &items);
        let bottom = expected_bottom(0);

        assert_eq!(
            lines.len(),
            expected.len(),
            "{label}: 行数が {} ではなく {}（内包はグリフが 0 でも真になるので先に数える）",
            expected.len(),
            lines.len()
        );
        let placed: usize = lines.iter().map(|l| l.glyphs.len()).sum();
        assert_eq!(
            placed,
            glyph_count(&items),
            "{label}: 配置されたグリフが {} 個しか無い（流したのは {} 個）",
            placed,
            glyph_count(&items)
        );
        assert_eq!(
            (region.left(), region.top(), region.right(), region.bottom()),
            (EXPECTED_LEFT, EXPECTED_TOP, EXPECTED_RIGHT, bottom),
            "{label}: 内包を測る描画範囲が期待値と違う（実測 {:?}）",
            (region.left(), region.top(), region.right(), region.bottom())
        );

        for (n, line) in lines.iter().enumerate() {
            for (i, glyph) in line.glyphs.iter().enumerate() {
                let (left, right) = (glyph.inline_pos, glyph.inline_pos + glyph.advance);
                let where_ = format!("{label}: {} 行目 {} 文字目 `{}`", n + 1, i + 1, glyph.ch);
                assert!(
                    left >= EXPECTED_LEFT,
                    "{where_} の左端 {left} が描画範囲の左辺 {EXPECTED_LEFT} より外に出た"
                );
                assert!(
                    right <= EXPECTED_RIGHT,
                    "{where_} の右端 {right} が描画範囲の右辺 {EXPECTED_RIGHT} を超えた\
                     （超過 {} px）",
                    right - EXPECTED_RIGHT
                );
                assert!(
                    line.rect.top >= EXPECTED_TOP,
                    "{where_} の上端 {} が描画範囲の上辺 {EXPECTED_TOP} より外に出た",
                    line.rect.top
                );
                assert!(
                    line.rect.bottom <= bottom,
                    "{where_} の下端 {} が描画範囲の下辺 {bottom} を超えた（超過 {} px）",
                    line.rect.bottom,
                    line.rect.bottom - bottom
                );
            }
        }
    }
}

// ── 貪欲充填の性質（設計 C2 の G 行）────────────────────────────────────────

/// 折返しが貪欲であること——行は入るだけ詰め、次の行の先頭の 1 文字を足すと遠辺を超える。
///
/// 「入るだけ詰めた」だけでは足りない（1 行 1 文字でも成り立つ）ので、「次の 1 文字を足すと
/// 超える」を対にして言う。この対は最終行では次の行が無く空回りするので、**実際に何回
/// 発火したか**を数えて固定する（3 本文 × 各 2 回 ＝ 6 回）。
#[test]
fn wrapping_fills_each_line_greedily() {
    let capacity = EXPECTED_RIGHT - EXPECTED_LEFT;
    assert_eq!(
        capacity, 287.0,
        "描画範囲の幅が 287 ではなく {capacity}（1 行の字数の導出がこの幅に依る）"
    );
    let expected_checks: usize = EXPECTED_LINE_LENGTHS
        .iter()
        .map(|(_, _, lens)| lens.len() - 1)
        .sum();
    let mut checks = 0usize;

    for (label, body, _) in EXPECTED_LINE_LENGTHS {
        let items = glyph_items(body);
        let (_, lines) = layout_in_scope(0, &items);

        for (n, line) in lines.iter().enumerate() {
            let sum: f32 = line.glyphs.iter().map(|g| g.advance).sum();
            assert!(
                sum <= capacity,
                "{label}: {} 行目の送り幅の合計 {sum} が 1 行の幅 {capacity} を超えた",
                n + 1
            );
            let Some(next_head) = lines.get(n + 1).and_then(|l| l.glyphs.first()) else {
                continue;
            };
            assert!(
                sum + next_head.advance > capacity,
                "{label}: {} 行目は送り幅 {sum} で、次の行の先頭 `{}`（送り幅 {}）を足しても \
                 {} で 1 行の幅 {capacity} に収まる——貪欲に詰めていない（早すぎる折返し）",
                n + 1,
                next_head.ch,
                next_head.advance,
                sum + next_head.advance
            );
            checks += 1;
        }
    }

    assert_eq!(
        checks, expected_checks,
        "貪欲充填の検査が {expected_checks} 回ではなく {checks} 回しか発火していない\
         （発火 0 なら何も主張していない）"
    );
}

/// 1 行の字数の導出の前提——半角と全角の送り幅が 6／12 であること。
///
/// [`super::region`] が同じ値を書体の門として固定しているが、本テーマの期待値
/// （23 字・47 字・31 字）はこの 2 値からの計算なので、配置されたグリフの側でも確かめる。
#[test]
fn placed_glyph_advances_match_the_half_and_full_width_values() {
    let items = glyph_items(MIXED_BODY);
    let (_, lines) = layout_in_scope(0, &items);
    let mut halves = 0usize;
    let mut fulls = 0usize;

    for line in &lines {
        for glyph in &line.glyphs {
            let expected = if glyph.ch.is_ascii() {
                halves += 1;
                EXPECTED_ADVANCE_HALF
            } else {
                fulls += 1;
                EXPECTED_ADVANCE_FULL
            };
            assert_eq!(
                glyph.advance, expected,
                "混在: `{}` の送り幅が {expected} ではなく {}（1 行の字数の導出が崩れる）",
                glyph.ch, glyph.advance
            );
        }
    }

    assert_eq!(
        (halves, fulls),
        (45, 45),
        "混在の本文は半角 45 字・全角 45 字のはず（実測 半角 {halves}・全角 {fulls}）"
    );
}

// ── 境界ちょうどはあふれではない（設計 C2 の G 行・要件 3.5）──────────────────

/// 相方側の低い枠（高さ 68）で 5 行を並べると、5 行目の下端が境界 88 ちょうどになり、
/// あふれ扱いにならない。
///
/// 下端が境界に**接している**ことを先に固定してから非発火を言う（接していなければ
/// 非発火は当たり前で、何も示さない）。
#[test]
fn the_kero_side_fits_exactly_five_lines_without_overflowing() {
    let items = five_short_lines();
    let (region, lines) = layout_in_scope(1, &items);
    let bottom = expected_bottom(1);

    assert_eq!(
        region.bottom(),
        bottom,
        "相方側の描画範囲の下辺が {bottom} ではなく {}",
        region.bottom()
    );
    assert_eq!(
        lines.len(),
        KERO_EXACT_FIT_LINES,
        "相方側に並べた行が {KERO_EXACT_FIT_LINES} 行ではなく {} 行",
        lines.len()
    );
    let last_bottom = lines[KERO_EXACT_FIT_LINES - 1].rect.bottom;
    assert_eq!(
        last_bottom, bottom,
        "{KERO_EXACT_FIT_LINES} 行目の下端が境界 {bottom} ちょうどではなく {last_bottom}\
         （境界に接していないと「ちょうどで非発火」は何も示さない）"
    );

    let window = LayoutEngine::visible_window(&lines, &region, WritingMode::HorizontalTb);
    assert_eq!(
        window.first_visible_line, 0,
        "下端が境界ちょうど（{bottom}）なのに先頭可視行が {} へ送られた（あふれ扱い）",
        window.first_visible_line
    );
    assert_eq!(
        window.block_offset, 0.0,
        "下端が境界ちょうど（{bottom}）なのに内容が {} px 送られた",
        window.block_offset
    );
}

/// 同じ 5 行を、下辺を 1 画素だけ狭めた描画範囲へ通すとあふれ扱いになる。
///
/// [`the_kero_side_fits_exactly_five_lines_without_overflowing`] の非発火が
/// 「あふれ判定が常に非発火」ではないことの対照である。
#[test]
fn one_pixel_narrower_bottom_turns_the_same_lines_into_an_overflow() {
    let items = five_short_lines();
    let (region, lines) = layout_in_scope(1, &items);
    let narrowed = narrowed_bottom_region(1);
    let last_bottom = lines[lines.len() - 1].rect.bottom;

    assert_eq!(
        last_bottom,
        narrowed.bottom() + 1.0,
        "対照の下辺 {} に対して最終行の下端が 1 画素だけ外（{}）になっていない",
        narrowed.bottom(),
        last_bottom
    );
    assert_eq!(
        region.bottom() - narrowed.bottom(),
        1.0,
        "対照の描画範囲が本来より 1 画素だけ狭いはず（実測 {} と {}）",
        region.bottom(),
        narrowed.bottom()
    );

    let window = LayoutEngine::visible_window(&lines, &narrowed, WritingMode::HorizontalTb);
    assert_eq!(
        window.first_visible_line, 1,
        "1 画素はみ出したのに先頭可視行が 1 ではなく {}（あふれ判定が発火していない）",
        window.first_visible_line
    );
    assert_eq!(
        window.block_offset, -EXPECTED_LINE_PITCH,
        "1 行送られたなら内容の移動量は {} のはずだが {}",
        -EXPECTED_LINE_PITCH, window.block_offset
    );
}

/// 相方側の枠へ並べる 5 行（1 行 1 文字・行区切りは改行マーカー）。
///
/// 折返しではなく行数そのものを問う検査なので、本文は 1 行 1 文字に留める。
fn five_short_lines() -> Vec<TextItem> {
    let mut items = Vec::new();
    for (n, ch) in "あいうえお".chars().enumerate() {
        if n > 0 {
            items.push(TextItem::LineBreak { ratio: 1.0 });
        }
        items.push(TextItem::Glyph { ch });
    }
    assert_eq!(
        glyph_count(&items),
        KERO_EXACT_FIT_LINES,
        "対照に使う本文は {KERO_EXACT_FIT_LINES} 字のはず"
    );
    items
}
