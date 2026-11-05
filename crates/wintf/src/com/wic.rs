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
/// `(PBGRA32 に変換した IWICBitmapSource, 変換前フレームの α 有無)`。
/// α 有無は番号つきの形式ならパレットの透明度から、それ以外は WIC の形式の情報から取る。
/// パレットや形式の情報を読めないときは WIC のエラーを返す。
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
    // 番号つきの形式は形式の GUID に α が出ないので、パレットの透明度で決める。
    let has_alpha = if INDEXED_FORMATS.contains(&pre_format) {
        palette_has_alpha(factory, &frame.cast()?)?
    } else {
        pixel_format_has_alpha(factory, &pre_format)?
    };

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

/// 番号つきの形式（1・2・4・8 ビット）。この集合に入る形式だけパレットを読む。
const INDEXED_FORMATS: &[GUID] = &[
    GUID_WICPixelFormat1bppIndexed,
    GUID_WICPixelFormat2bppIndexed,
    GUID_WICPixelFormat4bppIndexed,
    GUID_WICPixelFormat8bppIndexed,
];

/// フレームのパレットに透明度 255 未満の色が 1 つでも在るか（`IWICPalette::HasAlpha`）。
/// `load_bitmap_source` が番号つきの形式のときに使う。パレットを持たない絵では
/// WIC のエラー（`WINCODEC_ERR_PALETTEUNAVAILABLE`）をそのまま返す。
fn palette_has_alpha(factory: &IWICImagingFactory2, source: &IWICBitmapSource) -> Result<bool> {
    unsafe {
        let palette = factory.CreatePalette()?;
        source.CopyPalette(&palette)?;
        Ok(palette.HasAlpha()?.as_bool())
    }
}

/// 番号つきでない形式が α を持つかを、WIC の形式の情報（`IWICPixelFormatInfo2::SupportsTransparency`）に聞く。
/// 形式の情報が取れないときは WIC のエラーを返す。番号つきの形式は TRUE を返さないので `palette_has_alpha` で決める。
fn pixel_format_has_alpha(factory: &IWICImagingFactory2, format: &GUID) -> Result<bool> {
    unsafe {
        let info: IWICPixelFormatInfo2 = factory.CreateComponentInfo(format)?.cast()?;
        Ok(info.SupportsTransparency()?.as_bool())
    }
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

#[cfg(test)]
#[path = "wic_tests.rs"]
mod tests;
