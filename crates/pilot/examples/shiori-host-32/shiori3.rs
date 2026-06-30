//! host-32 先進坑: SHIORI/3.0 ワイヤコーデック（x64 親側に閉じる）。
//!
//! 本モジュールは x64 親プロセス専用であり、helper（i686）からは一切参照されない
//! （design.md Shiori3Codec §376–411・research.md §5.4: helper はバイト proxy に徹し、
//! SHIORI/3.0 の組立と `Value:` parse は x64 親側で行う＝本坑 x64 アダプタのミニチュア）。
//!
//! 提供する 2 機能（design.md Service Interface §395–402）:
//! - `build_onboot(ghostdir) -> Vec<u8>`: SHIORI/3.0 `OnBoot` リクエストを UTF-8 で生成（要件 4.1）。
//! - `parse_value(response) -> Option<String>`: 応答から `Value:`（さくらスクリプト本体）を抽出（要件 4.2/4.4）。
//!
//! charset は emo2 の UTF-8 固定（要件 4.4 / fixtures/emo2/ghost/master/descript.txt: `charset,UTF-8`）。

#![allow(dead_code)] // x64 親の後続タスク（ParentDriver）で配線するまでの暫定。

use std::path::Path;

/// main.rs から参照する最小マーカ（モジュールが x64 親ターゲットへ取り込まれている確認用）。
pub fn module_loaded() -> bool {
    true
}

/// Sender ヘッダ値（識別子・本先進坑の名乗り）。design.md §398 の例に倣う。
const SENDER: &str = "arekapilot";

/// SHIORI/3.0 `OnBoot` リクエストを UTF-8 バイト列で生成する（要件 4.1）。
///
/// `GET SHIORI/3.0` ＋必須ヘッダ（`ID: OnBoot` / `Charset: UTF-8` / `Sender`）を
/// CRLF 区切りで並べ、空行（`\r\n\r\n`）で終端する。これは応答を返す GET 経路であり
/// NOTIFY ではない（要件 4.1）。出力は常に有効な UTF-8（要件 4.4: emo2 は UTF-8 固定）。
///
/// `ghostdir`: SHIORI `load` に渡すゴーストディレクトリ。SHIORI/3.0 の `OnBoot`
/// リクエスト本体（ワイヤヘッダ）には ghostdir は現れない（ghostdir は別エントリ
/// `load(ghostdir)` の引数であり、リクエストヘッダの構成要素ではない。design.md §398 /
/// 要件 4.1）。本パラメータは design.md Service Interface §399 のシグネチャ整合
/// （`build_onboot(ghostdir: &Path) -> Vec<u8>`）のために受け取るが、ワイヤ生成には
/// 用いない（後続タスクで `load` と同一の値を共有する呼び出し側の便宜）。
fn build_onboot(ghostdir: &Path) -> Vec<u8> {
    let _ = ghostdir; // ワイヤ非関与（上記 doc 参照・シグネチャ整合のため保持）。
    let mut req = String::new();
    req.push_str("GET SHIORI/3.0\r\n");
    req.push_str("ID: OnBoot\r\n");
    req.push_str("Charset: UTF-8\r\n");
    req.push_str(&format!("Sender: {SENDER}\r\n"));
    req.push_str("SecurityLevel: local\r\n");
    req.push_str("\r\n"); // 空行終端（直前ヘッダの CRLF ＋ この空行 CRLF = "\r\n\r\n"）。
    req.into_bytes()
}

/// 応答バイト列（UTF-8 SHIORI/3.0）から `Value:` ヘッダの値を抽出する（要件 4.2/4.4）。
///
/// `Value:` 行が存在すれば、コロン以降をトリム（前後の空白・CRLF を除去）した
/// さくらスクリプト本体を `Some(String)` で返す。`Value:` が無ければ `None`
/// （例: `SHIORI/3.0 204 No Content`）。`Value:` と `Value: ` の空白差・CRLF/LF
/// いずれの改行にも頑健。
fn parse_value(response: &[u8]) -> Option<String> {
    // UTF-8 として解釈（要件 4.4）。不正バイトは None 扱いにせず lossy 回避のため from_utf8。
    let text = std::str::from_utf8(response).ok()?;
    for line in text.split("\r\n").flat_map(|l| l.split('\n')) {
        // ヘッダキーは大文字小文字を区別せず "Value" を探す（堅牢性）。
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
    use std::path::Path;

    #[test]
    fn build_onboot_produces_get_shiori3_request() {
        let req = build_onboot(Path::new(
            r"crates\pilot\examples\shiori-host-32\fixtures\emo2\ghost\master",
        ));
        // 有効な UTF-8 であること（要件 4.4: charset UTF-8）。
        let s = std::str::from_utf8(&req).expect("build_onboot output must be valid UTF-8");

        // GET 形式（応答を返す経路・NOTIFY でない）。design.md §398。
        assert!(s.starts_with("GET SHIORI/3.0\r\n"), "request:\n{s}");
        // 必須ヘッダ（要件 4.1）。
        assert!(s.contains("ID: OnBoot\r\n"), "request:\n{s}");
        assert!(s.contains("Charset: UTF-8\r\n"), "request:\n{s}");
        assert!(s.contains("Sender:"), "request:\n{s}");
        // 空行終端（CRLF + 空行 CRLF）。design.md §384。
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
}
