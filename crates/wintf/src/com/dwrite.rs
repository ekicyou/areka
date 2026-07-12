use windows::Win32::Graphics::DirectWrite::*;
use windows::core::*;

/// DWriteCreateFactory
pub fn dwrite_create_factory(factorytype: DWRITE_FACTORY_TYPE) -> Result<IDWriteFactory2> {
    unsafe { DWriteCreateFactory(factorytype) }
}

pub trait DWriteFactoryExt {
    /// CreateTextFormat
    fn create_text_format<P0, P1>(
        &self,
        fontfamilyname: P0,
        fontcollection: P1,
        fontweight: DWRITE_FONT_WEIGHT,
        fontstyle: DWRITE_FONT_STYLE,
        fontstretch: DWRITE_FONT_STRETCH,
        fontsize: f32,
        localename: P0,
    ) -> Result<IDWriteTextFormat>
    where
        P0: Param<PCWSTR>,
        P1: Param<IDWriteFontCollection>;

    /// CreateTextLayout
    fn create_text_layout<P0>(
        &self,
        text: P0,
        text_format: &IDWriteTextFormat,
        max_width: f32,
        max_height: f32,
    ) -> Result<IDWriteTextLayout>
    where
        P0: Param<PCWSTR>;
}

impl DWriteFactoryExt for IDWriteFactory2 {
    #[inline(always)]
    fn create_text_format<P0, P1>(
        &self,
        fontfamilyname: P0,
        fontcollection: P1,
        fontweight: DWRITE_FONT_WEIGHT,
        fontstyle: DWRITE_FONT_STYLE,
        fontstretch: DWRITE_FONT_STRETCH,
        fontsize: f32,
        localename: P0,
    ) -> Result<IDWriteTextFormat>
    where
        P0: Param<PCWSTR>,
        P1: Param<IDWriteFontCollection>,
    {
        unsafe {
            self.CreateTextFormat(
                fontfamilyname,
                fontcollection,
                fontweight,
                fontstyle,
                fontstretch,
                fontsize,
                localename,
            )
        }
    }

    fn create_text_layout<P0>(
        &self,
        text: P0,
        text_format: &IDWriteTextFormat,
        max_width: f32,
        max_height: f32,
    ) -> Result<IDWriteTextLayout>
    where
        P0: Param<PCWSTR>,
    {
        unsafe {
            let text_param = text.param();
            let text_pcwstr = text_param.abi();

            // null PCWSTR は空文字列として扱う（as_wide は非 null 前提のため事前に分岐）
            let text_slice: &[u16] = if text_pcwstr.is_null() {
                &[]
            } else {
                text_pcwstr.as_wide()
            };

            self.CreateTextLayout(text_slice, text_format, max_width, max_height)
        }
    }
}

pub trait DWriteTextFormatExt {
    /// SetTextAlignment
    fn set_text_alignment(&self, textalignment: DWRITE_TEXT_ALIGNMENT) -> Result<()>;
    /// SetParagraphAlignment
    fn set_paragraph_alignment(&self, paragraphalignment: DWRITE_PARAGRAPH_ALIGNMENT)
    -> Result<()>;
}

impl DWriteTextFormatExt for IDWriteTextFormat {
    #[inline(always)]
    fn set_text_alignment(&self, textalignment: DWRITE_TEXT_ALIGNMENT) -> Result<()> {
        unsafe { self.SetTextAlignment(textalignment) }
    }

    #[inline(always)]
    fn set_paragraph_alignment(
        &self,
        paragraphalignment: DWRITE_PARAGRAPH_ALIGNMENT,
    ) -> Result<()> {
        unsafe { self.SetParagraphAlignment(paragraphalignment) }
    }
}

// ============================================================
// DWriteTextLayoutExt - クラスタメトリクス取得API
// ============================================================

/// HitTestTextPosition の結果
#[derive(Debug, Clone)]
pub struct HitTestResult {
    pub point_x: f32,
    pub point_y: f32,
    pub metrics: DWRITE_HIT_TEST_METRICS,
}

pub trait DWriteTextLayoutExt {
    /// クラスタメトリクス取得
    fn get_cluster_metrics(&self) -> Result<Vec<DWRITE_CLUSTER_METRICS>>;

    /// クラスタ数取得
    fn get_cluster_count(&self) -> Result<u32>;

    /// テキスト位置からヒットテスト（描画位置取得用）
    fn hit_test_text_position(
        &self,
        text_position: u32,
        is_trailing_hit: bool,
    ) -> Result<HitTestResult>;

    /// レイアウトボックス各辺からのインクはみ出し量（[`DWRITE_OVERHANG_METRICS`]）を返す。
    ///
    /// 正の値＝レイアウトボックスの外側へインクがはみ出している量（DIP）、負の値＝内側に
    /// 余りがある。descent 側のはみ出し・イタリック右張り出し・アクセント・合字・装飾スワッシュ
    /// 等、em/レイアウトボックスからはみ出す実インク境界を得るのに用いる。**注意**: 値は
    /// レイアウトボックス基準ゆえ、`SetMaxWidth`/`SetMaxHeight` が巨大値（f32::MAX 等）のままだと
    /// 対応する辺のはみ出しが巨大な負値になる——タイトなインク境界が要る場合は先に `GetMetrics`
    /// の width/height を Max に設定すること（ブロック軸だけ欲しい場合は当該軸の箱寸のみ確定で足りる）。
    fn get_overhang_metrics(&self) -> Result<DWRITE_OVERHANG_METRICS>;
}

impl DWriteTextLayoutExt for IDWriteTextLayout {
    fn get_cluster_metrics(&self) -> Result<Vec<DWRITE_CLUSTER_METRICS>> {
        unsafe {
            // まず必要なサイズを取得
            let mut actual_count = 0u32;
            // GetClusterMetrics with None returns actual count needed
            let _ = self.GetClusterMetrics(None, &mut actual_count);

            if actual_count == 0 {
                return Ok(Vec::new());
            }

            // バッファを確保して再取得
            let mut metrics = vec![DWRITE_CLUSTER_METRICS::default(); actual_count as usize];
            self.GetClusterMetrics(Some(&mut metrics), &mut actual_count)?;
            metrics.truncate(actual_count as usize);

            Ok(metrics)
        }
    }

    fn get_cluster_count(&self) -> Result<u32> {
        unsafe {
            let mut actual_count = 0u32;
            let _ = self.GetClusterMetrics(None, &mut actual_count);
            Ok(actual_count)
        }
    }

    fn hit_test_text_position(
        &self,
        text_position: u32,
        is_trailing_hit: bool,
    ) -> Result<HitTestResult> {
        unsafe {
            let mut point_x = 0.0f32;
            let mut point_y = 0.0f32;
            let mut metrics = DWRITE_HIT_TEST_METRICS::default();

            self.HitTestTextPosition(
                text_position,
                is_trailing_hit.into(),
                &mut point_x,
                &mut point_y,
                &mut metrics,
            )?;

            Ok(HitTestResult {
                point_x,
                point_y,
                metrics,
            })
        }
    }

    fn get_overhang_metrics(&self) -> Result<DWRITE_OVERHANG_METRICS> {
        // SAFETY: GetOverhangMetrics は out 構造体を書き込むだけ（レイアウトは内部で確定される）。
        unsafe { self.GetOverhangMetrics() }
    }
}
