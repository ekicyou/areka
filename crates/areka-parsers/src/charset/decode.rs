//! 全体デコード（内部実装＋公開 facade `decode`）。
//!
//! `prescan` の検出結果（または呼び出し側指定の `DefaultEncoding`）に従い、
//! `encoding_rs` でバイト列全体を単一 `String` へデコードする。流れは次のとおり:
//! プリスキャン → `for_label` → 宣言 or 既定 → 全体デコード → had_errors 寛容 → String。

use super::model::DefaultEncoding;
use super::prescan;

/// バイト列を charset 宣言（または `default`）に従いデコードして 1 文字列を返す。
///
/// I/O・環境状態へアクセスしない純粋関数。同一 `(bytes, default)` に対し
/// 決定的。副作用は `tracing` の記録のみ。`Result` を返さず
/// panic しない。
///
/// 手順:
/// 1. charset 宣言の先読み（ファイル全体の行）で `charset,<name>` を抽出する（`prescan::prescan_charset`）。
/// 2. 宣言ありなら `encoding_rs::Encoding::for_label` で解決。解決可＝宣言エンコード採用、
///    未対応／解釈不能と ASCII と互換でないもの（UTF-16LE／UTF-16BE／replacement）＝
///    呼び出し側指定の既定へ寛容フォールバックし、`warn!` を 1 件残す。
/// 3. 宣言なし＝呼び出し側指定の既定を採用（`Ansi→SHIFT_JIS`／`Utf8→UTF_8`）。
/// 4. 選定エンコードでバイト列全体を `enc.decode(bytes)` でデコードする（BOM は
///    encoding_rs が sniff して吸収）。不正並びは U+FFFD 等で吸収され破棄しないが、
///    名前などが化けたことが既定の記録の段で見えるよう `warn!` を 1 件残す。
pub fn decode(bytes: &[u8], default: DefaultEncoding) -> String {
    let declared = prescan::prescan_charset(bytes);

    let enc = match declared {
        Some(name) => match encoding_rs::Encoding::for_label(name.as_bytes()) {
            // UTF-16 の 2 種と replacement は `charset,` の行と同じ字に読めず全体が壊れるので、
            // 解けないラベルと同じに扱う（`shiori-host32-host` の charset と同じ判定）。
            Some(enc) if enc.output_encoding() == enc => enc,
            _ => {
                tracing::warn!(
                    label = %name,
                    "未対応の charset ラベル: 呼び出し側指定の既定へフォールバック"
                );
                default.to_encoding()
            }
        },
        None => default.to_encoding(),
    };

    let (cow, actual, had_errors) = enc.decode(bytes);

    if had_errors {
        tracing::warn!(
            encoding = actual.name(),
            "デコード中に不正なバイト並びを検出: 代替文字で吸収して継続"
        );
    }

    cow.into_owned()
}
