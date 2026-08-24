//! SHIORI/3.0 リクエスト解析と応答分類の純粋ロジック（task 1.2）。
//!
//! 本モジュールは `IShiori::Get` が駆動する 2 段の純関数を提供する（design.md
//! "ReplayBrain・ReplayFactory" の `Get` 責務・i686 `shiori-host32-testdll` の
//! `parse_request`／`select_response` の x64/COM 版）:
//!
//! 1. [`parse_request`] — 受領 SHIORI/3.0 テキストから **request line の別（GET/NOTIFY/その他）**と
//!    **`ID:` ヘッダ値**のみを取り出す（design.md D-4「content 解析範囲: ID 行のみ読む」・
//!    References は不参照）。
//! 2. [`select_response`] — 解析結果を **収載（Snapshot）／未知（NoContent=204 相当）／
//!    構造不整合（BadRequest=400 相当）**の 3 分岐へ分類する（要件 2.2／2.3／2.4）。
//!    収載 ID → 凍結応答の解決は task 1.3 の責務ゆえ、`lookup` クロージャで注入する（脱結合）。
//!
//! いずれも panic しない総和的な純関数であり、malformed 入力は panic でなく
//! [`Selection::BadRequest`]（400 相当）として fail-visible に扱う（要件 2.4）。
//!
//! 本モジュールの項目は crate 内消費（`pub(crate)`）。task 1.4 で `lib.rs` の `ReplayBrain::Get`
//! が [`parse_request`]→[`select_response`]→[`response_text`] を呼び、`lookup` に task 1.3 の
//! `snapshot::snapshot_for` を無改修で差し込む（結線済み）。判定分岐は単体テストで全網羅する
//! （下記 `#[cfg(test)]`）。

/// SHIORI/3.0 request line の種別（design.md D-4: GET/NOTIFY の別を読む）。
///
/// request line 先頭トークン（メソッド名）で分類する。`GET`／`NOTIFY` 以外（空行・
/// garbage 先頭行・未知メソッド）は [`RequestLine::Other`]＝構造不整合の面。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequestLine {
    /// `GET SHIORI/3.0` 系（メソッド＝GET）。
    Get,
    /// `NOTIFY SHIORI/3.0` 系（メソッド＝NOTIFY）。
    Notify,
    /// GET／NOTIFY いずれでもない（空・garbage・未知メソッド）＝構造不整合の面。
    Other,
}

/// [`parse_request`] の結果（design.md D-4 の解析範囲＝request line の別＋`ID:` のみ）。
///
/// References／その他ヘッダは保持しない（不透明搬送・要件 2.5／D-4）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedRequest {
    /// request line の種別。
    pub(crate) line: RequestLine,
    /// `ID:` ヘッダ値（ヘッダ部に `ID` が無ければ `None`）。最初に一致した `ID` を採用する。
    pub(crate) id: Option<String>,
}

/// [`select_response`] の 3 分岐判定（要件 2.2／2.3／2.4）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Selection {
    /// 収載 ID → `lookup` が解決した凍結応答全文（要件 2.2）。task 1.3 のスナップショットデータ由来。
    Snapshot(&'static str),
    /// 未知・未収載 ID → 204 相当（silent success でなく明示的 No Content・要件 2.3）。
    NoContent,
    /// 構造不整合（GET request line 不在・ID 欠落等）→ 400 相当（fail-visible・panic しない・要件 2.4）。
    BadRequest,
}

/// 未知・未収載 ID への合成 204 応答 envelope（要件 2.3）。
///
/// これは判定応答であって**スナップショットではない**（凍結応答は task 1.3 の `SnapshotTable` 所有）。
pub(crate) const RESP_204_NO_CONTENT: &str = "SHIORI/3.0 204 No Content\r\n\r\n";

/// 構造不整合への合成 400 応答 envelope（要件 2.4）。
///
/// これは判定応答であって**スナップショットではない**（凍結応答は task 1.3 の `SnapshotTable` 所有）。
pub(crate) const RESP_400_BAD_REQUEST: &str = "SHIORI/3.0 400 Bad Request\r\n\r\n";

/// 受領 SHIORI/3.0 テキストから request line の別と `ID:` ヘッダ値を取り出す純関数（design.md D-4）。
///
/// - 行区切りは CRLF／LF いずれも許容（`\r` 除去→`\n` 分割・i686 testdll と同型）。
/// - request line 種別は先頭行の第 1 トークン（メソッド名）で分類（`GET`／`NOTIFY`／その他）。
/// - ヘッダ名照合は case-insensitive（`ID`／`id`／`Id` を同一視）。
/// - 空行（ヘッダ部終端）以降は走査しない（空行より後の `ID` は無視）。
/// - References／その他ヘッダは読まない（不透明搬送・要件 2.5／D-4）。
///
/// panic しない純関数。
pub(crate) fn parse_request(input: &str) -> ParsedRequest {
    // CRLF/LF 両対応: \r を除去してから \n 分割する（i686 testdll と同型）。
    let normalized = input.replace('\r', "");
    let mut lines = normalized.split('\n');

    // request line = 先頭行。第 1 トークン（メソッド名）で種別を分類する。
    let request_line = lines.next().unwrap_or("").trim();
    let method = request_line.split_whitespace().next().unwrap_or("");
    let line = match method {
        "GET" => RequestLine::Get,
        "NOTIFY" => RequestLine::Notify,
        _ => RequestLine::Other,
    };

    // ヘッダ部から `ID` を case-insensitive に拾う。空行（終端）以降は走査しない。
    let mut id: Option<String> = None;
    for header in lines {
        if header.is_empty() {
            // ヘッダ部終端。以降は走査しない。
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.trim().eq_ignore_ascii_case("ID")
        {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                // 最初に一致した非空 ID を採用する。
                id = Some(trimmed.to_string());
                break;
            }
        }
    }

    ParsedRequest { line, id }
}

/// 解析結果を収載／未知／構造不整合の 3 分岐へ分類する純関数（要件 2.2／2.3／2.4）。
///
/// `lookup` は収載 ID → 凍結応答全文を返す注入関数（task 1.3 の `snapshot::snapshot_for` を
/// 無改修で差し込めるよう脱結合する）。
///
/// GET 経路（`IShiori::Get` が駆動）の判定規則:
/// - request line が GET かつ `ID` 有り かつ `lookup(id) == Some(resp)` → [`Selection::Snapshot`]（収載・2.2）
/// - request line が GET かつ `ID` 有り かつ `lookup(id) == None` → [`Selection::NoContent`]（未知/未収載・2.3）
/// - request line が GET だが `ID` 欠落 → [`Selection::BadRequest`]（構造不整合・2.4）
/// - request line が GET でない（NOTIFY／Other／空）→ [`Selection::BadRequest`]（2.4）
///
/// NOTIFY line が本 GET 経路 selector へ到達した場合は構造不整合（BadRequest）として扱う。
/// 正典の NOTIFY 受領（204）は別メソッド `IShiori::Notify`（task 1.4）が担う（design.md
/// SnapshotTable narrowing）ため、本 GET 経路に NOTIFY は想定されない。
///
/// panic しない純関数。
pub(crate) fn select_response(
    parsed: &ParsedRequest,
    lookup: impl Fn(&str) -> Option<&'static str>,
) -> Selection {
    match (parsed.line, parsed.id.as_deref()) {
        // GET かつ ID 有り: lookup で収載（Snapshot）／未知（204）を分岐する。
        (RequestLine::Get, Some(id)) => match lookup(id) {
            Some(response) => Selection::Snapshot(response),
            None => Selection::NoContent,
        },
        // GET だが ID 欠落＝構造不整合（正典イベント要求になり得ない・要件 2.4）。
        (RequestLine::Get, None) => Selection::BadRequest,
        // 非 GET（NOTIFY／Other／空）は GET 経路 selector では構造不整合（要件 2.4）。
        // 正典 NOTIFY 受領（204）は別メソッド Notify（task 1.4）が担う。
        (RequestLine::Notify, _) | (RequestLine::Other, _) => Selection::BadRequest,
    }
}

/// [`Selection`] を最終応答テキストへ解決する純関数。
///
/// 収載は凍結応答全文、未知は [`RESP_204_NO_CONTENT`]、構造不整合は [`RESP_400_BAD_REQUEST`]。
pub(crate) fn response_text(selection: &Selection) -> &'static str {
    match selection {
        Selection::Snapshot(response) => response,
        Selection::NoContent => RESP_204_NO_CONTENT,
        Selection::BadRequest => RESP_400_BAD_REQUEST,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- parse_request の解析檻（design.md D-4） -------------------------------------------------

    #[test]
    fn parse_get_request_line_and_id() {
        let req = "GET SHIORI/3.0\r\nCharset: UTF-8\r\nSender: areka\r\nID: OnBoot\r\n\r\n";
        let parsed = parse_request(req);
        assert_eq!(parsed.line, RequestLine::Get);
        assert_eq!(parsed.id.as_deref(), Some("OnBoot"));
    }

    #[test]
    fn parse_notify_request_line_discriminated_from_get() {
        let req = "NOTIFY SHIORI/3.0\r\nID: OnInitialize\r\n\r\n";
        let parsed = parse_request(req);
        assert_eq!(parsed.line, RequestLine::Notify);
        assert_eq!(parsed.id.as_deref(), Some("OnInitialize"));
    }

    #[test]
    fn parse_lf_only_line_endings_tolerated() {
        let req = "GET SHIORI/3.0\nID: OnBoot\n\n";
        let parsed = parse_request(req);
        assert_eq!(parsed.line, RequestLine::Get);
        assert_eq!(parsed.id.as_deref(), Some("OnBoot"));
    }

    #[test]
    fn parse_id_header_name_case_insensitive() {
        let req = "GET SHIORI/3.0\r\nid: OnBoot\r\n\r\n";
        assert_eq!(parse_request(req).id.as_deref(), Some("OnBoot"));
        let req2 = "GET SHIORI/3.0\r\nId: OnBoot\r\n\r\n";
        assert_eq!(parse_request(req2).id.as_deref(), Some("OnBoot"));
    }

    #[test]
    fn parse_ignores_id_after_blank_header_terminator() {
        // 空行以降の ID: 行は走査しない（ヘッダ部終端）。
        let req = "GET SHIORI/3.0\r\n\r\nID: OnBoot\r\n";
        let parsed = parse_request(req);
        assert_eq!(parsed.line, RequestLine::Get);
        assert_eq!(parsed.id, None, "空行より後の ID は無視される");
    }

    #[test]
    fn parse_garbage_first_line_is_other() {
        let parsed = parse_request("BOGUS LINE\r\nID: OnBoot\r\n\r\n");
        assert_eq!(parsed.line, RequestLine::Other);
    }

    #[test]
    fn parse_empty_input_is_other_without_id() {
        let parsed = parse_request("");
        assert_eq!(parsed.line, RequestLine::Other);
        assert_eq!(parsed.id, None);
    }

    #[test]
    fn parse_does_not_read_references() {
        // References ヘッダは解析対象外（D-4・要件 2.5）。ID のみ拾う。
        let req = "GET SHIORI/3.0\r\nReference0: foo\r\nReference1: bar\r\nID: OnBoot\r\n\r\n";
        let parsed = parse_request(req);
        assert_eq!(parsed.id.as_deref(), Some("OnBoot"));
    }

    // ---- select_response の 3 分岐檻（要件 2.2／2.3／2.4・fake lookup で脱結合） -----------------

    /// 収載 GET id（fake lookup が Some を返す）→ Snapshot（凍結応答全文をそのまま搬送）。
    #[test]
    fn select_listed_get_id_returns_snapshot() {
        let parsed = ParsedRequest {
            line: RequestLine::Get,
            id: Some("OnBoot".to_string()),
        };
        let lookup = |id: &str| {
            if id == "OnBoot" {
                Some("SHIORI/3.0 200 OK\r\nValue: hi\r\n\r\n")
            } else {
                None
            }
        };
        let sel = select_response(&parsed, lookup);
        assert_eq!(
            sel,
            Selection::Snapshot("SHIORI/3.0 200 OK\r\nValue: hi\r\n\r\n")
        );
        assert_eq!(
            response_text(&sel),
            "SHIORI/3.0 200 OK\r\nValue: hi\r\n\r\n"
        );
    }

    /// 未知/未収載 GET id（fake lookup が None）→ NoContent（204 相当・要件 2.3）。
    #[test]
    fn select_unknown_get_id_returns_204() {
        let parsed = ParsedRequest {
            line: RequestLine::Get,
            id: Some("OnNeverSeen".to_string()),
        };
        let sel = select_response(&parsed, |_| None);
        assert_eq!(sel, Selection::NoContent);
        assert_eq!(response_text(&sel), RESP_204_NO_CONTENT);
        assert!(response_text(&sel).starts_with("SHIORI/3.0 204 No Content"));
    }

    /// GET だが ID 欠落 → 構造不整合 400（要件 2.4）。lookup は呼ばれても None を返さない fake で
    /// 誤って Snapshot にならないことを確認。
    #[test]
    fn select_get_without_id_is_bad_request() {
        let parsed = ParsedRequest {
            line: RequestLine::Get,
            id: None,
        };
        let sel = select_response(&parsed, |_| Some("SHOULD NOT BE USED"));
        assert_eq!(sel, Selection::BadRequest);
        assert_eq!(response_text(&sel), RESP_400_BAD_REQUEST);
    }

    /// 空入力由来（Other・ID なし）→ 400（要件 2.4）。
    #[test]
    fn select_empty_input_is_bad_request() {
        let parsed = parse_request("");
        let sel = select_response(&parsed, |_| Some("nope"));
        assert_eq!(sel, Selection::BadRequest);
        assert!(response_text(&sel).starts_with("SHIORI/3.0 400 Bad Request"));
    }

    /// garbage 先頭行（Other）→ 400（要件 2.4）。ID が既知でも request line が非 GET なら不整合。
    #[test]
    fn select_garbage_request_line_is_bad_request() {
        let parsed = parse_request("BOGUS LINE\r\nID: OnBoot\r\n\r\n");
        let sel = select_response(&parsed, |_| Some("SHOULD NOT BE USED"));
        assert_eq!(sel, Selection::BadRequest);
    }

    /// NOTIFY line が GET 経路 selector へ到達 → 400（正典 NOTIFY 受領は task 1.4 の Notify メソッド）。
    #[test]
    fn select_notify_line_on_get_path_is_bad_request() {
        let parsed = parse_request("NOTIFY SHIORI/3.0\r\nID: OnInitialize\r\n\r\n");
        assert_eq!(parsed.line, RequestLine::Notify);
        let sel = select_response(&parsed, |_| Some("SHOULD NOT BE USED"));
        assert_eq!(sel, Selection::BadRequest);
    }

    /// malformed 入力群で何も panic しないこと（fail-visible via BadRequest・要件 2.4）。
    #[test]
    fn malformed_inputs_never_panic() {
        for raw in [
            "",
            "\r\n\r\n",
            "GET",
            "GET SHIORI/3.0",
            "\n\n\n",
            "GET SHIORI/3.0\r\nID:\r\n\r\n",
            ": : :",
            "GET SHIORI/3.0\r\nGarbageHeaderNoColon\r\n\r\n",
        ] {
            let parsed = parse_request(raw);
            // どんな入力でも 3 分岐のいずれかへ総和的に落ち、panic しない。
            let _ = select_response(&parsed, |_| None);
        }
    }
}
