//! charset 宣言の先読み（ファイル全体の行・内部・非公開）。
//!
//! バイト列全体を実エンコード非依存に行で走査し `charset,<name>` 宣言を抽出する
//! （D1/D2）。charset 名は ASCII 前提（R1.5）ゆえ、宣言の行は ASCII で読め、
//! ほかの行の実エンコードに依存しない。宣言の位置は問わない（7.11(b)・
//! `shiori-host32-host` の `scan_charset_header` と同じ読み）。
//! 公開面（`mod.rs`）は本モジュールを `pub` 公開しない（内部専用・消費側は `decode`）。

/// ファイル全体の行の先読みで charset 宣言名を抽出する（D1/D2・R1.1–1.5・R5.2）。
///
/// 手順:
/// 1. 先頭の UTF-8 BOM（`EF BB BF`）を走査開始前に読み飛ばす（BOM note・R5.2）。
///    BOM を宣言の行の一部と読んで取り逃さないための一意方針。
/// 2. BOM 除去後のファイル全体を、位置を問わず走査対象とする（7.11(b)）。
///    非 ASCII の行は UTF-8 で読めなければ飛ばし、その後ろの行も見る。
/// 3. 走査対象を CRLF/LF 寛容に行分割し、各行を **最初のカンマのみ** で
///    `key` / `value` に分ける。`key` を trim・大小無視で `charset` と比較し、
///    一致すれば `value` を trim した文字列を返す（D2）。カンマ `,` が唯一の
///    区切りで、`charset:` 等の異体は一致しない。
///
/// 宣言がどの行にも見つからなければ「宣言なし」として `None` を返す（R1.4）。
///
/// 純粋関数（環境非依存・panic せず・`Result` を返さない）。消費側は `decode`。
pub(super) fn prescan_charset(bytes: &[u8]) -> Option<String> {
    // 1. 先頭 UTF-8 BOM を読み飛ばす（走査開始前）。
    let after_bom = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);

    // 2・3. ファイル全体を行単位（CRLF/LF 寛容）に走査する（位置を問わない・7.11(b)）。
    //    `\r` は行内容の trim で吸収されるため、`\n` 分割のみで CRLF に対応する。
    for line in after_bom.split(|&b| b == b'\n') {
        // UTF-8 で読めない行（Shift_JIS の本文など）は宣言の行でないので飛ばす。
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
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#charset_2c_6587_5b57_30b3_30fc_30c9:1
        if key.trim().eq_ignore_ascii_case("charset") {
            return Some(value.trim().to_string());
        }
    }

    None
}
