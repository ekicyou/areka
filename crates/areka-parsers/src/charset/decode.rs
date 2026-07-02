//! 全体デコード（内部実装＋公開 facade `decode`）。
//!
//! `prescan` の検出結果（または呼び出し側指定の `DefaultEncoding`）に従い、
//! `encoding_rs` でバイト列全体を単一 `String` へデコードする。フローは design
//! 「System Flows」の decode 図に対応する:
//! プリスキャン → `for_label` → 宣言 or 既定 → 全体デコード → had_errors 寛容 → String。

use super::model::DefaultEncoding;
use super::prescan;

/// バイト列を charset 宣言（または `default`）に従いデコードして 1 文字列を返す。
///
/// I/O・環境状態へアクセスしない純粋関数（R3.1）。同一 `(bytes, default)` に対し
/// 決定的（R3.2）。副作用は `tracing` 診断ログのみ（R3.3/R6.3）。`Result` を返さず
/// panic しない（R2.7/R6.1）。
///
/// 手順（design decode フロー・D6）:
/// 1. 冒頭プリスキャンで `charset,<name>` を抽出する（`prescan::prescan_charset`）。
/// 2. 宣言ありなら `encoding_rs::Encoding::for_label` で解決。解決可＝宣言エンコード採用、
///    未対応／解釈不能（R2.5）＝呼び出し側指定の既定へ寛容フォールバック。
/// 3. 宣言なし（R1.4/R2.4）＝呼び出し側指定の既定を採用（`Ansi→SHIFT_JIS`／`Utf8→UTF_8`）。
/// 4. 選定エンコードでバイト列全体を `enc.decode(bytes)` でデコードする（BOM は
///    encoding_rs が sniff して吸収・R5.2）。不正並びは U+FFFD 等で吸収され破棄しない（R2.6）。
pub fn decode(bytes: &[u8], default: DefaultEncoding) -> String {
    // 1. 冒頭プリスキャンで charset 宣言名を抽出（実エンコード非依存・BOM 寛容）。
    let declared = prescan::prescan_charset(bytes);

    // 2/3. 宣言ラベルを解決し、選定エンコードを確定する。
    let enc = match declared {
        Some(name) => match encoding_rs::Encoding::for_label(name.as_bytes()) {
            // 宣言ラベルを解決できた: 宣言エンコードを採用（R2.1/R2.2）。
            Some(enc) => enc,
            // 未対応／解釈不能ラベル: 既定へ寛容フォールバック（R2.5）。
            None => {
                tracing::debug!(
                    label = %name,
                    "未対応の charset ラベル: 呼び出し側指定の既定へフォールバック"
                );
                default.to_encoding()
            }
        },
        // 宣言なし: 呼び出し側指定の既定を採用（R1.4/R2.4）。
        None => default.to_encoding(),
    };

    // 4. 選定エンコードでバイト列全体をデコードする。`decode` は BOM を sniff して
    //    吸収し（R5.2）、不正並びを U+FFFD 等で置換する（Result/panic なし）。
    let (cow, _actual, had_errors) = enc.decode(bytes);

    // 不正並びは破棄せず（既に代替文字で吸収済み）、診断ログのみ出して String を返す（R2.6）。
    if had_errors {
        tracing::debug!(
            encoding = enc.name(),
            "デコード中に不正なバイト並びを検出: 代替文字で吸収して継続"
        );
    }

    cow.into_owned()
}
