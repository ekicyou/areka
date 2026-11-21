//! host-32 先進坑: SHIORI/3.0 ワイヤコーデック（x64 親側に閉じる）。
//!
//! helper（i686）はバイト proxy に徹し、SHIORI/3.0 の組立と `Value:` の parse は x64 親側で行う
//! （本坑の x64 アダプタのミニチュア）。
//!
//! charset は emo2 の UTF-8 固定（検体 emo2 の ghost/master/descript.txt: `charset,UTF-8`）。

/// Sender ヘッダ値（本先進坑の名乗り）。
const SENDER: &str = "arekapilot";

/// SHIORI/3.0 `OnBoot` リクエストを UTF-8 バイト列で生成する。
///
/// `GET SHIORI/3.0` ＋必須ヘッダ（`ID: OnBoot` / `Charset: UTF-8` / `Sender`）を
/// CRLF 区切りで並べ、空行（`\r\n\r\n`）で終端する。応答を返す GET 経路で、NOTIFY ではない。
pub fn build_onboot() -> Vec<u8> {
    let mut req = String::new();
    req.push_str("GET SHIORI/3.0\r\n");
    req.push_str("ID: OnBoot\r\n");
    req.push_str("Charset: UTF-8\r\n");
    req.push_str(&format!("Sender: {SENDER}\r\n"));
    req.push_str("SecurityLevel: local\r\n");
    req.push_str("\r\n"); // 空行終端（直前ヘッダの CRLF ＋ この空行 CRLF = "\r\n\r\n"）。
    req.into_bytes()
}

/// 応答バイト列（UTF-8 SHIORI/3.0）から `Value:` ヘッダの値を抽出する。
///
/// `Value:` 行が存在すれば、コロン以降をトリム（前後の空白・CRLF を除去）した
/// さくらスクリプト本体を `Some(String)` で返す。`Value:` が無ければ `None`
/// （例: `SHIORI/3.0 204 No Content`）。`Value:` と `Value: ` の空白差・CRLF/LF
/// いずれの改行にも頑健。
pub fn parse_value(response: &[u8]) -> Option<String> {
    // 不正な UTF-8 は lossy で直さず None にする。
    let text = std::str::from_utf8(response).ok()?;
    for line in text.lines() {
        // ヘッダキーは大文字小文字を区別せず "Value" を探す。
        if let Some((key, val)) = line.split_once(':') {
            if key.trim().eq_ignore_ascii_case("Value") {
                return Some(val.trim().to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_onboot_produces_get_shiori3_request() {
        let req = build_onboot();
        let s = std::str::from_utf8(&req).expect("build_onboot output must be valid UTF-8");

        // GET 形式（応答を返す経路・NOTIFY でない）。
        assert!(s.starts_with("GET SHIORI/3.0\r\n"), "request:\n{s}");
        // 必須ヘッダ。
        assert!(s.contains("ID: OnBoot\r\n"), "request:\n{s}");
        assert!(s.contains("Charset: UTF-8\r\n"), "request:\n{s}");
        assert!(s.contains("Sender:"), "request:\n{s}");
        // 空行終端（CRLF + 空行 CRLF）。
        assert!(s.ends_with("\r\n\r\n"), "request:\n{s}");
    }

    #[test]
    fn parse_value_extracts_sakura_script_body() {
        // 多バイト UTF-8 を含む現実的なさくらスクリプト本体（charset 取り扱いの確認）。
        let body = r"\w0\h\s[0]こんにちは\e";
        let response = format!(
            "SHIORI/3.0 200 OK\r\nCharset: UTF-8\r\nSender: pasta\r\nValue: {body}\r\n\r\n"
        );
        let got = parse_value(response.as_bytes());
        assert_eq!(got.as_deref(), Some(body));
    }

    #[test]
    fn parse_value_returns_none_when_absent() {
        // Value: ヘッダのない応答（204 No Content）。
        let response = "SHIORI/3.0 204 No Content\r\nCharset: UTF-8\r\nSender: pasta\r\n\r\n";
        assert_eq!(parse_value(response.as_bytes()), None);
    }

    #[test]
    fn parse_value_reads_lf_only_response() {
        // 改行が `\n` だけの応答でも Value を取り出す。
        let response = "SHIORI/3.0 200 OK\nCharset: UTF-8\nValue: \\h\\s[0]\\e\n\n";
        assert_eq!(
            parse_value(response.as_bytes()).as_deref(),
            Some(r"\h\s[0]\e")
        );
    }
}
