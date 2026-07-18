//! host-32 x64 側 SHIORI/3.0 ワイヤコーデック（純粋・決定的・`windows` 非依存）。
//!
//! 本モジュールは x64 親プロセス専用の SHIORI/3.0 codec であり、helper（i686）からは
//! 一切参照されない（design.md §Shiori3Codec・research.md §5.4: helper はバイト proxy に
//! 徹し、SHIORI/3.0 の組立と `Value:` parse は x64 親側に閉じる）。
//!
//! 本タスク（2.1）は **request 組立側のみ**を提供する:
//! - [`build_request`] — イベント（ID＋References）から SHIORI/3.0 request バイト列を
//!   組み立てる汎用ビルダ（要件 1.1〜1.6）。
//!
//! response 解析（`parse_response` / `ParsedResponse`）は後続タスク 2.2 が担う。
//!
//! ## 設計原則
//! - **汎用ビルダ**: `id` と `references` は verbatim に写す。OnBoot 等の特定イベントに
//!   固有の分岐や既定 Reference を埋め込まない（要件 1.5）。donor `build_onboot`
//!   （OnBoot 決め打ち）を method/id/references/sender 汎用へ一般化したもの。
//! - **UTF-8 固定**: 本仕様の対象範囲（emo2）は UTF-8。`Charset` ヘッダに `UTF-8` を
//!   宣言し、出力は常に有効な UTF-8（要件 1.6）。
//! - **単一差替点**: `Sender` は `ShioriRequest.sender` の値をそのまま書く。ハードコードした
//!   `"SSP"` 等の詐称はしない（呼び手が `"areka"` を渡す・design.md §送出ヘッダ最小集合）。

/// SHIORI/3.0 request の request line 種別（IShiori Get / Notify の wire 表現・要件 1.1/1.2）。
///
/// - [`Method::Get`] — 応答を要するイベント（`GET SHIORI/3.0`）。
/// - [`Method::Notify`] — 片道通知イベント（`NOTIFY SHIORI/3.0`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// 応答を要するイベント（request line `GET SHIORI/3.0`・要件 1.1）。
    Get,
    /// 片道通知イベント（request line `NOTIFY SHIORI/3.0`・要件 1.2）。
    Notify,
}

/// charset シーム（要件 1.6/1.7）。
///
/// 本仕様（emo2）は [`Charset::Utf8`] のみ実符号化する。Shift_JIS は将来の拡張
/// variant シームであり、本仕様では **実装しない**（要件 1.7: 切替シームのみ備える）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Charset {
    /// UTF-8（本仕様の唯一の実符号化・要件 1.6）。`Charset: UTF-8` として宣言する。
    Utf8, /* , ShiftJis (seam only・要件 1.7・本仕様は未実装) */
}

impl Charset {
    /// `Charset` ヘッダに宣言する charset 名を返す。
    #[must_use]
    const fn header_value(self) -> &'static str {
        match self {
            Charset::Utf8 => "UTF-8",
        }
    }
}

/// codec への入力（イベント個別知識を持たない汎用ビルダ入力・要件 1.5）。
///
/// `id` はイベント名（`ID` ヘッダ値）、`references` は `Reference0..N`（0 起点連番で
/// 連番付与）、`sender` は `Sender` ヘッダ値（呼び手が `"areka"` を渡す）。
pub struct ShioriRequest<'a> {
    /// request line 種別（`GET` / `NOTIFY`・要件 1.1/1.2）。
    pub method: Method,
    /// `ID` ヘッダ値（イベント名・要件 1.4/1.5）。汎用ゆえ特定イベントの分岐を持たない。
    pub id: &'a str,
    /// `Reference0..N` に 0 起点連番で写す References（要件 1.4）。空なら Reference 行なし。
    pub references: &'a [String],
    /// `Sender` ヘッダ値（例 `"areka"`・design.md §送出ヘッダ最小集合）。単一差替点。
    pub sender: &'a str,
    /// `Status` ヘッダの wire 値。`None` ⇒ ヘッダ行を出さない／`Some(v)` ⇒ `Status: v` を1行。
    /// **値は解釈しない**（汎用 codec・語彙は kanade が所有・DD-IT-6）。
    pub status: Option<&'a str>,
    /// charset（本仕様は [`Charset::Utf8`] 固定・要件 1.6）。
    pub charset: Charset,
}

/// SHIORI/3.0 request バイト列を組み立てる（CRLF 区切り・空行終端・UTF-8・要件 1.1〜1.6）。
///
/// # 組立内容（design.md §送出ヘッダ最小集合）
/// - request line: `GET SHIORI/3.0` / `NOTIFY SHIORI/3.0`（要件 1.1/1.2）
/// - `Charset: UTF-8`（要件 1.6）
/// - `Sender: <sender>`（`req.sender` をそのまま・単一差替点）
/// - `Status: <status>`（`req.status` が `Some` のときのみ・`Sender` の後・`ID` の前・DD-IT-6・要件 2.3）
/// - `ID: <id>`（要件 1.4・汎用・特定イベント分岐なし＝要件 1.5）
/// - `Reference0`・`Reference1`・…（`references` を 0 起点連番で・要件 1.4）
/// - `SecurityLevel: local`（pasta 実テスト準拠・de-facto）
///
/// 各行は CR+LF（0x0D 0x0A）で区切り、ヘッダ部の終端を空行（連続する CR+LF＝末尾
/// 二重 CRLF）で示す（要件 1.3）。`SenderType` / `SecurityOrigin` / `X-SSTP-PassThru`
/// は M1 最小のため送出しない（design.md）。
///
/// 出力は常に有効な UTF-8（`String` からの `into_bytes`・要件 1.6）。
#[must_use]
pub fn build_request(req: &ShioriRequest) -> Vec<u8> {
    let request_line = match req.method {
        Method::Get => "GET SHIORI/3.0",
        Method::Notify => "NOTIFY SHIORI/3.0",
    };

    let mut out = String::new();
    out.push_str(request_line);
    out.push_str("\r\n");
    out.push_str("Charset: ");
    out.push_str(req.charset.header_value());
    out.push_str("\r\n");
    out.push_str("Sender: ");
    out.push_str(req.sender);
    out.push_str("\r\n");
    // `Status` は `Sender` の後・`ID` の前（DD-IT-6）。`None` は行ごと省略（要件 2.3）。
    // 値は解釈せず verbatim に転記する（語彙は kanade が所有・DD-IT-6）。
    if let Some(status) = req.status {
        out.push_str("Status: ");
        out.push_str(status);
        out.push_str("\r\n");
    }
    out.push_str("ID: ");
    out.push_str(req.id);
    out.push_str("\r\n");
    for (n, reference) in req.references.iter().enumerate() {
        // Reference0..N（0 起点連番・要件 1.4）。
        out.push_str("Reference");
        out.push_str(&n.to_string());
        out.push_str(": ");
        out.push_str(reference);
        out.push_str("\r\n");
    }
    out.push_str("SecurityLevel: local\r\n");
    // 空行終端（直前ヘッダの CRLF ＋ この空行 CRLF ＝ 末尾 "\r\n\r\n"・要件 1.3）。
    out.push_str("\r\n");

    out.into_bytes()
}

// ---- response 解析（タスク 2.2）--------------------------------------------

use crate::error::ShioriError;

/// 解析済み SHIORI/3.0 response（status を潰さず保持・要件 2.x）。
///
/// `parse_response` は純粋なワイヤパーサであり、status を verbatim に保持する
/// （200/204/311/312/400/500/その他）。400/500 を `ShioriError::Status` へ写像したり
/// ドロップしたりはしない — その意味論的判断（400/500/`ErrorLevel` → エラー）は
/// 呼び手（`Shiori3Client::get`・タスク 3）が `status` を検分して行う。これにより
/// codec を純粋に保ち、client 側に timeout と SHIORI エラーの区別所有権を委ねる
/// （design.md §shiori3 codec / §Error Strategy）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedResponse {
    /// ステータスコード（200/204/311/312/400/500/その他・verbatim 保持・要件 2.1〜2.7）。
    pub status: u16,
    /// `Value` ヘッダ値（200 時にあり得る・欠落し得る＝`None`・要件 2.1）。
    pub value: Option<String>,
    /// `ErrorLevel` ヘッダ（SSP 拡張・存在時のみ `Some`・要件 2.5）。
    pub error_level: Option<String>,
    /// `ErrorDescription` ヘッダ（SSP 拡張・存在時のみ `Some`・要件 2.5）。
    pub error_description: Option<String>,
}

/// response バイト列を解析する（CRLF/LF 受理・寛容・要件 2.x）。
///
/// # 返り値
/// - well-formed（status 行から数値コードが取れる）→ `Ok(ParsedResponse{..})`。
///   400/500/311/312 を **含めて** status を verbatim に保持する（要件 2.4/2.7）。
/// - malformed（status 行が欠落・数値コードが取れない・不正 UTF-8）→ `Err(ShioriError::Parse)`。
///
/// 本関数は `ShioriError::Status` を **返さない**。非成功 status の意味論的写像は
/// 呼び手（client・タスク 3）の責務であり、ここでは wire の忠実な転記に徹する。
///
/// # 解析規則（design.md §Data Models 受信寛容集合・要件 2.1〜2.8）
/// - 行区切りは CR+LF を第一に、bare LF にも頑健（donor `parse_value` と同じ二段 split）。
/// - status 行（先頭行）: `SHIORI/3.0 <code> <reason>`。`<code>` を `u16` として抽出。
///   数値コードが取れなければ malformed（要件 malformed・fail fast）。
/// - ヘッダ行: `名前: 値`。名前・値は前後空白をトリム。ヘッダ名の一致は
///   大文字小文字を無視（donor 準拠の堅牢性）。
/// - `Value` → `value`（要件 2.1）。`ErrorLevel`/`ErrorDescription` → 各 `Option`（要件 2.5）。
/// - 未知ヘッダ（`Reference0`/`Marker`/任意）は無視し parse を失敗させない（要件 2.8）。
/// - `Charset` ヘッダ省略時は `request_charset` を継承（本仕様は UTF-8 のみ・要件 2.6）。
///   `request_charset` は charset シームとして受け取るが、本仕様の唯一の実符号化は UTF-8 で
///   あり、Shift_JIS 復号は実装しない（シームのみ）。
///
/// NUL 終端に依存せず、与えられたバイト長で解析する（len 厳守）。
pub fn parse_response(bytes: &[u8], request_charset: Charset) -> Result<ParsedResponse, ShioriError> {
    // charset シーム: 本仕様は UTF-8 のみ実符号化。request_charset は継承先の宣言だが、
    // Utf8 以外の variant は存在しないため（Shift_JIS はシームのみ）、UTF-8 として復号する。
    match request_charset {
        Charset::Utf8 => {}
    }

    // UTF-8 として復号。不正バイトは lossy にせず malformed として fail fast（要件: silent 失敗禁止）。
    let text = std::str::from_utf8(bytes).map_err(|_| ShioriError::Parse)?;

    // CRLF を第一に、bare LF にも頑健な行分割（donor `parse_value` と同じ二段 split）。
    let mut lines = text.split("\r\n").flat_map(|l| l.split('\n'));

    // status 行（先頭行）から数値コードを抽出。取れなければ malformed。
    let status_line = lines.next().ok_or(ShioriError::Parse)?;
    let status = parse_status_code(status_line).ok_or(ShioriError::Parse)?;

    let mut value = None;
    let mut error_level = None;
    let mut error_description = None;

    for line in lines {
        // ヘッダ以外（空行・終端）は素通し。`名前: 値` のみ処理。
        let Some((name, val)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        let val = val.trim();
        // ヘッダ名一致は大文字小文字無視（堅牢性）。既知ヘッダ以外は無視（要件 2.8）。
        if name.eq_ignore_ascii_case("Value") {
            value = Some(val.to_string());
        } else if name.eq_ignore_ascii_case("ErrorLevel") {
            error_level = Some(val.to_string());
        } else if name.eq_ignore_ascii_case("ErrorDescription") {
            error_description = Some(val.to_string());
        }
        // Charset / Reference0 / Marker / 未知ヘッダ等は読み飛ばす（要件 2.6/2.8）。
    }

    Ok(ParsedResponse {
        status,
        value,
        error_level,
        error_description,
    })
}

/// status 行 `SHIORI/3.0 <code> <reason>` から数値ステータスコードを抽出する。
///
/// バージョントークン（`SHIORI/3.0`）の直後の空白区切りトークンを `u16` として解釈する。
/// 数値コードが見つからなければ `None`（malformed）。前後の空白には頑健。
fn parse_status_code(status_line: &str) -> Option<u16> {
    // 空白区切りトークン列から、最初に u16 としてパースできるトークンを status とみなす。
    // 通常は 2 番目のトークン（`SHIORI/3.0` の次）だが、`SHIORI/3.0` 自体は
    // `/` `.` を含み u16 化に失敗するため、素朴な first-parseable で安全に拾える。
    status_line
        .split_whitespace()
        .find_map(|tok| tok.parse::<u16>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ShioriError;

    // ---- parse_response（タスク 2.2）テスト --------------------------------

    /// 200 + Value: さくらスクリプト本体（多バイト UTF-8）が `value: Some(..)` に抽出される（要件 2.1）。
    #[test]
    fn parse_200_extracts_value() {
        let body = r"\w0\h\s[0]こんにちは世界\e";
        let resp = format!(
            "SHIORI/3.0 200 OK\r\nCharset: UTF-8\r\nSender: pasta\r\nValue: {body}\r\n\r\n"
        );
        let parsed = parse_response(resp.as_bytes(), Charset::Utf8).expect("well-formed 200");
        assert_eq!(
            parsed,
            ParsedResponse {
                status: 200,
                value: Some(body.to_string()),
                error_level: None,
                error_description: None,
            }
        );
    }

    /// 200 だが Value ヘッダが無い場合は `value: None`（要件 2.1・Value は 200 でも欠落し得る）。
    #[test]
    fn parse_200_without_value_is_none() {
        let resp = "SHIORI/3.0 200 OK\r\nCharset: UTF-8\r\nSender: pasta\r\n\r\n";
        let parsed = parse_response(resp.as_bytes(), Charset::Utf8).expect("well-formed 200");
        assert_eq!(parsed.status, 200);
        assert_eq!(parsed.value, None);
    }

    /// 204 No Content: status=204・value=None を保持（要件 2.2・成功だがスクリプト無しを区別）。
    #[test]
    fn parse_204_no_content() {
        let resp = "SHIORI/3.0 204 No Content\r\nCharset: UTF-8\r\nSender: pasta\r\n\r\n";
        let parsed = parse_response(resp.as_bytes(), Charset::Utf8).expect("well-formed 204");
        assert_eq!(parsed.status, 204);
        assert_eq!(parsed.value, None);
    }

    /// 400 Bad Request: parse_response は Err にせず status=400 を Ok で保持（要件 2.4・意味判断は client）。
    #[test]
    fn parse_400_is_ok_status_preserved() {
        let resp = "SHIORI/3.0 400 Bad Request\r\nCharset: UTF-8\r\nSender: pasta\r\n\r\n";
        let parsed = parse_response(resp.as_bytes(), Charset::Utf8).expect("400 is a parseable response, not a parse error");
        assert_eq!(parsed.status, 400);
    }

    /// 500 Internal Server Error: 同様に Ok で status=500 を保持（要件 2.4）。
    #[test]
    fn parse_500_is_ok_status_preserved() {
        let resp = "SHIORI/3.0 500 Internal Server Error\r\nCharset: UTF-8\r\nSender: pasta\r\n\r\n";
        let parsed = parse_response(resp.as_bytes(), Charset::Utf8).expect("500 is parseable");
        assert_eq!(parsed.status, 500);
    }

    /// 311/312（OnTeach 系）: drop せず status で区別可能に Ok 保持（要件 2.7）。
    #[test]
    fn parse_311_and_312_not_dropped() {
        for code in [311u16, 312u16] {
            let resp = format!("SHIORI/3.0 {code} Teach\r\nCharset: UTF-8\r\nSender: pasta\r\n\r\n");
            let parsed = parse_response(resp.as_bytes(), Charset::Utf8).expect("3xx parseable");
            assert_eq!(parsed.status, code);
        }
    }

    /// ErrorLevel / ErrorDescription（SSP 拡張）が保持される（要件 2.5）。
    #[test]
    fn parse_error_level_and_description_preserved() {
        let resp = "SHIORI/3.0 500 Internal Server Error\r\nCharset: UTF-8\r\nErrorLevel: critical\r\nErrorDescription: boom happened\r\n\r\n";
        let parsed = parse_response(resp.as_bytes(), Charset::Utf8).expect("parseable");
        assert_eq!(parsed.status, 500);
        assert_eq!(parsed.error_level.as_deref(), Some("critical"));
        assert_eq!(parsed.error_description.as_deref(), Some("boom happened"));
    }

    /// 未知ヘッダ（Reference0 / Marker / X-Weird）が混在しても parse は成功し Value を抽出できる（要件 2.8）。
    #[test]
    fn parse_unknown_headers_tolerated() {
        let resp = "SHIORI/3.0 200 OK\r\nCharset: UTF-8\r\nSender: pasta\r\nReference0: x\r\nMarker: y\r\nX-Weird: z\r\nValue: \\s[0]hi\\e\r\n\r\n";
        let parsed = parse_response(resp.as_bytes(), Charset::Utf8).expect("unknown headers must not fail parse");
        assert_eq!(parsed.status, 200);
        assert_eq!(parsed.value.as_deref(), Some(r"\s[0]hi\e"));
    }

    /// Charset ヘッダ省略時も request_charset（UTF-8）を継承して UTF-8 として解析できる（要件 2.6）。
    #[test]
    fn parse_charset_omitted_inherits_request_charset() {
        // Charset ヘッダなし・多バイト UTF-8 body。
        let resp = "SHIORI/3.0 200 OK\r\nSender: pasta\r\nValue: 日本語\r\n\r\n";
        let parsed = parse_response(resp.as_bytes(), Charset::Utf8).expect("UTF-8 inherited");
        assert_eq!(parsed.status, 200);
        assert_eq!(parsed.value.as_deref(), Some("日本語"));
    }

    /// bare-LF（\n のみ）の応答にも頑健（要件 2.3・CRLF/LF 両受理）。
    #[test]
    fn parse_bare_lf_robust() {
        let resp = "SHIORI/3.0 200 OK\nCharset: UTF-8\nValue: hi\n\n";
        let parsed = parse_response(resp.as_bytes(), Charset::Utf8).expect("bare LF parseable");
        assert_eq!(parsed.status, 200);
        assert_eq!(parsed.value.as_deref(), Some("hi"));
    }

    /// 空バイト列は malformed（status 行が取れない）→ Err(Parse)（malformed・fail fast）。
    #[test]
    fn parse_empty_is_parse_error() {
        let err = parse_response(b"", Charset::Utf8).expect_err("empty must be a parse error");
        assert!(matches!(err, ShioriError::Parse));
    }

    /// status コードを含まない先頭行（GARBAGE）は malformed → Err(Parse)（過剰寛容を防ぐ）。
    #[test]
    fn parse_garbage_status_line_is_parse_error() {
        let err = parse_response(b"GARBAGE\r\nValue: x\r\n\r\n", Charset::Utf8)
            .expect_err("no numeric status must be a parse error");
        assert!(matches!(err, ShioriError::Parse));
    }

    /// 不正 UTF-8 バイト列は malformed → Err(Parse)（silent lossy にしない）。
    #[test]
    fn parse_invalid_utf8_is_parse_error() {
        // 0xFF は UTF-8 として不正。
        let err = parse_response(&[0xFF, 0xFE, 0x00], Charset::Utf8)
            .expect_err("invalid UTF-8 must be a parse error");
        assert!(matches!(err, ShioriError::Parse));
    }

    /// GET（Reference 1 件）: request line・必須ヘッダ・Reference0・空行終端を検証（要件 1.1/1.3/1.4/1.6）。
    #[test]
    fn build_get_with_one_reference() {
        let references = vec!["1".to_string()];
        let req = ShioriRequest {
            method: Method::Get,
            id: "OnBoot",
            references: &references,
            sender: "areka",
            status: None,
            charset: Charset::Utf8,
        };
        let bytes = build_request(&req);
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
            sender: "areka",
            status: None,
            charset: Charset::Utf8,
        };
        let bytes = build_request(&req);
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
            sender: "areka",
            status: None,
            charset: Charset::Utf8,
        };
        let bytes = build_request(&req);
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
            sender: "areka",
            status: None,
            charset: Charset::Utf8,
        };
        let bytes = build_request(&req);
        let s = std::str::from_utf8(&bytes).expect("valid UTF-8");
        assert!(!s.contains("Reference"), "unexpected Reference header:\n{s}");
        assert!(s.ends_with("\r\n\r\n"), "request:\n{s}");
    }

    /// 多バイト UTF-8 の Reference 値が UTF-8 バイトとして round-trip する（要件 1.6）。
    #[test]
    fn build_multibyte_reference_roundtrips_as_utf8() {
        let references = vec!["こんにちは世界".to_string()];
        let req = ShioriRequest {
            method: Method::Get,
            id: "OnTest",
            references: &references,
            sender: "areka",
            status: None,
            charset: Charset::Utf8,
        };
        let bytes = build_request(&req);
        let s = std::str::from_utf8(&bytes).expect("output must be valid UTF-8");
        assert!(s.contains("Reference0: こんにちは世界\r\n"), "request:\n{s}");
    }

    /// 汎用 ID: 任意の ID（リソース照会系含む）が特別扱いなく `ID: <that>` になる（要件 1.5）。
    #[test]
    fn build_generic_id_no_special_casing() {
        for id in ["OnSecondChange", "version", "name", "OnMouseClick"] {
            let req = ShioriRequest {
                method: Method::Get,
                id,
                references: &[],
                sender: "areka",
                status: None,
                charset: Charset::Utf8,
            };
            let bytes = build_request(&req);
            let s = std::str::from_utf8(&bytes).expect("valid UTF-8");
            assert!(s.contains(&format!("ID: {id}\r\n")), "id={id} request:\n{s}");
        }
    }

    /// Sender は `req.sender` をそのまま書く単一差替点（`"SSP"` 等ハードコードしない・design.md）。
    #[test]
    fn build_sender_is_verbatim_substitution_point() {
        let req = ShioriRequest {
            method: Method::Get,
            id: "OnBoot",
            references: &[],
            sender: "custom-baseware",
            status: None,
            charset: Charset::Utf8,
        };
        let bytes = build_request(&req);
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
            sender: "areka",
            status: Some("talking"),
            charset: Charset::Utf8,
        };
        let bytes = build_request(&req);
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
            sender: "areka",
            status: None,
            charset: Charset::Utf8,
        };
        let bytes = build_request(&req);
        let s = std::str::from_utf8(&bytes).expect("valid UTF-8");
        assert!(!s.contains("Status:"), "None なら Status 行を出してはならない:\n{s}");
    }

    /// Status 値は verbatim に写る（codec は解釈/分割/整形しない・DD-IT-6 語彙非漏洩）。
    #[test]
    fn build_status_value_is_verbatim() {
        let value = "talking,balloon(0=2/1=0)";
        let req = ShioriRequest {
            method: Method::Get,
            id: "OnSecondChange",
            references: &[],
            sender: "areka",
            status: Some(value),
            charset: Charset::Utf8,
        };
        let bytes = build_request(&req);
        let s = std::str::from_utf8(&bytes).expect("valid UTF-8");
        // カンマ・括弧・スラッシュ・等号を含む値がそのまま 1 行に載る（解釈しない）。
        assert!(
            s.contains("Status: talking,balloon(0=2/1=0)\r\n"),
            "Status 値は verbatim でなければならない:\n{s}"
        );
        // 語彙非漏洩の確認: 値に "Reference" が無い限り Reference 行を捏造しない（既存檻と非衝突）。
        assert!(!s.contains("Reference"), "Status 値が Reference 行を汚染してはならない:\n{s}");
    }
}
