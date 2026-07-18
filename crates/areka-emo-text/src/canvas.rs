//! # canvas — emo 共有描画基盤の内容キャンバス（純粋層）
//!
//! 描画面を「キャンバスに置かれる変換行列付き矩形コンテンツ（住人）」の集合として表現する
//! `ContentCanvas`／`Resident`／`ResidentContent`（GlyphRun／Image シーム／Surface シーム）／
//! `RegionTransform`（M1 は恒等/平行移動のみ）／`TextEffects`（M2 予約）を担う。
//!
//! **層規律**: 純粋層——`windows` 系 crate への依存を一切持たない（決定論檻）。
//! 行列は自前表現（emo-compose の行列原則と収束可能な統一形・emo-compose は改変しない）。
//!
//! ## 座標系（canvas 空間＝validrect-local・image px）
//!
//! [`ContentCanvas`] の空間は **validrect-local**（原点＝validrect 左上・単位＝image px・
//! `size`＝validrect 寸）。COM 層の TextSurface は「物理寸＝ceil(validrect 寸 × k)・
//! offset＝validrect 原点 × k」で装着されるため、canvas 空間はその描画先と 1:1 に対応する。
//! [`LayoutEngine`](crate::layout::LayoutEngine) の出力（image px 絶対座標）は
//! [`ContentCanvas::from_layout`] が validrect 原点を差し引いて転写する。
//!
//! ## 住人モデル（R8.4——グリフ・画像・将来の SERIKO サーフェスを同格に）
//!
//! 住人＝「キャンバスに置かれる変換行列付き矩形コンテンツ」。コンテンツはローカル座標
//! （原点 0,0 の矩形）で保持し、[`RegionTransform`] がキャンバス上の位置を与える——
//! emo-compose の surface 合成と同じ行列原則（element 配置＝D2D 変換行列・x,y 平行移動は
//! その特例）の同型＝将来収束可能な統一形（R8.6・emo-compose は改変しない）。
//! M1 の実装住人はグリフ（[`GlyphRunContent`]・1 行=1 住人）のみ・Image/Surface は
//! 型シーム（実挙動なし・R8.5——描画実行時の `warn!`＋skip は COM 層 draw の領分）。
//!
//! ## M2 予約（記録のみ・実装しない・R8.3/R10.3）
//!
//! 回転値・文字装飾（アウトライン/多色/シャドウ）は [`TextEffects`] 予約型と
//! 予約名定数（[`RESERVED_EFFECT_OUTLINE`]／[`RESERVED_EFFECT_MULTICOLOR`]／
//! [`RESERVED_EFFECT_SHADOW`]／[`RESERVED_EFFECT_ROTATION`]）として記録するに留め、
//! M1 では実挙動を一切実装しない。`\f` 系文字装飾・`disable.font.*` 拡張も同シームに属する。

use crate::layout::{PositionedGlyph, PositionedLine};
use crate::region::TextRegion;
use crate::writing::WritingMode;

/// M2 予約名: アウトライン装飾 `outline`（記録のみ・実装しない・R8.3/R10.3）。
pub const RESERVED_EFFECT_OUTLINE: &str = "outline";
/// M2 予約名: 多色装飾 `multicolor`（記録のみ・実装しない・R8.3/R10.3）。
pub const RESERVED_EFFECT_MULTICOLOR: &str = "multicolor";
/// M2 予約名: シャドウ装飾 `shadow`（記録のみ・実装しない・R8.3/R10.3）。
pub const RESERVED_EFFECT_SHADOW: &str = "shadow";
/// M2 予約名: 回転 `rotation`（記録のみ・実装しない・R8.2/R8.3）。
pub const RESERVED_EFFECT_ROTATION: &str = "rotation";

/// 変換行列付き領域の行列（R8.1）——3x2 アフィン行列の自前表現（windows 非依存）。
///
/// 行列レイアウトは D2D `Matrix3x2` と同型の `[m11, m12, m21, m22, dx, dy]`
/// （点の写像は `(x', y') = (m11·x + m21·y + dx, m12·x + m22·y + dy)`）。
/// **M1 はコンストラクタが恒等（[`identity`](Self::identity)）と平行移動
/// （[`translation`](Self::translation)）のみを生成する（R8.2）**——回転・拡縮の
/// 生成口は M2 で解禁する型シーム（予約名 [`RESERVED_EFFECT_ROTATION`]）。
/// 合成・適用は一般アフィン積で計算する（emo-compose の行列原則と収束可能な統一形・R8.6）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RegionTransform {
    /// `[m11, m12, m21, m22, dx, dy]`（行優先・D2D Matrix3x2 同型）。
    m: [f32; 6],
}

impl RegionTransform {
    /// 恒等変換（単位行列・平行移動なし）。
    pub fn identity() -> RegionTransform {
        RegionTransform {
            m: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        }
    }

    /// 平行移動 `(dx, dy)`（image px・端数は量子化せず素通し）。
    pub fn translation(dx: f32, dy: f32) -> RegionTransform {
        RegionTransform {
            m: [1.0, 0.0, 0.0, 1.0, dx, dy],
        }
    }

    /// 点 `(x, y)` へ変換を適用する（一般 3x2 アフィン写像）。
    pub fn apply(&self, point: (f32, f32)) -> (f32, f32) {
        self.debug_assert_m1_translation_only("apply");
        let [m11, m12, m21, m22, dx, dy] = self.m;
        (
            m11 * point.0 + m21 * point.1 + dx,
            m12 * point.0 + m22 * point.1 + dy,
        )
    }

    /// 合成 `self → next`（self を適用した後に next を適用する変換・一般 3x2 積）。
    pub fn then(&self, next: &RegionTransform) -> RegionTransform {
        let [a11, a12, a21, a22, adx, ady] = self.m;
        let [b11, b12, b21, b22, bdx, bdy] = next.m;
        let composed = RegionTransform {
            m: [
                a11 * b11 + a12 * b21,
                a11 * b12 + a12 * b22,
                a21 * b11 + a22 * b21,
                a21 * b12 + a22 * b22,
                adx * b11 + ady * b21 + bdx,
                adx * b12 + ady * b22 + bdy,
            ],
        };
        composed.debug_assert_m1_translation_only("then");
        composed
    }

    /// 平行移動成分 `(dx, dy)`。
    pub fn offset(&self) -> (f32, f32) {
        (self.m[4], self.m[5])
    }

    /// 行列 `[m11, m12, m21, m22, dx, dy]`（COM 層の D2D `Matrix3x2` 変換入力形）。
    pub fn to_matrix(&self) -> [f32; 6] {
        self.m
    }

    /// 線形成分が単位行列（＝恒等/平行移動のみ）か（M1 檻・R8.2）。
    pub fn is_translation_only(&self) -> bool {
        self.m[0] == 1.0 && self.m[1] == 0.0 && self.m[2] == 0.0 && self.m[3] == 1.0
    }

    /// M1 不変条件の表明: 回転・拡縮成分ゼロ（コンストラクタが translation のみ生成
    /// するため恒真——生成口が M2 で増えた時に檻が破れないための `debug_assert`・R8.2）。
    fn debug_assert_m1_translation_only(&self, context: &str) {
        debug_assert!(
            self.is_translation_only(),
            "M1 の RegionTransform は恒等/平行移動のみ（{context} で回転/拡縮成分を検出: {:?}）",
            self.m
        );
    }
}

/// 文字装飾の M2 予約型シーム（R8.3/R10.3——実挙動なし・フィールド未使用）。
///
/// 予約対象: アウトライン・多色・シャドウ・回転（予約名定数はモジュール doc 参照）。
/// `#[non_exhaustive]` により M2 でのフィールド追加は破壊的変更にならない。
/// M1 では [`Default`] 生成のみ可能で、描画へ一切影響しない。
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextEffects {}

/// グリフ住人の中身——1 行分のグリフ列（[`PositionedLine`] 由来・行ローカル座標）。
///
/// グリフの `inline_pos` は行ローカル 0 起点（行内軸の向きは [`WritingMode`] が定める——
/// 横書き＝x・縦書き＝y）。キャンバス上の位置は住人の [`RegionTransform`] が与える。
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphRunContent {
    /// 行内グリフ列（行ローカル座標・行内軸位置の昇順・空行は空列）。
    pub glyphs: Vec<PositionedGlyph>,
    /// 行ローカル矩形寸 `(幅, 高さ)`（image px。横書き＝(行内長, font 高)・縦書き＝(font 高, 行内長)）。
    pub size: (f32, f32),
}

/// `\_b` 画像住人の型シーム（実挙動なし・R8.5——fixture 実測で `\_b` 未使用）。
///
/// `#[non_exhaustive]`＝crate 外から構成不能（実挙動を持たせない構造保証）。
/// 実装（画像読込・inline 形/x,y 形の配置・`--option=fixed` 固定層）は後続ユニットの領分。
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ImageSeam {}

/// 将来の SERIKO サーフェス住人の型シーム（実挙動なし・R8.6）。
///
/// シェル/バルーン compositor 融合・背景 SERIKO の住人化は後続 roadmap ユニットが実装する。
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SurfaceSeam {}

/// 住人コンテンツ——グリフ・画像・将来の SERIKO サーフェスを**同格**に扱う enum（R8.4）。
///
/// M1 の実装住人は [`GlyphRun`](Self::GlyphRun) のみ。Image/Surface variant の描画実行は
/// COM 層 draw が `warn!`＋skip する（R8.5）。`#[non_exhaustive]` により住人種の追加は
/// 破壊的変更にならない。
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum ResidentContent {
    /// M1 唯一の実装住人: 1 行分のグリフ列（[`PositionedLine`] 由来）。
    GlyphRun(GlyphRunContent),
    /// `\_b` 画像（型シームのみ・実挙動なし・R8.5）。
    Image(ImageSeam),
    /// 将来の SERIKO サーフェス（シェル/バルーン融合ユニットが実装・R8.6）。
    Surface(SurfaceSeam),
}

/// 住人——キャンバスに置かれる変換行列付き矩形コンテンツ（R8.1/R8.4）。
#[derive(Clone, Debug, PartialEq)]
pub struct Resident {
    /// コンテンツ本体（ローカル座標・原点 0,0 の矩形）。
    pub content: ResidentContent,
    /// キャンバス上の配置変換（M1: 恒等/平行移動のみ・R8.2）。
    pub transform: RegionTransform,
    /// M2 予約の文字装飾シーム（アウトライン/多色/シャドウ/回転・R8.3）。
    pub effects: TextEffects,
}

/// バルーン内容キャンバス（validrect-local・image px 空間）。
///
/// スクロール可視窓（[`VisibleWindow`](crate::layout::VisibleWindow)）は保持しない——
/// canvas は全行を住人として持ち、可視窓の適用は COM 層 draw の領分（R7.4 の分離シーム）。
/// 住人 index は [`LayoutEngine::layout`](crate::layout::LayoutEngine::layout) の
/// 行 index と 1:1（`first_visible_line` がそのまま適用できる——空行も住人として保持）。
#[derive(Clone, Debug, PartialEq)]
pub struct ContentCanvas {
    /// 住人列（M1: layout 出力の行順のグリフ住人のみ）。
    pub residents: Vec<Resident>,
    /// validrect 寸（image px）。
    pub size: (f32, f32),
}

impl ContentCanvas {
    /// レイアウト結果からグリフ住人の内容キャンバスを生成する（純粋・決定論）。
    ///
    /// - 1 行（[`PositionedLine`]）＝1 グリフ住人。layout 出力の行を 1:1 で写す（空行を
    ///   渡されれば空のグリフ住人にする——ただし遅延意味論（newline-defer）では改行由来の
    ///   空行は layout から出力されないため実運用では現れない）。
    /// - 変換＝行矩形原点（左上）への平行移動（validrect-local へ validrect 原点を差引き・
    ///   端数は量子化せず素通し）。恒等/平行移動のみで構成される（R8.2）。
    /// - グリフはローカル座標へ転写: `inline_pos` は行内開始（横書き＝rect.left・
    ///   縦書き＝rect.top）0 起点。
    ///
    /// 同一入力→同一出力（R2.5 系）。失敗経路なし（全入力で値を返す純関数）。
    pub fn from_layout(
        lines: &[PositionedLine],
        region: &TextRegion,
        mode: WritingMode,
    ) -> ContentCanvas {
        let residents = lines
            .iter()
            .map(|line| {
                let rect = &line.rect;
                // 行内軸のローカル原点: 横書き＝x（rect.left）・縦書き＝y（rect.top）。
                let inline_origin = match mode {
                    WritingMode::HorizontalTb => rect.left,
                    WritingMode::VerticalRl | WritingMode::VerticalLr => rect.top,
                };
                let glyphs = line
                    .glyphs
                    .iter()
                    .map(|g| PositionedGlyph {
                        ch: g.ch,
                        inline_pos: g.inline_pos - inline_origin,
                        advance: g.advance,
                    })
                    .collect();
                Resident {
                    content: ResidentContent::GlyphRun(GlyphRunContent {
                        glyphs,
                        size: (rect.right - rect.left, rect.bottom - rect.top),
                    }),
                    transform: RegionTransform::translation(
                        rect.left - region.left(),
                        rect.top - region.top(),
                    ),
                    effects: TextEffects::default(),
                }
            })
            .collect();
        ContentCanvas {
            residents,
            size: (
                region.right() - region.left(),
                region.bottom() - region.top(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use areka_parsers::balloon::{
        BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
    };

    use super::{
        ContentCanvas, GlyphRunContent, RESERVED_EFFECT_MULTICOLOR, RESERVED_EFFECT_OUTLINE,
        RESERVED_EFFECT_ROTATION, RESERVED_EFFECT_SHADOW, RegionTransform, Resident,
        ResidentContent, TextEffects,
    };
    use crate::layout::{FixedMetrics, LayoutEngine, LineRect, PositionedGlyph, PositionedLine};
    use crate::region::TextRegion;
    use crate::state::TextItem;
    use crate::writing::WritingMode;

    /// テスト画像原寸（image px・layout.rs の檻と同一値）。
    const IMAGE: (u32, u32) = (400, 224);

    /// テスト用 BalloonModel 生成ヘルパ（幾何成分だけ指定・validrect 順序は top,bottom,left,right）。
    fn model(
        origin: (Option<i32>, Option<i32>),
        wordwrap: (Option<i32>, Option<i32>),
        validrect: (Option<i32>, Option<i32>, Option<i32>, Option<i32>),
    ) -> BalloonModel {
        BalloonModel::new(
            WindowPosition::new(None, None),
            Origin::new(origin.0, origin.1),
            WordWrapPoint::new(wordwrap.0, wordwrap.1),
            ValidRect::new(validrect.0, validrect.1, validrect.2, validrect.3),
            Font::new(None, None, FontColor::new(None, None, None)),
            None,
        )
    }

    /// n 個の全角グリフ（'あ'）item 列。
    fn glyphs(n: usize) -> Vec<TextItem> {
        std::iter::repeat_n(TextItem::Glyph { ch: 'あ' }, n).collect()
    }

    /// layout→canvas の通し（テスト用最短経路・visible は全量）。
    fn canvas_for(
        items: &[TextItem],
        region: &TextRegion,
        mode: WritingMode,
        font_height: f32,
    ) -> (Vec<PositionedLine>, ContentCanvas) {
        let visible = items
            .iter()
            .filter(|i| matches!(i, TextItem::Glyph { .. }))
            .count();
        let lines = LayoutEngine::layout(items, visible, region, mode, font_height, &FixedMetrics);
        let canvas = ContentCanvas::from_layout(&lines, region, mode);
        (lines, canvas)
    }

    /// 住人からグリフ住人の中身を取り出す（M1 の唯一の実装住人）。
    fn glyph_run(resident: &Resident) -> &GlyphRunContent {
        match &resident.content {
            ResidentContent::GlyphRun(run) => run,
            other => panic!("グリフ住人を期待したが {other:?} が現れた"),
        }
    }

    // ── R8.1/R8.2: RegionTransform——変換行列付き領域・M1 は恒等/平行移動のみ ──

    /// 恒等変換: 点を変えない・平行移動成分ゼロ・行列は単位行列。
    #[test]
    fn region_transform_identity_maps_points_unchanged() {
        let t = RegionTransform::identity();
        assert!(t.is_translation_only());
        assert_eq!(t.apply((3.5, -2.0)), (3.5, -2.0));
        assert_eq!(t.offset(), (0.0, 0.0));
        assert_eq!(t.to_matrix(), [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    }

    /// 平行移動: 点へオフセットが加算される（端数値も素通し・量子化しない）。
    #[test]
    fn region_transform_translation_offsets_points() {
        let t = RegionTransform::translation(10.0, -4.5);
        assert!(t.is_translation_only());
        assert_eq!(t.apply((2.0, 3.0)), (12.0, -1.5));
        assert_eq!(t.offset(), (10.0, -4.5));
        assert_eq!(t.to_matrix(), [1.0, 0.0, 0.0, 1.0, 10.0, -4.5]);
    }

    /// 合成: 平行移動同士の合成はオフセットの和・恒等は合成の単位元
    /// （一般 3x2 アフィン積で計算する——emo-compose 行列原則と収束可能な統一形・R8.6）。
    #[test]
    fn region_transform_composition_accumulates_translations() {
        let a = RegionTransform::translation(3.0, 4.0);
        let b = RegionTransform::translation(-1.0, 2.5);
        let c = a.then(&b);
        assert!(c.is_translation_only());
        assert_eq!(c.offset(), (2.0, 6.5));
        assert_eq!(c.apply((1.0, 1.0)), (3.0, 7.5));
        assert_eq!(a.then(&RegionTransform::identity()), a);
        assert_eq!(RegionTransform::identity().then(&a), a);
    }

    // ── R8.3/R10.3: TextEffects——M2 予約の型シーム（実挙動なし・予約名は定数記録のみ） ──

    /// M2 予約名（outline/multicolor/shadow/rotation）は定数として記録するに留める。
    /// TextEffects 自体はフィールド未使用の予約型（既定値のみ生成可能）。
    #[test]
    fn text_effects_is_reserved_seam_with_recorded_names() {
        assert_eq!(RESERVED_EFFECT_OUTLINE, "outline");
        assert_eq!(RESERVED_EFFECT_MULTICOLOR, "multicolor");
        assert_eq!(RESERVED_EFFECT_SHADOW, "shadow");
        assert_eq!(RESERVED_EFFECT_ROTATION, "rotation");
        assert_eq!(TextEffects::default(), TextEffects::default());
    }

    // ── R8.4/8.1: レイアウト結果→グリフ住人（1 行=1 住人・行 index 1:1・変換行列付き） ──

    /// 横書き 2 行（折返し）: 行ごとに 1 グリフ住人が生成され、変換は行矩形原点への
    /// 平行移動・グリフは行ローカル 0 起点・effects は予約既定値。
    #[test]
    fn from_layout_generates_one_glyph_resident_per_line() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (Some(50), None), (None, None, None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        // layout 檻と同一幾何: 全角 6・font 10 → 2 行（行 0: 5 グリフ・行 1: 1 グリフ）。
        let (lines, canvas) = canvas_for(&glyphs(6), &region, WritingMode::HorizontalTb, 10.0);
        assert_eq!(lines.len(), 2);
        assert_eq!(canvas.residents.len(), 2, "1 行 = 1 グリフ住人（index 1:1）");
        assert_eq!(canvas.size, (400.0, 224.0), "canvas 寸 = validrect 寸");

        // 行 0: 変換 = validrect-local の行矩形原点 (0,0)・グリフはローカル 0,10,…,40。
        let r0 = &canvas.residents[0];
        assert!(r0.transform.is_translation_only(), "M1 は恒等/平行移動のみ");
        assert_eq!(r0.transform.offset(), (0.0, 0.0));
        assert_eq!(r0.effects, TextEffects::default());
        let run0 = glyph_run(r0);
        assert_eq!(run0.size, (50.0, 10.0));
        let inline0: Vec<f32> = run0.glyphs.iter().map(|g| g.inline_pos).collect();
        assert_eq!(inline0, vec![0.0, 10.0, 20.0, 30.0, 40.0]);
        assert!(run0.glyphs.iter().all(|g| g.ch == 'あ' && g.advance == 10.0));

        // 行 1: 変換 = (0, 13)（pitch 分の平行移動）・ローカル 0 起点。
        let r1 = &canvas.residents[1];
        assert_eq!(r1.transform.offset(), (0.0, 13.0));
        let run1 = glyph_run(r1);
        assert_eq!(run1.size, (10.0, 10.0));
        assert_eq!(run1.glyphs[0].inline_pos, 0.0);
    }

    /// origin が validrect 内部でも同一規則: 変換 = 行矩形原点そのもの
    /// （validrect left/top = 0 のとき絶対座標と一致）・グリフローカル位置は 0 起点へ戻る。
    #[test]
    fn from_layout_translation_carries_line_origin() {
        let region = TextRegion::resolve(
            &model(
                (Some(100), Some(50)),
                (Some(150), None),
                (None, None, None, None),
            ),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        // layout 檻と同一幾何: font 10・6 グリフ → 行 1 は (100, 63) 起点。
        let (_, canvas) = canvas_for(&glyphs(6), &region, WritingMode::HorizontalTb, 10.0);
        assert_eq!(canvas.residents.len(), 2);
        assert_eq!(canvas.residents[0].transform.offset(), (100.0, 50.0));
        assert_eq!(canvas.residents[1].transform.offset(), (100.0, 63.0));
        for resident in &canvas.residents {
            let run = glyph_run(resident);
            assert_eq!(run.glyphs[0].inline_pos, 0.0, "グリフは行ローカル 0 起点");
        }
    }

    /// canvas 空間は validrect-local（image px）: validrect 原点が非零なら変換から
    /// 差し引かれ、canvas 寸は validrect 寸になる（TextSurface の物理寸/offset 契約と整合）。
    #[test]
    fn from_layout_rebases_to_validrect_local_space() {
        // vertical_rl・validrect top20/bottom224/left360/right400・書字開始角 (400, 20)。
        let region = TextRegion::resolve(
            &model(
                (None, None),
                (None, None),
                (Some(20), Some(224), Some(360), Some(400)),
            ),
            IMAGE,
            WritingMode::VerticalRl,
        );
        assert_eq!(region.start(), (400.0, 20.0));
        // 全角 2 グリフ・font 10: 列矩形 = x 帯 [390,400]・y 20..40。
        let (lines, canvas) = canvas_for(&glyphs(2), &region, WritingMode::VerticalRl, 10.0);
        assert_eq!(lines.len(), 1);
        assert_eq!(canvas.size, (40.0, 204.0), "canvas 寸 = validrect 寸");
        let resident = &canvas.residents[0];
        // validrect-local: (390−360, 20−20) = (30, 0)。
        assert_eq!(resident.transform.offset(), (30.0, 0.0));
        let run = glyph_run(resident);
        assert_eq!(run.size, (10.0, 20.0), "縦書き列: 幅=font 高・長さ=行内送り");
        // 縦書きの行内軸は y——ローカル 0 起点で 0, 10。
        let inline: Vec<f32> = run.glyphs.iter().map(|g| g.inline_pos).collect();
        assert_eq!(inline, vec![0.0, 10.0]);
    }

    /// 遅延意味論では改行由来の空行が layout 入力に来なくなる: trailing 改行 `[あ, \n]` は
    /// 行 1・住人 1（1:1 維持・空住人が生じない）。first_visible_line が layout 出力 index の
    /// まま canvas に適用できる不変条件は「layout 出力の行」に対する契約ゆえ保たれる。
    #[test]
    fn trailing_newline_yields_single_resident_under_deferred_semantics() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None), (None, None, None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = [TextItem::Glyph { ch: 'あ' }, TextItem::LineBreak { ratio: 1.0 }];
        let (lines, canvas) = canvas_for(&items, &region, WritingMode::HorizontalTb, 12.0);
        assert_eq!(lines.len(), 1, "末尾保留改行は蒸発＝空行なし");
        assert_eq!(canvas.residents.len(), 1, "行 1 = 住人 1（1:1）");
        assert_eq!(glyph_run(&canvas.residents[0]).glyphs.len(), 1);
    }

    /// canvas の写像自体は一般性を保つ（本番非改変の証）: 空グリフの [`PositionedLine`] を
    /// 直接渡せば従来どおり空グリフ住人へ 1:1 で写す。遅延意味論では layout が空行を出さなく
    /// なるだけで、写像側は空行を渡されれば空住人にする振る舞いを不変に保つ。
    #[test]
    fn from_layout_maps_empty_line_to_empty_resident() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None), (None, None, None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        // synthetic: 1 グリフ行＋空行（行送り軸 15 の零幅矩形）を直接構築する。
        let synthetic = [
            PositionedLine {
                rect: LineRect {
                    left: 0.0,
                    top: 0.0,
                    right: 12.0,
                    bottom: 12.0,
                },
                glyphs: vec![PositionedGlyph {
                    ch: 'あ',
                    inline_pos: 0.0,
                    advance: 12.0,
                }],
            },
            PositionedLine {
                rect: LineRect {
                    left: 0.0,
                    top: 15.0,
                    right: 0.0,
                    bottom: 27.0,
                },
                glyphs: vec![],
            },
        ];
        let canvas = ContentCanvas::from_layout(&synthetic, &region, WritingMode::HorizontalTb);
        assert_eq!(canvas.residents.len(), 2, "写像は行数と 1:1（空行も住人化）");
        let empty = glyph_run(&canvas.residents[1]);
        assert!(empty.glyphs.is_empty());
        assert_eq!(empty.size, (0.0, 12.0), "空行は行内零幅・行送り軸は font 高");
        assert_eq!(canvas.residents[1].transform.offset(), (0.0, 15.0));
    }

    /// ratio 付き改行の端数行送り（pitch 15 × 0.5 = 7.5）も平行移動へ端数のまま
    /// 転写される（整数量子化しない・端数檻——tasks.md Implementation Notes）。
    #[test]
    fn fractional_line_feed_survives_in_translation() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None), (None, None, None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = [
            TextItem::Glyph { ch: 'あ' },
            TextItem::LineBreak { ratio: 1.0 },
            TextItem::Glyph { ch: 'あ' },
            TextItem::LineBreak { ratio: 0.5 },
            TextItem::Glyph { ch: 'あ' },
        ];
        let (_, canvas) = canvas_for(&items, &region, WritingMode::HorizontalTb, 12.0);
        let offsets: Vec<(f32, f32)> = canvas
            .residents
            .iter()
            .map(|r| r.transform.offset())
            .collect();
        assert_eq!(offsets, vec![(0.0, 0.0), (0.0, 15.0), (0.0, 22.5)]);
    }

    // ── R8.4/R8.5: 画像・サーフェス住人は型シームのみ（同格 enum・実挙動なし） ──

    /// M1 の from_layout はグリフ住人のみを生成する（Image/Surface は現れない）。
    /// Image/Surface は同格の enum variant として存在する（型シーム・実挙動なし・
    /// 描画実行時の warn!＋skip は COM 層 draw の領分）。
    #[test]
    fn from_layout_yields_only_glyph_residents_and_seams_stay_inert() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None), (None, None, None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let (_, canvas) = canvas_for(&glyphs(3), &region, WritingMode::HorizontalTb, 12.0);
        assert!(
            canvas
                .residents
                .iter()
                .all(|r| matches!(r.content, ResidentContent::GlyphRun(_))),
            "M1 の実装住人はグリフのみ（R8.5）"
        );
        // 型シームは crate 内でのみ構成可能（enum 上でグリフと同格・R8.4）。
        let image = Resident {
            content: ResidentContent::Image(super::ImageSeam::default()),
            transform: RegionTransform::identity(),
            effects: TextEffects::default(),
        };
        let surface = Resident {
            content: ResidentContent::Surface(super::SurfaceSeam::default()),
            transform: RegionTransform::identity(),
            effects: TextEffects::default(),
        };
        assert!(matches!(image.content, ResidentContent::Image(_)));
        assert!(matches!(surface.content, ResidentContent::Surface(_)));
    }

    // ── R8.2 総括＋決定論: 恒等/平行移動のみで構成された canvas・同一入力→同一出力 ──

    /// 生成された canvas の全住人変換が恒等/平行移動のみで構成される（M1 檻・R8.2）。
    /// 3 方向すべてで成立し、同一入力からの再生成は完全一致する（純関数・決定論檻）。
    #[test]
    fn canvas_is_translation_only_and_deterministic_across_modes() {
        let m = model(
            (Some(0), Some(0)),
            (Some(50), Some(30)),
            (None, None, None, None),
        );
        let items = [
            TextItem::Glyph { ch: 'あ' },
            TextItem::LineBreak { ratio: 0.5 },
            TextItem::Glyph { ch: 'a' },
            TextItem::Glyph { ch: '漢' },
        ];
        for mode in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            let region = TextRegion::resolve(&m, IMAGE, mode);
            let (_, first) = canvas_for(&items, &region, mode, 10.0);
            let (_, second) = canvas_for(&items, &region, mode, 10.0);
            assert!(
                first.residents.iter().all(|r| r.transform.is_translation_only()),
                "mode {mode:?}: M1 の変換は恒等/平行移動のみ（R8.2）"
            );
            assert_eq!(first, second, "mode {mode:?} で決定論が崩れている");
        }
    }
}
