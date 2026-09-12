//! # draw_line_store — 行 TextLayout の生成・キャッシュ（COM 層・draw ファサードの子）
//!
//! [`LineLayoutStore`]（行 TextLayout の共有ストア）と、行の実測インクはみ出しを担う。
//! 比較専用オラクル（親ファサードの `DrawExecutor`）と本番 `ViewboxExecutor` が**同一経路**で
//! 行レイアウトを得るための抽出型。親ファサード `draw.rs` からの**純移動**——生成規則・キー・
//! 内容不変再利用・破棄規律はいずれも移動前と同一。
//!
//! **層規律**: COM 層——UI スレッド専有。失敗は log-first（`tracing::error!`＋`Err`）で
//! 扱い panic しない。
//!
//! 親から `pub(crate) use` で再輸出されるため、crate 内から見た入口は
//! `crate::draw::LineLayoutStore` のまま変わらない。

use std::collections::HashMap;

use windows::Win32::Graphics::DirectWrite::{
    IDWriteFactory2, IDWriteTextFormat, IDWriteTextLayout,
};
use windows::core::HSTRING;
use wintf::com::dwrite::{DWriteFactoryExt, DWriteTextLayoutExt};

use super::{PROBE_MAX_EXTENT, device_err};
use crate::TextLayerError;
use crate::viewbox::LineOverhang;
use crate::writing::WritingMode;

/// キャッシュ済みの行 TextLayout（行内容の正本文字列＋実測インクはみ出しと対で保持・
/// 内容不変なら再利用）。`overhang` は生成時に一度だけ [`DWriteTextLayoutExt::get_overhang_metrics`]
/// で実測（確定行は再計測しない）——ViewboxExecutor のダーティ矩形が em ボックス下端はみ出しを
/// 取りこぼさないための実測値（D2）。
struct CachedLineLayout {
    text: String,
    layout: IDWriteTextLayout,
    overhang: LineOverhang,
}

/// 行 TextLayout の生成・キャッシュを担う共有ストア（複数の描画実行が**同一経路**で
/// 行レイアウトを得るための抽出型・design.md「draw.rs の再編（LineLayoutStore 抽出）」）。
///
/// 生成規則（行内軸＝[`PROBE_MAX_EXTENT`]・行送り軸＝`font_height`・同一 format）・キー
/// （canvas 行 index）・内容不変再利用・破棄規律（[`clear`](Self::clear) のみ全破棄）は
/// 抽出前の `DrawExecutor` 内実装と同一——TextLayout 生成経路の完全共有により両描画実行の
/// **byte 等価**を構造化する（RN5）。UI スレッド専有（COM 層規律）。
///
/// `pub(crate)`: [`DrawExecutor`]（front へ全域再描画）と `ViewboxExecutor`
/// （back へダーティ描画・viewbox_draw.rs）が**同一経路**で行レイアウトを得るため
/// crate 内へ公開する（生成規則・キー・破棄規律は不変）。
pub(crate) struct LineLayoutStore {
    /// 行 TextLayout 生成用 factory（probe/描画と同一の `IDWriteFactory2`）。
    factory: IDWriteFactory2,
    /// 行 TextLayout キャッシュ（key＝canvas 行 index。追記単調ゆえ確定行の index/内容は
    /// 不変——リビール中＝最終行のみ内容が変わり都度更新される）。
    cache: HashMap<usize, CachedLineLayout>,
    /// 行 TextLayout の累計生成回数（**常時コンパイル**・後続 task の `DrawStats` へ集計する
    /// ため `#[cfg(test)]` にしない・design「Modified Files」）。
    creations: u64,
}

impl LineLayoutStore {
    /// factory を束ねて空ストアを生成する（factory は clone 保持）。
    pub(crate) fn new(factory: &IDWriteFactory2) -> LineLayoutStore {
        LineLayoutStore {
            factory: factory.clone(),
            cache: HashMap::new(),
            creations: 0,
        }
    }

    /// 行 TextLayout の取得（内容不変なら再利用・変化時のみ生成して置換）。
    ///
    /// 行の箱寸は「行内軸＝折返し無効寸（[`PROBE_MAX_EXTENT`]・折返しは純粋層で決定済み
    /// ＝再折返しさせない）・行送り軸＝`font_height`」。方向レシピ（LEADING/NEAR）により
    /// 行は箱の書字開始角に付くため、描画原点＝行矩形原点で位置が定まる。
    pub(crate) fn line_layout(
        &mut self,
        index: usize,
        text: &str,
        format: &IDWriteTextFormat,
        font_height: f32,
        mode: WritingMode,
    ) -> Result<IDWriteTextLayout, TextLayerError> {
        if let Some(cached) = self.cache.get(&index) {
            if cached.text == text {
                return Ok(cached.layout.clone());
            }
        }
        let (max_width, max_height) = match mode {
            WritingMode::HorizontalTb => (PROBE_MAX_EXTENT, font_height),
            WritingMode::VerticalRl | WritingMode::VerticalLr => (font_height, PROBE_MAX_EXTENT),
        };
        let layout = self
            .factory
            .create_text_layout(&HSTRING::from(text), format, max_width, max_height)
            .map_err(device_err("CreateTextLayout(line)"))?;
        self.creations += 1;
        // 実測インクはみ出し（生成時 1 回・確定行は再計測しない）。行ボックスのブロック軸寸は
        // font_height（横＝max_height／縦＝max_width）ゆえ、その軸の overhang が em ボックスからの
        // はみ出しを直接与える。行内軸は巨大 PROBE_MAX_EXTENT 箱ゆえ overhang は巨大負値＝`max(0.0)`
        // で 0 に丸まる（resident_rect はブロック軸の overhang のみ使う）。
        let overhang = measure_line_overhang(&layout)?;
        self.cache.insert(
            index,
            CachedLineLayout {
                text: text.to_owned(),
                layout: layout.clone(),
                overhang,
            },
        );
        Ok(layout)
    }

    /// キャッシュ済み行の実測インクはみ出し（[`LineOverhang`]）——`ViewboxExecutor` が plan へ渡す。
    /// 未生成 index は `None`（呼び手は既定 0＝em ボックス丈として扱う）。
    pub(crate) fn overhang(&self, index: usize) -> Option<LineOverhang> {
        self.cache.get(&index).map(|c| c.overhang)
    }

    /// キャッシュを全破棄する（Clear cue の適用点・破棄はこの口だけ）。
    pub(crate) fn clear(&mut self) {
        self.cache.clear();
    }

    /// 行 TextLayout の累計生成回数（常時コンパイル・`DrawStats` 集計とテスト観測の共通読み口）。
    /// `ViewboxExecutor::render`（viewbox_draw.rs）が本フレームの生成増分を `DrawStats`
    /// （`line_layout_creations`）へ集計するために非テストビルドでも読む。
    pub(crate) fn creations(&self) -> u64 {
        self.creations
    }
}

/// 行 TextLayout の実測インクはみ出し（[`LineOverhang`]・image px・全成分 ≥ 0）を返す。
///
/// [`DWriteTextLayoutExt::get_overhang_metrics`]（`GetOverhangMetrics`）はレイアウトボックス各辺
/// からのはみ出し（正＝外側・DIP）を返す。行ボックスのブロック軸寸が `font_height`（横＝`max_height`
/// ／縦＝`max_width`）に設定済みゆえ、その軸の値が em ボックス下端/上端（縦は左右）からのはみ出しを
/// 直接与える。行内軸は巨大 `PROBE_MAX_EXTENT` 箱ゆえ値は巨大負値＝`max(0.0)` で 0 に丸まる
/// （`resident_rect` はブロック軸の overhang のみ使うため、これで正しくブロック軸だけが効く）。
fn measure_line_overhang(layout: &IDWriteTextLayout) -> Result<LineOverhang, TextLayerError> {
    let o = layout
        .get_overhang_metrics()
        .map_err(device_err("GetOverhangMetrics(line)"))?;
    Ok(LineOverhang {
        top: o.top.max(0.0),
        bottom: o.bottom.max(0.0),
        left: o.left.max(0.0),
        right: o.right.max(0.0),
    })
}
