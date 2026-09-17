//! `viewbox_draw_decoration.rs` の兄弟檻（task 6.5・要件 3.5／3.6／5.1／5.2／5.6／5.7／5.8／
//! 9.8／12.1／13.3／14.2）。
//!
//! 固定する述語は 5 つ。
//!
//! 1. **区間の切り出し**——連続する同じ番号が 1 つの区間にまとまり、UTF-16 で 2 単位の文字が
//!    2 と数えられる（`segment_text_range` と同じ数え方）。区間に切らず行全体を 1 つにする誤りは
//!    区間数と範囲の両方で赤になる。
//! 2. **既定だけの行**——範囲指定・色の解除・ブラシ生成の**実回数が 0**（保持庫の要素数ではなく
//!    実際に発行した回数を数える）。描画統計と行レイアウトの生成回数は装飾導入前と同じ字義の値。
//! 3. **色の順序**——装飾の色を焼いたあとに重ね表示（hover）の色が勝つ。順序を逆転させると
//!    hover の範囲に装飾の色が残って赤になる。
//! 4. **解除の二重化を避ける**——選択肢の行は呼び手が既に全範囲を解除しているので、装飾の色の
//!    焼き込みは解除を発行しない（非選択肢の行では 1 回発行する）。
//! 5. **既定だけの行の箱寸**——[`line_styles`] を直接呼び、番号列が空・箱寸が `font_height`
//!    （行矩形の [`block_extent`] ではない）へ落ちること（要件 14.2／14.1 の丸め非依存）。較正は
//!    `default_only` を常に偽にする摂動と `extent` を常に [`block_extent`] にする摂動の 2 つで、
//!    どちらも [`default_only_line_keeps_font_height_and_empty_style_ids`] が赤になる。
//!
//! 較正（「区間に切らず行全体へ焼く」誤りを赤にする）は
//! [`decorated_run_paints_only_its_own_glyphs`] が担う——装飾の色が自分の区間の外へ出れば赤。

use super::decoration::{RANGE_CALLS, RESET_CALLS, block_extent, line_styles, style_runs};
use super::test_support::{Rig, build, geo_model, glyph_items, opaque_count};
use super::{DrawStats, ViewboxExecutor};
use crate::canvas::{
    ChoiceLineContent, ChoiceRowSegment, ContentCanvas, GlyphRunContent, HighlightPaint, Resident,
    ResidentContent,
};
use crate::draw::ResolvedFont;
use crate::layout::PositionedGlyph;
use crate::look::{StyleId, StyleTable, TextLook};
use crate::region::{ScaleContract, TextRegion};
use crate::writing::WritingMode;

/// 番号だけを与えたグリフ（位置と送りは区間の切り出しに効かないので 0）。
fn glyph(ch: char, style: StyleId) -> PositionedGlyph {
    PositionedGlyph {
        ch,
        inline_pos: 0.0,
        advance: 0.0,
        style,
    }
}

/// 区間を `(startPosition, length, 番号)` の組へ落とす（比較用）。
fn triples(glyphs: &[PositionedGlyph]) -> Vec<(u32, u32, u32)> {
    style_runs(glyphs)
        .iter()
        .map(|r| (r.range.startPosition, r.range.length, r.style.0))
        .collect()
}

/// 要件 3.6: 連続する同じ番号が 1 区間にまとまり、UTF-16 で 2 単位の文字（`𠮷`）が 2 と数えられる。
/// 行全体を 1 区間にする誤りは区間数（3）で赤になる。
#[test]
fn style_runs_group_consecutive_ids_and_count_utf16_units() {
    let one = StyleId(1);
    let glyphs = [
        glyph('A', StyleId::DEFAULT),
        glyph('𠮷', one), // UTF-16 で 2 単位（サロゲートペア）。
        glyph('B', one),
        glyph('C', StyleId::DEFAULT),
    ];

    assert_eq!(
        triples(&glyphs),
        vec![(0, 1, 0), (1, 3, 1), (4, 1, 0)],
        "連続する同じ番号は 1 区間・2 単位の文字は長さ 2 として累積する（要件 3.6）"
    );
}

/// 要件 14.2: 既定しか使われていない行は区間が 1 つ（番号 0）になる。空行は区間 0 個。
#[test]
fn style_runs_collapse_to_one_default_run_for_undecorated_line() {
    let glyphs: Vec<PositionedGlyph> = "あいう"
        .chars()
        .map(|c| glyph(c, StyleId::DEFAULT))
        .collect();

    assert_eq!(
        triples(&glyphs),
        vec![(0, 3, 0)],
        "既定だけの行は番号 0 の 1 区間（COM 層はここから範囲指定を 1 度も呼ばない）"
    );
    assert!(
        style_runs(&[]).is_empty(),
        "グリフの無い行は区間も 0 個（呼び手は素通しする）"
    );
}

/// 行の箱のブロック軸寸——横書きは高さ・縦書きは幅（書字方向の写像・要件 12.1）。
#[test]
fn block_extent_takes_the_block_axis_of_the_line_box() {
    assert_eq!(
        block_extent((123.0, 45.0), WritingMode::HorizontalTb),
        45.0,
        "横書きのブロック軸は高さ"
    );
    assert_eq!(
        block_extent((123.0, 45.0), WritingMode::VerticalRl),
        123.0,
        "縦書き（rl）のブロック軸は幅"
    );
    assert_eq!(
        block_extent((123.0, 45.0), WritingMode::VerticalLr),
        123.0,
        "縦書き（lr）のブロック軸は幅"
    );
}

/// 要件 14.2／14.1: 既定だけの行の切り出しは「番号列は空・箱寸は `font_height`」へ落ちる。
///
/// 行箱の高さ（37.5）を `font_height`（12.0）と**別の値**にしてあるので、`default_only` の
/// 分岐を潰す誤り（常に装飾ありとみなす／箱寸を常に [`block_extent`] にする）はここで赤になる。
/// 要件 14.1 の「1 画素も変えない」を行矩形の引き算の丸めに依存させないための分岐そのものを固定する。
#[test]
fn default_only_line_keeps_font_height_and_empty_style_ids() {
    let run = GlyphRunContent {
        glyphs: "あい".chars().map(|c| glyph(c, StyleId::DEFAULT)).collect(),
        size: (20.0, 37.5), // 行箱の高さを font_height と別値にする（丸め非依存の証拠）。
    };

    let line = line_styles(&run, 12.0, WritingMode::HorizontalTb);

    assert!(
        line.default_only,
        "番号 0 だけの行は既定だけの行と判定される（要件 14.2）"
    );
    assert!(
        line.ids.is_empty(),
        "既定だけの行は装飾番号列を持たない（保持庫の鍵が装飾導入前と同じになる）"
    );
    assert_eq!(
        line.extent, 12.0,
        "既定だけの行の箱寸は font_height をそのまま渡す（block_extent(size)=37.5 ではない・要件 14.1）"
    );

    // 対照: 1 グリフでも番号が付けば装飾のある行として切り出され、箱寸は行矩形側になる。
    let decorated = GlyphRunContent {
        glyphs: vec![glyph('あ', StyleId(1)), glyph('い', StyleId::DEFAULT)],
        size: (20.0, 37.5),
    };
    let line = line_styles(&decorated, 12.0, WritingMode::HorizontalTb);
    assert!(!line.default_only, "番号の付いた行は既定だけの行ではない");
    assert_eq!(
        line.ids,
        vec![StyleId(1), StyleId::DEFAULT],
        "装飾のある行はグリフ数ぶんの番号列を記述順のまま持つ"
    );
    assert_eq!(
        line.extent, 37.5,
        "装飾のある行の箱寸は行矩形のブロック軸（block_extent）"
    );
}

/// 番号 1 に `look` を積んだ表を返す（`intern` 経由＝本番と同じ畳み込み規則）。
fn table_with(look: &TextLook, default: &TextLook) -> (StyleTable, StyleId) {
    let mut table = StyleTable::default();
    let id = table.intern(look, default);
    assert_ne!(
        id,
        StyleId::DEFAULT,
        "検査用の見た目は既定と異なる（番号 0 に畳み込まれると檻が空振りする）"
    );
    (table, id)
}

/// canvas の GlyphRun 住人の先頭グリフだけへ番号を与える（1 行の混在を作る）。
fn decorate_first_glyph(canvas: &ContentCanvas, id: StyleId) -> ContentCanvas {
    let residents = canvas
        .residents
        .iter()
        .map(|r| match &r.content {
            ResidentContent::GlyphRun(run) => {
                let mut run = run.clone();
                if let Some(first) = run.glyphs.first_mut() {
                    first.style = id;
                }
                Resident {
                    content: ResidentContent::GlyphRun(run),
                    transform: r.transform,
                    effects: r.effects,
                }
            }
            _ => r.clone(),
        })
        .collect();
    ContentCanvas {
        residents,
        size: canvas.size,
    }
}

/// 赤インク（premultiplied BGRA: 青と緑が 0・赤が 0 より大きい）の画素数を列帯 `x0..x1` で数える。
/// アンチエイリアスの縁も拾えるよう完全一致ではなく成分の符号で判定する。
fn count_red(bytes: &[u8], w: u32, h: u32, x0: u32, x1: u32) -> usize {
    let mut n = 0usize;
    for y in 0..h {
        for x in x0..x1.min(w) {
            let o = ((y * w + x) * 4) as usize;
            let (b, g, r, a) = (bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]);
            if a != 0 && r > 0 && b == 0 && g == 0 {
                n += 1;
            }
        }
    }
    n
}

/// 白インク（3 成分とも 0 より大きい）の画素数を列帯 `x0..x1` で数える。
fn count_white(bytes: &[u8], w: u32, h: u32, x0: u32, x1: u32) -> usize {
    let mut n = 0usize;
    for y in 0..h {
        for x in x0..x1.min(w) {
            let o = ((y * w + x) * 4) as usize;
            let (b, g, r) = (bytes[o], bytes[o + 1], bytes[o + 2]);
            if b > 0 && g > 0 && r > 0 {
                n += 1;
            }
        }
    }
    n
}

/// 較正の本体（要件 3.6）: 1 行の中で番号の付いた**先頭グリフだけ**が装飾の色で描かれ、
/// 番号 0 のグリフは既定の色（黒）のまま残る。区間に切らず行全体へ焼く誤りは、2 文字目の帯に
/// 赤インクが出るので赤になる。
#[test]
fn decorated_run_paints_only_its_own_glyphs() {
    let mut rig = Rig::new();
    let image = (40u32, 20u32);
    let mut surface = rig.attach(image, 1.0);
    let (w, h) = surface.size();
    let mode = WritingMode::HorizontalTb;
    let font = ResolvedFont::resolve(&geo_model(Some(10)));
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);

    // 全角 2 文字（送り 10）＝先頭グリフは x 0..10・2 文字目は x 10..20。
    let (plain_canvas, window) = build(&glyph_items("あい"), &region, mode, 10.0);
    let red = TextLook {
        color: (255, 0, 0),
        ..font.looks.default.clone()
    };
    let (styles, id) = table_with(&red, &font.looks.default);
    let canvas = decorate_first_glyph(&plain_canvas, id);

    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
    exec.render_styled(
        &canvas,
        &window,
        &font,
        mode,
        &contract,
        &mut surface,
        &styles,
    )
    .expect("render_styled 失敗");
    let px = surface.read_back().expect("read_back 失敗");

    assert!(
        count_red(&px, w, h, 0, 10) > 0,
        "番号の付いた区間は装飾の色（赤）で描かれる（要件 3.6）"
    );
    assert_eq!(
        count_red(&px, w, h, 10, 20),
        0,
        "番号 0 の区間へ装飾の色が漏れない（行全体へ焼く誤りはここで赤になる）"
    );
    assert!(
        opaque_count(&px) > 0,
        "空の面と一致して偽の合格にならない（非退化）"
    );
}

/// 要件 14.2: 既定だけの行は範囲指定・色の解除・ブラシ生成の**実回数が 0**で、描画統計と
/// 行レイアウトの生成回数が装飾導入前の字義の値（2 行＝生成 2・描画 2・blit 0・全消去 0）と一致する。
///
/// 実回数は `RANGE_CALLS`／`RESET_CALLS`（発行のたびに 1 増やす素の勘定）で数える——保持庫の
/// 要素数で数えると同じ鍵で作り直しても増えず恒真になる。
#[test]
fn default_only_lines_issue_no_range_calls_and_keep_legacy_stats() {
    RANGE_CALLS.with(|c| c.set(0));
    RESET_CALLS.with(|c| c.set(0));

    let mut rig = Rig::new();
    let image = (80u32, 40u32);
    let mut surface = rig.attach(image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let font = ResolvedFont::resolve(&geo_model(Some(10)));
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);

    // 2 行（"あい" / "うえお"）＝GlyphRun 住人 2。初回フレームは全域ダーティ。
    let mut items = glyph_items("あい");
    items.push(crate::state::TextItem::LineBreak { ratio: 1.0 });
    items.extend(glyph_items("うえお"));
    let (canvas, window) = build(&items, &region, mode, 10.0);

    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
    let changed = exec
        .render_styled(
            &canvas,
            &window,
            &font,
            mode,
            &contract,
            &mut surface,
            &StyleTable::default(),
        )
        .expect("render_styled 失敗");
    assert!(changed, "初回フレームは変化あり（present 要）");

    let DrawStats {
        line_layout_creations,
        draw_text_layout_calls,
        blits,
        full_clears,
    } = exec.stats();
    assert_eq!(
        (
            line_layout_creations,
            draw_text_layout_calls,
            blits,
            full_clears
        ),
        (2, 2, 0, 0),
        "既定だけの 2 行の初回フレームは「生成 2・描画 2・blit 0・全消去 0」（装飾導入前と同じ字義の値）"
    );
    assert_eq!(
        RANGE_CALLS.with(|c| c.get()),
        0,
        "既定だけの行は範囲指定・色の解除・ブラシ生成のいずれも発行しない（要件 14.2）"
    );
    assert_eq!(
        RESET_CALLS.with(|c| c.get()),
        0,
        "既定だけの行は色の解除も発行しない"
    );
}

/// 装飾のある行では範囲指定が**実際に発行される**（上の 0 件の述語が「そもそも数えていない」
/// ために緑になっているのではないことの対照）。
#[test]
fn decorated_lines_actually_issue_range_calls() {
    RANGE_CALLS.with(|c| c.set(0));

    let mut rig = Rig::new();
    let image = (40u32, 20u32);
    let mut surface = rig.attach(image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let font = ResolvedFont::resolve(&geo_model(Some(10)));
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);

    let (plain_canvas, window) = build(&glyph_items("あい"), &region, mode, 10.0);
    let bold_red = TextLook {
        bold: true,
        color: (255, 0, 0),
        ..font.looks.default.clone()
    };
    let (styles, id) = table_with(&bold_red, &font.looks.default);
    let canvas = decorate_first_glyph(&plain_canvas, id);

    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
    exec.render_styled(
        &canvas,
        &window,
        &font,
        mode,
        &contract,
        &mut surface,
        &styles,
    )
    .expect("render_styled 失敗");

    assert!(
        RANGE_CALLS.with(|c| c.get()) >= 3,
        "太さ・色の解除・色の 3 つは少なくとも発行される（0 件の述語の対照）"
    );
}

/// 選択肢の行の hover セグメントを作る（1 行「あい」の先頭グリフだけがセグメント 0）。
fn choice_canvas(
    base: &ContentCanvas,
    highlight: Option<HighlightPaint>,
    hovered: Option<usize>,
) -> ContentCanvas {
    let residents = base
        .residents
        .iter()
        .map(|r| match &r.content {
            ResidentContent::GlyphRun(run) => Resident {
                content: ResidentContent::Choice(ChoiceLineContent {
                    run: run.clone(),
                    segments: vec![ChoiceRowSegment {
                        ordinal: 0,
                        inline_range: (0.0, 10.0),
                    }],
                    hovered,
                    highlight,
                    band_extent: 12.0,
                    band_offset: 0.0,
                }),
                transform: r.transform,
                effects: r.effects,
            },
            _ => r.clone(),
        })
        .collect();
    ContentCanvas {
        residents,
        size: base.size,
    }
}

/// 要件 3.5: 選択肢の文字にも装飾が効くが、hover の文字色は装飾の色より**後に**焼かれて勝つ。
/// 順序を逆転させると hover セグメントに装飾の赤が残るので赤になる。
/// 同じフレームの hover していないグリフには装飾の赤が残る（装飾の色の経路を実際に踏んでいる対照）。
#[test]
fn hover_color_wins_over_decoration_color() {
    let mut rig = Rig::new();
    let image = (40u32, 20u32);
    let mut surface = rig.attach(image, 1.0);
    let (w, h) = surface.size();
    let mode = WritingMode::HorizontalTb;
    let font = ResolvedFont::resolve(&geo_model(Some(10)));
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);

    let (plain_canvas, window) = build(&glyph_items("あい"), &region, mode, 10.0);
    let red = TextLook {
        color: (255, 0, 0),
        ..font.looks.default.clone()
    };
    let (styles, id) = table_with(&red, &font.looks.default);
    // 2 グリフとも赤の装飾。hover は先頭グリフだけ（塗り＋白文字）。
    let mut decorated = plain_canvas.clone();
    for resident in &mut decorated.residents {
        if let ResidentContent::GlyphRun(run) = &mut resident.content {
            for g in &mut run.glyphs {
                g.style = id;
            }
        }
    }
    let canvas = choice_canvas(
        &decorated,
        Some(HighlightPaint {
            fill: (105, 25, 25),
            text: (255, 255, 255),
        }),
        Some(0),
    );

    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
    exec.render_styled(
        &canvas,
        &window,
        &font,
        mode,
        &contract,
        &mut surface,
        &styles,
    )
    .expect("render_styled 失敗");
    let px = surface.read_back().expect("read_back 失敗");

    assert!(
        count_white(&px, w, h, 0, 10) > 0,
        "hover セグメントの文字は重ね表示の色（白）で描かれる（装飾の色より後に焼く）"
    );
    assert_eq!(
        count_red(&px, w, h, 0, 10),
        0,
        "hover セグメントに装飾の色が残らない（順序を逆転させるとここが赤になる）"
    );
    assert!(
        count_red(&px, w, h, 10, 20) > 0,
        "hover していないグリフには装飾の色が残る（装飾の色の経路を実際に踏んでいる対照）"
    );
}

/// 選択肢の行は呼び手が既に全範囲を解除しているので、装飾の色の焼き込みは解除を**発行しない**。
/// 非選択肢の行では 1 回だけ発行する（二重化を避けつつ、必要な側では確かに解除している）。
#[test]
fn color_reset_is_issued_once_and_not_duplicated_on_choice_lines() {
    let mut rig = Rig::new();
    let image = (40u32, 20u32);
    let mode = WritingMode::HorizontalTb;
    let font = ResolvedFont::resolve(&geo_model(Some(10)));
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);

    let (plain_canvas, window) = build(&glyph_items("あい"), &region, mode, 10.0);
    let red = TextLook {
        color: (255, 0, 0),
        ..font.looks.default.clone()
    };
    let (styles, id) = table_with(&red, &font.looks.default);
    let decorated = decorate_first_glyph(&plain_canvas, id);

    // ① 通常の行——装飾の色を焼く前に全範囲の解除を 1 回だけ発行する。
    RESET_CALLS.with(|c| c.set(0));
    let mut surface = rig.attach(image, 1.0);
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
    exec.render_styled(
        &decorated,
        &window,
        &font,
        mode,
        &contract,
        &mut surface,
        &styles,
    )
    .expect("render_styled(GlyphRun) 失敗");
    assert_eq!(
        RESET_CALLS.with(|c| c.get()),
        1,
        "通常の行は装飾の色の手前で全範囲を 1 回解除する"
    );

    // ② 選択肢の行——呼び手が既に解除済みなので装飾の側は解除を発行しない。
    RESET_CALLS.with(|c| c.set(0));
    let choice = choice_canvas(&decorated, None, None);
    let mut surface2 = rig.attach(image, 1.0);
    let mut exec2 = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
    exec2
        .render_styled(
            &choice,
            &window,
            &font,
            mode,
            &contract,
            &mut surface2,
            &styles,
        )
        .expect("render_styled(Choice) 失敗");
    assert_eq!(
        RESET_CALLS.with(|c| c.get()),
        0,
        "選択肢の行は既に解除済み——解除を二重に発行しない"
    );
}
