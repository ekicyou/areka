//! `client` の 2.x の結線の兄弟のテスト（shiori2-protocol 要件 3.3・3.5・3.6・4.1・4.5・7.2）。
//!
//! 2.x の前半と後半（`encode_for`・`parse_for` の 2.x の腕）は `get_with_headers`・
//! `notify_with_headers` が実際に呼ぶ本番の関数そのもので、窓なしで結線を確かめられる。

use super::*;

use crate::charset::Charset;

/// 結線が組み立てる要求と同形の [`ShioriRequest`]（`sender` は既定・References なし）。
fn a_request(method: Method, id: &str) -> ShioriRequest<'_> {
    ShioriRequest {
        method,
        id,
        references: &[],
        headers: RequestHeaders::internal(DEFAULT_SENDER),
        status: None,
        charset: Charset::UTF_8,
    }
}

/// 2.x の答えを後半に通し、3.0 と同じ値の写像へ戻す。
fn reply2(resp: &[u8]) -> Result<Option<ValueReply>, RequestError> {
    let mut negotiator = CharsetNegotiator::new(Charset::UTF_8, false);
    let req = a_request(Method::Get, "OnBoot");
    map_get_reply(parse_for(
        ShioriVersion::Shiori2,
        &mut negotiator,
        &req,
        resp,
        false,
    )?)
}

/// 3.0 の答えを今の後半に通し、値の写像へ渡す。
fn reply3(resp: &[u8]) -> Result<Option<ValueReply>, RequestError> {
    let mut negotiator = CharsetNegotiator::new(Charset::UTF_8, false);
    let req = a_request(Method::Get, "OnBoot");
    map_get_reply(parse_for(
        ShioriVersion::Shiori3,
        &mut negotiator,
        &req,
        resp,
        false,
    )?)
}

/// 2.x の `Sentence` 付き 200 は 3.0 の `Value` 付き 200 と同じ値の答えになる（要件 4.1・7.2）。
#[test]
fn a_2x_sentence_200_is_the_same_answer_as_a_3_0_value_200() {
    let two = reply2(b"SHIORI/2.6 200 OK\r\nSentence: hi\r\n\r\n").expect("2.x の 200 は Ok");
    let three = reply3(b"SHIORI/3.0 200 OK\r\nValue: hi\r\n\r\n").expect("3.0 の 200 は Ok");
    assert_eq!(two, three);
    assert_eq!(two.map(|r| r.value).as_deref(), Some("hi"));
}

/// 2.x の 400 は 3.0 の 400 と同じ状態の誤りになる（要件 4.5・7.2）。
#[test]
fn a_2x_400_is_the_same_status_error_as_a_3_0_400() {
    let two = reply2(b"SHIORI/2.6 400 Bad Request\r\n\r\n").expect_err("2.x の 400 は誤り");
    let three = reply3(b"SHIORI/3.0 400 Bad Request\r\n\r\n").expect_err("3.0 の 400 は誤り");
    assert!(matches!(
        two,
        RequestError::Shiori(ShioriError::Status { status: 400, .. })
    ));
    assert_eq!(format!("{two:?}"), format!("{three:?}"));
}

/// 2.x の GET の答えの `Charset` は今の規則で採用される（要件 3.6）。
#[test]
fn a_2x_get_reply_charset_is_adopted() {
    let euc_jp = Charset::for_label("euc-jp").expect("EUC-JP は解決できる");
    let mut resp = b"SHIORI/2.6 200 OK\r\nCharset: EUC-JP\r\nSentence: ".to_vec();
    resp.extend_from_slice(&[0xA4, 0xA2]); // 「あ」の EUC-JP
    resp.extend_from_slice(b"\r\n\r\n");

    let mut negotiator = CharsetNegotiator::new(Charset::UTF_8, false);
    let req = a_request(Method::Get, "OnBoot");
    let parsed =
        parse_for(ShioriVersion::Shiori2, &mut negotiator, &req, &resp, false).expect("解析できる");
    assert_eq!(parsed.value.as_deref(), Some("あ"));
    assert_eq!(negotiator.current(), euc_jp);
}

/// 資源の印は前半まで届き、2.x の `GET String` で送られる（要件 3.3）。
#[test]
fn the_resource_mark_reaches_the_2x_request() {
    let mut negotiator = CharsetNegotiator::new(Charset::UTF_8, false);
    let bytes = encode_for(
        ShioriVersion::Shiori2,
        &mut negotiator,
        &a_request(Method::Get, "username"),
        true,
    )
    .expect("資源の問い合わせは送る");
    assert!(bytes.starts_with(b"GET String SHIORI/2.5\r\n"));
}

/// 送らないイベントは送らず、DEBUG（宛先 `shiori2`）がちょうど 1 行出る（要件 3.5）。
#[test]
fn an_event_not_sent_to_2x_logs_exactly_one_debug_line() {
    let mut negotiator = CharsetNegotiator::new(Charset::UTF_8, false);
    let req = a_request(Method::Notify, "basewareversion");
    let (bytes, events) = log_capture_kit::capture(|| {
        encode_for(ShioriVersion::Shiori2, &mut negotiator, &req, false)
    });
    assert_eq!(bytes, None, "2.x の相手へ送ってはならない");
    let lines: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("shiori2_event_not_sent"))
        .collect();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].target, "shiori2");
    assert_eq!(lines[0].level, tracing::Level::DEBUG);
    assert_eq!(lines[0].field_str("id"), Some("basewareversion"));
}

/// 確かめの往復を窓なしで回す: 今の文字コードで要求を組み、送受の結果を判定の半分へ渡す。
/// 答えを採用の根拠にしないこと（要件 6.1）は [`the_probe_body_contains_no_adoption_path`] が縛る。
fn probe_with(result: Result<Vec<u8>, SendError>) -> VersionVerdict {
    let negotiator = CharsetNegotiator::new(Charset::UTF_8, false);
    let req = shiori2::version_probe_request(negotiator.current(), DEFAULT_SENDER);
    assert!(req.starts_with(b"GET Version SHIORI/2.6\r\nCharset: UTF-8\r\n"));
    verdict_of_probe(result)
}

/// 確かめの本体が採用の経路を一切踏まないことを、`client.rs` の本文で機械的に確かめる
/// （要件 6.1・先例 `the_one_way_event_body_contains_no_adoption_path`）。
///
/// 呼ばれていないことは実行では観測できないため、本文そのものを判定材料にする。採用の
/// 経路を後から確かめの側へ繋ぐと赤になる。コメントも検査に入るので、確かめの側の説明
/// では採用の経路を名指ししない。
#[test]
fn the_probe_body_contains_no_adoption_path() {
    const SELF_SOURCE: &str = include_str!("client.rs");
    // 改行の綴り（CRLF／LF）に依存しないよう CR を落としてから切り出す。
    let source = SELF_SOURCE.replace('\r', "");
    let after = source
        .split_once("    pub fn probe_version(")
        .expect("probe_version の定義行が見つからない")
        .1;
    let body = after
        .split_once("\n    }\n")
        .expect("probe_version の閉じ括弧が見つからない")
        .0;
    assert!(
        body.contains("verdict_of_probe("),
        "確かめは判定の半分を通るべき"
    );
    for forbidden in ["parse_for", "note_response", "parse_response"] {
        assert!(
            !body.contains(forbidden),
            "確かめの本体に採用の経路 `{forbidden}` がある（要件 6.1 が破れている）"
        );
    }
}

/// 2.x の答え（`Charset` 付きでも）は 2.x に決まる（要件 2.1・6.1）。
#[test]
fn a_2x_probe_reply_decides_shiori2() {
    let verdict = probe_with(Ok(
        b"SHIORI/2.6 200 OK\r\nCharset: Shift_JIS\r\nString: x\r\n\r\n".to_vec(),
    ));
    assert_eq!(
        verdict,
        VersionVerdict::Decided {
            version: ShioriVersion::Shiori2,
            evidence: "SHIORI/2.6 200 OK".to_string(),
        }
    );
}

/// 3.0 の答え（204）は 3.0 に決まる（要件 2.1）。
#[test]
fn a_3_0_probe_reply_decides_shiori3() {
    let verdict = probe_with(Ok(
        b"SHIORI/3.0 204 No Content\r\nCharset: UTF-8\r\n\r\n".to_vec()
    ));
    assert_eq!(
        verdict,
        VersionVerdict::Decided {
            version: ShioriVersion::Shiori3,
            evidence: "SHIORI/3.0 204 No Content".to_string(),
        }
    );
}

/// 送りの失敗は panic せず「決められない」になり、理由は失敗の表示（要件 2.5）。
#[test]
fn a_failed_probe_send_is_undecided_with_the_error_display() {
    let err = SendError::Ipc(shiori_host32_ipc::IpcError::Timeout);
    let reason = err.to_string();
    assert_eq!(probe_with(Err(err)), VersionVerdict::Undecided { reason });
}
