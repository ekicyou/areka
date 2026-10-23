//! 同じプロセスの経路の要求の組み立てと応答のレベル（areka-P0-script-security-level 要件 2.8・3.1）。

use super::*;
use shiori_host32_host::{ValueReply, request_of};

/// `S_OK`（即時応答）の HRESULT。
const S_OK: HRESULT = HRESULT(0);

/// 同じヘッダから同じプロセスの経路が組んだ入力は、共通の手（UTF-8）で組んだ要求の文と
/// バイト列で一致する（GET・NOTIFY・`Status` の有無の 3 通り・要件 2.8）。
#[test]
fn build_input_equals_request_of_bytes() {
    let headers = RequestHeaders {
        security_level: "external",
        security_origin: "null",
        sender: "MCP",
        sender_type: "external,sstp",
    };
    let references = ["0".to_owned(), "あ".to_owned()];
    for (method, status) in [
        (Method::Get, Some("talking")),
        (Method::Get, None),
        (Method::Notify, None),
    ] {
        let expected = build_request(&request_of(
            method,
            "OnTest",
            &references,
            status,
            headers,
            Charset::UTF_8,
        ))
        .bytes;
        let input = build_input(method, "OnTest", &references, status, headers);
        assert_eq!(input.to_string().into_bytes(), expected);
    }
}

/// 結果の写しは応答の `SecurityLevel` の生の値（前後の空白を落とした値）を返す（要件 3.1）。
#[test]
fn map_get_outcome_carries_the_raw_level() {
    let bytes =
        b"SHIORI/3.0 200 OK\r\nCharset: UTF-8\r\nSecurityLevel:  External \r\nValue: hi\r\n\r\n";
    assert_eq!(
        map_get_outcome(S_OK, bytes).expect("200+Value は Ok"),
        Some(ValueReply {
            value: "hi".to_owned(),
            security_level: Some("External".to_owned()),
        })
    );
}

/// 応答に `SecurityLevel` が無ければ生の値も無し（要件 3.1）。
#[test]
fn map_get_outcome_without_level_is_none_level() {
    let bytes = b"SHIORI/3.0 200 OK\r\nCharset: UTF-8\r\nValue: hi\r\n\r\n";
    assert_eq!(
        map_get_outcome(S_OK, bytes).expect("200+Value は Ok"),
        Some(ValueReply {
            value: "hi".to_owned(),
            security_level: None,
        })
    );
}
