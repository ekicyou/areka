use super::*;
use shiori_host32_ipc::{FramingError, IpcError};

use crate::charset::Charset;

use crate::error::HandshakeError;

// --- map_send_error（THE core observable・要件 5.1〜5.4）-------------------

/// CRITICAL（要件 5.1/5.4）: `IpcError::Timeout` は `RequestError::Timeout` へ写り、
/// **`RequestError::Ipc` には含まれない**（timeout を単一不透明失敗へ潰さない）。
#[test]
fn map_send_error_timeout_routes_to_timeout_not_ipc() {
    let mapped = map_send_error(SendError::Ipc(IpcError::Timeout));
    assert!(
        matches!(mapped, RequestError::Timeout),
        "IpcError::Timeout は RequestError::Timeout へ（要件 5.1）: got {mapped:?}"
    );
    // Ipc へ流れていないことを明示（区別保持・要件 5.4）。
    assert!(
        !matches!(mapped, RequestError::Ipc(_)),
        "Timeout を Ipc へ潰してはならない（要件 5.4）: got {mapped:?}"
    );
}

/// `IpcError::SendFailed`（送出失敗＝helper 応答不能の一態様）は `RequestError::Ipc` へ（要件 5.3）。
#[test]
fn map_send_error_send_failed_routes_to_ipc() {
    let mapped = map_send_error(SendError::Ipc(IpcError::SendFailed));
    assert!(
        matches!(mapped, RequestError::Ipc(IpcError::SendFailed)),
        "SendFailed は RequestError::Ipc(SendFailed) へ（要件 5.3）: got {mapped:?}"
    );
}

/// `IpcError::CorruptFrame`（framing 破損）も transport 失敗として `RequestError::Ipc` へ（要件 5.3）。
#[test]
fn map_send_error_corrupt_frame_routes_to_ipc() {
    let mapped = map_send_error(SendError::Ipc(IpcError::CorruptFrame(
        FramingError::UnknownTag(0xFF),
    )));
    assert!(
        matches!(
            mapped,
            RequestError::Ipc(IpcError::CorruptFrame(FramingError::UnknownTag(0xFF)))
        ),
        "CorruptFrame は RequestError::Ipc へ（要件 5.3）: got {mapped:?}"
    );
}

/// `SendError::Handshake(Incomplete)` は `RequestError::Handshake` へ（要件 3.3・区別保持）。
#[test]
fn map_send_error_handshake_incomplete_routes_to_handshake() {
    let mapped = map_send_error(SendError::Handshake(HandshakeError::Incomplete));
    assert!(
        matches!(mapped, RequestError::Handshake(HandshakeError::Incomplete)),
        "Handshake(Incomplete) は RequestError::Handshake へ: got {mapped:?}"
    );
}

/// `SendError::Handshake(Timeout)`（ハンドシェイク timeout）も `RequestError::Handshake` へ。
///
/// 注意: これは **ハンドシェイク** timeout（HandshakeError::Timeout）であり、**wire** timeout
/// （IpcError::Timeout→RequestError::Timeout）とは別語彙である（要件 5.4 の区別保持）。
#[test]
fn map_send_error_handshake_timeout_routes_to_handshake_not_wire_timeout() {
    let mapped = map_send_error(SendError::Handshake(HandshakeError::Timeout));
    assert!(
        matches!(mapped, RequestError::Handshake(HandshakeError::Timeout)),
        "Handshake(Timeout) は RequestError::Handshake へ: got {mapped:?}"
    );
    // wire timeout（RequestError::Timeout）へ混同していないこと。
    assert!(
        !matches!(mapped, RequestError::Timeout),
        "ハンドシェイク timeout を wire timeout へ混同しない（要件 5.4）: got {mapped:?}"
    );
}

// --- map_get_reply（areka-P0-script-security-level 要件 3.5）---------------

/// 200 ＋ Value は値と応答の `SecurityLevel` の生の値をそのまま運ぶ。
#[test]
fn map_get_reply_carries_the_raw_security_level() {
    let parsed = ParsedResponse {
        status: 200,
        value: Some("hi".to_string()),
        error_level: None,
        error_description: None,
        charset_header: None,
        decode_had_errors: false,
        security_level: Some("external".to_string()),
    };
    assert_eq!(
        map_get_reply(parsed).expect("200+value は Ok"),
        Some(ValueReply {
            value: "hi".to_string(),
            security_level: Some("external".to_string()),
        })
    );
}

/// 判定の順は今のまま: `SecurityLevel` があっても 400 は誤り・204 は無し（要件 3.5）。
#[test]
fn map_get_reply_keeps_the_judgment_order() {
    let with = |status: u16| ParsedResponse {
        status,
        value: Some("hi".to_string()),
        error_level: None,
        error_description: None,
        charset_header: None,
        decode_had_errors: false,
        security_level: Some("local".to_string()),
    };
    assert!(matches!(
        map_get_reply(with(400)),
        Err(RequestError::Shiori(ShioriError::Status {
            status: 400,
            ..
        }))
    ));
    assert_eq!(map_get_reply(with(204)).expect("204 は Ok"), None);
}

// --- map_get_result（要件 4.1/2.1/2.2/2.7/5.2）---------------------------

/// 200 ＋ Value → `Ok(Some(v))`（要件 2.1）。
#[test]
fn map_get_result_200_with_value_is_some() {
    let parsed = ParsedResponse {
        status: 200,
        value: Some(r"\s[0]hi\e".to_string()),
        error_level: None,
        error_description: None,
        charset_header: None,
        decode_had_errors: false,
        security_level: None,
    };
    // RequestError は PartialEq を持たない（error.rs は本タスクで不変）ため、Ok 側を取り出して比較する。
    let ok = map_get_result(parsed).expect("200+value は Ok");
    assert_eq!(ok, Some(r"\s[0]hi\e".to_string()));
}

/// 200 で Value 欠落 → `Ok(None)`（要件 2.1・Value は 200 でも欠落し得る）。
#[test]
fn map_get_result_200_without_value_is_none() {
    let parsed = ParsedResponse {
        status: 200,
        value: None,
        error_level: None,
        error_description: None,
        charset_header: None,
        decode_had_errors: false,
        security_level: None,
    };
    let ok = map_get_result(parsed).expect("200 (Value 欠落) は Ok");
    assert_eq!(ok, None);
}

/// 204 → `Ok(None)`（成功・スクリプト無し・要件 2.2）。
#[test]
fn map_get_result_204_is_none() {
    let parsed = ParsedResponse {
        status: 204,
        value: None,
        error_level: None,
        error_description: None,
        charset_header: None,
        decode_had_errors: false,
        security_level: None,
    };
    let ok = map_get_result(parsed).expect("204 は Ok");
    assert_eq!(ok, None);
}

/// 400 → `Err(RequestError::Shiori(ShioriError::Status{status:400,..}))`（要件 5.2）。
#[test]
fn map_get_result_400_is_shiori_status_error() {
    let parsed = ParsedResponse {
        status: 400,
        value: None,
        error_level: None,
        error_description: None,
        charset_header: None,
        decode_had_errors: false,
        security_level: None,
    };
    let err = map_get_result(parsed).expect_err("400 は SHIORI エラー");
    assert!(
        matches!(
            err,
            RequestError::Shiori(ShioriError::Status { status: 400, .. })
        ),
        "400 は Shiori(Status{{status:400}}) へ: got {err:?}"
    );
}

/// 500 → `Err(RequestError::Shiori(ShioriError::Status{status:500,..}))`（要件 5.2）。
#[test]
fn map_get_result_500_is_shiori_status_error() {
    let parsed = ParsedResponse {
        status: 500,
        value: None,
        error_level: Some("critical".to_string()),
        error_description: Some("boom".to_string()),
        charset_header: None,
        decode_had_errors: false,
        security_level: None,
    };
    let err = map_get_result(parsed).expect_err("500 は SHIORI エラー");
    assert!(
        matches!(
            err,
            RequestError::Shiori(ShioriError::Status {
                status: 500,
                error_level: Some(_),
                error_description: Some(_),
            })
        ),
        "500 は Shiori(Status{{status:500, error_level:Some, error_description:Some}}) へ: got {err:?}"
    );
}

/// 200 だが `ErrorLevel` 付き → エラー扱い（要件 5.2・error_level 先読み）。
#[test]
fn map_get_result_200_with_error_level_is_error() {
    let parsed = ParsedResponse {
        status: 200,
        value: Some("ignored".to_string()),
        error_level: Some("warning".to_string()),
        error_description: None,
        charset_header: None,
        decode_had_errors: false,
        security_level: None,
    };
    let err = map_get_result(parsed).expect_err("ErrorLevel 付きはエラー");
    assert!(
        matches!(
            err,
            RequestError::Shiori(ShioriError::Status {
                status: 200,
                error_level: Some(_),
                ..
            })
        ),
        "ErrorLevel 付きは status に関わらず Shiori エラー: got {err:?}"
    );
}

/// 311 → `Ok(value)`（OnTeach 系・drop せず区別可能・要件 2.7）。
#[test]
fn map_get_result_311_tolerated_returns_value() {
    let parsed = ParsedResponse {
        status: 311,
        value: Some("teach".to_string()),
        error_level: None,
        error_description: None,
        charset_header: None,
        decode_had_errors: false,
        security_level: None,
    };
    let ok = map_get_result(parsed).expect("311 は許容 status で Ok");
    assert_eq!(ok, Some("teach".to_string()));
}

/// 312 → `Ok(value)`（OnTeach 系・要件 2.7）。
#[test]
fn map_get_result_312_tolerated_returns_value() {
    let parsed = ParsedResponse {
        status: 312,
        value: None,
        error_level: None,
        error_description: None,
        charset_header: None,
        decode_had_errors: false,
        security_level: None,
    };
    let ok = map_get_result(parsed).expect("312 は許容 status で Ok");
    assert_eq!(ok, None);
}

// --- 構築（既定 sender・単一差替点）--------------------------------------

/// 既定 sender が `"areka"` であることを固定する（design.md §送出ヘッダ最小集合）。
#[test]
fn default_sender_is_areka() {
    assert_eq!(DEFAULT_SENDER, "areka");
}

// --- 交渉状態の結線（タスク 3.3・要件 5.1/5.3）----------------------------
//
// 1 往復の前半（`encode_for`）と後半（`parse_for`）は `get`／`notify` が
// 実際に呼ぶ本番関数そのものである。窓も 32bit 成果物も要さずに結線を検査できるのは、
// 交渉に関わる判断がこの 2 つに閉じているため（要件 9.8）。

/// 「あ」の EUC-JP バイト列（符号化器から導かない定数）。
const A_EUC_JP: &[u8] = &[0xA4, 0xA2];

/// `Charset: EUC-JP` を名乗り、Value に「あ」の EUC-JP バイト列を載せた 200 応答。
fn euc_jp_response() -> Vec<u8> {
    let mut bytes = b"SHIORI/3.0 200 OK\r\nCharset: EUC-JP\r\nValue: ".to_vec();
    bytes.extend_from_slice(A_EUC_JP);
    bytes.extend_from_slice(b"\r\n\r\n");
    bytes
}

/// `Charset: EUC-JP` を名乗る、Value を持たない応答（ステータス行だけを差し替えて使う）。
///
/// 200 以外の応答でも `Charset` ヘッダが採用対象であることを見るための固定物。
fn euc_jp_response_with_status(status_line: &str) -> Vec<u8> {
    let mut bytes = status_line.as_bytes().to_vec();
    bytes.extend_from_slice(b"\r\nCharset: EUC-JP\r\n\r\n");
    bytes
}

/// 結線が組み立てる要求と同形の [`ShioriRequest`]（`sender` は既定・References なし）。
fn a_request(method: Method, id: &str, charset: Charset) -> ShioriRequest<'_> {
    ShioriRequest {
        method,
        id,
        references: &[],
        headers: RequestHeaders::internal(DEFAULT_SENDER),
        status: None,
        charset,
    }
}

/// 応答の `Charset` ヘッダから採用が起きるのは **後半を通ったときだけ**であり、
/// 片道イベントが通る前半だけでは採用が起きない（要件 5.3 の零）。
#[test]
fn only_the_response_half_of_the_chain_adopts_a_charset() {
    let euc_jp = Charset::for_label("euc-jp").expect("EUC-JP は解決できる");

    // 応答待ちのイベントが通る後半: 解析と記録で EUC-JP を採用する。
    let mut answered = CharsetNegotiator::new(Charset::UTF_8, false);
    let parsed = parse3(&mut answered, &euc_jp_response()).expect("200 応答は解析できる");
    assert_eq!(
        parsed.value.as_deref(),
        Some("あ"),
        "宣言された文字コードで復号されていない"
    );
    assert_eq!(
        answered.current(),
        euc_jp,
        "応答の Charset ヘッダが採用されていない（要件 4.2）"
    );

    // 片道イベントが通る前半だけでは、同じ応答が来ても現在の文字コードは動かない。
    let mut one_way = CharsetNegotiator::new(Charset::UTF_8, false);
    let charset = one_way.current();
    let bytes = encode3(
        &mut one_way,
        &a_request(Method::Notify, "OnSecondChange", charset),
    );
    assert!(
        bytes.starts_with(b"NOTIFY SHIORI/3.0\r\nCharset: UTF-8\r\n"),
        "片道イベントの要求が現在の文字コードで出ていない"
    );
    assert_eq!(
        one_way.current(),
        Charset::UTF_8,
        "片道イベントの経路で採用が起きてはならない（要件 5.3）"
    );
}

/// 片道イベントの本体が採用の経路を一切踏まないことを、本ファイルの本文で機械的に
/// 確かめる（要件 5.3 の零＝「NOTIFY 応答からの採用 0」を件数で数え直す）。
///
/// 呼ばれていないことは実行では観測できない（呼び出しが無い＝出来事が無い）ため、
/// 本文そのものを判定材料にする。採用の経路を後から片道側へ繋ぐと赤になる。関数の本文は
/// コメントも含めて検査するので、片道側の説明では採用の経路を名指ししない（名指しすると
/// 赤になる——検査を緩めるのではなく説明の方を言い換えること）。
#[test]
fn the_one_way_event_body_contains_no_adoption_path() {
    const SELF_SOURCE: &str = include_str!("client.rs");
    // 改行の綴り（CRLF／LF）に依存しないよう CR を落としてから切り出す。
    let source = SELF_SOURCE.replace('\r', "");
    let after = source
        .split_once("    pub fn notify_with_headers(")
        .expect("notify の定義行が見つからない")
        .1;
    let body = after
        .split_once("\n    }\n")
        .expect("notify の閉じ括弧が見つからない")
        .0;
    assert!(
        body.contains("encode_for("),
        "片道イベントは前半（符号化と置換の記録）を通るべき（要件 3.5/5.1）"
    );
    for forbidden in ["parse_for", "note_response", "parse_response"] {
        assert!(
            !body.contains(forbidden),
            "片道イベントの本体に採用の経路 `{forbidden}` がある（要件 5.3 の零が破れている）"
        );
    }
}

/// 交渉状態は結線より長生きし、応答待ちのイベントで採用した文字コードが**次の片道
/// イベント**の要求バイト列に現れる（要件 5.1＝双方のイベントが同じ文字コードを使う）。
#[test]
fn adoption_in_an_answered_event_reaches_the_next_one_way_request() {
    let mut negotiator = CharsetNegotiator::new(Charset::UTF_8, false);

    let charset = negotiator.current();
    let first = encode3(&mut negotiator, &a_request(Method::Get, "OnBoot", charset));
    assert!(
        first.starts_with(b"GET SHIORI/3.0\r\nCharset: UTF-8\r\n"),
        "最初の要求は初期の文字コードで出る"
    );

    parse3(&mut negotiator, &euc_jp_response()).expect("200 応答は解析できる");

    let charset = negotiator.current();
    let second = encode3(
        &mut negotiator,
        &a_request(Method::Notify, "OnSecondChange", charset),
    );
    assert!(
        second.starts_with(b"NOTIFY SHIORI/3.0\r\nCharset: EUC-JP\r\n"),
        "採用した文字コードが次の片道イベントの要求に現れていない（要件 5.1）"
    );
}

/// **200 以外の応答でも** `Charset` ヘッダは採用対象である（要件 4.2）。
///
/// design.md §System Flows「交渉の一周（GET）」の流れの決め事 1 つめ——「`note_response` は
/// `map_get_result` より前。応答の status が 204・400・500 であっても `Charset` ヘッダは
/// 採用対象（4.2）。emo2 の最初の応答待ちイベント `username` 照会が 204 でも UTF-8 を
/// 採用する」——が本タスクの結線の契約そのものであり、その順序をここで押さえる。
///
/// `note_response` が status を引数に取らないのは構造上の備えであって検査ではない。後半を
/// 「200 のときだけ採用する」へ退化させると本テストが赤になる（204・400 の 2 系統）。
#[test]
fn a_non_200_response_still_adopts_its_charset_header() {
    let euc_jp = Charset::for_label("euc-jp").expect("EUC-JP は解決できる");

    // 204（スクリプト無しの成功）——emo2 の `username` 照会がこの形。
    let mut after_204 = CharsetNegotiator::new(Charset::UTF_8, false);
    let parsed = parse3(
        &mut after_204,
        &euc_jp_response_with_status("SHIORI/3.0 204 No Content"),
    )
    .expect("204 応答は解析できる");
    assert_eq!(
        parsed.status, 204,
        "固定物の status 行が 204 になっていない"
    );
    assert_eq!(
        after_204.current(),
        euc_jp,
        "204 応答の Charset ヘッダが採用されていない（要件 4.2・design §交渉の一周）"
    );

    // 400（SHIORI エラー応答）——結果の写しはエラーでも、採用は先に済んでいる。
    let mut after_400 = CharsetNegotiator::new(Charset::UTF_8, false);
    let parsed = parse3(
        &mut after_400,
        &euc_jp_response_with_status("SHIORI/3.0 400 Bad Request"),
    )
    .expect("400 応答も（SHIORI エラーではあるが）解析自体は成功する");
    assert_eq!(
        parsed.status, 400,
        "固定物の status 行が 400 になっていない"
    );
    assert_eq!(
        after_400.current(),
        euc_jp,
        "400 応答の Charset ヘッダが採用されていない（要件 4.2・design §交渉の一周）"
    );
    // 結果の写しは 400 をエラーにする（採用が起きたことと両立する・要件 5.2）。
    assert!(
        map_get_result(parsed).is_err(),
        "400 は SHIORI エラーとして写るべき（採用の有無とは別の主張）"
    );
}

/// 復号で代替文字への置換が起きたという事実が、後半の結線を通って記録まで届く
/// （要件 4.7・7.1＝記録なしに握り潰さない）。
///
/// 解析側（置換の検出）と交渉状態側（記録の発火）はそれぞれ別の決定論テストが押さえて
/// いるが、その 2 つを繋ぐ 1 語——後半が `parse_response` の返した置換の有無をそのまま
/// `note_response` へ渡すこと——はここでしか見ていない。渡す値を `false` に固定すると
/// 本テストだけが赤になる。
#[test]
fn the_decode_replacement_fact_reaches_the_record() {
    // `Charset: EUC-JP` を名乗りながら、EUC-JP として不正なバイトを値に持つ応答。
    // 0x80 は EUC-JP の先行バイト域（0xA1〜0xFE）の外にあり、代替文字へ吸収される。
    let mut bytes = b"SHIORI/3.0 200 OK\r\nCharset: EUC-JP\r\nValue: ".to_vec();
    bytes.push(0x80);
    bytes.extend_from_slice(b"\r\n\r\n");

    let mut negotiator = CharsetNegotiator::new(Charset::UTF_8, false);
    let (parsed, events) = log_capture_kit::capture(|| parse3(&mut negotiator, &bytes));
    let parsed = parsed.expect("不正な並びでも解析は成功する（要件 4.7）");
    assert!(
        parsed.decode_had_errors,
        "解析側が置換を検出していない（固定物が不正な並びになっていない）"
    );

    let recorded: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("charset_invalid_bytes_replaced"))
        .collect();
    assert_eq!(
        recorded.len(),
        1,
        "置換の事実が記録まで届いていない（要件 4.7・7.1）"
    );
}

// --- 版で分ける 2 か所（指摘 15・5）----------------------------------------

/// 3.0 の前半（`encode_for` の 3.0 の腕）。3.0 は送らない要求を持たない。
fn encode3(negotiator: &mut CharsetNegotiator, req: &ShioriRequest<'_>) -> Vec<u8> {
    encode_for(ShioriVersion::Shiori3, negotiator, req, false).expect("3.0 は必ず送る")
}

/// 3.0 の後半（`parse_for` の 3.0 の腕）。3.0 の読みは要求を見ない。
fn parse3(negotiator: &mut CharsetNegotiator, resp: &[u8]) -> Result<ParsedResponse, ShioriError> {
    let charset = negotiator.current();
    parse_for(
        ShioriVersion::Shiori3,
        negotiator,
        &a_request(Method::Get, "OnBoot", charset),
        resp,
        false,
    )
}

/// 版で符号化器を選ぶ: 同じ資源の問い合わせが 2.x では `GET String`・3.0 では `GET SHIORI/3.0`、
/// 2.x へ送らない片道イベントは 2.x だけ `None`。腕を入れ替えると全部が逆になる。
#[test]
fn encode_for_picks_the_codec_by_version() {
    let mut negotiator = CharsetNegotiator::new(Charset::UTF_8, false);
    let get = a_request(Method::Get, "username", Charset::UTF_8);
    let two = encode_for(ShioriVersion::Shiori2, &mut negotiator, &get, true).expect("2.x は送る");
    assert!(two.starts_with(b"GET String SHIORI/2.5\r\n"));
    let three =
        encode_for(ShioriVersion::Shiori3, &mut negotiator, &get, true).expect("3.0 は送る");
    assert!(three.starts_with(b"GET SHIORI/3.0\r\n"));

    let not_for_2x = a_request(Method::Notify, "basewareversion", Charset::UTF_8);
    assert_eq!(
        encode_for(ShioriVersion::Shiori2, &mut negotiator, &not_for_2x, false),
        None
    );
    assert!(encode_for(ShioriVersion::Shiori3, &mut negotiator, &not_for_2x, false).is_some());
}

/// 版で読み手を選ぶ: 2.x の資源の答えは `String:` の値、3.0 の答えは `Value:` の値を読む。
/// 腕を入れ替えると、どちらも値なしになる。
#[test]
fn parse_for_picks_the_reader_by_version() {
    let req = a_request(Method::Get, "username", Charset::UTF_8);
    let read = |version, resp: &[u8]| {
        let mut negotiator = CharsetNegotiator::new(Charset::UTF_8, false);
        parse_for(version, &mut negotiator, &req, resp, true)
            .expect("解析できる")
            .value
    };
    let resp2 = b"SHIORI/2.6 200 OK\r\nString: x\r\n\r\n";
    let resp3 = b"SHIORI/3.0 200 OK\r\nValue: y\r\n\r\n";
    assert_eq!(read(ShioriVersion::Shiori2, resp2).as_deref(), Some("x"));
    assert_eq!(read(ShioriVersion::Shiori3, resp2), None);
    assert_eq!(read(ShioriVersion::Shiori3, resp3).as_deref(), Some("y"));
    assert_eq!(read(ShioriVersion::Shiori2, resp3), None);
}

/// 読めない応答は `ERROR`（`shiori_response_unreadable`）を 1 行書いてから `Parse` で返す。
/// 行には応答の長さと 1 行目（先頭 80 バイトまで）が載る。2 つの版のどちらでも同じ。
#[test]
fn an_unreadable_response_is_logged_as_error() {
    let garbage = [b"x".repeat(100).as_slice(), b"\r\nmore\r\n\r\n"].concat();
    for version in [ShioriVersion::Shiori3, ShioriVersion::Shiori2] {
        let mut negotiator = CharsetNegotiator::new(Charset::UTF_8, false);
        let req = a_request(Method::Get, "OnBoot", Charset::UTF_8);
        let (result, events) =
            log_capture_kit::capture(|| parse_for(version, &mut negotiator, &req, &garbage, false));
        assert!(matches!(result, Err(ShioriError::Parse)), "{version:?}");
        let lines: Vec<_> = events
            .iter()
            .filter(|e| e.field_str("event") == Some("shiori_response_unreadable"))
            .collect();
        assert_eq!(lines.len(), 1, "{version:?}");
        assert_eq!(lines[0].level, tracing::Level::ERROR);
        assert_eq!(lines[0].target, "shiori-host32");
        assert_eq!(
            lines[0].field("len"),
            Some(garbage.len().to_string().as_str())
        );
        assert_eq!(
            lines[0].field_str("first_line"),
            Some("x".repeat(80).as_str())
        );
    }
}
