//! host-32 x64 側 SHIORI/3.0 ワイヤコーデック（純粋・決定的・`windows` 非依存）。
//!
//! 本モジュールは x64 親プロセス専用の SHIORI/3.0 codec であり、helper（i686）からは
//! 一切参照されない（design.md §Shiori3Codec・research.md §5.4: helper はバイト proxy に
//! 徹し、SHIORI/3.0 の組立と `Value:` parse は x64 親側に閉じる）。
//!
//! 本モジュールが提供するもの:
//! - [`build_request`] — イベント（ID＋References）から SHIORI/3.0 request バイト列を
//!   組み立てる汎用ビルダ。
//! - [`parse_response`] / [`ParsedResponse`] — response バイト列の寛容なワイヤパーサ。
//!
//! ## 設計原則
//! - **汎用ビルダ**: `id` と `references` は verbatim に写す。OnBoot 等の特定イベントに
//!   固有の分岐や既定 Reference を埋め込まない（要件 1.5）。donor `build_onboot`
//!   （OnBoot 決め打ち）を method/id/references/headers 汎用へ一般化したもの。
//! - **文字コードで分岐しない**: 組み立てた全文を `req.charset` の 1 本の経路で符号化し、
//!   `Charset` ヘッダにはその正規名を書く（綴りと実バイト列が常に一致・要件 3.1〜3.3）。
//!   UTF-8 を特別扱いする近道は置かない（areka-P0-charset-canon 要件 1.4）。
//! - **単一差替点**: `Sender` は `ShioriRequest.headers.sender` の値をそのまま書く。ハードコードした
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

use crate::charset::Charset;
use crate::request_headers::RequestHeaders;

/// codec への入力（イベント個別知識を持たない汎用ビルダ入力・要件 1.5）。
///
/// `id` はイベント名（`ID` ヘッダ値）、`references` は `Reference0..N`（0 起点連番で
/// 連番付与）、`headers` は追加のヘッダ（`SecurityLevel`・`SecurityOrigin`・`Sender`・`SenderType`）。
pub struct ShioriRequest<'a> {
    /// request line 種別（`GET` / `NOTIFY`・要件 1.1/1.2）。
    pub method: Method,
    /// `ID` ヘッダ値（イベント名・要件 1.4/1.5）。汎用ゆえ特定イベントの分岐を持たない。
    pub id: &'a str,
    /// `Reference0..N` に 0 起点連番で写す References（要件 1.4）。空なら Reference 行なし。
    pub references: &'a [String],
    /// 追加のヘッダ（解釈しない wire 値・`Sender` を含む単一差替点）。
    pub headers: RequestHeaders<'a>,
    /// `Status` ヘッダの wire 値。`None` ⇒ ヘッダ行を出さない／`Some(v)` ⇒ `Status: v` を1行。
    /// **値は解釈しない**（汎用 codec・語彙は kanade が所有・DD-IT-6）。
    pub status: Option<&'a str>,
    /// 全文の符号化に使う文字コード。`Charset` ヘッダにはこの正規名を書く（要件 3.1/3.2）。
    pub charset: Charset,
}

/// 欠番の印。Reference の並びの中でこの値と一致する位置は、行を出さずに番号だけ進める。
///
/// NUL 1 文字は見出しの値として線に載せられない文字で、実際の Reference の値と重ならない。
/// `OnTranslate` の Reference3 の区切り（バイト値 1）とも重ならない。
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnTranslate:1
pub const ABSENT_REFERENCE: &str = "\u{0}";

/// 符号化済みの要求（2 つの事実）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedRequest {
    /// `req.charset` で符号化した request 全文のバイト列（そのまま helper／IPC へ渡す・要件 3.6）。
    pub bytes: Vec<u8>,
    /// 表せない文字を数値文字参照へ置換した文字数（0 なら置換なし・要件 3.5・10.1）。
    /// 警告ログは呼び手（`CharsetNegotiator::note_request`）が担う。
    pub replaced: usize,
}

/// SHIORI/3.0 request バイト列を組み立てる（CRLF 区切り・空行終端・要件 1.1〜1.5・3.1〜3.5）。
///
/// # 組立内容（design.md §送出ヘッダ最小集合）
/// - request line: `GET SHIORI/3.0` / `NOTIFY SHIORI/3.0`（要件 1.1/1.2）
/// - `Charset: <正規名>`（要求行の直後＝ASCII 範囲外を含み得る行より前・要件 3.2/3.3）
/// - `SecurityLevel`・`SecurityOrigin`・`Sender`・`SenderType`（`req.headers` をこの順にそのまま・
///   解釈しない・`Sender` は単一差替点・areka-P0-script-security-level 要件 2.1）
/// - `Status: <status>`（`req.status` が `Some` のときのみ・`SenderType` の後・`ID` の前・DD-IT-6・要件 2.3）
/// - `ID: <id>`（要件 1.4・汎用・特定イベント分岐なし＝要件 1.5）
/// - `Reference0`・`Reference1`・…（`references` を 0 起点連番で・要件 1.4）。値が
///   [`ABSENT_REFERENCE`] の位置は行を出さず番号だけ進める（欠番）
///
/// 各行は CR+LF（0x0D 0x0A）で区切り、ヘッダ部の終端を空行（連続する CR+LF＝末尾
/// 二重 CRLF）で示す（要件 1.3）。`X-SSTP-PassThru` は送出しない。
///
/// 組み立ては一旦 `String` で行い、**全文をまとめて** `req.charset` で符号化する
/// （ISO-2022-JP でも `Encoding::encode` が終端で ASCII 状態へ戻すため 1 回で足りる）。
/// UTF-8 のときの出力は入力文字列のバイト列と同一で、本仕様適用前と 1 バイトも変わらない
/// （要件 3.4）。
#[must_use]
pub fn build_request(req: &ShioriRequest) -> EncodedRequest {
    let request_line = match req.method {
        Method::Get => "GET SHIORI/3.0",
        Method::Notify => "NOTIFY SHIORI/3.0",
    };

    let mut out = String::new();
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#_30e1_30bd_30c3_30c9:1
    out.push_str(request_line);
    out.push_str("\r\n");
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#Charset:1
    out.push_str("Charset: ");
    out.push_str(req.charset.name());
    out.push_str("\r\n");
    // 追加のヘッダは `Status`・`ID` より前に、この順で値を解釈せず verbatim に書く
    // （areka-P0-script-security-level 要件 2.1・DD-IT-6）。
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#SecurityLevel:1
    out.push_str("SecurityLevel: ");
    out.push_str(req.headers.security_level);
    out.push_str("\r\n");
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#SecurityOrigin_20_5bSSP_62e1_5f35_5d:1
    out.push_str("SecurityOrigin: ");
    out.push_str(req.headers.security_origin);
    out.push_str("\r\n");
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#Sender:1
    out.push_str("Sender: ");
    out.push_str(req.headers.sender);
    out.push_str("\r\n");
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#SenderType_20_5bSSP_202.5.05_7e_62e1_5f35_5d:1
    out.push_str("SenderType: ");
    out.push_str(req.headers.sender_type);
    out.push_str("\r\n");
    // `Status` は `SenderType` の後・`ID` の前（DD-IT-6）。`None` は行ごと省略（要件 2.3）。
    // 値は解釈せず verbatim に転記する（語彙は kanade が所有・DD-IT-6）。
    if let Some(status) = req.status {
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#Status_20_5bSSP_62e1_5f35_5d:1
        out.push_str("Status: ");
        out.push_str(status);
        out.push_str("\r\n");
    }
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#ID:1
    out.push_str("ID: ");
    out.push_str(req.id);
    out.push_str("\r\n");
    push_references(&mut out, req.references);
    // 空行終端（直前ヘッダの CRLF ＋ この空行 CRLF ＝ 末尾 "\r\n\r\n"・要件 1.3）。
    out.push_str("\r\n");

    // 全文を 1 本の経路で符号化する（文字コードで分岐しない・要件 1.4）。UTF-8 では
    // `Cow::Borrowed` が返るので `into_owned` の複製 1 回で済み、バイト列は同一。
    let (bytes, replaced) = req.charset.encode(&out);
    EncodedRequest {
        bytes: bytes.into_owned(),
        replaced,
    }
}

/// `Reference0..N` の行を `out` へ足す（0 起点連番・要件 1.4）。2.x の組み立ても使う。
///
/// 欠番の印（[`ABSENT_REFERENCE`]）の位置は行を出さず、番号だけ進める（詰めない）。
// ukadoc: https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#Reference_2a:1
pub(crate) fn push_references(out: &mut String, references: &[String]) {
    for (n, reference) in references.iter().enumerate() {
        if reference == ABSENT_REFERENCE {
            continue;
        }
        out.push_str("Reference");
        out.push_str(&n.to_string());
        out.push_str(": ");
        out.push_str(reference);
        out.push_str("\r\n");
    }
}

// ---- response 解析 ----------------------------------------------------------

use crate::charset::CharsetPolicy;
use crate::error::ShioriError;

/// 解析済み SHIORI/3.0 response（status を潰さず保持・要件 2.x）。
///
/// `parse_response` は純粋なワイヤパーサであり、status を verbatim に保持する
/// （200/204/311/312/400/500/その他）。400/500 を `ShioriError::Status` へ写像したり
/// ドロップしたりはしない — その意味論的判断（400/500/`ErrorLevel` → エラー）は
/// 呼び手（`Shiori3Client::get`）が `status` を検分して行う。これにより
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
    /// 応答の `Charset` ヘッダの生の値（前後空白を落とした値・省略時 `None`）。
    ///
    /// **採用するかの判断はここでは行わない**——解決も正規化もせず生の綴りを返すだけで、
    /// 採用と後退の規則は `CharsetNegotiator::note_response`（`charset.rs`）が所有する
    /// （areka-P0-charset-canon 要件 4.1/4.2/4.4）。
    pub charset_header: Option<String>,
    /// 復号で代替文字（U+FFFD）への置換があったか（areka-P0-charset-canon 要件 4.7・10.2）。
    ///
    /// 置換があっても解析は続行する。警告ログは呼び手（`note_response`）が担う。
    pub decode_had_errors: bool,
    /// 応答の `SecurityLevel` ヘッダの生の値（前後空白を落とした値・無ければ `None`）。
    ///
    /// 解釈しない（`local`／`external` の判定と読めない値の warn は kanade・
    /// areka-P0-script-security-level 要件 3.5）。
    pub security_level: Option<String>,
}

/// 復号より先に、応答の `Charset` ヘッダの生の値をバイト列から走査する（要件 4.1）。
///
/// 行を LF で切り、行末の CR を落とし、`:` の手前をヘッダ名として ASCII 一致で比べる。
/// **位置は問わない**（正典は「望ましい」位置を示すのみ）。最初に一致した行の値を
/// 前後空白を落として返し、`Charset` 行が無ければ `None`。
///
/// 復号前ゆえバイト列のまま走るが、ASCII 走査で足りる: 対応集合（`Charset`）には UTF-16 系が
/// 入らないためヘッダ名の並ぶ領域は常に ASCII 互換である（ISO-2022-JP も ASCII 状態から
/// 始まる・design.md §走査が ASCII で成り立つ根拠）。非 ASCII を含む値は
/// `from_utf8_lossy` で文字列化され、`Charset::for_label` が解決できないラベルとして退ける。
fn scan_charset_header(bytes: &[u8]) -> Option<String> {
    for line in bytes.split(|&b| b == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let Some(colon) = line.iter().position(|&b| b == b':') else {
            continue;
        };
        if line[..colon].trim_ascii().eq_ignore_ascii_case(b"charset") {
            // ukadoc: https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#Charset:2
            return Some(
                String::from_utf8_lossy(&line[colon + 1..])
                    .trim()
                    .to_owned(),
            );
        }
    }
    None
}

/// response バイト列を解析する（CRLF/LF 受理・寛容・要件 2.x）。
///
/// # 返り値
/// - well-formed（status 行から数値コードが取れる）→ `Ok(ParsedResponse{..})`。
///   400/500/311/312 を **含めて** status を verbatim に保持する（要件 2.4/2.7）。
/// - malformed → `Err(ShioriError::Parse)`。**理由は status 行の欠落・数値コードなしだけ**で、
///   不正なバイト並びでは失敗しない（代替文字で吸収して続行・areka-P0-charset-canon
///   要件 4.7・10.2）。
///
/// 本関数は `ShioriError::Status` を **返さない**。非成功 status の意味論的写像は
/// 呼び手（client）の責務であり、ここでは wire の忠実な転記に徹する。
///
/// # 復号（areka-P0-charset-canon 要件 4.1〜4.4）
/// ⑴ 復号より先に [`scan_charset_header`] で `Charset` ヘッダの生の値を拾う（位置不問）。
/// ⑵ 復号に使う文字コードを `policy` から決める——[`CharsetPolicy::Negotiate`] はヘッダが
///    `Charset::for_label` で解決できればそれ・できなければ方針の中の値、
///    [`CharsetPolicy::Force`] は常に方針の中の値（ヘッダを復号に用いない・要件 4.5/5.4）。
/// ⑶ 全体を 1 本の経路で復号する（文字コードで分岐しない）。BOM は吸収せず 1 文字として
///    残る（`Charset::decode` は `decode_without_bom_handling` 相当。宣言と食い違う BOM で
///    交渉結果を黙って覆さないため）。
/// ⑷ 以降は下記の行解析を無変更で通す。
///
/// 採用（以後の要求にどの文字コードを使うか）はここでは決めない。生の値と置換の有無を
/// 事実として返し、`CharsetNegotiator::note_response` が判断する。
///
/// # 解析規則（design.md §Data Models 受信寛容集合・要件 2.1〜2.8）
/// - 行区切りは CR+LF を第一に、bare LF にも頑健（donor `parse_value` と同じ二段 split）。
/// - status 行（先頭行）: `SHIORI/3.0 <code> <reason>`。`<code>` を `u16` として抽出。
///   数値コードが取れなければ malformed（要件 malformed・fail fast）。
/// - ヘッダ行: `名前: 値`。名前・値は前後空白をトリム。ヘッダ名の一致は
///   大文字小文字を無視（donor 準拠の堅牢性）。
/// - `Value` → `value`（要件 2.1）。`ErrorLevel`/`ErrorDescription` → 各 `Option`（要件 2.5）。
///   `SecurityLevel` → `security_level`（生の値・areka-P0-script-security-level 要件 3.5）。
/// - 未知ヘッダ（`Reference0`/`Marker`/任意）は無視し parse を失敗させない（要件 2.8）。
/// - `Charset` ヘッダ省略時は方針の中の文字コード（＝要求に用いた文字コード）を継承（要件 2.6）。
///
/// NUL 終端に依存せず、与えられたバイト長で解析する（len 厳守）。
pub fn parse_response(bytes: &[u8], policy: CharsetPolicy) -> Result<ParsedResponse, ShioriError> {
    parse_response_with(bytes, policy, "Value")
}

/// [`parse_response`] の本体。値の見出しの名前だけを `value_header` で受け取る
/// （3.0 は `Value`・2.x は `Sentence` など・shiori2-protocol 要件 4.1〜4.3）。
/// 状態の番号・`Charset` の拾い方・読めない応答の判定は 3.0 と同じ。
pub(crate) fn parse_response_with(
    bytes: &[u8],
    policy: CharsetPolicy,
    value_header: &str,
) -> Result<ParsedResponse, ShioriError> {
    // ⑴ 復号前にヘッダの生の値を走査する（位置不問・要件 4.1）。
    let charset_header = scan_charset_header(bytes);

    // ⑵ 復号に使う文字コードを方針から決める（要件 4.2〜4.5）。
    let charset = match policy {
        CharsetPolicy::Negotiate(inherited) => charset_header
            .as_deref()
            .and_then(|label| Charset::for_label(label).ok())
            .unwrap_or(inherited),
        CharsetPolicy::Force(forced) => forced,
    };

    // ⑶ 全体を復号。不正な並びは代替文字へ吸収し、解析は続行する（要件 4.7・10.2）。
    let (text, decode_had_errors) = charset.decode(bytes);

    // ⑷ 以降は行解析。CRLF を第一に、bare LF にも頑健（donor `parse_value` と同じ二段 split）。
    let mut lines = text.split("\r\n").flat_map(|l| l.split('\n'));

    // status 行（先頭行）から数値コードを抽出。取れなければ malformed。
    let status_line = lines.next().ok_or(ShioriError::Parse)?;
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#_30b9_30c6_30fc_30bf_30b9_30b3_30fc_30c9:1
    let status = parse_status_code(status_line).ok_or(ShioriError::Parse)?;

    let mut value = None;
    let mut error_level = None;
    let mut error_description = None;
    let mut security_level = None;

    for line in lines {
        // ヘッダ以外（空行・終端）は素通し。`名前: 値` のみ処理。
        let Some((name, val)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        let val = val.trim();
        // ヘッダ名一致は大文字小文字無視（堅牢性）。既知ヘッダ以外は無視（要件 2.8）。
        if name.eq_ignore_ascii_case(value_header) {
            // ukadoc: https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#Value:1
            value = Some(val.to_string());
        } else if name.eq_ignore_ascii_case("ErrorLevel") {
            // ukadoc: https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#ErrorLevel_20_5bSSP_62e1_5f35_5d:1
            error_level = Some(val.to_string());
        } else if name.eq_ignore_ascii_case("ErrorDescription") {
            // ukadoc: https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#ErrorDescription_20_5bSSP_62e1_5f35_5d:1
            error_description = Some(val.to_string());
        } else if name.eq_ignore_ascii_case("SecurityLevel") {
            // ukadoc: https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#SecurityLevel_20_5bSSP_62e1_5f35_5d:1
            security_level = Some(val.to_string());
        }
        // Reference0 / Marker / 未知ヘッダ等は読み飛ばす（要件 2.8）。`Charset` は復号前に
        // `scan_charset_header` が拾い終えているので、ここでは腕を持たない（要件 4.1）。
    }

    Ok(ParsedResponse {
        status,
        value,
        error_level,
        error_description,
        charset_header,
        decode_had_errors,
        security_level,
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
#[path = "shiori3_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "shiori3_charset_tests.rs"]
mod charset_tests;
