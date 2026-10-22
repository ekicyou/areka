//! 追加のヘッダの入れ物と要求の文の並び（areka-P0-script-security-level 要件 2.1・2.9）。

use super::RequestHeaders;
use crate::charset::Charset;
use crate::shiori3::{Method, ShioriRequest, build_request};

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
