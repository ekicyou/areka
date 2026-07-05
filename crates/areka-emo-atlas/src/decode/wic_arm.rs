//! 既定デコード腕＝WIC（COM 隔離・`windows` の WIC 型を直接利用）。
//!
//! 設計決定 **D4**（要件 **R2.1, R2.2**）。
//!
//! `ElementDecoder` の既定実装。`windows` クレートの WIC 経路
//! （`CreateDecoderFromFilename → GetFrame(0) → IWICFormatConverter で
//! GUID_WICPixelFormat32bppPBGRA へ変換 → CopyPixels`）で PBGRA raw バッファを
//! 抽出する（wintf の `load_bitmap_source` 相当・本層は wintf 本体へ依存せず
//! `windows` を直接利用）。COM 依存は本モジュールに隔離し、MTA/COM 規律に従う。
//! 変換前フレームのピクセルフォーマット由来の α 有無を `DecodedImage` へ確定する。

use std::path::Path;

use windows::Win32::Foundation::GENERIC_READ;
// IWICImagingFactory2 は D2D サブモジュール経由で露出する（wintf の import 経路に倣う）。
use windows::Win32::Graphics::Imaging::D2D::IWICImagingFactory2;
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat128bppPRGBAFloat, GUID_WICPixelFormat128bppRGBAFloat,
    GUID_WICPixelFormat32bppBGRA, GUID_WICPixelFormat32bppPBGRA, GUID_WICPixelFormat32bppPRGBA,
    GUID_WICPixelFormat32bppRGBA, GUID_WICPixelFormat32bppRGBA1010102,
    GUID_WICPixelFormat32bppRGBA1010102XR, GUID_WICPixelFormat64bppBGRA,
    GUID_WICPixelFormat64bppPBGRA, GUID_WICPixelFormat64bppPRGBA, GUID_WICPixelFormat64bppRGBA,
    IWICBitmapSource, WICBitmapDitherTypeNone, WICBitmapPaletteTypeMedianCut,
    WICDecodeMetadataCacheOnDemand,
};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::core::{GUID, HSTRING, Interface};

use crate::decode::{DecodeError, DecodedImage, ElementDecoder};

/// 既定デコード腕（WIC 経由）。COM/unsafe をこの腕へ隔離する（D4）。
///
/// COM は呼び出しスレッドで**呼び出し側が**初期化済みであること（`CoInitializeEx`）が前提。
/// factory は構築時に一度生成して保持する。
pub struct WicDecoderArm {
    factory: IWICImagingFactory2,
}

impl WicDecoderArm {
    /// WIC ファクトリを生成して腕を組む。
    ///
    /// COM は呼び出しスレッドで初期化済みでなければならない（未初期化なら失敗）。
    /// ファクトリ生成失敗は COM/環境エラーゆえ `windows::core::Result` で返す
    /// （デコード個別の `DecodeError` とは別レイヤ）。
    pub fn new() -> windows::core::Result<Self> {
        // wintf::com::wic::wic_factory() 相当（emo-atlas は境界制約で wintf を import しない）。
        let factory: IWICImagingFactory2 =
            unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)? };
        Ok(Self { factory })
    }

    /// WIC 経路で PBGRA バッファと変換前 α 有無を抽出する（`windows` エラーをそのまま返す）。
    ///
    /// wintf `com/wic.rs::load_bitmap_source` の手順を raw COM で再現する。
    /// `has_alpha` は **変換前**フレームの `GetPixelFormat`（Critical Issue 1）から取る。
    fn decode_inner(&self, path: &Path) -> windows::core::Result<DecodedImage> {
        let path_hstring = HSTRING::from(path.to_string_lossy().as_ref());

        // デコーダー作成
        let decoder = unsafe {
            self.factory.CreateDecoderFromFilename(
                &path_hstring,
                None,
                GENERIC_READ,
                WICDecodeMetadataCacheOnDemand,
            )?
        };

        // 最初のフレーム
        let frame = unsafe { decoder.GetFrame(0)? };

        // 変換前フォーマットから α 有無を確定（Critical Issue 1）。
        // PBGRA 変換後は必ず α 付きになり判別不能になるため、変換前にここで取る。
        let pre_format = unsafe { frame.GetPixelFormat()? };
        let has_alpha = pixel_format_has_alpha(&pre_format);

        // PBGRA32 へ変換（α 無し画像も 100% 不透明として埋める）
        let converter = unsafe { self.factory.CreateFormatConverter()? };
        unsafe {
            converter.Initialize(
                &frame.cast::<IWICBitmapSource>()?,
                &GUID_WICPixelFormat32bppPBGRA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeMedianCut,
            )?;
        }
        let source: IWICBitmapSource = converter.cast()?;

        // 寸法取得
        let mut width = 0u32;
        let mut height = 0u32;
        unsafe { source.GetSize(&mut width, &mut height)? };

        let stride = width * 4;
        let mut bgra = vec![0u8; (stride * height) as usize];
        unsafe { source.CopyPixels(std::ptr::null(), stride, &mut bgra)? };

        Ok(DecodedImage { width, height, stride, bgra, has_alpha })
    }
}

impl ElementDecoder for WicDecoderArm {
    fn decode(&self, path: &Path) -> Result<DecodedImage, DecodeError> {
        // 不在は診断可能な NotFound として返す（2.2・panic しない）。
        if !path.exists() {
            return Err(DecodeError::NotFound { path: path.to_path_buf() });
        }
        // 在るが復号不能（破損等）は Decode として返す（2.2・panic しない）。
        self.decode_inner(path).map_err(|e| DecodeError::Decode {
            path: path.to_path_buf(),
            source: e.to_string(),
        })
    }

    fn probe_pna(&self, path: &Path) -> bool {
        // 同名 `.pna` の兄弟存在（emo2 は `.pna` を持たないため常に false）。
        path.with_extension("pna").exists()
    }
}

/// WIC ピクセルフォーマット GUID が α チャンネルを持つかを判定する。
///
/// wintf `com/wic.rs` の `has_alpha`（task 1.2）を再現する。emo-atlas は境界制約
/// （wintf は bevy_ecs 等を引き込む）ゆえ wintf を import せず同一集合をここに持つ。
/// windows-rs の GUID 定数は非 UPPER_CASE ゆえ `match`/`matches!` のパターンに置くと
/// `non_upper_case_globals` future-incompat 警告が出る。集合 `.contains()`（値比較）で回避する。
fn pixel_format_has_alpha(format: &GUID) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

    /// COM 初期化下でクロージャを実行するヘルパ（WIC は CPU-only だが COM init 必須）。
    /// wintf の既存 WIC テスト（MTA）と同じパターン。
    fn with_com_initialized<F: FnOnce()>(f: F) {
        unsafe {
            // 既に初期化済みでも RPC_E_CHANGED_MODE を許容し、続行する。
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        f();
        unsafe {
            CoUninitialize();
        }
    }

    /// emo2 fixture 実資産のパスを組む。
    /// `CARGO_MANIFEST_DIR` = `crates/areka-emo-atlas`。fixtures はパイロット crate 配下。
    fn emo2_path(rel: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../pilot/examples/shiori-host-32/fixtures/emo2")
            .join(rel)
    }

    /// 2.1: 実 emo2 element PNG（α 付き）を復号し、契約（寸法・stride・長さ・has_alpha）を確認。
    #[test]
    fn decode_real_emo2_png_satisfies_contract() {
        with_com_initialized(|| {
            let arm = WicDecoderArm::new().expect("WIC factory creates under COM init");
            let png = emo2_path("emo2-kakukaku/online0.png");
            assert!(png.exists(), "fixture must exist: {}", png.display());

            let img = arm.decode(&png).expect("real emo2 PNG decodes");
            assert!(img.width > 0 && img.height > 0, "non-empty dimensions");
            assert_eq!(img.stride, img.width * 4, "PBGRA stride == width*4");
            // Postcondition: bgra.len() == stride * height。
            assert_eq!(img.bgra.len(), (img.stride * img.height) as usize);
            // emo2 は use_self_alpha=1 かつ α 付き PNG（color type 6）→ has_alpha=true。
            assert!(img.has_alpha, "emo2 online0.png is RGBA → has_alpha");
        });
    }

    /// 2.2: 不在パスは NotFound（パス付き）で返す（panic しない）。
    #[test]
    fn missing_path_yields_not_found() {
        with_com_initialized(|| {
            let arm = WicDecoderArm::new().expect("WIC factory");
            let missing = emo2_path("does_not_exist_xyz.png");
            assert!(!missing.exists());
            match arm.decode(&missing) {
                Err(DecodeError::NotFound { path }) => assert_eq!(path, missing),
                other => panic!("expected NotFound, got {other:?}"),
            }
        });
    }

    /// 2.2: 実在するが画像でないファイルは Decode（パス＋source 付き）で返す（panic しない）。
    #[test]
    fn non_image_file_yields_decode_error() {
        with_com_initialized(|| {
            let arm = WicDecoderArm::new().expect("WIC factory");
            let not_image = emo2_path("emo2-kakukaku/descript.txt");
            assert!(not_image.exists(), "fixture must exist: {}", not_image.display());
            match arm.decode(&not_image) {
                Err(DecodeError::Decode { path, source }) => {
                    assert_eq!(path, not_image);
                    assert!(!source.is_empty(), "decode error carries a source string");
                }
                other => panic!("expected Decode, got {other:?}"),
            }
        });
    }

    /// D5/emo2: emo2 の PNG には `.pna` 兄弟が無いため probe_pna は false。
    ///
    /// 注意: 他の WIC テストと同じく `with_com_initialized` で包む。以前の実装は
    /// 「未初期化のまま `new()` → 失敗したら空 `with_com_initialized(||{})` で
    /// init+uninit（差引ゼロ）してから再 `new()`」という壊れたフォールバックを持ち、
    /// 暗黙 MTA（他テストが同時に MTA を保持）に依存していた。単一スレッド実行や
    /// ワークスペース一括実行では暗黙 MTA が無く CoCreateInstance が
    /// CO_E_NOTINITIALIZED で失敗し、フレーキーだった。COM を張った状態で
    /// factory 生成・使用まで完結させることで実行順・スレッド数に非依存にする。
    #[test]
    fn probe_pna_false_for_emo2() {
        with_com_initialized(|| {
            let arm = WicDecoderArm::new().expect("WIC factory");
            let png = emo2_path("emo2-kakukaku/online0.png");
            assert!(png.exists());
            // `online0.pna` は存在しない。
            assert!(!emo2_path("emo2-kakukaku/online0.pna").exists());
            assert!(!arm.probe_pna(&png), "emo2 has no sibling .pna");
        });
    }

    /// 2.1（抽象境界）: `&dyn ElementDecoder` 経由でポート契約を満たすことを確認する。
    #[test]
    fn satisfies_port_via_trait_object() {
        with_com_initialized(|| {
            let arm = WicDecoderArm::new().expect("WIC factory");
            let dyn_dec: &dyn ElementDecoder = &arm;
            let png = emo2_path("emo2-kakukaku/online0.png");
            assert!(png.exists());

            let _ = dyn_dec.probe_pna(&png);
            let img = dyn_dec.decode(&png).expect("decodes via dyn port");
            assert_eq!(img.bgra.len(), (img.stride * img.height) as usize);
            assert!(img.has_alpha);
        });
    }
}
