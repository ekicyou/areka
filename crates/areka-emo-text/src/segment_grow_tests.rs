//! # segment_grow_tests — 届いた字の区切りを後ろだけ伸ばす檻（純粋層・兄弟テスト）
//!
//! spec `emo-text-reveal-incremental` 件 1（要件 1.5／1.6）。数千字を 1 字ずつ届けて区切りに
//! かけた字の数が字の数に比例すること、各時点の区切りが全部を区切った結果（`segment_plan`）と
//! 一致すること、消した回数が変わったときの作り直しを確かめる。`budouy` の版を上げたら必ず回す。
//!
//! [`super`]（`segment.rs`）の子モジュールとして接続してある。

use super::*;
use crate::state::CursorCoord;

/// 文字列を `TextItem::Glyph` 列へ（クラスタの定義点で切る）。
fn glyphs(s: &str) -> Vec<TextItem> {
    areka_sakura::cluster::clusters(s)
        .map(TextItem::glyph)
        .collect()
}

fn br() -> TextItem {
    TextItem::LineBreak { ratio: 1.0 }
}

/// `\_l`（run の区切り・通し番号を消費しない）。
fn cursor() -> TextItem {
    TextItem::CursorMove {
        x: CursorCoord::Omitted,
        y: CursorCoord::Omitted,
    }
}

fn reset_count() {
    SEGMENTED_CHARS.with(|c| c.set(0));
}

fn count() -> usize {
    SEGMENTED_CHARS.with(|c| c.get())
}

/// 改行の無い 1 つの run の 4,000 グリフ（1 グリフ 1 字）の日本語。
fn long_run() -> Vec<TextItem> {
    let base = "今日はいい天気ですね明日も晴れるといいなと思いながら散歩に出かけた";
    let items: Vec<TextItem> = glyphs(&base.repeat(4000 / base.chars().count() + 1))
        .into_iter()
        .take(4000)
        .collect();
    assert_eq!(items.len(), 4000);
    for it in &items {
        let TextItem::Glyph { text } = it else {
            panic!("改行が混ざった")
        };
        assert_eq!(text.chars().count(), 1, "1 グリフ 1 字でない: {text:?}");
    }
    items
}

/// `steps` を繰り返した数ずつ items を届け、届くたびに覚えの計画を 1 回取る（最後は全部）。
/// 各時点で `check(memo, k)` を呼ぶ。
fn feed(items: &[TextItem], steps: &[usize], mut check: impl FnMut(&mut BreakMemo, usize)) {
    let mut memo = BreakMemo::default();
    let mut k = 0;
    for &step in steps.iter().cycle() {
        if k == items.len() {
            break;
        }
        k = (k + step).min(items.len());
        check(&mut memo, k);
    }
}

// ── 要件 1.5: 区切りにかけた字の数は届いた字の数に比例する ──

fn assert_linear_total(steps: &[usize]) {
    let items = long_run();
    reset_count();
    let mut last = SegmentPlan::default();
    feed(&items, steps, |memo, k| {
        last = memo.plan(0, &items[..k]).clone();
    });
    let total = count();
    assert!(
        total <= 6 * 4000,
        "刻み {steps:?}: 区切りにかけた字の合計 {total} が 6 × 4,000 を超えた"
    );
    // 数えに影響しないよう、正との比べは数えた後に。
    assert_eq!(
        last,
        segment_plan(&items),
        "刻み {steps:?}: 最後の計画が違う"
    );
}

#[test]
fn four_thousand_glyphs_one_at_a_time_segment_linear_chars() {
    assert_linear_total(&[1]);
}

#[test]
fn four_thousand_glyphs_several_at_a_time_segment_linear_chars() {
    assert_linear_total(&[3]);
    assert_linear_total(&[2, 5, 1, 7]);
}

// ── 要件 1.2・1.6: 各時点の区切りは全部を区切った結果と同じ ──

/// 漢字・かな・カタカナ・英数・約物・結合文字・`\n`・`\n\n`・`\_l` を混ぜた台本。
fn mixed_script() -> Vec<TextItem> {
    let mut items = glyphs("今日はいい天気ですね。カタカナのテストもABC123で、");
    items.push(br());
    items.extend(glyphs("か\u{309A}き\u{309A}く\u{309A}の話！"));
    items.push(br());
    items.push(br());
    items.extend(glyphs("Hello, world! 明日も晴れるといいな"));
    items.push(cursor());
    items.extend(glyphs("「選択肢」その一、ふたつめ…"));
    items.push(cursor());
    items.push(br());
    items.extend(glyphs("最後のひと続きはまだ開いている"));
    items
}

/// 各時点で `segment_plan(&items[..k])` と等しく、計画を新しく計算した回数がちょうど 1 増える。
/// 同じ項目でもう一度取ると覚えた計画が返り、回数は増えない。
fn assert_matches_every_step(items: &[TextItem], steps: &[usize]) {
    let mut runs = 0;
    feed(items, steps, |memo, k| {
        let got = memo.plan(0, &items[..k]).clone();
        assert_eq!(got, segment_plan(&items[..k]), "刻み {steps:?}・k = {k}");
        assert_eq!(memo.runs, runs + 1, "刻み {steps:?}・k = {k}: 計算の回数");
        runs = memo.runs;
        assert_eq!(memo.plan(0, &items[..k]), &got, "k = {k}: 覚えた計画");
        assert_eq!(memo.runs, runs, "k = {k}: 同じ鍵で計算し直した");
    });
}

#[test]
fn mixed_script_matches_segment_plan_at_every_step() {
    let items = mixed_script();
    for steps in [
        &[1][..],
        &[1, 2, 3],
        &[5, 1, 7, 2],
        &[4, 3, 1, 1, 6],
        &[2],
        &[11],
    ] {
        assert_matches_every_step(&items, steps);
    }
}

/// run の字の数 `L = 1, 2, 3, 4` の所へ 1〜6 グリフが届く（窓の端の場面）。
#[test]
fn arrivals_when_the_open_run_holds_one_to_four_chars_match_segment_plan() {
    let tails = [
        "天気ですね明日も晴れ",
        "カタカナのテストです",
        "ABC123、かな",
        "か\u{309A}き\u{309A}く\u{309A}の話",
    ];
    for tail in tails {
        let run = glyphs(tail);
        for l in 1..=4 {
            for next in 1..=6 {
                // 前に閉じた run を 1 つ置き、開いている run の頭の番号が 0 でない場面にする。
                let mut items = glyphs("前の行");
                items.push(br());
                let head = items.len();
                items.extend(run.iter().cloned());
                let mut memo = BreakMemo::default();
                let first = head + l.min(run.len());
                let second = (first + next).min(items.len());
                for k in [first, second, items.len()] {
                    assert_eq!(
                        memo.plan(0, &items[..k]),
                        &segment_plan(&items[..k]),
                        "{tail:?}・L = {l}・次 {next}・k = {k}"
                    );
                }
            }
        }
    }
}

/// 短い run（1〜4 字）が改行と `\_l` で次々に閉じる台本でも各時点で一致する。
#[test]
fn short_runs_closed_one_after_another_match_segment_plan() {
    let mut items = Vec::new();
    for (i, run) in ["あ", "いう", "えおか", "きくけこ", "さしすせそ"]
        .iter()
        .enumerate()
    {
        items.extend(glyphs(run));
        items.push(if i % 2 == 0 { br() } else { cursor() });
    }
    items.extend(glyphs("た"));
    for steps in [&[1][..], &[2], &[3, 1], &[1, 4, 2]] {
        assert_matches_every_step(&items, steps);
    }
}

// ── 要件 1.3: 作り直し ──

/// `\c` を模して消した回数を進め、項目を空から書き直すと、各時点で `segment_plan` と等しく、
/// 計算の回数が増える。
#[test]
fn clear_count_change_rebuilds_from_empty_items() {
    let before = mixed_script();
    let after = glyphs("消した後のひと続きは、か\u{309A}も含めて伸びる");
    let mut memo = BreakMemo::default();
    assert_eq!(memo.plan(0, &before), &segment_plan(&before));
    let mut runs = memo.runs;

    // 消した直後（項目は空）。
    assert_eq!(memo.plan(1, &[]), &SegmentPlan::default());
    assert_eq!(memo.runs, runs + 1, "空の作り直しも 1 回");
    runs = memo.runs;

    for k in 1..=after.len() {
        assert_eq!(
            memo.plan(1, &after[..k]),
            &segment_plan(&after[..k]),
            "k = {k}"
        );
        assert_eq!(memo.runs, runs + 1, "k = {k}: 計算の回数");
        runs = memo.runs;
    }

    // 消した回数が変わり、前より長い項目でいきなり届いても作り直す（伸ばしと取り違えない）。
    let longer = mixed_script();
    assert!(longer.len() > after.len());
    assert_eq!(memo.plan(2, &longer), &segment_plan(&longer));
    assert_eq!(memo.runs, runs + 1);
}

/// 消した回数が同じで項目が前より少ない列を渡しても `segment_plan` と等しい（作り直す）。
#[test]
fn same_clears_fewer_items_rebuilds() {
    let items = mixed_script();
    let mut memo = BreakMemo::default();
    assert_eq!(memo.plan(0, &items), &segment_plan(&items));
    for k in [items.len() - 1, items.len() / 2, 3, 1, 0] {
        let runs = memo.runs;
        assert_eq!(
            memo.plan(0, &items[..k]),
            &segment_plan(&items[..k]),
            "k = {k}"
        );
        assert_eq!(memo.runs, runs + 1, "k = {k}: 計算の回数");
    }
    // 少なくした後からまた伸ばしても一致する。
    for k in 1..=items.len() {
        assert_eq!(
            memo.plan(0, &items[..k]),
            &segment_plan(&items[..k]),
            "k = {k}"
        );
    }
}
