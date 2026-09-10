//! # 行送りの後も収まる行は全部見える（症状 F の追試・R17.1〜17.4）
//!
//! 実機の相方側バルーン（`emo2-kakukaku` の kero・描画範囲の高さ 93 image px・`font.height,28`
//! ＝行送り 30）で観測された症状——「2 行に折り返した台詞 → 改行 150%（段落間隔）→ 次の台詞を
//! 1 字ずつ」と進めると、あふれて 1 行送った後に**先頭可視行が描かれず**最新の 1 行だけが
//! 残る——と**同じ形の入力列（冒頭の保留改行を含まない）**を決定論で駆動する。
//! 2026-09-10 の根因確定（`visible_window` の原点・タスク 6.13）で、症状 F の引き金は台詞冒頭の
//! `\n[150]` だったと判明した——本檻の台本はそれを含まないので緑は当然で、以後は描画対象の
//! 非回帰の檻として残す。
//!
//! **この檻は 2026-09-07 時点の HEAD で緑である**——すなわち症状 F はこの層（`ViewboxExecutor`
//! の差分描画）では再現しない。緑であること自体が所見であり、症状の引き当てを別の層へ送る
//! 根拠になる。檻が症状を捕まえられることは較正で確かめてある——`derive_dirty_with_overhangs`
//! の描画対象から先頭可視行を 1 つ落とす欠陥を注入すると、byte 等価の主張が赤になる。
//!
//! 期待（正典・R17.1）: 送りの前後で画素が変わってよいのは送られた分だけであり、先頭可視行
//! （送り後の 1 行目）と最新行の両方にインクが在る。判定の正本は全域再描画のオラクル
//! （[`DrawExecutor`]）との **byte 等価**（R17.3）で、これに加えて「先頭可視行の帯にインクが
//! 在る」「最新行の帯にインクが在る」を直接読み戻して主張する（等価だけだと両方式が揃って
//! 描き落とす筋を排除できない）。
//!
//! 送りは 1 回では終わらない——台詞が続けば 1 行増えるたびに送りが起きる。実機の所見
//! 「常に一番下の行しか表示されない」は送りを重ねるほど上が失われる形ゆえ、檻も**送りを
//! 何度も跨ぐ**台本で駆動する。
//!
//! **本文は全角だけで組む。** 純粋レイアウトの `FixedMetrics` は半角を `font_height / 2`（14）
//! で送るが、実フォント（Yu Gothic UI 28）の半角の送りはそれより広い。行矩形が実インクより
//! 狭くなるので、ダーティ矩形が行内軸で 1 画素インクを切り、オラクルとの byte 等価が破れる
//! ——**檻の作り方の都合**であって製品の欠陥ではない（本番は同じ `DWriteMetrics` が配置と
//! 描画の両方へ同じ送り幅を配る）。全角では両者が一致するので全角で組む。

use super::ViewboxExecutor;
use super::test_support::{Rig, glyph_items, live_diff_model_font, opaque_count};
use crate::canvas::ContentCanvas;
use crate::draw::{DrawExecutor, ResolvedFont};
use crate::layout::{FixedMetrics, LayoutEngine, VisibleWindow, WrapPlan};
use crate::region::{ScaleContract, TextRegion};
use crate::state::TextItem;
use crate::surface::TextSurface;
use crate::writing::WritingMode;

/// 相方側バルーン相当の描画範囲（image px）——高さ 93 は実機 `balloonk0s.txt` の validrect 丈、
/// 幅 216 は同じく行内軸の丈。`font.height,28` ＋ 行間 2 ＝ 行送り 30。
const IMAGE: (u32, u32) = (216, 93);
/// 実機と同じ字の丈（image px）。
const FONT_HEIGHT: f32 = 28.0;

/// 実機の台本（`boot.pasta:78-90`）と同じ形の入力列を組む——2 行に折り返す台詞 → `\n[150]`
/// （＝`LineBreak { ratio: 1.5 }`・段落間隔 45＝1.5 × 行送り 30）→ 次の台詞（さらに折り返して
/// 送りを重ねる長さ）→ もう 1 度の段落間隔 → 3 本目の台詞。
fn boot_items() -> (Vec<TextItem>, usize) {
    let mut items = Vec::new();
    let mut glyphs = 0;
    for (i, text) in [
        "僕はエモクール系娘。",
        "イイジャンええとそれでね。",
        "そこ自分でいう。",
    ]
    .iter()
    .enumerate()
    {
        if i > 0 {
            items.push(TextItem::LineBreak { ratio: 1.5 });
        }
        items.extend(glyph_items(text));
        glyphs += text.chars().count();
    }
    (items, glyphs)
}

/// 行送り軸の帯 `[y0, y1)` に非透明ピクセルがいくつ在るか（BGRA 密配列・物理 px）。
fn ink_in_band(bytes: &[u8], w: u32, y0: u32, y1: u32) -> usize {
    let mut count = 0;
    for y in y0..y1 {
        for x in 0..w {
            if bytes[((y * w + x) * 4 + 3) as usize] != 0 {
                count += 1;
            }
        }
    }
    count
}

/// オラクル（全域再描画）と viewbox（差分描画）へ同一入力を流した 1 フレームの読み戻し。
struct Frames {
    /// オラクル面の読み戻し（BGRA 密配列）。
    oracle: Vec<u8>,
    /// viewbox 面の読み戻し（BGRA 密配列）。
    viewbox: Vec<u8>,
    /// このフレームの可視窓（先頭可視行・ブロック軸オフセット）。
    window: VisibleWindow,
    /// 最新行の描画面上の上端（物理 px・`canvas-local 上端 + block_offset`）。
    latest_top: f32,
}

/// 二面リグ: 同一 core／compositor／font／region で、オラクルと viewbox を同一入力で回す。
struct ScrollRig {
    oracle_surface: TextSurface,
    viewbox_surface: TextSurface,
    oracle: DrawExecutor,
    viewbox: ViewboxExecutor,
    region: TextRegion,
    font: ResolvedFont,
    contract: ScaleContract,
    mode: WritingMode,
    /// World／Compositor／Core／DispatcherQueue の寿命を束ねる（供給面より後に drop）。
    #[allow(dead_code)]
    rig: Rig,
}

impl ScrollRig {
    /// 実機と同じフォント（Yu Gothic UI 28px＝下端はみ出しが行間 2px を超える）で二面を装着する。
    /// レイアウトは [`FixedMetrics`]（決定論）ゆえ折返し位置はフォント非依存で、フォント名は
    /// ラスタライズ（インクの形・はみ出し量）にだけ効く。
    fn new() -> ScrollRig {
        let mut rig = Rig::new();
        let oracle_surface = rig.attach(IMAGE, 1.0);
        let viewbox_surface = rig.attach(IMAGE, 1.0);
        let model = live_diff_model_font(Some("Yu Gothic UI"), Some(FONT_HEIGHT as u32));
        let mode = WritingMode::HorizontalTb;
        let font = ResolvedFont::resolve(&model);
        let region = TextRegion::resolve(&model, IMAGE, mode);
        let oracle = DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗");
        let viewbox = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
        ScrollRig {
            oracle_surface,
            viewbox_surface,
            oracle,
            viewbox,
            region,
            font,
            contract: ScaleContract::new(1.0, None),
            mode,
            rig,
        }
    }

    /// `visible` 字ぶんを見せた 1 フレームを両方式へ流し、読み戻しと可視窓を返す。
    fn frame(&mut self, label: &str, items: &[TextItem], visible: usize) -> Frames {
        let lines = LayoutEngine::layout(
            items,
            visible,
            &self.region,
            self.mode,
            FONT_HEIGHT,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        let window = LayoutEngine::visible_window(&lines, &self.region, self.mode);
        let canvas = ContentCanvas::from_layout(&lines, &self.region, self.mode);
        // 最新行の描画面上の上端（validrect-local 上端＋可視窓オフセット・k=1.0）。
        let latest_top = lines
            .last()
            .map(|line| line.rect.top - self.region.top() + window.block_offset)
            .unwrap_or(0.0);

        self.oracle
            .render(
                &canvas,
                &window,
                &self.font,
                self.mode,
                &self.contract,
                &mut self.oracle_surface,
            )
            .unwrap_or_else(|e| panic!("{label}: オラクル render 失敗: {e:?}"));
        self.viewbox
            .render(
                &canvas,
                &window,
                &self.font,
                self.mode,
                &self.contract,
                &mut self.viewbox_surface,
            )
            .unwrap_or_else(|e| panic!("{label}: viewbox render 失敗: {e:?}"));

        let oracle = self
            .oracle_surface
            .read_back()
            .unwrap_or_else(|e| panic!("{label}: オラクル read_back 失敗: {e:?}"));
        let viewbox = self
            .viewbox_surface
            .read_back()
            .unwrap_or_else(|e| panic!("{label}: viewbox read_back 失敗: {e:?}"));
        Frames {
            oracle,
            viewbox,
            window,
            latest_top,
        }
    }
}

/// 症状 F の再現檻（R17.1/17.2/17.3）——2 行に折り返す台詞 → `\n[150]` → 次の台詞を 1 字ずつ、を
/// 送りを何度も跨ぐまで進め、送りが起きたフレームとその後のフレームの読み戻しに
/// **先頭可視行のインク**と**最新行のインク**の両方が在り、全域再描画のオラクルと byte 等価で
/// あることを主張する。
///
/// 期待配置（image px・validrect-local）: 行 0 上端 0・行 1 上端 30・行 2 上端 75（＝30 ＋ 段落
/// 間隔 45）。行 2 の下端 103 > 93 であふれ → 可視窓は先頭可視行 1・オフセット −30 → 行 1 が 0・
/// 行 2 が 45 へ。先頭可視行は送りの式より**常に描画面の上端 0** に来るので、帯 `[0,28)` を
/// 先頭可視行の帯として読む。
#[test]
fn scroll_frames_keep_first_visible_line_ink() {
    let mut rig = ScrollRig::new();
    let (items, total_glyphs) = boot_items();
    let (w, _h) = rig.oracle_surface.size();
    let mut scrolled_frames = 0usize;

    for visible in 1..=total_glyphs {
        let label = format!("{visible} 字");
        let frames = rig.frame(&label, &items, visible);

        assert!(
            opaque_count(&frames.oracle) > 0,
            "{label}: オラクル面にインクが在る（空面同士の一致を排除）"
        );

        if frames.window.first_visible_line > 0 {
            scrolled_frames += 1;
            // 先頭可視行は送りの式（`block_offset = -(先頭可視行の上端 - 先頭行の上端)`）より
            // 常に描画面の上端 0 に来る。最新行は `latest_top`。
            let latest_top = frames.latest_top.round().max(0.0) as u32;
            let latest_bottom = (latest_top + FONT_HEIGHT as u32).min(IMAGE.1);
            let head_oracle = ink_in_band(&frames.oracle, w, 0, FONT_HEIGHT as u32);
            let latest_oracle = ink_in_band(&frames.oracle, w, latest_top, latest_bottom);
            assert!(
                head_oracle > 0,
                "{label}: オラクル面の先頭可視行の帯 [0,28) にインクが在る（檻の非退化）"
            );
            assert!(
                latest_oracle > 0,
                "{label}: オラクル面の最新行の帯 [{latest_top},{latest_bottom}) にインクが在る（檻の非退化）"
            );

            let head_viewbox = ink_in_band(&frames.viewbox, w, 0, FONT_HEIGHT as u32);
            let latest_viewbox = ink_in_band(&frames.viewbox, w, latest_top, latest_bottom);
            assert!(
                head_viewbox > 0,
                "{label}: 行送りの後も**先頭可視行**（送り後の 1 行目）のインクが在る\
                 ——帯 [0,28) の非透明画素が 0（症状 F: 最新の 1 行しか見えない）\
                 ／最新行の帯 [{latest_top},{latest_bottom}) は {latest_viewbox} 画素\
                 ／先頭可視行 {}",
                frames.window.first_visible_line
            );
            assert!(
                latest_viewbox > 0,
                "{label}: 最新行の帯 [{latest_top},{latest_bottom}) にインクが在る"
            );
            assert_eq!(
                frames.oracle, frames.viewbox,
                "{label}: 送りの前後で オラクル（全域再描画）と viewbox（差分描画）が byte 等価\
                 （先頭可視行の帯: オラクル {head_oracle} 画素 vs viewbox {head_viewbox} 画素）"
            );
        } else {
            assert_eq!(
                frames.oracle, frames.viewbox,
                "{label}: 送り前も オラクル（全域再描画）と viewbox（差分描画）が byte 等価"
            );
        }
    }

    assert!(
        scrolled_frames >= 2,
        "台本は送りを 2 回以上跨ぐ（実際 {scrolled_frames} フレーム）——1 回だけでは\
         「送りを重ねるほど上が失われる」形を突けない"
    );
}

/// 送りを跨ぐ入力の掃き出し（R17.1/17.3）——段落間隔の比率・1 コマで進む字数・台詞の長さ
/// （＝折返し位置）を振って、**どの組み合わせでも**差分描画が全域再描画のオラクルと byte 等価で
/// あることを主張する。1 本の台本だけでは「たまたま通る経路」を掃けないため、送りが何度も
/// 起きる組み合わせを機械的に回す。
#[test]
fn scroll_frame_sweep_matches_full_redraw_oracle() {
    // 字数を振って折返し位置をずらす（全角のみ——冒頭の但し書き）。
    let bodies = [
        "僕はエモクール系娘。",
        "僕はエモ。ク一ル系の可愛い娘。",
        "アイウエオカキクケコサシスセソタチツテト",
        "ソレデネ、コウイウコトナノ。ウン。",
    ];
    let ratios = [1.0f32, 1.5, 2.0];
    let strides = [1usize, 2, 5];
    let mut scrolled_total = 0usize;

    for ratio in ratios {
        for stride in strides {
            let mut items = Vec::new();
            let mut total = 0usize;
            for (i, body) in bodies.iter().enumerate() {
                if i > 0 {
                    items.push(TextItem::LineBreak { ratio });
                }
                items.extend(glyph_items(body));
                total += body.chars().count();
            }
            let mut rig = ScrollRig::new();
            let (w, _h) = rig.oracle_surface.size();
            let mut visible = 0usize;
            while visible < total {
                visible = (visible + stride).min(total);
                let label = format!("ratio={ratio} stride={stride} visible={visible}");
                let frames = rig.frame(&label, &items, visible);
                assert!(
                    opaque_count(&frames.oracle) > 0,
                    "{label}: オラクル面にインクが在る（空面同士の一致を排除）"
                );
                if frames.window.first_visible_line > 0 {
                    scrolled_total += 1;
                    let head = ink_in_band(&frames.viewbox, w, 0, FONT_HEIGHT as u32);
                    assert!(
                        head > 0,
                        "{label}: 行送りの後も先頭可視行の帯 [0,28) にインクが在る\
                         （症状 F: 最新の 1 行しか見えない）"
                    );
                }
                assert_eq!(
                    frames.oracle, frames.viewbox,
                    "{label}: オラクル（全域再描画）と viewbox（差分描画）が byte 等価"
                );
            }
        }
    }
    assert!(
        scrolled_total >= 30,
        "掃き出しは送りの起きたフレームを 30 以上通る（実際 {scrolled_total}）"
    );
}
