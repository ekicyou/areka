//! `wic` の非公開の判定の失敗の道（要件 4.2）。パレットを持たない絵で
//! `palette_has_alpha` が推測の既定値へ落ちず WIC のエラーを返すことを確かめる。

use super::*;

#[test]
fn palette_has_alpha_fails_without_palette() {
    // MTA は作らない。同じバイナリの com::wuc の ASTA の Compositor のテストと並ぶと落ちるため、
    // このスレッドだけの STA で WIC を使う（WIC はインプロセスでどのアパートメントでも動く）。
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets/test_8x8_rgba.png");
    let factory = wic_factory().unwrap();
    let frame = factory
        .create_decoder_from_filename(
            &HSTRING::from(path.to_string_lossy().as_ref()),
            None,
            GENERIC_READ,
            WICDecodeMetadataCacheOnDemand,
        )
        .unwrap()
        .frame(0)
        .unwrap();
    let err = palette_has_alpha(&factory, &frame.cast().unwrap()).unwrap_err();
    assert_eq!(err.code(), WINCODEC_ERR_PALETTEUNAVAILABLE);
    drop((err, frame, factory));
    unsafe { CoUninitialize() };
}

/// 15 番: 番号つきでない形式の α の有無は WIC の形式の情報で決まる。
/// 手書きの 12 の一覧に無かった α つきの形式（5551・RGBAHalf・CMYKAlpha）も「有り」と答える。
#[test]
fn pixel_format_has_alpha_asks_wic() {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };
    let factory = wic_factory().unwrap();
    let with_alpha = [
        ("32bppBGRA", GUID_WICPixelFormat32bppBGRA),
        ("32bppPBGRA", GUID_WICPixelFormat32bppPBGRA),
        ("32bppRGBA", GUID_WICPixelFormat32bppRGBA),
        ("32bppPRGBA", GUID_WICPixelFormat32bppPRGBA),
        ("64bppRGBA", GUID_WICPixelFormat64bppRGBA),
        ("64bppPRGBA", GUID_WICPixelFormat64bppPRGBA),
        ("64bppBGRA", GUID_WICPixelFormat64bppBGRA),
        ("64bppPBGRA", GUID_WICPixelFormat64bppPBGRA),
        ("32bppRGBA1010102", GUID_WICPixelFormat32bppRGBA1010102),
        ("32bppRGBA1010102XR", GUID_WICPixelFormat32bppRGBA1010102XR),
        ("128bppRGBAFloat", GUID_WICPixelFormat128bppRGBAFloat),
        ("128bppPRGBAFloat", GUID_WICPixelFormat128bppPRGBAFloat),
        ("16bppBGRA5551", GUID_WICPixelFormat16bppBGRA5551),
        ("64bppRGBAHalf", GUID_WICPixelFormat64bppRGBAHalf),
        ("40bppCMYKAlpha", GUID_WICPixelFormat40bppCMYKAlpha),
    ];
    let without_alpha = [
        ("24bppBGR", GUID_WICPixelFormat24bppBGR),
        ("32bppBGR", GUID_WICPixelFormat32bppBGR),
        ("8bppGray", GUID_WICPixelFormat8bppGray),
        ("48bppRGB", GUID_WICPixelFormat48bppRGB),
    ];
    for (name, format) in with_alpha {
        assert!(pixel_format_has_alpha(&factory, &format).unwrap(), "{name}");
    }
    for (name, format) in without_alpha {
        assert!(
            !pixel_format_has_alpha(&factory, &format).unwrap(),
            "{name}"
        );
    }
    // 形式の情報が取れない GUID は推測で答えず WIC のエラーを返す。
    assert!(pixel_format_has_alpha(&factory, &GUID::zeroed()).is_err());
    drop(factory);
    unsafe { CoUninitialize() };
}

/// 125 番: 対のないサロゲートを含む名のファイルも開ける（名を UTF-8 へ化かさず UTF-16 のまま渡す）。
/// 置き場はワークツリーの `target` の下。
#[test]
fn load_bitmap_source_opens_name_with_lone_surrogate() {
    use std::os::windows::ffi::OsStringExt;
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/wic_lone_surrogate_test");
    std::fs::create_dir_all(&dir).unwrap();
    let name: Vec<u16> = [0xD800].into_iter().chain("a.png".encode_utf16()).collect();
    let path = dir.join(std::ffi::OsString::from_wide(&name));
    let asset = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets/test_8x8_rgba.png");
    std::fs::copy(asset, &path).unwrap();

    let factory = wic_factory().unwrap();
    let loaded = load_bitmap_source(&factory, &path);
    let _ = std::fs::remove_file(&path);
    assert!(loaded.is_ok(), "{:?}", loaded.err());
    drop((loaded, factory));
    unsafe { CoUninitialize() };
}
