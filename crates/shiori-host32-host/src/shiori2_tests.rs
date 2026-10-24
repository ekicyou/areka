//! `shiori2` の決定論テスト（タスク 2.1〜2.3・要件 2.1/2.5/3.1〜3.7/4.1〜4.4/4.6/4.7/7.1〜7.3）。
//!
//! 窓も I/O も持たない純関数の入出力だけで成立する（x64 のみ・32bit 成果物不要）。
//! 参照値のバイト列は符号化器から導かず、研究記録 10.1・10.2 節から手で写した定数として持つ。

use super::{ShioriVersion, VersionVerdict, build_request, decide_version, version_probe_request};
use crate::charset::Charset;
use crate::request_headers::RequestHeaders;
use crate::shiori3::{ABSENT_REFERENCE, Method, ShioriRequest};

/// 研究記録 10.1 節の確かめの要求（SSP が華和梨へ送ったもの）の全文。
const PROBE_UTF8_SSP: &[u8] = b"GET Version SHIORI/2.6\r\nCharset: UTF-8\r\nSender: SSP\r\n\r\n";

/// 研究記録 10.1 節の華和梨 7.4 の答えの全文。
const KAWARI_REPLY: &[u8] = b"SHIORI/2.6 200 OK\r\nCharset: Shift_JIS\r\nCraftman: KawariDevelopmentTeam\r\nID: KAWARI.kdt\r\nVersion: 7.4.0\r\n\r\n";

fn decided(version: ShioriVersion, evidence: &str) -> VersionVerdict {
    VersionVerdict::Decided {
        version,
        evidence: evidence.to_string(),
    }
}

/// 記録に書く綴りは "SHIORI/3.0"／"SHIORI/2.x"。
#[test]
fn version_spelling_for_logs() {
    assert_eq!(ShioriVersion::Shiori3.as_str(), "SHIORI/3.0");
    assert_eq!(ShioriVersion::Shiori2.as_str(), "SHIORI/2.x");
}

/// 確かめの要求の全文のバイトが研究記録 10.1 節と一致する（要件 2.1）。
#[test]
fn probe_request_bytes_match_ssp_record() {
    assert_eq!(version_probe_request(Charset::UTF_8, "SSP"), PROBE_UTF8_SSP);
}

/// `Charset` の行は渡された文字コードの正規名で書く。
#[test]
fn probe_request_uses_canonical_charset_name() {
    let sjis = Charset::for_label("sjis").expect("sjis は解決できる");
    assert_eq!(
        version_probe_request(sjis, "areka"),
        b"GET Version SHIORI/2.6\r\nCharset: Shift_JIS\r\nSender: areka\r\n\r\n"
    );
}

/// 3.0 の答えは状態の番号によらず 3.0（要件 7.3）。
#[test]
fn shiori3_replies_decide_shiori3() {
    assert_eq!(
        decide_version(b"SHIORI/3.0 204 No Content\r\nCharset: UTF-8\r\nSender: pasta\r\n\r\n"),
        decided(ShioriVersion::Shiori3, "SHIORI/3.0 204 No Content")
    );
    assert_eq!(
        decide_version(b"SHIORI/3.0 400 Bad Request\r\n\r\n"),
        decided(ShioriVersion::Shiori3, "SHIORI/3.0 400 Bad Request")
    );
}

/// 華和梨の答えは 2.x・根拠は先頭の行そのまま（要件 7.3）。
#[test]
fn kawari_reply_decides_shiori2() {
    assert_eq!(
        decide_version(KAWARI_REPLY),
        decided(ShioriVersion::Shiori2, "SHIORI/2.6 200 OK")
    );
}

/// 空・版の字の無い先頭の行・2.x でも 3.x でもない版は決められない（要件 2.5・7.3）。
#[test]
fn unknown_replies_are_undecided() {
    for reply in [&b""[..], b"garbage", b"SHIORI/1.1 200 OK\r\n\r\n"] {
        assert!(
            matches!(decide_version(reply), VersionVerdict::Undecided { .. }),
            "{:?} は決められないはず",
            String::from_utf8_lossy(reply)
        );
    }
}

// ---- 2.x の要求の組み立て（タスク 2.2・要件 3.1〜3.7・7.1） ----------------

/// 研究記録 10.2 節の通知の `OnInitialize`（3.0 では NOTIFY）を SSP が送った全文。
const ON_INITIALIZE_SSP: &[u8] = b"GET Sentence SHIORI/2.2\r\nCharset: Shift_JIS\r\nSecurityLevel: local\r\nSender: SSP\r\nEvent: OnInitialize\r\nReference0: \r\n\r\n";

/// 研究記録 10.2 節の `OnSecondChange`（`Status` あり）の全文。
const ON_SECOND_CHANGE_SSP: &[u8] = b"GET Sentence SHIORI/2.2\r\nCharset: Shift_JIS\r\nSecurityLevel: local\r\nSender: SSP\r\nEvent: OnSecondChange\r\nStatus: talking,timecritical,balloon(0=0)\r\nReference0: 193\r\nReference1: 0\r\nReference2: 0\r\nReference3: 0\r\nReference4: 64\r\n\r\n";

/// 研究記録 10.2 節の `OnFirstBoot` の全文から `BaseEvent: OnBoot` の行を除いたもの
/// （areka は 3.0 でも `BaseID` を送っていないので書かない・設計の「2.x の要求の形」）。
const ON_FIRST_BOOT_SSP: &[u8] = b"GET Sentence SHIORI/2.2\r\nCharset: Shift_JIS\r\nSecurityLevel: local\r\nSender: SSP\r\nEvent: OnFirstBoot\r\nReference0: 0\r\nReference1: \r\nReference2: \r\nReference3: \r\nReference4: \r\nReference5: \r\nReference6: \r\nReference7: master\r\n\r\n";

/// 研究記録 10.2 節の資源の問い合わせ `username` の全文。
const USERNAME_SSP: &[u8] = b"GET String SHIORI/2.5\r\nCharset: Shift_JIS\r\nSecurityLevel: local\r\nSecurityOrigin: null\r\nSender: SSP\r\nID: username\r\n\r\n";

/// 研究記録 10.2 節の翻訳（`Status: talking`）の形。台本は記録で省かれているので短い台本を置く。
const TRANSLATE_SSP: &[u8] = b"TRANSLATE Sentence SHIORI/2.6\r\nCharset: Shift_JIS\r\nSecurityLevel: local\r\nSecurityOrigin: null\r\nSender: SSP\r\nStatus: talking\r\nSentence: \\t\\h\\s[6]\\e\r\n\r\n";

fn refs(values: &[&str]) -> Vec<String> {
    values.iter().map(|v| (*v).to_string()).collect()
}

fn sjis() -> Charset {
    Charset::for_label("Shift_JIS").expect("Shift_JIS は解決できる")
}

/// SSP の記録と同じ値の見出し（`SenderType` は 2.x では書かれないはず）。
fn ssp_headers() -> RequestHeaders<'static> {
    RequestHeaders::internal("SSP")
}

fn bytes_of(
    method: Method,
    id: &str,
    references: &[String],
    status: Option<&str>,
    is_resource: bool,
) -> Option<Vec<u8>> {
    let req = ShioriRequest {
        method,
        id,
        references,
        headers: ssp_headers(),
        status,
        charset: sjis(),
    };
    build_request(&req, is_resource).map(|e| e.bytes)
}

fn assert_bytes(actual: Option<Vec<u8>>, expected: &[u8]) {
    let actual = actual.expect("送るはず");
    assert_eq!(
        actual,
        expected,
        "\n実際: {}\n期待: {}",
        String::from_utf8_lossy(&actual),
        String::from_utf8_lossy(expected)
    );
}

/// NOTIFY の `OnInitialize` は `GET Sentence SHIORI/2.2` で送る（要件 3.2）。
#[test]
fn notify_on_initialize_is_get_sentence() {
    assert_bytes(
        bytes_of(Method::Notify, "OnInitialize", &refs(&[""]), None, false),
        ON_INITIALIZE_SSP,
    );
}

/// `Status` ありの台本を求めるイベント（要件 3.1・3.6）。
#[test]
fn get_with_status_is_get_sentence() {
    assert_bytes(
        bytes_of(
            Method::Get,
            "OnSecondChange",
            &refs(&["193", "0", "0", "0", "64"]),
            Some("talking,timecritical,balloon(0=0)"),
            false,
        ),
        ON_SECOND_CHANGE_SSP,
    );
}

/// `Status` なし・空の References を含む `OnFirstBoot`（要件 3.1）。
#[test]
fn get_on_first_boot_is_get_sentence() {
    assert_bytes(
        bytes_of(
            Method::Get,
            "OnFirstBoot",
            &refs(&["0", "", "", "", "", "", "", "master"]),
            None,
            false,
        ),
        ON_FIRST_BOOT_SSP,
    );
}

/// 欠番の印の位置は行を出さず番号だけ進める（3.0 と同じ）。
#[test]
fn absent_reference_skips_line_keeps_numbering() {
    assert_bytes(
        bytes_of(
            Method::Get,
            "OnFirstBoot",
            &refs(&["0", ABSENT_REFERENCE, "master"]),
            None,
            false,
        ),
        b"GET Sentence SHIORI/2.2\r\nCharset: Shift_JIS\r\nSecurityLevel: local\r\nSender: SSP\r\nEvent: OnFirstBoot\r\nReference0: 0\r\nReference2: master\r\n\r\n",
    );
}

/// `On` で始まらない外からのイベント名でも、資源の印が無ければ `GET Sentence`（要件 3.1・3.7）。
#[test]
fn non_on_event_without_resource_mark_is_get_sentence() {
    assert_bytes(
        bytes_of(Method::Get, "installedghostname", &refs(&["a"]), None, false),
        b"GET Sentence SHIORI/2.2\r\nCharset: Shift_JIS\r\nSecurityLevel: local\r\nSender: SSP\r\nEvent: installedghostname\r\nReference0: a\r\n\r\n",
    );
}

/// 資源の印つきの GET は `GET String SHIORI/2.5`（要件 3.3・3.6）。
#[test]
fn resource_get_is_get_string() {
    assert_bytes(
        bytes_of(Method::Get, "username", &[], None, true),
        USERNAME_SSP,
    );
}

/// 翻訳は `TRANSLATE Sentence SHIORI/2.6`・`Sentence` は Reference0・References は書かない（要件 3.4）。
#[test]
fn on_translate_is_translate_sentence() {
    assert_bytes(
        bytes_of(
            Method::Get,
            "OnTranslate",
            &refs(&["\\t\\h\\s[6]\\e", "", "", "x"]),
            Some("talking"),
            false,
        ),
        TRANSLATE_SSP,
    );
}

/// `basewareversion` の NOTIFY は送らない（要件 3.5）。
#[test]
fn basewareversion_notify_is_not_sent() {
    assert_eq!(
        bytes_of(
            Method::Notify,
            "basewareversion",
            &refs(&["2.6"]),
            None,
            false
        ),
        None
    );
}

/// 2 バイト文字の値も全文 1 本の経路で Shift_JIS に符号化する（要件 3.6）。
#[test]
fn shift_jis_value_is_encoded_with_whole_text() {
    // 「さくら」の Shift_JIS は 82 B3 82 AD 82 E7（手で写した値）。
    assert_bytes(
        bytes_of(
            Method::Get,
            "OnTranslate",
            &refs(&["\u{3055}\u{304f}\u{3089}"]),
            None,
            false,
        ),
        b"TRANSLATE Sentence SHIORI/2.6\r\nCharset: Shift_JIS\r\nSecurityLevel: local\r\nSecurityOrigin: null\r\nSender: SSP\r\nSentence: \x82\xb3\x82\xad\x82\xe7\r\n\r\n",
    );
}

// ---- 2.x の答えの読み（タスク 2.3・要件 4.1〜4.4/4.6/4.7/7.2） ----

fn parse2(
    reply: &[u8],
    method: Method,
    id: &str,
    is_resource: bool,
) -> Result<crate::shiori3::ParsedResponse, crate::error::ShioriError> {
    super::parse_response(
        reply,
        crate::charset::CharsetPolicy::Negotiate(sjis()),
        method,
        id,
        is_resource,
    )
}

#[test]
fn sentence_200_reads_script() {
    let r = parse2(
        b"SHIORI/2.2 200 OK\r\nCharset: Shift_JIS\r\nSentence: \\h\\s[0]hi\\e\r\n\r\n",
        Method::Get,
        "OnBoot",
        false,
    )
    .expect("読める");
    assert_eq!(r.status, 200);
    assert_eq!(r.value.as_deref(), Some(r"\h\s[0]hi\e"));
    assert_eq!(r.charset_header.as_deref(), Some("Shift_JIS"));
}

#[test]
fn translate_reads_sentence() {
    let r = parse2(
        b"SHIORI/2.6 200 OK\r\nSentence: \\t\\h\\s[6]\\e\r\n\r\n",
        Method::Get,
        "OnTranslate",
        false,
    )
    .expect("読める");
    assert_eq!(r.value.as_deref(), Some(r"\t\h\s[6]\e"));
}

#[test]
fn resource_200_reads_string_not_sentence() {
    let reply = b"SHIORI/2.5 200 OK\r\nSentence: wrong\r\nString: sakura\r\n\r\n";
    let r = parse2(reply, Method::Get, "username", true).expect("読める");
    assert_eq!(r.status, 200);
    assert_eq!(r.value.as_deref(), Some("sakura"));
    // 台本の要求では String を値として読まない。
    let r = parse2(
        b"SHIORI/2.2 200 OK\r\nString: sakura\r\n\r\n",
        Method::Get,
        "OnBoot",
        false,
    )
    .expect("読める");
    assert_eq!(r.value, None);
}

#[test]
fn no_content_and_headerless_200_have_no_value() {
    let r = parse2(
        b"SHIORI/2.2 204 No Content\r\n\r\n",
        Method::Get,
        "OnBoot",
        false,
    )
    .expect("読める");
    assert_eq!((r.status, r.value), (204, None));
    let r = parse2(b"SHIORI/2.2 200 OK\r\n\r\n", Method::Get, "OnBoot", false).expect("読める");
    assert_eq!((r.status, r.value), (200, None));
}

#[test]
fn error_statuses_are_kept_verbatim() {
    for (reply, code) in [
        (&b"SHIORI/2.2 400 Bad Request\r\n\r\n"[..], 400),
        (&b"SHIORI/2.2 500 Internal Server Error\r\n\r\n"[..], 500),
    ] {
        let r = parse2(reply, Method::Get, "OnBoot", false).expect("読める");
        assert_eq!((r.status, r.value), (code, None));
    }
}

#[test]
fn reply_without_status_line_is_parse_error() {
    for reply in [&b""[..], &b"Sentence: \\e\r\n\r\n"[..]] {
        assert!(matches!(
            parse2(reply, Method::Get, "OnBoot", false),
            Err(crate::error::ShioriError::Parse)
        ));
    }
}

#[test]
fn value_header_alone_is_no_value_in_2x() {
    for is_resource in [false, true] {
        let r = parse2(
            b"SHIORI/2.2 200 OK\r\nValue: \\e\r\n\r\n",
            Method::Get,
            "username",
            is_resource,
        )
        .expect("読める");
        assert_eq!(r.value, None);
    }
}

#[test]
fn headers_absent_in_2x_are_not_read() {
    let r = parse2(
        b"SHIORI/2.2 200 OK\r\nSentence: \\e\r\nErrorLevel: warning\r\nErrorDescription: x\r\nSecurityLevel: external\r\n\r\n",
        Method::Get,
        "OnBoot",
        false,
    )
    .expect("読める");
    assert_eq!(r.value.as_deref(), Some(r"\e"));
    assert_eq!(r.error_level, None);
    assert_eq!(r.error_description, None);
    assert_eq!(r.security_level, None);
}
