//! # segment — 分かち書き境界の計算（純粋層・budouy 消費の唯一の場所）
//!
//! `TextItem` 列を **run**（`TextItem::LineBreak` で区切られた極大 `Glyph`
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
//!   run の文字列は各グリフ（書記素クラスタ）の文字列の連結。budouy `parse(&str) -> Vec<String>`
//!   の各チャンクのバイト長を `assign_items_to_chunks` へ渡し、各グリフを「先頭バイトが属する
//!   塊」へ入れて `(start, len)` へ写す。チャンクの境界がクラスタの途中に落ちても（例:
//!   「か゚」の結合文字の手前）クラスタは前の塊に入り割れず、空になった塊は生まないので、
//!   写像は無損失（全グリフをちょうど一度ずつ被覆）。
//! - **何で区切るか（R7.1・INV-1）**: [`segment_plan`] は渡された `items` の全部で区切る
//!   （見えている字だけには切らない）。ただし字は台本のタグとタグの間のひと続きごとに、
//!   それぞれの時刻に分かれて届くので、届いた字の列は台詞の途中までのことが多い。何を渡すかは
//!   呼び手が決める——再生の前に知らされた台本の全部から作った区間の全文（`lookahead.rs` の
//!   `TalkLookahead` が区間ごとに 1 度だけ区切る）と、行を割り当てる関数 `arrange_lines`
//!   （`actor_present.rs`）が全文を持たないとき・届いた字の列が全文の先頭と食い違うときに
//!   区切る届いた字の列の 2 通りである。後者は字が届くたびに末尾の塊が変わりうる。
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

/// 渡された items の全部（区間の全文、または届いた字の列——モジュール doc「何で区切るか」）から
/// run 別に分かち書き境界を計算する（純関数・決定論・R2.1/2.5/7.1/8.1）。
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
    // 現在の run の各グリフの文字列のバイト長（序数順）。
    let mut run_item_lens: Vec<usize> = Vec::new();
    // 現在の run 先頭の glyph 通し番号。
    let mut run_start = glyph_index;

    // 現在蓄積した run を budouy で分割し segments へ写す。空 run は何もしない。
    let flush_run =
        |segments: &mut Vec<Segment>, run_text: &str, run_item_lens: &[usize], run_start: usize| {
            if run_text.is_empty() {
                return;
            }
            let chunk_byte_lens: Vec<usize> =
                parser.parse(run_text).iter().map(String::len).collect();
            segments.extend(assign_items_to_chunks(
                &chunk_byte_lens,
                run_item_lens,
                run_start,
            ));
        };

    for item in items {
        match item {
            TextItem::Glyph { text } => {
                run_text.push_str(text);
                run_item_lens.push(text.len());
                glyph_index += 1;
            }
            TextItem::LineBreak { .. } => {
                // run 境界: 蓄積済み run を分割し、次 run を開始する（LineBreak は通し番号を消費しない）。
                flush_run(&mut segments, &run_text, &run_item_lens, run_start);
                run_text.clear();
                run_item_lens.clear();
                run_start = glyph_index;
            }
            TextItem::CursorMove { .. } => {
                // カーソル指定も run 境界（非グリフ＝通し番号を消費しない）。カーソルジャンプを
                // 跨いで語分割塊を結合しないよう、LineBreak と同様に蓄積 run を分割する。
                flush_run(&mut segments, &run_text, &run_item_lens, run_start);
                run_text.clear();
                run_item_lens.clear();
                run_start = glyph_index;
            }
        }
    }
    // 末尾 run（末尾に LineBreak が無い場合）を処理。
    flush_run(&mut segments, &run_text, &run_item_lens, run_start);

    SegmentPlan { segments }
}

/// run の各アイテムを「先頭バイトが属する塊」へ割り当て、空でない塊を序数の列にする（純粋・budouy 非依存）。
/// `chunk_byte_lens`: budouy の各塊のバイト長（連結が run の文字列と一致する前提）。
/// `item_byte_lens`: run の各アイテムの文字列のバイト長（序数順）。
///
/// 塊の末尾がアイテムの途中に落ちたら、そのアイテムは前の塊に入る（境界を後ろへ寄せる）。
/// どのアイテムも始まらない塊は `Segment` を生まない。塊の合計を超える位置のアイテム
/// （前提が崩れたときだけ）は最後の塊へ入れる（記録は出さない）。
fn assign_items_to_chunks(
    chunk_byte_lens: &[usize],
    item_byte_lens: &[usize],
    run_start: usize,
) -> Vec<Segment> {
    let last_chunk = chunk_byte_lens.len().saturating_sub(1);
    let mut segments: Vec<Segment> = Vec::new();
    // 今見ている塊の番号とその累積末尾（バイト）。
    let mut chunk = 0usize;
    let mut chunk_end = chunk_byte_lens.first().copied().unwrap_or(0);
    // 直前の Segment が属する塊の番号。
    let mut segment_chunk: Option<usize> = None;
    // 今のアイテムの先頭バイト位置。
    let mut item_start = 0usize;

    for (i, &item_len) in item_byte_lens.iter().enumerate() {
        while item_start >= chunk_end && chunk < last_chunk {
            chunk += 1;
            chunk_end += chunk_byte_lens[chunk];
        }
        match segments.last_mut() {
            Some(seg) if segment_chunk == Some(chunk) => seg.len += 1,
            _ => {
                segments.push(Segment {
                    start: run_start + i,
                    len: 1,
                });
                segment_chunk = Some(chunk);
            }
        }
        item_start += item_len;
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 文字列を `TextItem::Glyph` 列へ（run 構築ヘルパ・クラスタの定義点で切る）。
    fn glyphs(s: &str) -> Vec<TextItem> {
        areka_sakura::cluster::clusters(s)
            .map(TextItem::glyph)
            .collect()
    }

    /// 明示改行マーカー（ratio は境界計算に無関係——LineBreak であることだけが効く）。
    fn br() -> TextItem {
        TextItem::LineBreak { ratio: 1.0 }
    }

    /// plan の各塊が被覆するグリフ列（run 別ではなく全 items 通し番号上のクラスタ列）を復元する。
    /// items 中の Glyph のみを通し番号順に並べた列を、各 Segment の [start, start+len) で切り出す。
    fn glyph_chars(items: &[TextItem]) -> Vec<&str> {
        items
            .iter()
            .filter_map(|it| match it {
                TextItem::Glyph { text } => Some(&**text),
                TextItem::LineBreak { .. } | TextItem::CursorMove { .. } => None,
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
        assert_eq!(
            expected_start, n,
            "全グリフを被覆していない（末尾に到達しない）"
        );

        // 無損失: 各塊の range から復元したクラスタ連結が原グリフ列と一致。
        let mut reconstructed: Vec<&str> = Vec::new();
        for seg in plan.segments() {
            reconstructed.extend_from_slice(&all[seg.start..seg.start + seg.len]);
        }
        assert_eq!(
            reconstructed, all,
            "塊 range からの復元が原文と不一致（損失あり）"
        );
    }

    // ── R2.1: glyph 通し番号は LineBreak を数えない ──

    #[test]
    fn glyph_index_ignores_line_break() {
        // [G, G, LineBreak, G]: run2 の先頭 glyph 通し番号は 2（LineBreak は消費しない・3 でない）。
        let items = vec![
            TextItem::glyph("a"),
            TextItem::glyph("b"),
            br(),
            TextItem::glyph("c"),
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
            assert_eq!(
                seg.start, expected_start,
                "混在 run でも隙間/重複なし: {seg:?}"
            );
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

    // ── R4.2: 塊をクラスタの序数へ写す規則（budouy 無し・手組みの数で全分岐） ──

    /// 塊の末尾とアイテムの末尾が一致する: 塊ごとにそのまま 1 つの Segment になる。
    #[test]
    fn assign_chunk_ends_on_item_boundaries() {
        let segs = assign_items_to_chunks(&[3, 6], &[3, 3, 3], 10);
        assert_eq!(
            segs,
            vec![Segment { start: 10, len: 1 }, Segment { start: 11, len: 2 }]
        );
    }

    /// 塊の末尾がアイテムの途中に落ちる: そのアイテムは前の塊へ入り、次の塊が短くなる。
    #[test]
    fn assign_chunk_end_inside_an_item_moves_the_item_to_the_earlier_chunk() {
        // 塊 [0,2) [2,6)・アイテム [0,3) [3,6): 境界 2 はアイテム 0 の途中。
        let segs = assign_items_to_chunks(&[2, 4], &[3, 3], 0);
        assert_eq!(
            segs,
            vec![Segment { start: 0, len: 1 }, Segment { start: 1, len: 1 }]
        );
    }

    /// 塊が丸ごと 1 つのアイテムの中に入る: 数が 0 の塊は Segment を生まない。
    #[test]
    fn assign_chunk_inside_an_item_yields_no_empty_segment() {
        // 塊 [0,1) [1,2) [2,6)・アイテム [0,3) [3,6): 真ん中の塊にはどのアイテムも始まらない。
        let segs = assign_items_to_chunks(&[1, 1, 4], &[3, 3], 0);
        assert_eq!(
            segs,
            vec![Segment { start: 0, len: 1 }, Segment { start: 1, len: 1 }]
        );
    }

    /// 塊の合計を超える位置のアイテム（前提が崩れたときだけ）は最後の塊へ入る。
    #[test]
    fn assign_items_past_the_last_chunk_join_the_last_chunk() {
        let segs = assign_items_to_chunks(&[3, 3], &[3, 3, 3, 3], 5);
        assert_eq!(
            segs,
            vec![Segment { start: 5, len: 1 }, Segment { start: 6, len: 3 }]
        );
        // 塊が 1 つも無くても、アイテムを落とさず 1 つの塊にまとめる。
        let segs = assign_items_to_chunks(&[], &[3, 3], 0);
        assert_eq!(segs, vec![Segment { start: 0, len: 2 }]);
    }

    /// budouy の実物が結合文字 U+309A の手前で切る「か゚き゚く゚の話」でも、
    /// クラスタを割らずに塊の数を `[1, 2, 1, 1]`（合計 5＝クラスタ数）へ写す。
    #[test]
    fn budouy_chunk_boundary_inside_a_cluster_keeps_the_cluster_whole() {
        let sentence = "か\u{309A}き\u{309A}く\u{309A}の話";
        // 前提: budouy の塊の境界がクラスタの途中に落ちる例であること。
        let parser = budouy::model::load_default_japanese_parser();
        assert_eq!(
            parser.parse(sentence),
            vec!["か", "\u{309A}き\u{309A}く", "\u{309A}の", "話"],
            "budouy の塊が調査時と変わった"
        );

        let items = glyphs(sentence);
        assert_eq!(items.len(), 5, "クラスタは 5 つ");
        let lens: Vec<usize> = segment_plan(&items)
            .segments()
            .iter()
            .map(|seg| seg.len)
            .collect();
        assert_eq!(lens, vec![1, 2, 1, 1]);
    }
}
