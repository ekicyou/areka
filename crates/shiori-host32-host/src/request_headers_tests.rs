//! 追加のヘッダの入れ物と要求の文の並び（areka-P0-script-security-level 要件 2.1・2.9）。

use super::{RequestHeaders, request_of};
use crate::charset::{Charset, CharsetPolicy};
use crate::shiori3::{Method, ParsedResponse, ShioriRequest, build_request, parse_response};

/// 既定の作り方は今の値（`local`・`null`・送り手の名前・`internal`）。
#[test]
fn internal_is_the_current_default() {
    assert_eq!(
        RequestHeaders::internal("areka"),
        RequestHeaders {
            security_level: "local",
            security_origin: "null",
            sender: "areka",
            sender_type: "internal",
        }
    );
}

fn build(status: Option<&str>, headers: RequestHeaders<'_>) -> String {
    let references = ["0".to_owned()];
    let bytes = build_request(&ShioriRequest {
        method: Method::Get,
        id: "OnSecondChange",
        references: &references,
        headers,
        status,
        charset: Charset::UTF_8,
    })
    .bytes;
    String::from_utf8(bytes).expect("UTF-8")
}

/// 並びは「要求行・Charset・SecurityLevel・SecurityOrigin・Sender・SenderType・Status・ID・
/// Reference*」。末尾の `SecurityLevel: local` は無い（要件 2.1）。
#[test]
fn headers_precede_status_and_id() {
    let headers = RequestHeaders {
        security_level: "external",
        security_origin: "null",
        sender: "MCP",
        sender_type: "external,sstp",
    };
    assert_eq!(
        build(Some("talking"), headers),
        "GET SHIORI/3.0\r\n\
         Charset: UTF-8\r\n\
         SecurityLevel: external\r\n\
         SecurityOrigin: null\r\n\
         Sender: MCP\r\n\
         SenderType: external,sstp\r\n\
         Status: talking\r\n\
         ID: OnSecondChange\r\n\
         Reference0: 0\r\n\
         \r\n"
    );
}

/// `Status` が無くても `SecurityLevel` は `ID` より前に 1 行だけ出る（要件 2.1）。
#[test]
fn security_level_precedes_id_without_status() {
    let s = build(None, RequestHeaders::internal("areka"));
    assert_eq!(
        s,
        "GET SHIORI/3.0\r\n\
         Charset: UTF-8\r\n\
         SecurityLevel: local\r\n\
         SecurityOrigin: null\r\n\
         Sender: areka\r\n\
         SenderType: internal\r\n\
         ID: OnSecondChange\r\n\
         Reference0: 0\r\n\
         \r\n"
    );
    assert_eq!(s.matches("SecurityLevel:").count(), 1, "{s}");
}

/// `Status` が在っても `SecurityLevel` は `ID` より前・1 行だけ（要件 2.1）。
#[test]
fn security_level_precedes_id_with_status() {
    let s = build(Some("talking"), RequestHeaders::internal("areka"));
    let level = s.find("SecurityLevel:").expect("SecurityLevel present");
    let status = s.find("Status:").expect("Status present");
    let id = s.find("ID:").expect("ID present");
    assert!(level < status && status < id, "{s}");
    assert_eq!(s.matches("SecurityLevel:").count(), 1, "{s}");
}

/// 共通の組み立ての手から組んだ要求は、同じ値を手で並べた要求とバイト列まで同じ（要件 2.8）。
#[test]
fn request_of_builds_the_same_bytes_as_the_hand_built_request() {
    let references = ["0".to_owned(), "x".to_owned()];
    let headers = RequestHeaders {
        security_level: "external",
        security_origin: "null",
        sender: "MCP",
        sender_type: "external,sstp",
    };
    let via = build_request(&request_of(
        Method::Notify,
        "OnSecondChange",
        &references,
        Some("talking"),
        headers,
        Charset::UTF_8,
    ));
    let hand = build_request(&ShioriRequest {
        method: Method::Notify,
        id: "OnSecondChange",
        references: &references,
        headers,
        status: Some("talking"),
        charset: Charset::UTF_8,
    });
    assert_eq!(via, hand);
    assert_eq!(
        String::from_utf8(via.bytes).expect("UTF-8"),
        "NOTIFY SHIORI/3.0\r\n\
         Charset: UTF-8\r\n\
         SecurityLevel: external\r\n\
         SecurityOrigin: null\r\n\
         Sender: MCP\r\n\
         SenderType: external,sstp\r\n\
         Status: talking\r\n\
         ID: OnSecondChange\r\n\
         Reference0: 0\r\n\
         Reference1: x\r\n\
         \r\n"
    );
}

fn parse(resp: &str) -> ParsedResponse {
    parse_response(resp.as_bytes(), CharsetPolicy::Negotiate(Charset::UTF_8)).expect("well-formed")
}

/// 応答の `SecurityLevel` は頭の名前の大文字小文字を無視し、前後の空白を落とした生の値（要件 3.5）。
#[test]
fn response_security_level_is_the_trimmed_raw_value_case_insensitive() {
    for (line, expected) in [
        ("SecurityLevel: local", "local"),
        ("securitylevel:   External  ", "External"),
        ("SECURITYLEVEL:remote", "remote"),
    ] {
        let resp = format!("SHIORI/3.0 200 OK\r\nCharset: UTF-8\r\n{line}\r\nValue: hi\r\n\r\n");
        assert_eq!(
            parse(&resp).security_level.as_deref(),
            Some(expected),
            "{line}"
        );
    }
}

/// 応答に `SecurityLevel` が無いときは無し（要件 3.5）。
#[test]
fn response_without_security_level_is_none() {
    let resp = "SHIORI/3.0 200 OK\r\nCharset: UTF-8\r\nValue: hi\r\n\r\n";
    assert_eq!(parse(resp).security_level, None);
}

/// `SecurityLevel` が並んでも `Value`・`ErrorLevel`・`ErrorDescription` の読みは今のまま（要件 3.5）。
#[test]
fn security_level_does_not_change_value_or_error_level_reading() {
    let resp = "SHIORI/3.0 500 Internal Server Error\r\nCharset: UTF-8\r\n\
                SecurityLevel: external\r\nValue: \\s[0]hi\\e\r\n\
                ErrorLevel: critical\r\nErrorDescription: boom\r\n\r\n";
    let parsed = parse(resp);
    assert_eq!(parsed.status, 500);
    assert_eq!(parsed.value.as_deref(), Some(r"\s[0]hi\e"));
    assert_eq!(parsed.error_level.as_deref(), Some("critical"));
    assert_eq!(parsed.error_description.as_deref(), Some("boom"));
    assert_eq!(parsed.security_level.as_deref(), Some("external"));
}
