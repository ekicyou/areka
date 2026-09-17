use super::test_support::{phys, window};
use super::{DirtyRect, ScrollPlanner, line_fingerprint};
use crate::canvas::{
    ChoiceLineContent, ChoiceRowSegment, ContentCanvas, GlyphRunContent, RegionTransform, Resident,
    ResidentContent, TextEffects,
};
use crate::layout::PositionedGlyph;
use crate::look::StyleId;
use crate::region::ScaleContract;
use crate::writing::WritingMode;

// ── 5.3 R11.4/15.5: 行指紋の装飾番号列（styles）——装飾だけが違う行を再利用しない ──
//
// 行の再利用（変化行検出）の唯一の根拠は `CommittedLine`。文字列・位置・行寸・hover 印だけを
// 見ていた時代は「同じ文字列で装飾だけが違う行」が一致し、古い見た目のまま再利用された。
// 番号列（`PositionedGlyph.style.0` の列）を指紋へ写すことで当該行だけが変化行になる。

/// 行内 n グリフの GlyphRunContent（inline_pos 連番・全角 advance 10・番号列を明示指定）。
/// 番号列の長さは文字数と一致させる（配置が `glyph_styles[ordinal]` を写す不変条件と同じ形）。
fn run_content_styled(text: &str, ids: &[u32]) -> GlyphRunContent {
    let count = text.chars().count();
    assert_eq!(count, ids.len(), "番号列の長さは文字数と一致させる");
    let glyphs = text
        .chars()
        .zip(ids)
        .enumerate()
        .map(|(i, (ch, &id))| PositionedGlyph {
            ch,
            inline_pos: i as f32 * 10.0,
            advance: 10.0,
            style: StyleId(id),
        })
        .collect();
    GlyphRunContent {
        glyphs,
        size: (count as f32 * 10.0, 10.0),
    }
}

/// GlyphRun 住人（非 Choice・ブロック軸位置 dy・番号列指定）。
fn glyph_resident(text: &str, dy: f32, ids: &[u32]) -> Resident {
    Resident {
        content: ResidentContent::GlyphRun(run_content_styled(text, ids)),
        transform: RegionTransform::translation(0.0, dy),
        effects: TextEffects::default(),
    }
}

/// Choice 住人（hover なし・ブロック軸位置 dy・番号列指定）。素描画は GlyphRun と同一ゆえ
/// 指紋も同一の作り方（内包 `run` から番号列を写す）。
fn choice_resident(text: &str, dy: f32, ids: &[u32]) -> Resident {
    let run = run_content_styled(text, ids);
    let w = run.size.0;
    Resident {
        content: ResidentContent::Choice(ChoiceLineContent {
            run,
            segments: vec![ChoiceRowSegment {
                ordinal: 0,
                inline_range: (0.0, w),
            }],
            hovered: None,
            highlight: None,
            band_extent: 10.0,
            band_offset: 0.0,
        }),
        transform: RegionTransform::translation(0.0, dy),
        effects: TextEffects::default(),
    }
}

/// 同じ文字列・同じ位置・同じ行寸・同じ hover 印で**番号列だけ**が違う 2 行は、指紋が異なる
/// （＝変化ありと判定される・R11.4）。文字列が本当に同じであることも述語に含め、「片方の文字列が
/// 違っていたのを装飾のせいと誤認する」経路を塞ぐ。
/// 較正: `line_fingerprint` が番号列を写さない（`styles` を常に空にする）誤りを入れると、
/// 他の 4 欄はすべて一致するので `assert_ne!` と番号列の比較が赤になる。
#[test]
fn same_text_with_different_style_ids_is_a_changed_line() {
    let mode = WritingMode::HorizontalTb;
    // 文字列・位置・行寸は完全に同一。違うのは 2 文字目の装飾番号だけ。
    let plain = glyph_resident("こんにちは", 13.0, &[0, 0, 0, 0, 0]);
    let decorated = glyph_resident("こんにちは", 13.0, &[0, 7, 0, 0, 0]);
    let fp_plain = line_fingerprint(&plain, mode);
    let fp_decorated = line_fingerprint(&decorated, mode);

    // 「文字列が本当に同じ」ことを先に断言する（差分の原因を装飾に限定する）。
    assert_eq!(
        fp_plain.text, "こんにちは",
        "指紋の文字列は行の文字そのもの"
    );
    assert_eq!(
        fp_plain.text, fp_decorated.text,
        "2 行の文字列は同一（差は装飾番号だけ）"
    );
    // 他の欄も同一であることを断言する（位置・行寸・hover 印が差の原因ではない）。
    assert_eq!(
        fp_plain.block_pos_bits, fp_decorated.block_pos_bits,
        "ブロック軸位置は同一"
    );
    assert_eq!(fp_plain.extent_bits, fp_decorated.extent_bits, "行寸は同一");
    assert_eq!(
        fp_plain.choice_marker, fp_decorated.choice_marker,
        "hover 印は同一（どちらも非 Choice の 0）"
    );

    // 本題: 差は番号列だけ＝この 1 欄が指紋に写っていなければ 2 行は一致してしまう。
    // 上の 4 つの assert_eq! が「他の欄はすべて同一」を先に固定しているので、この assert_ne! が
    // 真になれる根拠は番号列しかない。
    assert_ne!(
        fp_plain, fp_decorated,
        "同じ文字列でも番号列が違えば変化ありと判定される（R11.4）"
    );
    // 番号列は `PositionedGlyph.style.0` の列そのもの。
    assert_eq!(fp_plain.styles, vec![0, 0, 0, 0, 0], "既定だけの行の番号列");
    assert_eq!(
        fp_decorated.styles,
        vec![0, 7, 0, 0, 0],
        "2 文字目だけ装飾番号 7 の行の番号列"
    );
}

/// 番号列が同じなら指紋も同じ（装飾を含めても「変化なし」の側が壊れていない＝
/// 常に変化ありと言うだけの恒真な指紋になっていないことの対）。
#[test]
fn same_text_with_same_style_ids_is_an_unchanged_line() {
    let mode = WritingMode::HorizontalTb;
    let a = glyph_resident("こんにちは", 13.0, &[0, 7, 0, 0, 0]);
    let b = glyph_resident("こんにちは", 13.0, &[0, 7, 0, 0, 0]);
    assert_eq!(
        line_fingerprint(&a, mode),
        line_fingerprint(&b, mode),
        "文字列も番号列も同じ行は変化なし"
    );
}

/// Choice 住人も内包 `run` の番号列を写す（素描画が GlyphRun と同格ゆえ指紋も同格・R9.5）。
#[test]
fn choice_line_carries_style_ids_in_its_fingerprint() {
    let mode = WritingMode::HorizontalTb;
    let plain = choice_resident("えらぶ", 26.0, &[0, 0, 0]);
    let decorated = choice_resident("えらぶ", 26.0, &[0, 0, 3]);
    let fp_plain = line_fingerprint(&plain, mode);
    let fp_decorated = line_fingerprint(&decorated, mode);
    assert_eq!(fp_plain.text, fp_decorated.text, "選択肢行の文字列は同一");
    assert_eq!(fp_plain.styles, vec![0, 0, 0], "既定だけの選択肢行");
    assert_eq!(fp_decorated.styles, vec![0, 0, 3], "末尾だけ装飾の選択肢行");
    assert_ne!(
        fp_plain, fp_decorated,
        "選択肢行も番号列だけの差で変化ありと判定される"
    );
}

/// 非グリフ住人（シーム）の番号列は空（M1 は `from_layout` で生成されないが、指紋は一様に作る）。
#[test]
fn non_glyph_resident_has_an_empty_style_id_list() {
    let mode = WritingMode::HorizontalTb;
    let resident = Resident {
        content: ResidentContent::Image(crate::canvas::ImageSeam::default()),
        transform: RegionTransform::translation(0.0, 0.0),
        effects: TextEffects::default(),
    };
    assert!(
        line_fingerprint(&resident, mode).styles.is_empty(),
        "非グリフ住人の番号列は空"
    );
}

/// 実経路（`committed_lines`→`derive_dirty`）で、装飾番号だけが変わった行が**その行だけ**
/// ダーティになる（全域ダーティへ縮退しない・R11.4）。指紋レベルでなく変化行検出の出力を見る。
#[test]
fn style_id_change_dirties_only_that_line_through_derive_dirty() {
    let mode = WritingMode::HorizontalTb;
    let contract = ScaleContract::new(1.0, None);
    let surface = (400u32, 224u32);
    // 4 行・全角 4 文字（行寸 40×10）・ブロック軸 0/13/26/39。
    let before = vec![
        glyph_resident("いろはに", 0.0, &[0, 0, 0, 0]),
        glyph_resident("ほへとち", 13.0, &[0, 0, 0, 0]),
        glyph_resident("りぬるを", 26.0, &[0, 0, 0, 0]),
        glyph_resident("わかよた", 39.0, &[0, 0, 0, 0]),
    ];
    // index 1 の行だけ「同じ文字列のまま」装飾番号を付ける（位置・行寸は不変）。
    let after = vec![
        glyph_resident("いろはに", 0.0, &[0, 0, 0, 0]),
        glyph_resident("ほへとち", 13.0, &[5, 5, 0, 0]),
        glyph_resident("りぬるを", 26.0, &[0, 0, 0, 0]),
        glyph_resident("わかよた", 39.0, &[0, 0, 0, 0]),
    ];
    // 文字列が本当に同じであることを先に確かめる（差分の原因を装飾に限定する）。
    for (b, a) in before.iter().zip(&after) {
        assert_eq!(
            line_fingerprint(b, mode).text,
            line_fingerprint(a, mode).text,
            "前後で行の文字列は 1 行も変わっていない"
        );
    }

    let before_canvas = ContentCanvas {
        residents: before,
        size: (400.0, 224.0),
    };
    let after_canvas = ContentCanvas {
        residents: after,
        size: (400.0, 224.0),
    };
    let prev = ScrollPlanner::committed_lines(&before_canvas, mode);
    let next = ScrollPlanner::committed_lines(&after_canvas, mode);
    let changed: Vec<usize> = (0..prev.len()).filter(|&i| prev[i] != next[i]).collect();
    assert_eq!(changed, vec![1], "変化行は装飾を付けた index 1 のみ");

    // blit=0（スクロールなし）＝露出帯なし・dirty は変化行の矩形のみ。
    let (dirty, draw): (Vec<DirtyRect>, Vec<usize>) = ScrollPlanner::derive_dirty(
        &after_canvas,
        &window(0, 0.0),
        mode,
        &contract,
        (0, 0),
        surface,
        &prev,
    );
    assert_eq!(dirty.len(), changed.len(), "dirty 行数 == 変化行数（1）");
    assert_eq!(
        dirty,
        vec![phys(0, 12, 41, 12)],
        "装飾を付けた行 index 1 の矩形のみ・全域ダーティでない"
    );
    assert_eq!(draw, vec![1], "描画対象も当該行のみ");
}
