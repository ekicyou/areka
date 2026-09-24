//! `winhttp.rs` の純粋な判定だけを固定する（OS もネットも呼ばない）。

use super::*;
use windows::core::HRESULT;

#[test]
fn win32_table_maps_every_row() {
    assert_eq!(from_win32(12007), FetchError::NameResolution);
    assert_eq!(from_win32(12029), FetchError::Connect);
    assert_eq!(from_win32(12030), FetchError::Connect);
    assert_eq!(from_win32(12002), FetchError::Timeout);
    assert_eq!(from_win32(12175), FetchError::Tls);
    assert_eq!(from_win32(12188), FetchError::Tls);
}

#[test]
fn win32_table_falls_through_to_other_with_the_code() {
    // https → http の降格拒否（ERROR_WINHTTP_REDIRECT_FAILED）はここに落ちる。
    assert_eq!(from_win32(12156), FetchError::Other { code: 12156 });
    assert_eq!(from_win32(0), FetchError::Other { code: 0 });
}

#[test]
fn os_error_is_mapped_by_its_low_16_bits() {
    let e = Error::from_hresult(HRESULT::from_win32(12007));
    assert_eq!(from_os(&e), FetchError::NameResolution);
    let e = Error::from_hresult(HRESULT::from_win32(12156));
    assert_eq!(from_os(&e), FetchError::Other { code: 12156 });
}

#[test]
fn status_2xx_is_ok() {
    assert_eq!(status_to_result(200), Ok(()));
    assert_eq!(status_to_result(299), Ok(()));
}

#[test]
fn status_404_is_not_found() {
    assert_eq!(status_to_result(404), Err(FetchError::NotFound));
}

#[test]
fn status_other_carries_the_code() {
    assert_eq!(status_to_result(300), Err(FetchError::Status { code: 300 }));
    assert_eq!(status_to_result(403), Err(FetchError::Status { code: 403 }));
    assert_eq!(status_to_result(500), Err(FetchError::Status { code: 500 }));
    assert_eq!(status_to_result(199), Err(FetchError::Status { code: 199 }));
}

#[test]
fn status_beyond_u16_saturates() {
    assert_eq!(
        status_to_result(65_536),
        Err(FetchError::Status { code: u16::MAX })
    );
}

#[test]
fn empty_url_is_invalid_before_any_os_call() {
    assert_eq!(check_url(&[]), Err(FetchError::Other { code: 12005 }));
    assert_eq!(check_url(&[u16::from(b'h')]), Ok(()));
}
