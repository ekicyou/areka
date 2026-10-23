//! `client` の 2.x の結線の兄弟のテスト（shiori2-protocol 要件 3.3・3.5・3.6・4.1・4.5・7.2）。
//!
//! 2.x の前半（`encode2_and_note`）と後半（`parse2_and_note`）は `get_with_headers`・
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
    map_get_reply(parse2_and_note(&mut negotiator, &req, resp, false)?)
}

/// 3.0 の答えを今の後半に通し、値の写像へ渡す。
fn reply3(resp: &[u8]) -> Result<Option<ValueReply>, RequestError> {
    let mut negotiator = CharsetNegotiator::new(Charset::UTF_8, false);
    map_get_reply(parse_and_note(&mut negotiator, resp)?)
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
    let parsed = parse2_and_note(&mut negotiator, &req, &resp, false).expect("解析できる");
    assert_eq!(parsed.value.as_deref(), Some("あ"));
    assert_eq!(negotiator.current(), euc_jp);
}

/// 資源の印は前半まで届き、2.x の `GET String` で送られる（要件 3.3）。
#[test]
fn the_resource_mark_reaches_the_2x_request() {
    let mut negotiator = CharsetNegotiator::new(Charset::UTF_8, false);
    let bytes = encode2_and_note(&mut negotiator, &a_request(Method::Get, "username"), true)
        .expect("資源の問い合わせは送る");
    assert!(bytes.starts_with(b"GET String SHIORI/2.5\r\n"));
}

/// 送らないイベントは送らず、DEBUG（宛先 `shiori2`）がちょうど 1 行出る（要件 3.5）。
#[test]
fn an_event_not_sent_to_2x_logs_exactly_one_debug_line() {
    let mut negotiator = CharsetNegotiator::new(Charset::UTF_8, false);
    let req = a_request(Method::Notify, "basewareversion");
    let (bytes, events) =
        log_capture_kit::capture(|| encode2_and_note(&mut negotiator, &req, false));
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
