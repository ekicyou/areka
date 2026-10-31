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
