//! 公開パス契約テスト（charset デコード起点・`decode → parse_kv` 通し）。
//!
//! 本ファイルは spec の**統合／公開パス契約層**として、公開エントリ
//! `crate::charset::decode`（`pub fn decode(&[u8], DefaultEncoding) -> String`）を
//! 起点に、`crate::kv::parse_kv` へ通した end-to-end アサーションで以下を固定する:
//! - emo2 `descript.txt`（UTF-8）由来リテラル入力の既定 `Utf8` 通し（R7.1/R7.4）。
//! - `SHIFT_JIS.encode` ラウンドトリップ合成入力での宣言優先デコード
//!   （宣言 Shift_JIS が既定 `Utf8` に勝ち文字化けしない・R7.2/R7.3/R2.2/D3）。
//! - charset 宣言なし入力の**既定別**デコード（指定どおりのエンコードが支配・R7.5/R2.4）。
//!
//! 期待値はすべてリテラル直書き（クレート跨ぎ `include_str!` 不使用・R7.3）。
//! fixture 由来値には採取元の正本ファイル名・行をコメントで明示する。

use crate::charset::{decode, DefaultEncoding};
use crate::kv::parse_kv;

// ───────────────────────────────────────────────────────────────────
// 1. emo2 UTF-8 通し（R7.1, R7.4）
// ───────────────────────────────────────────────────────────────────

/// emo2 `descript.txt`（UTF-8）冒頭のリテラルバイト列を既定 `Utf8` で
/// `decode` → `parse_kv` に通し、`type`／`name` の期待値を固定する。
///
/// 採取元: `crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku/descript.txt`
/// L1 `charset,UTF-8` / L2 `type,balloon` / L3 `name,kakukaku for emo-gs`（実測 verbatim）。
/// charset 宣言 `UTF-8` は既定 `Utf8` と一致し、UTF-8 として素直に読める（R7.1）。
/// 公開 API パス（`decode`→`parse_kv`）を経由する契約固定（R7.4）。
#[test]
fn emo2_descript_utf8_roundtrip_pins_type_and_name() {
    // descript.txt L1–L3 由来のリテラル入力（UTF-8）。
    let bytes = b"charset,UTF-8\ntype,balloon\nname,kakukaku for emo-gs\n";

    let text = decode(bytes, DefaultEncoding::Utf8);
    let kv = parse_kv(&text);

    assert_eq!(kv.get("type").map(String::as_str), Some("balloon"));
    assert_eq!(
        kv.get("name").map(String::as_str),
        Some("kakukaku for emo-gs"),
    );
}

// ───────────────────────────────────────────────────────────────────
// 2. Shift_JIS 合成通し（R7.2, R7.3, R2.2, D3）
// ───────────────────────────────────────────────────────────────────

/// 宣言 `charset,Shift_JIS` を持つ入力は、既定 `Utf8` を渡しても宣言優先で
/// Shift_JIS デコードされ、日本語値が文字化けなく復元されることを固定する。
///
/// - 入力は `encoding_rs::SHIFT_JIS.encode(...)` のラウンドトリップで生成した
///   **合成入力**であり、fixture に SJIS 実ファイルは無い（R7.2）。
/// - このテストは `encoding_rs::Encoding::for_label(b"Shift_JIS")` 解決の
///   **生き証人**である: ukadoc 表記 `Shift_JIS` のラベル正規化がラベル解決
///   経路を通り、宣言エンコードが採用されること（既定 `Utf8` に勝つ＝R2.2）を
///   このケースが担保する（D3）。
/// - 期待文字列 `かくかく` はリテラル直書きで契約を固定する（R7.3）。
#[test]
fn synthesized_shift_jis_declaration_wins_over_utf8_default() {
    // SHIFT_JIS ラウンドトリップで合成（fixture に SJIS 実ファイル無し・R7.2）。
    let (bytes, _enc, had_errors) =
        encoding_rs::SHIFT_JIS.encode("charset,Shift_JIS\r\nname,かくかく\r\n");
    assert!(!had_errors, "合成入力は Shift_JIS へ無損失変換できる前提");

    // 既定 `Utf8` を渡しても宣言 `Shift_JIS` が優先される（for_label 解決の生き証人）。
    let text = decode(&bytes, DefaultEncoding::Utf8);
    let kv = parse_kv(&text);

    assert_eq!(kv.get("name").map(String::as_str), Some("かくかく"));
}

// ───────────────────────────────────────────────────────────────────
// 3. charset 宣言なし・既定別（R7.5, R2.4）
// ───────────────────────────────────────────────────────────────────

/// charset 宣言のない入力は、呼び出し側が渡した既定エンコードが支配し、
/// 指定どおりのエンコードでデコードされることを固定する（R7.5/R2.4）。
///
/// - `Ansi` パスの生き証人: charset 行なしの日本語 KV を SHIFT_JIS バイト列で
///   与え、`Ansi`（→CP932 固定写像）でデコードすると `name→かくかく` に復元される。
/// - `Utf8` パス: charset 行なしの ASCII/UTF-8 バイト列を `Utf8` でデコードすると
///   `name→hello` に復元される。
///
/// いずれも宣言に依らず**既定指定のみ**でエンコードが決まることを示す。
#[test]
fn no_charset_declaration_decodes_per_default_encoding() {
    // Ansi 既定: charset 行なしの日本語 KV（SHIFT_JIS バイト列・合成）。
    let (ansi_bytes, _enc, had_errors) = encoding_rs::SHIFT_JIS.encode("name,かくかく\r\n");
    assert!(!had_errors, "合成入力は Shift_JIS へ無損失変換できる前提");
    let ansi_text = decode(&ansi_bytes, DefaultEncoding::Ansi);
    let ansi_kv = parse_kv(&ansi_text);
    assert_eq!(ansi_kv.get("name").map(String::as_str), Some("かくかく"));

    // Utf8 既定: charset 行なしの ASCII/UTF-8 バイト列。
    let utf8_bytes = b"name,hello\n";
    let utf8_text = decode(utf8_bytes, DefaultEncoding::Utf8);
    let utf8_kv = parse_kv(&utf8_text);
    assert_eq!(utf8_kv.get("name").map(String::as_str), Some("hello"));
}
