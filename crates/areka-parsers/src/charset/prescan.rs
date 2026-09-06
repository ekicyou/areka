//! 冒頭 ASCII プリスキャン（内部・非公開）。
//!
//! バイト列先頭を実エンコード非依存に走査し `charset,<name>` 宣言を抽出する
//! （D1/D2）。charset 名は ASCII 前提（R1.5）ゆえ、非 ASCII 行より前の ASCII
//! 接頭部のみを検査すれば足り、ファイルの実エンコードに依存しない。
//! 公開面（`mod.rs`）は本モジュールを `pub` 公開しない（内部専用・消費側の
//! `decode` はタスク 2.3 で結線）。

/// プリスキャン走査上限（バイト）。過大スキャン防止の安全弁（D1）。
const SCAN_CAP: usize = 4096;

/// 冒頭プリスキャンで charset 宣言名を抽出する（D1/D2・R1.1–1.5・R5.2）。
///
/// 手順:
/// 1. 先頭の UTF-8 BOM（`EF BB BF`）を走査開始前に読み飛ばす（BOM note・R5.2）。
///    BOM を非 ASCII 打ち切りに該当させて宣言を取り逃さないための一意方針。
/// 2. BOM 除去後、先頭から **最初の非 ASCII バイト（`>= 0x80`）まで、かつ上限
///    4096 バイトまで** を走査対象とする（D1）。非 ASCII 出現／上限到達で打ち切る。
/// 3. 走査対象を CRLF/LF 寛容に行分割し、各行を **最初のカンマのみ** で
///    `key` / `value` に分ける。`key` を trim・大小無視で `charset` と比較し、
///    一致すれば `value` を trim した文字列を返す（D2）。カンマ `,` が唯一の
///    区切りで、`charset:` 等の異体は一致しない。
///
/// 宣言が上限内に見つからなければ「宣言なし」として `None` を返す（R1.4）。
///
/// 純粋関数（環境非依存・panic せず・`Result` を返さない）。消費側 `decode`
/// はタスク 2.3 で結線するため、それまで未使用警告を抑止する。
#[allow(dead_code)]
pub(super) fn prescan_charset(bytes: &[u8]) -> Option<String> {
    // 1. 先頭 UTF-8 BOM を読み飛ばす（走査開始前）。
    let after_bom = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);

    // 2. 走査窓を確定: 先頭〜最初の非 ASCII バイト、かつ上限 4096 バイト。
    let mut end = 0;
    while end < after_bom.len() && end < SCAN_CAP {
        if after_bom[end] >= 0x80 {
            break; // 非 ASCII 出現で打ち切り（charset 名は ASCII 前提・R1.5）。
        }
        end += 1;
    }
    let window = &after_bom[..end];

    // 3. 行単位（CRLF/LF 寛容）に走査。窓は ASCII のみゆえ UTF-8 として安全。
    //    `\r` は行内容の trim で吸収されるため、`\n` 分割のみで CRLF に対応する。
    for line in window.split(|&b| b == b'\n') {
        // ASCII 窓につき from_utf8 は必ず成功するが、寛容に握りつぶす。
        let Ok(text) = core::str::from_utf8(line) else {
            continue;
        };
        // 最初のカンマのみで分割（カンマが唯一の区切り・D2）。
        let Some((key, value)) = text.split_once(',') else {
            continue; // カンマ無し行（`charset:` 異体を含む）はスキップ。
        };
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#charset_2c_6587_5b57_30b3_30fc_30c9:1
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#charset_2c_6587_5b57_30b3_30fc_30c9:1
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#charset_2c_6587_5b57_30b3_30fc_30c9:1
        if key.trim().eq_ignore_ascii_case("charset") {
            return Some(value.trim().to_string());
        }
    }

    None
}
