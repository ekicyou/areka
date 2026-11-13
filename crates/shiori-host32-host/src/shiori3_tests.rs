use super::*;
use crate::error::ShioriError;

// ---- parse_response テスト ---------------------------------------------

/// 200 + Value: さくらスクリプト本体（多バイト UTF-8）が `value: Some(..)` に抽出される（要件 2.1）。
#[test]
fn parse_200_extracts_value() {
    let body = r"\w0\h\s[0]こんにちは世界\e";
    let resp =
        format!("SHIORI/3.0 200 OK\r\nCharset: UTF-8\r\nSender: pasta\r\nValue: {body}\r\n\r\n");
    let parsed = parse_response(resp.as_bytes(), CharsetPolicy::Negotiate(Charset::UTF_8))
        .expect("well-formed 200");
    assert_eq!(
        parsed,
        ParsedResponse {
            status: 200,
            value: Some(body.to_string()),
            error_level: None,
            error_description: None,
            charset_header: Some("UTF-8".to_string()),
            decode_had_errors: false,
            security_level: None,
        }
    );
}

/// 値の見出しに `Sentence` を渡すと 2.x の答えの台本が `value` に入る
/// （shiori2-protocol 要件 4.1）。同じバイト列を 3.0 の読み（`Value`）に通すと
/// `value` は `None` のまま——3.0 の読み方は見出しを替えた分しか動かない（要件 6.2）。
#[test]
fn parse_with_sentence_header_reads_2x_value() {
    let body = r"\h\s[0]こんにちは\e";
    let resp = format!("SHIORI/2.5 200 OK\r\nCharset: UTF-8\r\nSentence: {body}\r\n\r\n");
    let policy = CharsetPolicy::Negotiate(Charset::UTF_8);
    let parsed = parse_response_with(resp.as_bytes(), policy, "Sentence").expect("well-formed");
    assert_eq!(parsed.status, 200);
    assert_eq!(parsed.value, Some(body.to_string()));
    assert_eq!(parsed.charset_header, Some("UTF-8".to_string()));
    let as_3 = parse_response(resp.as_bytes(), policy).expect("well-formed");
    assert_eq!(as_3.value, None);
}

/// 200 だが Value ヘッダが無い場合は `value: None`（要件 2.1・Value は 200 でも欠落し得る）。
#[test]
fn parse_200_without_value_is_none() {
    let resp = "SHIORI/3.0 200 OK\r\nCharset: UTF-8\r\nSender: pasta\r\n\r\n";
    let parsed = parse_response(resp.as_bytes(), CharsetPolicy::Negotiate(Charset::UTF_8))
        .expect("well-formed 200");
    assert_eq!(parsed.status, 200);
    assert_eq!(parsed.value, None);
}

/// 204 No Content: status=204・value=None を保持（要件 2.2・成功だがスクリプト無しを区別）。
#[test]
fn parse_204_no_content() {
    let resp = "SHIORI/3.0 204 No Content\r\nCharset: UTF-8\r\nSender: pasta\r\n\r\n";
    let parsed = parse_response(resp.as_bytes(), CharsetPolicy::Negotiate(Charset::UTF_8))
        .expect("well-formed 204");
    assert_eq!(parsed.status, 204);
    assert_eq!(parsed.value, None);
}

/// 400 Bad Request: parse_response は Err にせず status=400 を Ok で保持（要件 2.4・意味判断は client）。
#[test]
fn parse_400_is_ok_status_preserved() {
    let resp = "SHIORI/3.0 400 Bad Request\r\nCharset: UTF-8\r\nSender: pasta\r\n\r\n";
    let parsed = parse_response(resp.as_bytes(), CharsetPolicy::Negotiate(Charset::UTF_8))
        .expect("400 is a parseable response, not a parse error");
    assert_eq!(parsed.status, 400);
}

/// 500 Internal Server Error: 同様に Ok で status=500 を保持（要件 2.4）。
#[test]
fn parse_500_is_ok_status_preserved() {
    let resp = "SHIORI/3.0 500 Internal Server Error\r\nCharset: UTF-8\r\nSender: pasta\r\n\r\n";
    let parsed = parse_response(resp.as_bytes(), CharsetPolicy::Negotiate(Charset::UTF_8))
        .expect("500 is parseable");
    assert_eq!(parsed.status, 500);
}

/// 311/312（OnTeach 系）: drop せず status で区別可能に Ok 保持（要件 2.7）。
#[test]
fn parse_311_and_312_not_dropped() {
    for code in [311u16, 312u16] {
        let resp = format!("SHIORI/3.0 {code} Teach\r\nCharset: UTF-8\r\nSender: pasta\r\n\r\n");
        let parsed = parse_response(resp.as_bytes(), CharsetPolicy::Negotiate(Charset::UTF_8))
            .expect("3xx parseable");
        assert_eq!(parsed.status, code);
    }
}

/// ErrorLevel / ErrorDescription（SSP 拡張）が保持される（要件 2.5）。
#[test]
fn parse_error_level_and_description_preserved() {
    let resp = "SHIORI/3.0 500 Internal Server Error\r\nCharset: UTF-8\r\nErrorLevel: critical\r\nErrorDescription: boom happened\r\n\r\n";
    let parsed = parse_response(resp.as_bytes(), CharsetPolicy::Negotiate(Charset::UTF_8))
        .expect("parseable");
    assert_eq!(parsed.status, 500);
    assert_eq!(parsed.error_level.as_deref(), Some("critical"));
    assert_eq!(parsed.error_description.as_deref(), Some("boom happened"));
}

/// 未知ヘッダ（Reference0 / Marker / X-Weird）が混在しても parse は成功し Value を抽出できる（要件 2.8）。
#[test]
fn parse_unknown_headers_tolerated() {
    let resp = "SHIORI/3.0 200 OK\r\nCharset: UTF-8\r\nSender: pasta\r\nReference0: x\r\nMarker: y\r\nX-Weird: z\r\nValue: \\s[0]hi\\e\r\n\r\n";
    let parsed = parse_response(resp.as_bytes(), CharsetPolicy::Negotiate(Charset::UTF_8))
        .expect("unknown headers must not fail parse");
    assert_eq!(parsed.status, 200);
    assert_eq!(parsed.value.as_deref(), Some(r"\s[0]hi\e"));
}

/// Charset ヘッダ省略時も request_charset（UTF-8）を継承して UTF-8 として解析できる（要件 2.6）。
#[test]
fn parse_charset_omitted_inherits_request_charset() {
    // Charset ヘッダなし・多バイト UTF-8 body。
    let resp = "SHIORI/3.0 200 OK\r\nSender: pasta\r\nValue: 日本語\r\n\r\n";
    let parsed = parse_response(resp.as_bytes(), CharsetPolicy::Negotiate(Charset::UTF_8))
        .expect("UTF-8 inherited");
    assert_eq!(parsed.status, 200);
    assert_eq!(parsed.value.as_deref(), Some("日本語"));
}

/// bare-LF（\n のみ）の応答にも頑健（要件 2.3・CRLF/LF 両受理）。
#[test]
fn parse_bare_lf_robust() {
    let resp = "SHIORI/3.0 200 OK\nCharset: UTF-8\nValue: hi\n\n";
    let parsed = parse_response(resp.as_bytes(), CharsetPolicy::Negotiate(Charset::UTF_8))
        .expect("bare LF parseable");
    assert_eq!(parsed.status, 200);
    assert_eq!(parsed.value.as_deref(), Some("hi"));
}

/// 空バイト列は malformed（status 行が取れない）→ Err(Parse)（malformed・fail fast）。
#[test]
fn parse_empty_is_parse_error() {
    let err = parse_response(b"", CharsetPolicy::Negotiate(Charset::UTF_8))
        .expect_err("empty must be a parse error");
    assert!(matches!(err, ShioriError::Parse));
}

/// status コードを含まない先頭行（GARBAGE）は malformed → Err(Parse)（過剰寛容を防ぐ）。
#[test]
fn parse_garbage_status_line_is_parse_error() {
    let err = parse_response(
        b"GARBAGE\r\nValue: x\r\n\r\n",
        CharsetPolicy::Negotiate(Charset::UTF_8),
    )
    .expect_err("no numeric status must be a parse error");
    assert!(matches!(err, ShioriError::Parse));
}

/// status 行を持たないバイト列は malformed → Err(Parse)（期待値は本 spec の前後で不変）。
///
/// `[FF FE 00]` には status 行が無いので、事後条件「`Err(Parse)` は status 行の欠落・
/// 数値コードなしのみ」の下でも従来どおり `Err(Parse)` である（不正バイトが理由ではない）。
#[test]
fn parse_without_status_line_is_parse_error() {
    let err = parse_response(
        &[0xFF, 0xFE, 0x00],
        CharsetPolicy::Negotiate(Charset::UTF_8),
    )
    .expect_err("status 行が無い入力は parse エラー");
    assert!(matches!(err, ShioriError::Parse));
}

/// 不正なバイト並びは解析を失敗させず、代替文字で吸収して置換ありを報告する
/// （areka-P0-charset-canon 要件 4.7・10.2・4.8 の唯一の例外）。
#[test]
fn parse_invalid_bytes_are_replaced_not_parse_error() {
    // status 行あり・`Value` の値に UTF-8 として不正な 0xFF を 1 バイト混ぜる。
    let mut resp = b"SHIORI/3.0 200 OK\r\nValue: ".to_vec();
    resp.push(0xFF);
    resp.extend_from_slice(b"\r\n\r\n");
    let parsed = parse_response(&resp, CharsetPolicy::Negotiate(Charset::UTF_8))
        .expect("不正バイトで解析を失敗させてはならない（要件 4.7）");
    assert_eq!(parsed.status, 200);
    assert_eq!(parsed.value.as_deref(), Some("\u{FFFD}"));
    assert!(parsed.decode_had_errors, "置換ありを報告すること");
}

/// 文字コードヘッダは位置を問わず拾う（要件 4.1——正典は「望ましい」位置を示すのみ）。
#[test]
fn parse_charset_header_found_at_non_first_position() {
    let resp =
        "SHIORI/3.0 200 OK\r\nSender: pasta\r\nMarker: x\r\nCharset: EUC-JP\r\nValue: hi\r\n\r\n";
    let parsed = parse_response(resp.as_bytes(), CharsetPolicy::Negotiate(Charset::UTF_8))
        .expect("well-formed");
    assert_eq!(parsed.charset_header.as_deref(), Some("EUC-JP"));
}

/// SSP に合わせ（research.md §14.13）、値は頭の半角の空白とタブだけを削る。
/// 後ろの半角の空白とタブ、全角の空白（U+3000）は残す（指摘 22・要件 10.8）。
/// 後ろに空白の残る `Charset` の値も文字コードとして解決される（継承の Shift_JIS でなく UTF-8 で読む）。
#[test]
fn a_value_keeps_full_width_spaces() {
    let resp = "SHIORI/3.0 200 OK\r\n\tCharset: \tUTF-8 \t\r\n Value: \t\u{3000}こんにちは\u{3000} \t\r\n\r\n";
    let parsed = parse_response(
        resp.as_bytes(),
        CharsetPolicy::Negotiate(Charset::SHIFT_JIS),
    )
    .expect("well-formed");
    assert_eq!(
        parsed.value.as_deref(),
        Some("\u{3000}こんにちは\u{3000} \t")
    );
    assert_eq!(parsed.charset_header.as_deref(), Some("UTF-8 \t"));
    assert!(!parsed.decode_had_errors);
}

/// 同じ名前のヘッダが 2 つあれば、どれも後ろのものを採る（SSP に合わせた・指摘 23・要件 10.8）。
#[test]
fn the_last_of_duplicated_headers_wins() {
    let resp = "SHIORI/3.0 200 OK\r\nCharset: UTF-8\r\nValue: first\r\nErrorLevel: warning\r\n\
                ErrorDescription: d1\r\nSecurityLevel: local\r\nCharset: Shift_JIS\r\nValue: second\r\n\
                ErrorLevel: error\r\nErrorDescription: d2\r\nSecurityLevel: external\r\n\r\n";
    let parsed = parse_response(resp.as_bytes(), CharsetPolicy::Negotiate(Charset::UTF_8))
        .expect("well-formed");
    assert_eq!(parsed.value.as_deref(), Some("second"));
    assert_eq!(parsed.charset_header.as_deref(), Some("Shift_JIS"));
    assert_eq!(parsed.error_level.as_deref(), Some("error"));
    assert_eq!(parsed.error_description.as_deref(), Some("d2"));
    assert_eq!(parsed.security_level.as_deref(), Some("external"));
}

// 復号の 4 経路（解決できるヘッダ／省略時の継承／解決不能ラベル／強制時の無視）は、
// 兄弟ファイル `shiori3_charset_tests.rs` が 3〜4 系統の定数バイト列で固定する
// （タスク 2.3・要件 9.2/9.3）。ASCII と Shift_JIS だけの固定物より強い被覆なので、
// ここに置いていた同趣旨の 4 本は重複として外した。

/// GET（Reference 1 件）: request line・必須ヘッダ・Reference0・空行終端を検証（要件 1.1/1.3/1.4/1.6）。
#[test]
fn build_get_with_one_reference() {
    let references = vec!["1".to_string()];
    let req = ShioriRequest {
        method: Method::Get,
        id: "OnBoot",
        references: &references,
        headers: RequestHeaders::internal("areka"),
        status: None,
        charset: Charset::UTF_8,
    };
    let bytes = build_request(&req).bytes;
    let s = std::str::from_utf8(&bytes).expect("build_request output must be valid UTF-8");

    assert!(s.starts_with("GET SHIORI/3.0\r\n"), "request:\n{s}");
    assert!(s.contains("Charset: UTF-8\r\n"), "request:\n{s}");
    assert!(s.contains("Sender: areka\r\n"), "request:\n{s}");
    assert!(s.contains("ID: OnBoot\r\n"), "request:\n{s}");
    assert!(s.contains("Reference0: 1\r\n"), "request:\n{s}");
    assert!(s.contains("SecurityLevel: local\r\n"), "request:\n{s}");
    // 末尾は二重 CRLF（空行終端・要件 1.3）。
    assert!(s.ends_with("\r\n\r\n"), "request:\n{s}");
}

/// NOTIFY: request line が `NOTIFY SHIORI/3.0` から始まる（要件 1.2）。
#[test]
fn build_notify_request_line() {
    let req = ShioriRequest {
        method: Method::Notify,
        id: "OnSecondChange",
        references: &[],
        headers: RequestHeaders::internal("areka"),
        status: None,
        charset: Charset::UTF_8,
    };
    let bytes = build_request(&req).bytes;
    let s = std::str::from_utf8(&bytes).expect("valid UTF-8");
    assert!(s.starts_with("NOTIFY SHIORI/3.0\r\n"), "request:\n{s}");
    assert!(s.ends_with("\r\n\r\n"), "request:\n{s}");
}

/// 複数 Reference: Reference0/Reference1/Reference2 が 0 起点順で並ぶ（要件 1.4）。
#[test]
fn build_multiple_references_zero_based() {
    let references = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let req = ShioriRequest {
        method: Method::Get,
        id: "OnTest",
        references: &references,
        headers: RequestHeaders::internal("areka"),
        status: None,
        charset: Charset::UTF_8,
    };
    let bytes = build_request(&req).bytes;
    let s = std::str::from_utf8(&bytes).expect("valid UTF-8");
    assert!(s.contains("Reference0: a\r\n"), "request:\n{s}");
    assert!(s.contains("Reference1: b\r\n"), "request:\n{s}");
    assert!(s.contains("Reference2: c\r\n"), "request:\n{s}");
    // 順序（0→1→2）の検証。
    let i0 = s.find("Reference0").expect("Reference0 present");
    let i1 = s.find("Reference1").expect("Reference1 present");
    let i2 = s.find("Reference2").expect("Reference2 present");
    assert!(i0 < i1 && i1 < i2, "reference order wrong:\n{s}");
}

/// Reference 空: Reference 行が一切なく、なお二重 CRLF で終端する（要件 1.3/1.4）。
#[test]
fn build_empty_references_still_terminated() {
    let req = ShioriRequest {
        method: Method::Get,
        id: "version",
        references: &[],
        headers: RequestHeaders::internal("areka"),
        status: None,
        charset: Charset::UTF_8,
    };
    let bytes = build_request(&req).bytes;
    let s = std::str::from_utf8(&bytes).expect("valid UTF-8");
    assert!(
        !s.contains("Reference"),
        "unexpected Reference header:\n{s}"
    );
    assert!(s.ends_with("\r\n\r\n"), "request:\n{s}");
}

/// 多バイト UTF-8 の Reference 値が UTF-8 バイトとして round-trip する（完了仕様 areka-P0-host32-request 要件 1.6）。
#[test]
fn build_multibyte_reference_roundtrips_as_utf8() {
    let references = vec!["こんにちは世界".to_string()];
    let req = ShioriRequest {
        method: Method::Get,
        id: "OnTest",
        references: &references,
        headers: RequestHeaders::internal("areka"),
        status: None,
        charset: Charset::UTF_8,
    };
    let bytes = build_request(&req).bytes;
    let s = std::str::from_utf8(&bytes).expect("output must be valid UTF-8");
    assert!(
        s.contains("Reference0: こんにちは世界\r\n"),
        "request:\n{s}"
    );
}

/// 汎用 ID: 任意の ID（リソース照会系含む）が特別扱いなく `ID: <that>` になる（要件 1.5）。
#[test]
fn build_generic_id_no_special_casing() {
    for id in ["OnSecondChange", "version", "name", "OnMouseClick"] {
        let req = ShioriRequest {
            method: Method::Get,
            id,
            references: &[],
            headers: RequestHeaders::internal("areka"),
            status: None,
            charset: Charset::UTF_8,
        };
        let bytes = build_request(&req).bytes;
        let s = std::str::from_utf8(&bytes).expect("valid UTF-8");
        assert!(
            s.contains(&format!("ID: {id}\r\n")),
            "id={id} request:\n{s}"
        );
    }
}

/// Sender は `req.headers.sender` をそのまま書く単一差替点（`"SSP"` 等ハードコードしない・design.md）。
#[test]
fn build_sender_is_verbatim_substitution_point() {
    let req = ShioriRequest {
        method: Method::Get,
        id: "OnBoot",
        references: &[],
        headers: RequestHeaders::internal("custom-baseware"),
        status: None,
        charset: Charset::UTF_8,
    };
    let bytes = build_request(&req).bytes;
    let s = std::str::from_utf8(&bytes).expect("valid UTF-8");
    assert!(s.contains("Sender: custom-baseware\r\n"), "request:\n{s}");
    assert!(!s.contains("Sender: SSP"), "must not hardcode SSP:\n{s}");
}

/// `Some(status)` → `Status:` 行が発行され、その位置が `Sender:` の後・`ID:` の前（DD-IT-6・要件 2.3/5.3）。
#[test]
fn build_with_status_emits_line_between_sender_and_id() {
    let req = ShioriRequest {
        method: Method::Get,
        id: "OnSecondChange",
        references: &[],
        headers: RequestHeaders::internal("areka"),
        status: Some("talking"),
        charset: Charset::UTF_8,
    };
    let bytes = build_request(&req).bytes;
    let s = std::str::from_utf8(&bytes).expect("valid UTF-8");
    assert!(s.contains("Status: talking\r\n"), "Status 行が無い:\n{s}");
    // 位置関係: Sender < Status < ID（DD-IT-6）。
    let sender = s.find("Sender:").expect("Sender present");
    let status = s.find("Status:").expect("Status present");
    let id = s.find("ID:").expect("ID present");
    assert!(
        sender < status && status < id,
        "Status の位置は Sender の後・ID の前でなければならない（DD-IT-6）:\n{s}"
    );
}

/// `None` → `Status:` 行が一切出ない（要件 2.3・空集合はヘッダ行そのものを省略）。
#[test]
fn build_without_status_emits_no_line() {
    let req = ShioriRequest {
        method: Method::Get,
        id: "OnSecondChange",
        references: &[],
        headers: RequestHeaders::internal("areka"),
        status: None,
        charset: Charset::UTF_8,
    };
    let bytes = build_request(&req).bytes;
    let s = std::str::from_utf8(&bytes).expect("valid UTF-8");
    assert!(
        !s.contains("Status:"),
        "None なら Status 行を出してはならない:\n{s}"
    );
}

/// Status 値は verbatim に写る（codec は解釈/分割/整形しない・DD-IT-6 語彙非漏洩）。
#[test]
fn build_status_value_is_verbatim() {
    let value = "talking,balloon(0=2/1=0)";
    let req = ShioriRequest {
        method: Method::Get,
        id: "OnSecondChange",
        references: &[],
        headers: RequestHeaders::internal("areka"),
        status: Some(value),
        charset: Charset::UTF_8,
    };
    let bytes = build_request(&req).bytes;
    let s = std::str::from_utf8(&bytes).expect("valid UTF-8");
    // カンマ・括弧・スラッシュ・等号を含む値がそのまま 1 行に載る（解釈しない）。
    assert!(
        s.contains("Status: talking,balloon(0=2/1=0)\r\n"),
        "Status 値は verbatim でなければならない:\n{s}"
    );
    // 語彙非漏洩の確認: 値に "Reference" が無い限り Reference 行を捏造しない（既存檻と非衝突）。
    assert!(
        !s.contains("Reference"),
        "Status 値が Reference 行を汚染してはならない:\n{s}"
    );
}

/// 複合した値 `talking,balloon(0=2/1=0)` が `Status:` 行にそのまま書かれ、行の位置が
/// 今までどおり `SenderType` の直後・`ID` の直前であることを、リクエスト全文の逐語で固定する
/// （areka-P0-status-execution-states 要件 1.5・8.2）。
#[test]
fn build_compound_status_full_request_is_verbatim_in_place() {
    let references = ["0".to_owned(), "0".to_owned()];
    let req = ShioriRequest {
        method: Method::Get,
        id: "OnSecondChange",
        references: &references,
        headers: RequestHeaders::internal("areka"),
        status: Some("talking,balloon(0=2/1=0)"),
        charset: Charset::UTF_8,
    };
    let bytes = build_request(&req).bytes;
    let s = std::str::from_utf8(&bytes).expect("valid UTF-8");
    assert_eq!(
        s,
        "GET SHIORI/3.0\r\n\
         Charset: UTF-8\r\n\
         SecurityLevel: local\r\n\
         SecurityOrigin: null\r\n\
         Sender: areka\r\n\
         SenderType: internal\r\n\
         Status: talking,balloon(0=2/1=0)\r\n\
         ID: OnSecondChange\r\n\
         Reference0: 0\r\n\
         Reference1: 0\r\n\
         \r\n"
    );
}

/// 欠番の印の位置は `Reference1:` の行を出さず、`Reference2:`・`Reference3:` の番号を
/// 詰めずに保つ（areka-P0-translate-pipeline 要件 3.3・正典「該当がない場合欠番」）。
#[test]
fn build_absent_reference_skips_line_and_keeps_numbering() {
    let references = [
        "台詞".to_owned(),
        ABSENT_REFERENCE.to_owned(),
        "OnBoot".to_owned(),
        "master".to_owned(),
    ];
    let req = ShioriRequest {
        method: Method::Get,
        id: "OnTranslate",
        references: &references,
        headers: RequestHeaders::internal("areka"),
        status: None,
        charset: Charset::UTF_8,
    };
    let bytes = build_request(&req).bytes;
    let s = std::str::from_utf8(&bytes).expect("valid UTF-8");
    assert_eq!(
        s,
        "GET SHIORI/3.0\r\n\
         Charset: UTF-8\r\n\
         SecurityLevel: local\r\n\
         SecurityOrigin: null\r\n\
         Sender: areka\r\n\
         SenderType: internal\r\n\
         ID: OnTranslate\r\n\
         Reference0: 台詞\r\n\
         Reference2: OnBoot\r\n\
         Reference3: master\r\n\
         \r\n"
    );
}

/// 印を含まない要求は、空文字・バイト値 1 を含む値でも今までどおり全行を書く
/// （印は NUL 1 文字の完全一致だけ・印の無い要求のバイト列は 1 バイトも変わらない）。
#[test]
fn build_without_absent_marker_is_byte_identical() {
    let references = [
        String::new(),
        "\u{1}".to_owned(),
        "a\u{0}b".to_owned(),
        "x".to_owned(),
    ];
    let req = ShioriRequest {
        method: Method::Get,
        id: "OnTest",
        references: &references,
        headers: RequestHeaders::internal("areka"),
        status: None,
        charset: Charset::UTF_8,
    };
    let bytes = build_request(&req).bytes;
    assert_eq!(
        bytes,
        "GET SHIORI/3.0\r\n\
         Charset: UTF-8\r\n\
         SecurityLevel: local\r\n\
         SecurityOrigin: null\r\n\
         Sender: areka\r\n\
         SenderType: internal\r\n\
         ID: OnTest\r\n\
         Reference0: \r\n\
         Reference1: \u{1}\r\n\
         Reference2: a\u{0}b\r\n\
         Reference3: x\r\n\
         \r\n"
            .as_bytes()
    );
}
