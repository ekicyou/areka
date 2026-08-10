use super::*;
use shiori_proxy::ProxyError;

// 要件 5.1/6.4: 新規確立成功（Ok）→ ack[1]。
#[test]
fn ok_maps_to_ack_one() {
    let r: Result<(), ProxyError> = Ok(());
    assert_eq!(load_result_to_ack(&r), LOAD_ACK_OK);
    assert_eq!(LOAD_ACK_OK, 1);
}

// 要件 6.1/6.4: あらゆる ProxyError → ack[0]（DLL 不在・エクスポート欠落・load→false 等）。
#[test]
fn every_proxy_error_maps_to_ack_zero() {
    let errors: [ProxyError; 3] = [
        ProxyError::EntryNotFound("load"),
        ProxyError::EncodingFailed,
        ProxyError::LoadReturnedFalse,
    ];
    for e in errors {
        let r: Result<(), ProxyError> = Err(e);
        assert_eq!(load_result_to_ack(&r), LOAD_ACK_FAIL);
    }
    assert_eq!(LOAD_ACK_FAIL, 0);
}
