use std::path::Path;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Imaging::{D2D::*, *};
use windows::Win32::System::Com::*;
use windows::core::*;

pub fn wic_factory() -> Result<IWICImagingFactory2> {
    unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) }
}

/// WIC で画像を読み込み、PBGRA32 形式の `IWICBitmapSource` と、
/// **変換前フレームの α 有無** (`has_alpha`) を返す。
///
/// この関数は元々 `ecs/widget/bitmap_source/systems.rs` に存在したが、
/// ECS 非依存の COM 層へ移設した（design Decision D2）。挙動は不変。
/// `systems.rs` は本関数を re-export して既存のパス互換を維持する。
///
/// `has_alpha` は PBGRA32 変換で失われるため（変換は α 無し画像も
/// 100% 不透明として PBGRA へ埋める）、**変換前フレームの
/// `GetPixelFormat` の GUID** から判定する（design Critical Issue 1）。
///
/// # Arguments
/// * `factory` - WIC ファクトリ
/// * `path` - 画像ファイルパス
///
/// # Returns
/// `(PBGRA32 に変換した IWICBitmapSource, 変換前フレームの α 有無)`
pub fn load_bitmap_source(
    factory: &IWICImagingFactory2,
    path: &Path,
) -> Result<(IWICBitmapSource, bool)> {
    // パスを HSTRING に変換
    let path_str = path.to_string_lossy();
    let path_hstring = HSTRING::from(path_str.as_ref());

    // デコーダー作成
    let decoder = factory.create_decoder_from_filename(
        &path_hstring,
        None,
        GENERIC_READ,
        WICDecodeMetadataCacheOnDemand,
    )?;

    // 最初のフレームを取得
    let frame = decoder.frame(0)?;

    // 変換前フレームのピクセルフォーマットから α 有無を判定（Critical Issue 1）。
    // PBGRA 変換後は必ず α 付きフォーマットになり判別不能になるため、
    // 変換前にここで確定させる。
    let pre_format = unsafe { frame.GetPixelFormat()? };
    let has_alpha = pixel_format_has_alpha(&pre_format);

    // PBGRA32 に変換（α チャネルがなくても 100% 不透明として変換）
    let converter = factory.create_format_converter()?;
    converter.init(
        &frame.cast::<IWICBitmapSource>()?,
        &GUID_WICPixelFormat32bppPBGRA,
        WICBitmapDitherTypeNone,
        None,
        0.0,
        WICBitmapPaletteTypeMedianCut,
    )?;

    Ok((converter.cast()?, has_alpha))
}

/// WIC ピクセルフォーマット GUID が α チャンネルを持つかを判定する。
///
/// design Critical Issue 1 の指示どおり、α 付きフォーマット集合との
/// 一致で判定する（既知の非 α フォーマットは default で `false`）。
fn pixel_format_has_alpha(format: &GUID) -> bool {
    // windows-rs の GUID 定数は非 UPPER_CASE のため `match`/`matches!` の
    // パターンに置くと `non_upper_case_globals` future-incompat 警告が出る。
    // 等値比較 (`==`) なら定数を値として扱うため警告を回避できる。
    const ALPHA_FORMATS: &[GUID] = &[
        GUID_WICPixelFormat32bppBGRA,
        GUID_WICPixelFormat32bppPBGRA,
        GUID_WICPixelFormat32bppRGBA,
        GUID_WICPixelFormat32bppPRGBA,
        GUID_WICPixelFormat64bppRGBA,
        GUID_WICPixelFormat64bppPRGBA,
        GUID_WICPixelFormat64bppBGRA,
        GUID_WICPixelFormat64bppPBGRA,
        GUID_WICPixelFormat32bppRGBA1010102,
        GUID_WICPixelFormat32bppRGBA1010102XR,
        GUID_WICPixelFormat128bppRGBAFloat,
        GUID_WICPixelFormat128bppPRGBAFloat,
    ];
    ALPHA_FORMATS.contains(format)
}

pub trait WICImagingFactoryExt {
    /// CreateDecoderFromFilename
    fn create_decoder_from_filename<P0>(
        &self,
        wzfilename: P0,
        pguidvendor: Option<&GUID>,
        dwdesiredaccess: GENERIC_ACCESS_RIGHTS,
        metadataoptions: WICDecodeOptions,
    ) -> Result<IWICBitmapDecoder>
    where
        P0: Param<PCWSTR>;

    /// CreateFormatConverter
    fn create_format_converter(&self) -> Result<IWICFormatConverter>;
}

impl WICImagingFactoryExt for IWICImagingFactory2 {
    #[inline(always)]
    fn create_decoder_from_filename<P0>(
        &self,
        wzfilename: P0,
        pguidvendor: Option<&GUID>,
        dwdesiredaccess: GENERIC_ACCESS_RIGHTS,
        metadataoptions: WICDecodeOptions,
    ) -> Result<IWICBitmapDecoder>
    where
        P0: Param<PCWSTR>,
    {
        unsafe {
            self.CreateDecoderFromFilename(
                wzfilename,
                pguidvendor.map(|guid| guid as *const GUID),
                dwdesiredaccess,
                metadataoptions,
            )
        }
    }

    #[inline(always)]
    fn create_format_converter(&self) -> Result<IWICFormatConverter> {
        unsafe { self.CreateFormatConverter() }
    }
}

pub trait WICBitmapDecoderExt {
    /// GetFrame
    fn frame(&self, index: u32) -> Result<IWICBitmapFrameDecode>;
}

impl WICBitmapDecoderExt for IWICBitmapDecoder {
    #[inline(always)]
    fn frame(&self, index: u32) -> Result<IWICBitmapFrameDecode> {
        unsafe { self.GetFrame(index) }
    }
}

pub trait WICFormatConverterExt {
    /// Initialize
    fn init(
        &self,
        pisource: &IWICBitmapSource,
        dstformat: &GUID,
        dither: WICBitmapDitherType,
        pipalette: Option<&IWICPalette>,
        alpha_threshold_percent: f64,
        palette_translate: WICBitmapPaletteType,
    ) -> Result<()>;
}

impl WICFormatConverterExt for IWICFormatConverter {
    #[inline(always)]
    fn init(
        &self,
        pisource: &IWICBitmapSource,
        dstformat: &GUID,
        dither: WICBitmapDitherType,
        pipalette: Option<&IWICPalette>,
        alpha_threshold_percent: f64,
        palette_translate: WICBitmapPaletteType,
    ) -> Result<()> {
        unsafe {
            self.Initialize(
                pisource,
                dstformat as *const GUID,
                dither,
                pipalette,
                alpha_threshold_percent,
                palette_translate,
            )
        }
    }
}

/// IWICBitmapSource 拡張トレイト
pub trait WICBitmapSourceExt {
    /// 画像サイズを取得
    fn get_size(&self) -> Result<(u32, u32)>;

    /// ピクセルデータをバッファにコピー
    ///
    /// # Arguments
    /// - `rect`: コピー対象の矩形（Noneで全体）
    /// - `stride`: 行あたりのバイト数
    /// - `buffer`: 出力バッファ
    fn copy_pixels(&self, rect: Option<&WICRect>, stride: u32, buffer: &mut [u8]) -> Result<()>;
}

impl WICBitmapSourceExt for IWICBitmapSource {
    #[inline(always)]
    fn get_size(&self) -> Result<(u32, u32)> {
        let mut width = 0u32;
        let mut height = 0u32;
        unsafe {
            self.GetSize(&mut width, &mut height)?;
        }
        Ok((width, height))
    }

    #[inline(always)]
    fn copy_pixels(&self, rect: Option<&WICRect>, stride: u32, buffer: &mut [u8]) -> Result<()> {
        let rect_ptr = rect
            .map(|r| r as *const WICRect)
            .unwrap_or(std::ptr::null());
        unsafe { self.CopyPixels(rect_ptr, stride, buffer) }
    }
}
