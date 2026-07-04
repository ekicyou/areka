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
    /// charset（本仕様は [`Charset::Utf8`] 固定・要件 1.6）。
    pub charset: Charset,
}

/// SHIORI/3.0 request バイト列を組み立てる（CRLF 区切り・空行終端・UTF-8・要件 1.1〜1.6）。
///
/// # 組立内容（design.md §送出ヘッダ最小集合）
/// - request line: `GET SHIORI/3.0` / `NOTIFY SHIORI/3.0`（要件 1.1/1.2）
/// - `Charset: UTF-8`（要件 1.6）
/// - `Sender: <sender>`（`req.sender` をそのまま・単一差替点）
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

#[cfg(test)]
mod tests {
    use super::*;

    /// GET（Reference 1 件）: request line・必須ヘッダ・Reference0・空行終端を検証（要件 1.1/1.3/1.4/1.6）。
    #[test]
    fn build_get_with_one_reference() {
        let references = vec!["1".to_string()];
        let req = ShioriRequest {
            method: Method::Get,
            id: "OnBoot",
            references: &references,
            sender: "areka",
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
            charset: Charset::Utf8,
        };
        let bytes = build_request(&req);
        let s = std::str::from_utf8(&bytes).expect("valid UTF-8");
        assert!(s.contains("Sender: custom-baseware\r\n"), "request:\n{s}");
        assert!(!s.contains("Sender: SSP"), "must not hardcode SSP:\n{s}");
    }
}
