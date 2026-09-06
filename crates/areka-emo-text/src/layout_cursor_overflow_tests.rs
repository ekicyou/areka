use super::test_support::{IMAGE, broken_lines, model_rect};
use super::{FixedMetrics, LayoutEngine, PositionedLine, VisibleWindow, WrapPlan};
use crate::region::TextRegion;
use crate::state::{CursorCoord, CursorUnit, TextItem};
use crate::writing::WritingMode;

// ── あふれ判定の不変と行構造の観測（検証表 V8・Requirement 2.8／6.1／6.2／6.3） ──
//
// 兄弟ファイルとの分担（重複させない）:
// - `layout_visible_window_tests.rs` — `\_l` を含まない並びのあふれ判定（3 方向・境界・飽和）。
// - `layout_cursor_wiring_tests.rs` — H1／H4／H5／H6 と範囲外記録の肯定側。**H6 は
//   `あ\_l[10,]あ`（2 行）対 `あ\_l[,]あ`（1 行）を前進の並びで見ている**ので、本ファイルは
//   同じ命題を**行送り方向へ後戻りする並び**という別の観測点で見る（値も並びも重ならない）。
// - `layout_cursor_tests.rs`／`layout_cursor_order_tests.rs` — 軸ごと合成・実効位置・書かれた順。
//
// 本ファイルが見るのは、`\_l` が**行送り方向へ後戻りする行**を作ったときに
// [`LayoutEngine::visible_window`] が返す値である。式は 1 行も変えない（Requirement 2.8）——
// よって期待値は「正しい値」ではなく「**既存の式が返す値**」であり、実装の戻り値を書き写さず
// `layout.rs` のあふれ判定式（`last_far <= boundary` なら非発火・超過なら最小スキップ探索）から
// **手で計算して**書く。手計算の過程は各テストの doc コメントに残す。
//
// 幾何の共通前提（`layout_visible_window_tests.rs` の既存檻と同一）: `FixedMetrics`・
// font 10 → 全角 'あ' の advance 10・pitch `10 + 行間 2 = 12`・折返し閾値 400。

/// 共通前提: 文字描画開始点 `(0, 0)` 宣言・validrect `top 0 / bottom 34 / left 0 / right 400`。
///
/// あふれ境界（横書き＝`validrect.bottom`）が **34** ＝ちょうど 3 行ぶん（行下端 10／22／34）に
/// なるので、4 行目以降が超過する。境界は「3 行ちょうど」という意図を保つため新しい格子へ
/// 導き直した（旧格子は pitch 13 の 3 行目下端 36・新格子は pitch 12 の 3 行目下端 34）。
/// `layout_visible_window_tests.rs` の既存檻と同一の矩形を
/// 使うのは、**`\_l` の有無だけが差分になる**ようにするためである。
fn region_overflow() -> TextRegion {
    let region = TextRegion::resolve(
        &model_rect((Some(0), Some(0)), (Some(0), Some(34), Some(0), Some(400))),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    // 檻の前提を檻にする（fixture が意図した矩形・開始点になっていることの確認）。
    assert_eq!(
        (region.left(), region.top(), region.right(), region.bottom()),
        (0.0, 0.0, 400.0, 34.0)
    );
    assert_eq!(region.start(), (0.0, 0.0));
    region
}

/// 横書きで layout を回す（可視は items 中の全グリフ）。
fn layout_h(items: &[TextItem], region: &TextRegion) -> Vec<PositionedLine> {
    let visible = items
        .iter()
        .filter(|i| matches!(i, TextItem::Glyph { .. }))
        .count();
    LayoutEngine::layout(
        items,
        visible,
        region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    )
}

/// 全角グリフ 1 個（'あ'・advance 10）。
fn glyph() -> TextItem {
    TextItem::Glyph { ch: 'あ' }
}

/// `\_l[,@<value>lh]`（X 省略・Y は `@` 相対の行送り単位＝**後戻りさせるための書式**）。
fn cursor_back_lh(value: f32) -> TextItem {
    TextItem::CursorMove {
        x: CursorCoord::Omitted,
        y: CursorCoord::Relative {
            value,
            unit: CursorUnit::Lh,
        },
    }
}

/// 行矩形を `(left, top, right, bottom)` の組へ落とす（期待値を逐語で並べるため）。
fn rects(lines: &[PositionedLine]) -> Vec<(f32, f32, f32, f32)> {
    lines
        .iter()
        .map(|l| (l.rect.left, l.rect.top, l.rect.right, l.rect.bottom))
        .collect()
}

// ── V8: 後戻り行があるときの可視窓（Requirement 2.8） ──

/// **V8**: あふれた後に `\_l[,@-2lh]あ` を書いて**行送り方向へ後戻りする行**を作ると、
/// 可視窓は既存の式どおり「最新行の遠端で判定する」——後戻りによって最新行が境界の内側へ
/// 戻るので、あふれは**非発火**になる。
///
/// 手計算（前提: `start = (0, 0)`・font 10・pitch 12・境界 `region.bottom() = 34`）。
///
/// 並び `[あ, \n, あ, \n, あ, \n, あ, \_l[,@-2lh], あ]` の行:
/// - 4 個目までは素の改行 4 行 → 行矩形の top は 0／12／24／36（bottom は +10）。
/// - `\_l` 到達時の実効位置は `(inline 10, block 36)`。X は省略＝不動、Y ＝
///   `current.y + (−2) × pitch = 36 − 2×12 = 12`（design.md 解決表「位置 = 基点 + 値 × 係数」）。
/// - 次のグリフで実体化 → 4 行目 `{0, 36, 10, 46}` が閉じ、**5 行目が top 12 で開く**
///   （`inline_pos` は 10 のまま＝X 省略の逐語）→ `{10, 12, 20, 22}`。
///
/// あふれ判定（`layout.rs` の `visible_window`・横書きの行）:
/// - `last_far` ＝最新行（5 行目）の `rect.bottom` ＝ **22**。
/// - `boundary` ＝ `region.bottom()` ＝ **34**。
/// - `22 <= 34` ＝**超えていない** → 既定窓 `{ first_visible_line: 0, block_offset: 0.0 }`。
///
/// **この値は「正しい」のではなく「既存の式が返す値」である**（Requirement 2.8 が式の変更を
/// 禁じている）。4 行目（bottom 46）は境界の外に残ったままスクロールされない——式が最新行だけを
/// 見るからで、後戻り行は式にとって未知の状況である。所見は spec の申し送りへ回す。
///
/// **対照**（同じ観測点で 0 でない値が出ること＝主張が経路ごと素通りして静かに緑になる形を塞ぐ）:
/// `\_l` を書かない `[あ, \n, あ, \n, あ, \n, あ]` は最新行の bottom が 46 > 34 で発火し、
/// 1 行スキップで `46 − 12 = 34 <= 34` → `{ 1, −12.0 }` になる。
#[test]
fn backward_line_after_overflow_stops_the_untouched_overflow_formula() {
    let region = region_overflow();

    // 対照（`\_l` 無し）: 同じ 4 行があふれて 1 行スキップする。
    let control = layout_h(&broken_lines(4), &region);
    assert_eq!(
        rects(&control),
        vec![
            (0.0, 0.0, 10.0, 10.0),
            (0.0, 12.0, 10.0, 22.0),
            (0.0, 24.0, 10.0, 34.0),
            (0.0, 36.0, 10.0, 46.0),
        ],
        "対照は素の改行 4 行（最新行の bottom 46 が境界 34 を超える）"
    );
    assert_eq!(
        LayoutEngine::visible_window(&control, &region, WritingMode::HorizontalTb),
        VisibleWindow {
            first_visible_line: 1,
            block_offset: -12.0
        },
        "対照は同じ観測点で 0 でない窓を出す（本檻の 0 が恒真でないことの証跡）"
    );

    // V8 本体: 末尾に後戻りの `\_l` と 1 グリフを足す。
    let mut items = broken_lines(4);
    items.push(cursor_back_lh(-2.0));
    items.push(glyph());
    let lines = layout_h(&items, &region);
    assert_eq!(
        rects(&lines),
        vec![
            (0.0, 0.0, 10.0, 10.0),
            (0.0, 12.0, 10.0, 22.0),
            (0.0, 24.0, 10.0, 34.0),
            (0.0, 36.0, 10.0, 46.0),
            (10.0, 12.0, 20.0, 22.0),
        ],
        "5 行目は top 12＝4 行目の top 36 より手前（行送り方向へ後戻りしている）"
    );
    assert!(
        lines[4].rect.top < lines[3].rect.top,
        "本檻の観測点は「後戻り行」そのもの——単調な並びになったら前提が崩れている"
    );
    assert_eq!(
        LayoutEngine::visible_window(&lines, &region, WritingMode::HorizontalTb),
        VisibleWindow {
            first_visible_line: 0,
            block_offset: 0.0
        },
        "最新行の遠端 22 は境界 34 を超えない＝既存の式ではあふれ非発火"
    );
}

/// **V8（超過が残る側）**: 後戻りしてもなお最新行が境界を超えるときは、既存の式の
/// **最小スキップ探索**がそのまま走る——探索は行列を先頭から舐めて「最新行が収まる最初の行」を
/// 選ぶので、後戻りで非単調になった行列でも式は 1 ビットも変わらない。
///
/// 手計算（前提は同じ・境界 34）。並び `[あ, \n, …, \n, あ（6 個）, \_l[,@-1lh], あ]`:
/// - 素の 6 行の top は 0／12／24／36／48／60。`\_l` 到達時の実効位置は `(10, 60)`。
/// - Y ＝ `60 − 1×12 = 48` → 6 行目 `{0, 60, 10, 70}` が閉じ、7 行目が top 48 で開く
///   → `{10, 48, 20, 58}`。
///
/// あふれ判定:
/// - `last_far` ＝ 7 行目の bottom ＝ **58** > 34 → 発火。
/// - `origin` ＝先頭行の近端 ＝ **0**。条件は `58 − (top_i − 0) <= 34` すなわち `top_i >= 24`。
/// - 先頭から最初に満たすのは 3 行目（top 24）＝ index **2**。
/// - `block_offset = −block_dir × (24 − 0) = −24.0`（横書きの `block_dir` は +1）。
///
/// 条件を満たす最初の行が index 2 であることは、**探索が実際に走っている**ことの証跡でもある
/// （index 0／1 で止まる実装も、最新行へ飽和する実装〔index 6〕も赤になる）。
#[test]
fn backward_line_that_still_overflows_runs_the_untouched_minimum_skip_search() {
    let region = region_overflow();
    let mut items = broken_lines(6);
    items.push(cursor_back_lh(-1.0));
    items.push(glyph());
    let lines = layout_h(&items, &region);
    assert_eq!(
        rects(&lines),
        vec![
            (0.0, 0.0, 10.0, 10.0),
            (0.0, 12.0, 10.0, 22.0),
            (0.0, 24.0, 10.0, 34.0),
            (0.0, 36.0, 10.0, 46.0),
            (0.0, 48.0, 10.0, 58.0),
            (0.0, 60.0, 10.0, 70.0),
            (10.0, 48.0, 20.0, 58.0),
        ],
        "7 行目は top 48＝6 行目の top 60 より手前（後戻り行）"
    );
    assert_eq!(
        LayoutEngine::visible_window(&lines, &region, WritingMode::HorizontalTb),
        VisibleWindow {
            first_visible_line: 2,
            block_offset: -24.0
        },
        "最新行の遠端 58 に対し top >= 24 が最小スキップ＝index 2・オフセット −24"
    );
}

/// **Requirement 2.8 の逐語**: あふれ判定とスクロールは「`\_l` を使わない場合と**同一の規則**」で
/// 適用される。同じ行矩形の並びを ⑴ `\_l` の絶対座標で作った場合と ⑵ 素の改行で作った場合とで、
/// **行矩形も可視窓も完全に一致する**。
///
/// 手計算（前提: `start = (0, 0)`・pitch 12）。`[あ, \_l[0,12], あ, \_l[0,24], あ, \_l[0,36], あ]`:
/// - 各 `\_l` は X ＝ `origin.x + 0 = 0`（行頭へ戻す）・Y ＝ `origin.y + n`（絶対 px）。
/// - 実体化のたびに現在行が閉じて次の行が top n で開くので、行矩形は素の改行 4 行
///   （top 0／12／24／36）と 1 ビットも変わらない。
/// - よってあふれ判定の入力が同一 → 出力も同一 `{ 1, −12.0 }`（対照と同じ手計算）。
///
/// **構造上の根拠**（DD-9）: [`LayoutEngine::visible_window`] の引数は
/// `(&[PositionedLine], &TextRegion, WritingMode)` だけで、`PositionedLine { rect, glyphs }` は
/// 「分割の由来」を持たない。したがって式が `\_l` を**見分ける手段そのものが無い**——本檻は
/// その不在を、2 経路の出力が完全一致することとして観測する。行矩形の逐語も併せて固定するので、
/// 両経路が同時に壊れる摂動でも逐語側が赤になる。
#[test]
fn overflow_window_is_identical_whether_lines_came_from_cursor_or_newline() {
    let region = region_overflow();
    let absolute_px = |y: f32| TextItem::CursorMove {
        x: CursorCoord::Absolute {
            value: 0.0,
            unit: CursorUnit::Px,
        },
        y: CursorCoord::Absolute {
            value: y,
            unit: CursorUnit::Px,
        },
    };
    let via_cursor = layout_h(
        &[
            glyph(),
            absolute_px(12.0),
            glyph(),
            absolute_px(24.0),
            glyph(),
            absolute_px(36.0),
            glyph(),
        ],
        &region,
    );
    let via_newline = layout_h(&broken_lines(4), &region);

    let expected = vec![
        (0.0, 0.0, 10.0, 10.0),
        (0.0, 12.0, 10.0, 22.0),
        (0.0, 24.0, 10.0, 34.0),
        (0.0, 36.0, 10.0, 46.0),
    ];
    assert_eq!(rects(&via_cursor), expected, "`\\_l` 経路の行矩形（逐語）");
    assert_eq!(rects(&via_newline), expected, "改行経路の行矩形（逐語）");
    assert_eq!(
        via_cursor, via_newline,
        "`\\_l` で分割した行と改行で分割した行は完全に一致する（由来を持たない）"
    );

    let expected_window = VisibleWindow {
        first_visible_line: 1,
        block_offset: -12.0,
    };
    assert_eq!(
        LayoutEngine::visible_window(&via_cursor, &region, WritingMode::HorizontalTb),
        expected_window,
        "手計算: 最新行の bottom 46 > 34・1 行スキップで 46 − 12 = 34 <= 34"
    );
    assert_eq!(
        LayoutEngine::visible_window(&via_newline, &region, WritingMode::HorizontalTb),
        expected_window,
        "同じ式が同じ入力へ適用される（Requirement 2.8）"
    );
}

// ── 行構造: 後戻りの並びでの行数（Requirement 6.1・6.2） ──

/// **Requirement 6.1／6.2 を後戻りの並びで見る**: 少なくとも一方の軸で移動が成立するときだけ
/// `\_l` は行の分割点になり、両軸とも成立しないときは行を分割しない。
///
/// 観測点は `layout_cursor_wiring_tests.rs` の H6（`あ\_l[10,]あ`＝2 行／`あ\_l[,]あ`＝1 行・
/// 前進の並び）と**重ならない**——ここでは行送り方向へ**後戻り**する `\_l[,@-2lh]` と、
/// **同じ位置で解釈に失敗する** `\_l[,@-2zz]`（`parse_cursor_coord` が
/// [`CursorCoord::Invalid`] を返す形。未知サフィックスが数値本体に残る。X は省略）を対にする。
/// 書き手の意図は同じで、片方だけが解決に成功する——成功側だけが行を分割することを行数で固定する。
///
/// 手計算（前提: 素の改行 4 行のあと `\_l` と 1 グリフ）:
/// - 成立側 `\_l[,@-2lh]`: Y ＝ `36 − 24 = 12` → 実体化で 4 行目が閉じ 5 行目が開く → **5 行**。
/// - 不成立側 `\_l[,@-2zz]`: X 省略・Y 解釈不能＝両軸とも移動が成立しない → 完全 no-op で
///   行を分割せず、5 個目のグリフは 4 行目の続きに置かれる → **4 行**。
///   `\_l` を 1 文字も書かなかった `[あ, \n, あ, \n, あ, \n, あ, あ]` と行矩形が完全に一致する
///   ことも併せて固定する（「分割しない」を行数だけで見ると、行が 1 本消える別の壊れ方と
///   区別できない）。
#[test]
fn backward_arrangement_splits_a_line_only_when_a_move_succeeds() {
    let region = region_overflow();
    let base = broken_lines(4);

    // 成立側: Y が後戻りで解決する。
    let mut resolved = base.clone();
    resolved.push(cursor_back_lh(-2.0));
    resolved.push(glyph());
    let resolved_lines = layout_h(&resolved, &region);
    assert_eq!(
        resolved_lines.len(),
        5,
        "Y 軸の移動が成立する＝\\_l は行の分割点になる（R6.1）"
    );
    assert_eq!(
        (resolved_lines[4].rect.left, resolved_lines[4].rect.top),
        (10.0, 12.0),
        "分割された 5 行目は後戻り先（top 12）で、X は省略ゆえ 10 のまま"
    );

    // 不成立側: 同じ位置で Y の解釈に失敗する（X は省略）。
    let mut degraded = base.clone();
    degraded.push(TextItem::CursorMove {
        x: CursorCoord::Omitted,
        y: CursorCoord::Invalid,
    });
    degraded.push(glyph());
    let degraded_lines = layout_h(&degraded, &region);
    assert_eq!(
        degraded_lines.len(),
        4,
        "両軸とも移動が成立しない＝行を分割しない（R6.2）"
    );

    // 「分割しない」は「\\_l を書かなかったのと同じ」まで含む（完全 no-op）。
    let mut without_cursor = base.clone();
    without_cursor.push(glyph());
    let without_cursor_lines = layout_h(&without_cursor, &region);
    assert_eq!(
        degraded_lines, without_cursor_lines,
        "不成立の \\_l は行構造にも位置にも痕跡を残さない"
    );
    assert_eq!(
        rects(&degraded_lines).last().copied(),
        Some((0.0, 36.0, 20.0, 46.0)),
        "5 個目のグリフは 4 行目の続き（行内 10 から）に置かれる"
    );
}

// ── DD-9: 行構造の観測（配置層は内容の無い行を出さない） ──

/// **DD-9 の確認**（`\c[line]` を**実装しない**ことの確認であって、その実装ではない）。
///
/// design.md `:440` は、行が生まれるのは文字が置かれたときだけで `\_l`／`\n` のどちらの
/// 保留も**内容の無い行を作らない**こと・`PositionedLine` にフィールドは足さないこと・
/// `\c[line]` の行数を配置層が供給しないことを定める
/// （開発者裁定 2026-09-04：`\_l` も `\n` と同じく「実体が発行されるまで確定しない座標
/// 指定」）。本テストはそのうち**機械で確かめられる 2 つ**を固定する。
///
/// ⑴ **内容の無い行は 1 本も出ない**: `PositionedLine::glyphs` がそれ自体で内容の有無を
///    表しており、`Vec<PositionedLine>` に**内容の無い行は 1 本も現れない**（行の確定は
///    `finish_pending_line` の `if current.is_empty() { return; }` を通るため）。よって
///    Requirement 6.3 が言う「`\_l` 以外で分割された内容の無いものは行と見なさない」は、
///    現状の行境界がそのまま満たしている——`\c[line]` 自体は実装しない（開発者裁定
///    2026-09-05: SSP はバルーン文字領域をビットマップで保持し areka はグリフを保持する＝
///    哲学が異なり同様にはなりえない）ので、配置層の行境界は**内容のある行だけ**を供給する。後戻りの `\_l` で分割された行も例外ではない。
///
/// ⑵ **行を閉じる場所は有限で局所である**: `finish_line` は `layout.rs` の私有関数で、
///    呼び出しは **3 箇所**（折返しの行送り・最終行の確定・`finish_pending_line` の内側）、
///    `finish_pending_line` の呼び出しは **2 箇所**（保留フラッシュ・`LineBreak` 腕の
///    先行実体化）。⑴ の「内容の無い行は出ない」が成り立つのは、この 5 箇所の入口が
///    `finish_pending_line` の `if current.is_empty() { return; }` という**一つの門に集まっている**
///    からである。
///
/// 件数を逐語で持つのは、**行を閉じる入口が黙って増える経路を塞ぐ**ためである。走査する語
/// `finish_line(`／`finish_pending_line(` は関数定義と呼び出しにしか現れない（`layout.rs` の
/// 説明文は括弧を伴わない `[finish_pending_line]` の形で書かれている）。
#[test]
fn no_content_less_line_is_ever_emitted_and_line_closing_sites_are_pinned() {
    // ⑴ 内容の無い行は 1 本も出ない（後戻りの `\_l` で分割された行を含む）。
    let region = region_overflow();
    let mut items = broken_lines(4);
    items.push(cursor_back_lh(-2.0));
    items.push(glyph());
    let lines = layout_h(&items, &region);
    assert_eq!(lines.len(), 5);
    assert!(
        lines.iter().all(|l| !l.glyphs.is_empty()),
        "内容の有無は `glyphs` で観測でき、内容の無い行は行境界に現れない（R6.3）"
    );
    // 空行が作られうる並び（**先頭が改行**＝実体化の (1) が空の現在行に当たる形）でも
    // 内容の無い行は現れない。上の並びは先頭がグリフなので (1) が空に当たらず、この主張が
    // 素通りしてしまう——同じ観測点で「作られうるのに作られない」を見るために足す。
    // 手計算: `[\n, あ, \_l[,@-1lh], あ]` は 1 行目 `{0, 12, 10, 22}`（先頭の改行で block 12）・
    // `\_l` の Y ＝ `12 − 12 = 0` で 2 行目 `{10, 0, 20, 10}` の **2 行**（空行 0 本）。
    let leading_newline = layout_h(
        &[
            TextItem::LineBreak { ratio: 1.0 },
            glyph(),
            cursor_back_lh(-1.0),
            glyph(),
        ],
        &region,
    );
    assert_eq!(
        rects(&leading_newline),
        vec![(0.0, 12.0, 10.0, 22.0), (10.0, 0.0, 20.0, 10.0)],
        "先頭の改行は空行を作らない（保留のみでは行を開かない）"
    );
    assert!(
        leading_newline.iter().all(|l| !l.glyphs.is_empty()),
        "空行が作られうる並びでも内容の無い行は 1 本も出ない（R6.3）"
    );

    // ⑵ 行を閉じる場所の数（⑴ を支える門の母集団）。
    const LAYOUT_SRC: &str = include_str!("layout.rs");
    // 定義 1 つ＋呼び出し 3 つ。
    assert_eq!(
        LAYOUT_SRC.matches("finish_line(").count(),
        4,
        "`finish_line` は定義 1・呼び出し 3——増えたら「行を閉じる入口」の母集団を数え直すこと"
    );
    // 定義 1 つ＋呼び出し 2 つ。
    assert_eq!(
        LAYOUT_SRC.matches("finish_pending_line(").count(),
        3,
        "`finish_pending_line` は定義 1・呼び出し 2——同上"
    );
    // 門が同一ファイル内の私有関数に閉じていること（外から別経路で行を開かれない）。
    assert!(
        LAYOUT_SRC.contains("fn finish_pending_line("),
        "`finish_pending_line` は `layout.rs` の私有関数（呼び出し元は同ファイルに閉じている）"
    );
}
