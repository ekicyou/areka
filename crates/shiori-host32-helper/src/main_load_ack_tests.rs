use super::*;
use shiori_proxy::ProxyError;

#[test]
fn ok_maps_to_ack_one() {
    let r: Result<(), ProxyError> = Ok(());
    assert_eq!(load_result_to_ack(&r), LOAD_ACK_OK);
    assert_eq!(LOAD_ACK_OK, 1);
}

#[test]
fn every_proxy_error_maps_to_ack_zero() {
    let errors: [ProxyError; 3] = [
        ProxyError::EntryNotFound("load"),
        ProxyError::EncodingFailed,
        // last_error の値に依らず ack[0]。
        ProxyError::LoadReturnedFalse { last_error: 5 },
    ];
    for e in errors {
        let r: Result<(), ProxyError> = Err(e);
        assert_eq!(load_result_to_ack(&r), LOAD_ACK_FAIL);
    }
    assert_eq!(LOAD_ACK_FAIL, 0);
}
