//! # segment — 分かち書き境界の計算（純粋層・budouy 消費の唯一の場所）
//!
//! `TextItem` 列（追記正本）を **run**（`TextItem::LineBreak` で区切られた極大 `Glyph`
//! 列）単位で budouy によりセグメント化し、glyph 通し番号上の塊境界列 [`SegmentPlan`] へ
//! 写す。budouy への依存はこのモジュールに封じ込め、layout は [`SegmentPlan`] 値のみを
//! 消費する（budouy 非依存で判断分岐を全網羅できる・[test-only-decision-branches] 適用）。
//!
//! **層規律**: 純粋層——`windows` 系 crate 非依存（budouy も非 windows 依存）。
//!
//! ## 計算規則（design.md「SegmentPlan（segment.rs・新規）」正本）
//!
//! - **run 分割規則（R2.5）**: `TextItem::LineBreak` で区切られた各極大 `Glyph` 列を
//!   独立に budouy へかけ、run をまたいで塊を結合しない。空 run（連続 LineBreak 間・
//!   先頭/末尾 LineBreak）は塊を生まない。
//! - **glyph index 写像（R2.1）**: glyph 通し番号は **全 items 中の `Glyph` のみ**を
//!   0 起点で数えた値（visible gate と同じ数え方——`LineBreak` は番号を消費しない）。
//!   budouy `parse(&str) -> Vec<String>` の各チャンクを `chars().count()` で累積し、
//!   run 内グリフを `(start, len)` へ 1:1 に写す。グリフ単位は Rust `char`（state.rs 正準・
//!   M1 は書記素クラスタ結合なし）ゆえ写像は無損失。
//! - **全文 lookahead（R7.1・INV-1）**: 入力は常に全 `items`（可視 prefix ではない）。
//!   構造的に [`segment_plan`] は全 items を受け取る。
//! - **budouy Parser のライフサイクル（R8.1）**: `static PARSER: OnceLock<budouy::Parser>` を
//!   モジュール内部に持ち、初回のみ `budouy::model::load_default_japanese_parser()`（infallible・
//!   vendored-models 同梱データのロードのみ＝ネットワーク/ファイル I/O なし）でロードして
//!   以後は再利用する。同一入力 → 同一境界（決定論・オフライン CI 整合）。
//!
//! ## 不変条件（Postconditions / Invariants）
//!
//! - segments は昇順・互いに素・全 `Glyph` をちょうど一度ずつ被覆・各塊は単一 run 内。
//! - budouy チャンクの連結は run の原文字列と一致（無損失）。
//! - 同一 items → 同一 plan（決定論・R8.1）。

use std::sync::OnceLock;

use crate::state::TextItem;

/// 1 塊（分かち書きセグメント）。glyph は全 items 中の `Glyph` 通し番号（0 起点）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Segment {
    /// 塊先頭の glyph 通し番号。
    pub start: usize,
    /// 塊のグリフ数（≥ 1）。
    pub len: usize,
}

/// 全 items から計算した塊境界列（run 内で連続・run を跨がない・全グリフを被覆）。
///
/// 不変条件: `segments` は昇順・互いに素・全 `Glyph` を被覆・各塊は単一 run 内。
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SegmentPlan {
    /// 塊境界列（昇順・非重複・被覆）。
    segments: Vec<Segment>,
}

impl SegmentPlan {
    /// `glyph_index` を先頭とする塊（存在すれば）。layout のゲート③が塊先頭検出に使う。
    ///
    /// segments は昇順・互いに素ゆえ、先頭が `glyph_index` に一致する塊は高々 1 個。
    pub fn segment_starting_at(&self, glyph_index: usize) -> Option<Segment> {
        self.segments
            .iter()
            .copied()
            .find(|seg| seg.start == glyph_index)
    }

    /// 全塊の昇順列（テスト・診断用）。
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// 塊境界列から直接 [`SegmentPlan`] を組む（テスト専用の手組み注入口）。
    ///
    /// budouy に依存せず layout ゲート③の判断分岐（塊先決・縮退・plan 非被覆）を全網羅
    /// するため、layout.rs のテストが正準/不整合な plan を任意に構築する用途に限る
    /// （[test-only-decision-branches] の適用・design「テスト形: 手組み SegmentPlan」）。
    #[cfg(test)]
    pub(crate) fn from_segments(segments: Vec<Segment>) -> Self {
        SegmentPlan { segments }
    }
}

/// budouy 既定日本語 parser（初回のみロード・以後再利用）。
///
/// `budouy::Parser: Sync + Send`（task 1 spike でコンパイル証明済み）ゆえ `OnceLock` static で
/// よい（`thread_local!` への退避は不要）。`load_default_japanese_parser` は infallible。
static PARSER: OnceLock<budouy::Parser> = OnceLock::new();

/// 全 items（追記正本）から run 別に分かち書き境界を計算する（純関数・決定論・R2.1/2.5/7.1/8.1）。
///
/// run（`TextItem::LineBreak` 区切りの極大 `Glyph` 列）ごとに独立して budouy にかけ、
/// 各チャンクを glyph 通し番号（全 items 中の `Glyph` のみを 0 起点で数えた値）上の塊へ写す。
/// run をまたいで塊を結合しない。空 run は塊を生まない。
pub fn segment_plan(items: &[TextItem]) -> SegmentPlan {
    let parser = PARSER.get_or_init(budouy::model::load_default_japanese_parser);

    let mut segments: Vec<Segment> = Vec::new();
    // 全 items 中の Glyph のみを 0 起点で数えた通し番号（LineBreak は消費しない）。
    let mut glyph_index: usize = 0;
    // 現在の run のグリフ文字列（budouy 入力）。
    let mut run_text = String::new();
    // 現在の run 先頭の glyph 通し番号。
    let mut run_start = glyph_index;

    // 現在蓄積した run を budouy で分割し segments へ写す。空 run は何もしない。
    let flush_run =
        |segments: &mut Vec<Segment>, run_text: &str, run_start: usize| {
            if run_text.is_empty() {
                return;
            }
            let mut chunk_start = run_start;
            for chunk in parser.parse(run_text) {
                let len = chunk.chars().count();
                // budouy は非空チャンクのみ返す前提（spike で確認）。防御的に空はスキップ。
                if len == 0 {
                    continue;
                }
                segments.push(Segment {
                    start: chunk_start,
                    len,
                });
                chunk_start += len;
            }
        };

    for item in items {
        match item {
            TextItem::Glyph { ch } => {
                run_text.push(*ch);
                glyph_index += 1;
            }
            TextItem::LineBreak { .. } => {
                // run 境界: 蓄積済み run を分割し、次 run を開始する（LineBreak は通し番号を消費しない）。
                flush_run(&mut segments, &run_text, run_start);
                run_text.clear();
                run_start = glyph_index;
            }
        }
    }
    // 末尾 run（末尾に LineBreak が無い場合）を処理。
    flush_run(&mut segments, &run_text, run_start);

    SegmentPlan { segments }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 文字列を `TextItem::Glyph` 列へ（run 構築ヘルパ）。
    fn glyphs(s: &str) -> Vec<TextItem> {
        s.chars().map(|ch| TextItem::Glyph { ch }).collect()
    }

    /// 明示改行マーカー（ratio は境界計算に無関係——LineBreak であることだけが効く）。
    fn br() -> TextItem {
        TextItem::LineBreak { ratio: 1.0 }
    }

    /// plan の各塊が被覆するグリフ列（run 別ではなく全 items 通し番号上の char 列）を復元する。
    /// items 中の Glyph のみを通し番号順に並べた列を、各 Segment の [start, start+len) で切り出す。
    fn glyph_chars(items: &[TextItem]) -> Vec<char> {
        items
            .iter()
            .filter_map(|it| match it {
                TextItem::Glyph { ch } => Some(*ch),
                TextItem::LineBreak { .. } => None,
            })
            .collect()
    }

    // ── R2.5: run 分割（LineBreak をまたいで塊を結合しない） ──

    #[test]
    fn segments_never_span_a_line_break() {
        let mut items = glyphs("今日はいい天気");
        items.push(br());
        items.extend(glyphs("明日も晴れ"));

        // run1 は glyph 通し番号 0..7、run2 は 7..12（LineBreak は番号を消費しない）。
        let run1_end = 7; // "今日はいい天気" は 7 char
        let plan = segment_plan(&items);

        // どの塊も run 境界 run1_end をまたがない（塊は完全に run1 内 または 完全に run2 内）。
        for seg in plan.segments() {
            let seg_end = seg.start + seg.len;
            let wholly_in_run1 = seg_end <= run1_end;
            let wholly_in_run2 = seg.start >= run1_end;
            assert!(
                wholly_in_run1 || wholly_in_run2,
                "塊 {seg:?} が run 境界 {run1_end} をまたいでいる"
            );
        }
        // run 境界がハードカット: 先頭が run1_end ちょうどの塊が存在する（run2 の最初の塊）。
        assert!(
            plan.segment_starting_at(run1_end).is_some(),
            "run2 の先頭 glyph {run1_end} を先頭とする塊が無い（run 境界がハードカットでない）"
        );
    }

    // ── R2.1: 被覆・無損失（昇順・互いに素・全グリフをちょうど一度） ──

    #[test]
    fn single_run_segments_cover_all_glyphs_losslessly() {
        let items = glyphs("今日はいい天気ですね");
        let all = glyph_chars(&items);
        let n = all.len();
        let plan = segment_plan(&items);

        // 昇順・互いに素・被覆: 先頭が 0 から始まり、隙間・重複なく n まで連続。
        let mut expected_start = 0usize;
        for seg in plan.segments() {
            assert_eq!(seg.start, expected_start, "隙間/重複がある: {seg:?}");
            assert!(seg.len >= 1, "空塊がある: {seg:?}");
            expected_start += seg.len;
        }
        assert_eq!(expected_start, n, "全グリフを被覆していない（末尾に到達しない）");

        // 無損失: 各塊の range から復元した char 連結が原グリフ列と一致。
        let mut reconstructed: Vec<char> = Vec::new();
        for seg in plan.segments() {
            reconstructed.extend_from_slice(&all[seg.start..seg.start + seg.len]);
        }
        assert_eq!(reconstructed, all, "塊 range からの復元が原文と不一致（損失あり）");
    }

    // ── R2.1: glyph 通し番号は LineBreak を数えない ──

    #[test]
    fn glyph_index_ignores_line_break() {
        // [G, G, LineBreak, G]: run2 の先頭 glyph 通し番号は 2（LineBreak は消費しない・3 でない）。
        let items = vec![
            TextItem::Glyph { ch: 'a' },
            TextItem::Glyph { ch: 'b' },
            br(),
            TextItem::Glyph { ch: 'c' },
        ];
        let plan = segment_plan(&items);

        // run2（"c"）の塊は start=2。
        let seg = plan
            .segment_starting_at(2)
            .expect("run2 の先頭 glyph 通し番号は 2 のはず（LineBreak を数えない）");
        assert_eq!(seg, Segment { start: 2, len: 1 });
        // start=3 の塊は存在しない（LineBreak が番号を消費していない証左）。
        assert!(plan.segment_starting_at(3).is_none());
    }

    // ── R2.5 edge: 空入力・LineBreak のみ・先頭/末尾/連続 LineBreak → 空 run は塊を生まない ──

    #[test]
    fn empty_items_yield_empty_plan() {
        let plan = segment_plan(&[]);
        assert!(plan.segments().is_empty());
        assert_eq!(plan, SegmentPlan::default());
    }

    #[test]
    fn only_line_breaks_yield_empty_plan() {
        let items = vec![br(), br(), br()];
        let plan = segment_plan(&items);
        assert!(
            plan.segments().is_empty(),
            "LineBreak のみ（空 run のみ）は塊を生まない: {:?}",
            plan.segments()
        );
    }

    #[test]
    fn leading_trailing_and_consecutive_line_breaks_produce_no_empty_segments() {
        // 先頭 LineBreak・連続 LineBreak・末尾 LineBreak を含む: 空 run は塊を生まず、
        // 実グリフ run のみが塊化される。通し番号は LineBreak を数えない。
        let mut items = vec![br()]; // 先頭 LineBreak（空 run）
        items.extend(glyphs("あい")); // run: glyph 0..2
        items.push(br());
        items.push(br()); // 連続 LineBreak（間の run は空）
        items.extend(glyphs("うえお")); // run: glyph 2..5
        items.push(br()); // 末尾 LineBreak（後続 run は空）

        let plan = segment_plan(&items);
        // 空塊なし・全塊 len>=1・被覆は 0..5。
        let mut expected_start = 0usize;
        for seg in plan.segments() {
            assert_eq!(seg.start, expected_start);
            assert!(seg.len >= 1);
            expected_start += seg.len;
        }
        assert_eq!(expected_start, 5, "実グリフ 5 個を過不足なく被覆");
        // 最初の実 run は glyph 0 起点、2 番目の実 run は glyph 2 起点（LineBreak を数えない）。
        assert!(plan.segment_starting_at(0).is_some());
        assert!(plan.segment_starting_at(2).is_some());
    }

    // ── R2.5: ASCII/記号混在 run の受理（panic しない） ──

    #[test]
    fn ascii_and_symbol_mixed_run_is_accepted() {
        let items = glyphs("Hello, world! 今日は。");
        let all = glyph_chars(&items);
        let n = all.len();
        let plan = segment_plan(&items);

        // panic せず、被覆は保たれる。
        let mut expected_start = 0usize;
        for seg in plan.segments() {
            assert_eq!(seg.start, expected_start, "混在 run でも隙間/重複なし: {seg:?}");
            assert!(seg.len >= 1);
            expected_start += seg.len;
        }
        assert_eq!(expected_start, n, "混在 run でも全グリフ被覆");
    }

    // ── R8.1: 決定論（同一 items → 同一 plan） ──

    #[test]
    fn same_items_yield_identical_plan() {
        let mut items = glyphs("今日はいい天気ですね");
        items.push(br());
        items.extend(glyphs("明日も晴れるといいな"));

        let a = segment_plan(&items);
        let b = segment_plan(&items);
        assert_eq!(a, b, "同一 items に対し境界が非決定論的");
    }

    // ── R8.1: 代表和文の実境界ピン（vendored モデルの回帰檻） ──

    /// 代表和文が >=2 塊へ分かれ、`segment_plan` の境界が budouy の実際の分割
    /// （チャンク長）と一致することをピンする（task 1 spike の segment_plan 版）。
    #[test]
    fn representative_japanese_boundaries_match_budouy_chunks() {
        let sentence = "今日はいい天気ですね";
        let items = glyphs(sentence);
        let plan = segment_plan(&items);

        // 期待境界を budouy の生 API から直接算出（実装と独立に同じ真実源を引く）。
        let parser = budouy::model::load_default_japanese_parser();
        let chunks: Vec<String> = parser.parse(sentence);
        assert!(
            chunks.len() >= 2,
            "代表和文が単一塊に潰れている（分かち書きが機能していない）: {chunks:?}"
        );

        let mut expected: Vec<Segment> = Vec::new();
        let mut start = 0usize;
        for chunk in &chunks {
            let len = chunk.chars().count();
            expected.push(Segment { start, len });
            start += len;
        }
        assert_eq!(
            plan.segments(),
            expected.as_slice(),
            "segment_plan の境界が budouy の実チャンク境界と不一致"
        );
    }
}
