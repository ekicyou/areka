//! 公開パス契約テスト（charset 行なしファイルの `decode → parse_kv` 通し）。
//!
//! 本ファイルは spec の**統合／公開パス契約層**として、公開エントリ
//! `crate::charset::decode` → `crate::kv::parse_kv` の通しで、charset 宣言の
//! ない実 fixture 由来入力の値が**文字列のまま保持**されること（数値化・型付け
//! しない・R4.7）を固定する。期待値はリテラル直書き（`include_str!` 不使用・R7.3）。

use crate::charset::{decode, DefaultEncoding};
use crate::kv::parse_kv;

// ───────────────────────────────────────────────────────────────────
// charset なしファイル・値は文字列保持（R7.4, R1.4, R4.7）
// ───────────────────────────────────────────────────────────────────

/// emo2 `balloons0s.txt`（charset 行なし）由来のリテラルバイト列を既定 `Utf8` で
/// `decode` → `parse_kv` に通し、数値に見える値も**文字列のまま**保持される
/// ことを固定する（数値化・符号解釈しない・R4.7）。
///
/// 採取元: `crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku/balloons0s.txt`
/// L1 `windowposition.x,266` / L2 `windowposition.y,-129` / L3 空行 / L4 `wordwrappoint.x,-49`
/// （実測 verbatim・L3 の空行 gap も fixture どおり再現）。
/// - charset 宣言が無いため、呼び出し側指定の既定 `Utf8` が支配する（R1.4）。
/// - 空行はスキップされる（`parse_kv` の R4.5）。
/// - 値 `266` / `-129` / `-49` はいずれも文字列であり、数値化されない（R4.7）。
///
/// 公開 API パス（`decode`→`parse_kv`）を経由する契約固定（R7.4）。
#[test]
fn emo2_balloons0s_no_charset_values_stay_string() {
    // balloons0s.txt L1/L2/L4 由来のリテラル入力（L3 の空行 gap も再現・charset 行なし）。
    let bytes = b"windowposition.x,266\nwindowposition.y,-129\n\nwordwrappoint.x,-49\n";

    let text = decode(bytes, DefaultEncoding::Utf8);
    let kv = parse_kv(&text);

    // 数値に見えても文字列のまま（数値化しない・R4.7）。
    assert_eq!(kv.get("windowposition.x").map(String::as_str), Some("266"));
    assert_eq!(kv.get("windowposition.y").map(String::as_str), Some("-129"));
    assert_eq!(kv.get("wordwrappoint.x").map(String::as_str), Some("-49"));
}
