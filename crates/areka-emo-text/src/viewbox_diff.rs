//! viewbox の子: 描き直す範囲の導出（行指紋・逆向きの縮みの判定・露出帯と変化行からのダーティ矩形）。
//! 足す予定の spec: text-reveal-fade（`ScrollPlanner` の差分）・balloon-markers。

use super::{
    CommittedLine, DIRTY_GUARD_IMG_PX, DirtyRect, LineOverhang, PhysicalRect, ScrollPlanner,
};
use crate::canvas::{ContentCanvas, Resident, ResidentContent};
use crate::layout::VisibleWindow;
use crate::region::ScaleContract;
use crate::writing::WritingMode;

impl ScrollPlanner {
    /// canvas の全住人から行指紋列を作る（純粋・決定論・DD4）。
    ///
    /// 住人 index と 1:1（layout 行 index と一致）。M1 の実装住人はグリフのみだが、画像/
    /// サーフェスのシーム住人も一様に扱える（空 text・零寸の指紋）。位置/寸は canvas-local
    /// ゆえスクロール非依存——`derive_dirty` の `prev_lines` として次フレームへ渡す。
    pub(crate) fn committed_lines(canvas: &ContentCanvas, mode: WritingMode) -> Vec<CommittedLine> {
        canvas
            .residents
            .iter()
            .map(|resident| line_fingerprint(resident, mode))
            .collect()
    }

    /// 後方縮退（面内 blit＋差分描画で保持できない content 減少）の判定（純粋・決定論・DD-9）。
    ///
    /// 次のいずれかで true——(a) 行数減少（`new_lines.len() < prev_lines.len()`＝行がスクロール
    /// アウト/消滅）、(b) 共通 prefix の同一 index 行で **block 軸位置が動いた**（旧位置のインクを
    /// 新変化行矩形が覆えない）、(c) 同一 index 行で **extent が縮んだ**（旧インクの外側＝退避分を
    /// 新矩形が覆えない）。前方 typewriter（prefix 伸長で extent 増加・block 不動）では false ゆえ
    /// 増分描画のホットパスを維持する。行内開始位置は layout の不変則（全行同一の行内開始）ゆえ、
    /// block 位置＋extent の指紋だけで被覆可否は健全に判定できる（text 内容は覆えれば無関係）。
    pub(super) fn is_backward_shrink(
        prev_lines: &[CommittedLine],
        new_lines: &[CommittedLine],
    ) -> bool {
        if new_lines.len() < prev_lines.len() {
            return true;
        }
        prev_lines.iter().zip(new_lines).any(|(prev, new)| {
            // (b) block 軸位置が動いた行は、旧位置のインクを新変化行矩形が覆えない。
            if prev.block_pos_bits != new.block_pos_bits {
                return true;
            }
            // (c) extent が縮んだ行は、旧インクの退避分（縮小の外側）を新矩形が覆えない。
            let (pw, ph) = (
                f32::from_bits(prev.extent_bits.0),
                f32::from_bits(prev.extent_bits.1),
            );
            let (nw, nh) = (
                f32::from_bits(new.extent_bits.0),
                f32::from_bits(new.extent_bits.1),
            );
            nw < pw || nh < ph
        })
    }

    /// ダーティ矩形と描画対象行を導出する（純粋・状態不変・DD4）。
    ///
    /// dirty ＝（a）露出帯（`blit` の逆側に生じる未保持領域・blit≠0 のとき 1 枚）∪（b）変化行
    /// （`prev_lines` と新 canvas の指紋差分行の物理矩形——**新スクロール位置**）∪（c）スクロール
    /// で可視窓の外へ出た行の**残滓**（下記）。各矩形は物理 px 整数格子へ拡張（min を floor・max を
    /// ceil）し、ガード余白 `ceil(DIRTY_GUARD_IMG_PX × k)` を全辺へ加えて面寸 `surface_size` へ
    /// クランプ（退化矩形は除外）。
    ///
    /// `prev_lines` が空（初回・Clear 後・format 再構築）のときは**全域ダーティ**（面全域 1 枚・
    /// 描画対象＝可視窓の GlyphRun 住人）を返す。各 [`DirtyRect`] はその矩形とブロック軸で交差
    /// する GlyphRun 住人 index だけを持ち（クリップにより描画結果は当該矩形内へ限定される）、
    /// 返り値 `draw_lines` はその**和集合**（昇順・重複なし）。ただし**可視窓より前の行
    /// （`first_visible_line` 未満）は含めない**——全域再描画のオラクル（`DrawExecutor`）が
    /// `skip(first_visible_line)` で描かないため、含めると
    /// スクロールアウトした行の下端インクが画面上端へ描き込まれ、両者が食い違う。
    ///
    /// 状態遷移（plan/commit・`FramePlan` の enum 化）は後続タスクの領分——本メソッドは
    /// 「変化なし＝空 dirty・空 draw_lines」「全域＝面全域」の導出結果を返すに留める。
    ///
    /// 実測はみ出し無し（em ボックス丈）の従来経路——測定値を持たない pure 層 unit 向けの薄い
    /// ラッパで、[`Self::derive_dirty_with_overhangs`] に空スライスを渡す。
    pub(crate) fn derive_dirty(
        canvas: &ContentCanvas,
        window: &VisibleWindow,
        mode: WritingMode,
        contract: &ScaleContract,
        blit: (i32, i32),
        surface_size: (u32, u32),
        prev_lines: &[CommittedLine],
    ) -> (Vec<DirtyRect>, Vec<usize>) {
        Self::derive_dirty_with_overhangs(
            canvas,
            window,
            mode,
            contract,
            blit,
            surface_size,
            prev_lines,
            &[],
        )
    }

    /// 実測インクはみ出し（[`LineOverhang`]・住人 index と 1:1）付きでダーティ矩形と描画対象行を
    /// 導出する（純粋・状態不変・DD4／D2）。変化行の em ボックスを `overhangs` の実測分だけ外側へ
    /// 広げてはみ出しインクを含める（`overhangs` が index を欠く／空なら既定 0＝em ボックス丈）。
    ///
    /// **スクロールアウトした行の残滓**（(c)）: 行送りは「字の丈 ＋ 行間 2px」ゆえ、行と行の
    /// 隙間は 2px しかない。下端のはみ出しインクがそれより大きいフォント（Yu Gothic UI 28px は
    /// 実測 3px）では、可視窓の外へ出た行の下端インクが blit 後も面内へ 1px 残る。可視窓の外の行は
    /// 描かれないので、この残滓は消す側（ダーティ）で拾う——`blit ≠ 0` のフレームに限り、
    /// `first_visible_line` より前の行の矩形もダーティへ入れる（面内へ届かない行は退化して落ちる）。
    pub(crate) fn derive_dirty_with_overhangs(
        canvas: &ContentCanvas,
        window: &VisibleWindow,
        mode: WritingMode,
        contract: &ScaleContract,
        blit: (i32, i32),
        surface_size: (u32, u32),
        prev_lines: &[CommittedLine],
        overhangs: &[LineOverhang],
    ) -> (Vec<DirtyRect>, Vec<usize>) {
        // 住人 index の実測はみ出し（無ければ既定 0＝em ボックス丈・テスト等の非測定経路）。
        let overhang_of = |i: usize| overhangs.get(i).copied().unwrap_or_default();
        // ── 全域ダーティ（初回・Clear 後・format 再構築）: 面全域 1 枚・全 GlyphRun 住人 ──
        // 矩形が 1 枚ゆえ「矩形ごとの交差行」＝可視窓の全住人＝和集合（和と積が一致する唯一の形）。
        if prev_lines.is_empty() {
            let (w, h) = surface_size;
            let full = PhysicalRect { x: 0, y: 0, w, h };
            let draw_lines: Vec<usize> = glyph_run_indices(canvas)
                .filter(|&i| i >= window.first_visible_line)
                .collect();
            let dirty = if full.is_empty() {
                Vec::new()
            } else {
                vec![DirtyRect {
                    rect: full,
                    lines: draw_lines.clone(),
                }]
            };
            return (dirty, draw_lines);
        }

        let mut rects: Vec<PhysicalRect> = Vec::new();

        // (a) 露出帯: blit の逆側の未保持領域（1 枚）。
        if let Some(band) = exposure_band(blit, surface_size) {
            if let Some(rect) = expand_guard_clamp(
                band.x as f32,
                band.y as f32,
                (band.x + band.w) as f32,
                (band.y + band.h) as f32,
                contract,
                surface_size,
            ) {
                rects.push(rect);
            }
        }

        // (b) 変化行: 指紋差分行の物理矩形（新スクロール位置）。
        // (c) スクロールアウトした行の残滓: 可視窓の外の行は描かれないが、行間が 2px しかない
        //     ため下端のはみ出しインク（実測 3px 級）が blit 後の面内へ残る。blit したフレームで
        //     その矩形もダーティへ入れ、消してから可視窓の先頭行を描き直す。行送りは行単位ゆえ
        //     可視窓の外の行の em ボックスは面の外にあり、はみ出しが 0 の行は残滓を作らない
        //     （＝可視窓だけが動いたフレームのダーティは露出帯 1 枚のまま）。
        let block_offset = window.block_offset;
        for (i, resident) in canvas.residents.iter().enumerate() {
            let fingerprint = line_fingerprint(resident, mode);
            let changed = match prev_lines.get(i) {
                Some(prev) => *prev != fingerprint,
                None => true, // 新規行（prev.len() を超える index）。
            };
            let scrolled_out_residue = blit != (0, 0)
                && i < window.first_visible_line
                && block_axis_overhang(overhang_of(i), mode) > 0.0;
            if changed || scrolled_out_residue {
                if let Some(rect) = resident_rect(
                    resident,
                    block_offset,
                    mode,
                    contract,
                    surface_size,
                    overhang_of(i),
                ) {
                    rects.push(rect);
                }
            }
        }

        // 描画対象住人の物理矩形を 1 度だけ求める（矩形ごとの割当と和集合の共通入力）。可視窓より
        // 前の行はオラクル（DrawExecutor）が描かないので、こちらも描かない（上の doc を参照）。
        let visible_rects: Vec<(usize, PhysicalRect)> = glyph_run_indices(canvas)
            .filter(|&i| i >= window.first_visible_line)
            .filter_map(|i| {
                resident_rect(
                    &canvas.residents[i],
                    block_offset,
                    mode,
                    contract,
                    surface_size,
                    overhang_of(i),
                )
                .map(|rect| (i, rect))
            })
            .collect();

        // 矩形ごとの交差行（昇順・住人 index の順に見るので並びは昇順）と、その和集合。
        let dirty: Vec<DirtyRect> = rects
            .into_iter()
            .map(|rect| {
                let lines = visible_rects
                    .iter()
                    .filter(|(_, r)| rect.intersects_block_axis(r, mode))
                    .map(|(i, _)| *i)
                    .collect();
                DirtyRect { rect, lines }
            })
            .collect();
        let draw_lines: Vec<usize> = visible_rects
            .iter()
            .filter(|(_, r)| dirty.iter().any(|d| d.rect.intersects_block_axis(r, mode)))
            .map(|(i, _)| *i)
            .collect();

        (dirty, draw_lines)
    }
}

/// グリフ描画対象住人の index を昇順で返す（描画対象・全域ダーティの draw_lines 共通口）。
/// Choice 住人は内包 `run` を GlyphRun と同格に素描画するため同じく対象に含める（R9.5）。
fn glyph_run_indices(canvas: &ContentCanvas) -> impl Iterator<Item = usize> + '_ {
    canvas
        .residents
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            matches!(
                r.content,
                ResidentContent::GlyphRun(_) | ResidentContent::Choice(_)
            )
        })
        .map(|(i, _)| i)
}

/// 住人 1 行の行指紋（内容文字列・ブロック軸位置・行寸・hover 印・装飾番号列）を作る
/// （canvas-local・DD4）。
pub(super) fn line_fingerprint(resident: &Resident, mode: WritingMode) -> CommittedLine {
    let (text, styles, extent) = match &resident.content {
        ResidentContent::GlyphRun(run) => (
            run.glyphs.iter().map(|g| &*g.text).collect::<String>(),
            run.glyphs.iter().map(|g| g.style.0).collect::<Vec<u32>>(),
            run.size,
        ),
        // Choice 住人は内包 run から GlyphRun と同一の指紋を作る（非 hover の素描画が
        // GlyphRun と同一ゆえ指紋も同一・R9.5。hover セグメントの指紋反映は task 6.1）。
        ResidentContent::Choice(choice) => (
            choice
                .run
                .glyphs
                .iter()
                .map(|g| &*g.text)
                .collect::<String>(),
            choice
                .run
                .glyphs
                .iter()
                .map(|g| g.style.0)
                .collect::<Vec<u32>>(),
            choice.run.size,
        ),
        // 画像/サーフェスのシーム住人は空 text・空番号列・零寸（M1 は from_layout で生成されない）。
        ResidentContent::Image(_) | ResidentContent::Surface(_) => {
            (String::new(), Vec::new(), (0.0, 0.0))
        }
    };
    let (dx, dy) = resident.transform.offset();
    // ブロック軸位置: 横書き＝dy・縦書き＝dx（canvas-local・スクロール非依存）。
    let block_pos = match mode {
        WritingMode::HorizontalTb => dy,
        WritingMode::VerticalRl | WritingMode::VerticalLr => dx,
    };
    // hover 印: Choice 行のみ非 0（非 hover=0・hover ordinal o=o+1 で 0 と衝突させない）。
    // 非 Choice 住人は常に 0。hover の付与/切替/解除で当該 Choice 行の指紋だけが変わり、
    // 既存 line_fingerprint/derive_dirty のアルゴリズムを無改変のまま当該行のみをダーティ化する
    // （R4.4）。
    let choice_marker = match &resident.content {
        ResidentContent::Choice(choice) => choice.hovered.map_or(0, |o| o as u32 + 1),
        ResidentContent::GlyphRun(_) | ResidentContent::Image(_) | ResidentContent::Surface(_) => 0,
    };
    CommittedLine {
        text,
        block_pos_bits: block_pos.to_bits(),
        extent_bits: (extent.0.to_bits(), extent.1.to_bits()),
        choice_marker,
        styles,
    }
}

/// 住人の描画面矩形（新スクロール位置・物理 px 整数・ガード＋クランプ済み）を返す。
///
/// canvas-local 位置 `transform.offset()` にブロック軸の `block_offset` を加算し × k した float
/// 矩形を [`expand_guard_clamp`] へ通す。GlyphRun／Choice 住人は内包 `run` から同一のインク矩形を
/// 導く（R9.5）。非グリフ住人（シーム）は描画実体を持たないため `None`（描画対象にもダーティ
/// にも寄与しない——COM 層 draw の warn!＋skip と整合）。
fn resident_rect(
    resident: &Resident,
    block_offset: f32,
    mode: WritingMode,
    contract: &ScaleContract,
    surface_size: (u32, u32),
    overhang: LineOverhang,
) -> Option<PhysicalRect> {
    let run = match &resident.content {
        ResidentContent::GlyphRun(run) => run,
        // Choice 住人は内包 run から GlyphRun と同一のインク矩形を導く（R9.5）。
        ResidentContent::Choice(choice) => &choice.run,
        // 非グリフ住人（シーム）は描画実体を持たない＝ダーティにも描画対象にも寄与しない。
        ResidentContent::Image(_) | ResidentContent::Surface(_) => return None,
    };
    let (dx, dy) = resident.transform.offset();
    let (w, h) = run.size;
    let k = contract.scale;
    // em ボックス（image px・validrect-local + block_offset）をブロック軸の**実測はみ出し**
    // （[`LineOverhang`]・`GetOverhangMetrics` 由来）だけ外側へ広げ、はみ出しインクをダーティに
    // 含める（byte 等価の前提・D2）。行内軸は em 寸（run.size）のまま——行内はみ出し（イタリック
    // 右張り出し等）は tight box が要るため後続（現状 overhang.left/right は縦書きのブロック軸用）。
    // ブロック軸: horizontal＝Y（上へ top・下へ bottom）／vertical＝X（左へ left・右へ right）。
    let (ix0, iy0, ix1, iy1) = match mode {
        WritingMode::HorizontalTb => {
            let (x0, y0) = (dx, dy + block_offset);
            (x0, y0 - overhang.top, x0 + w, y0 + h + overhang.bottom)
        }
        WritingMode::VerticalRl | WritingMode::VerticalLr => {
            let (x0, y0) = (dx + block_offset, dy);
            (x0 - overhang.left, y0, x0 + w + overhang.right, y0 + h)
        }
    };
    expand_guard_clamp(ix0 * k, iy0 * k, ix1 * k, iy1 * k, contract, surface_size)
}

/// ブロック軸のインクはみ出し量（両端の大きい方・image px）を返す（純粋）。
///
/// 横書きはブロック軸が Y ゆえ `top`／`bottom`、縦書きは X ゆえ `left`／`right` を見る
/// （行内軸のはみ出しは [`LineOverhang`] の定義どおり 0 に丸められている）。0 なら
/// その行のインクは em ボックスの外へ出ない＝スクロールアウトしても残滓を作らない。
fn block_axis_overhang(overhang: LineOverhang, mode: WritingMode) -> f32 {
    match mode {
        WritingMode::HorizontalTb => overhang.top.max(overhang.bottom),
        WritingMode::VerticalRl | WritingMode::VerticalLr => overhang.left.max(overhang.right),
    }
}

/// スクロール blit の逆側に生じる露出帯（未保持領域・物理 px 整数・ガード前）を返す。
///
/// 写像正準表（DD1）: 横書き（blit=y）＝内容が上（by<0）で下端露出・内容が下（by>0）で上端露出。
/// 縦書き（blit=x）＝vertical_lr の内容が左（bx<0）で右端露出・vertical_rl の内容が右（bx>0）で
/// 左端露出。blit=0（露出なし）は `None`。帯幅は面寸でクランプ（`|blit| ≥ 面寸`は全域）。
fn exposure_band(blit: (i32, i32), surface_size: (u32, u32)) -> Option<PhysicalRect> {
    let (w, h) = surface_size;
    let (bx, by) = blit;
    if by != 0 {
        let mag = by.unsigned_abs().min(h);
        if mag == 0 {
            return None;
        }
        return Some(if by < 0 {
            PhysicalRect {
                x: 0,
                y: h - mag,
                w,
                h: mag,
            } // 内容が上へ → 下端露出
        } else {
            PhysicalRect {
                x: 0,
                y: 0,
                w,
                h: mag,
            } // 内容が下へ → 上端露出
        });
    }
    if bx != 0 {
        let mag = bx.unsigned_abs().min(w);
        if mag == 0 {
            return None;
        }
        return Some(if bx < 0 {
            PhysicalRect {
                x: w - mag,
                y: 0,
                w: mag,
                h,
            } // 内容が左へ → 右端露出
        } else {
            PhysicalRect {
                x: 0,
                y: 0,
                w: mag,
                h,
            } // 内容が右へ → 左端露出
        });
    }
    None
}

/// float 矩形を物理 px 整数格子へ拡張（min を floor・max を ceil）し、ガード余白
/// `ceil(DIRTY_GUARD_IMG_PX × k)` を全辺へ加えて面寸へクランプする（退化は `None`）。
fn expand_guard_clamp(
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    contract: &ScaleContract,
    surface_size: (u32, u32),
) -> Option<PhysicalRect> {
    let (w, h) = (surface_size.0 as i64, surface_size.1 as i64);
    // ガード余白（物理 px・全辺）。負や面外は i64 で扱ってから 0..面寸へクランプする。
    let guard = (DIRTY_GUARD_IMG_PX * contract.scale).ceil() as i64;
    let x0 = (min_x.floor() as i64 - guard).clamp(0, w);
    let y0 = (min_y.floor() as i64 - guard).clamp(0, h);
    let x1 = (max_x.ceil() as i64 + guard).clamp(0, w);
    let y1 = (max_y.ceil() as i64 + guard).clamp(0, h);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some(PhysicalRect {
        x: x0 as u32,
        y: y0 as u32,
        w: (x1 - x0) as u32,
        h: (y1 - y0) as u32,
    })
}
