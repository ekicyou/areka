//! decode 単体テスト（宣言あり/なし/未対応/不正並び/BOM/決定性）。
//!
//! 公開 facade `charset::decode` の契約（R2/R3/R5.2/R6）を Observable に固定する。
//! フローは design「System Flows」の decode 図に対応:
//! プリスキャン → for_label → 宣言 or 既定 → 全体デコード → had_errors 寛容 → String。

use super::decode;
use super::model::DefaultEncoding;

/// R2.1: 宣言 UTF-8 で全体をデコードする（宣言どおり・全内容が復元される）。
#[test]
fn declared_utf8_decodes_whole_content() {
    let bytes = b"charset,UTF-8\ntype,balloon\n";
    let out = decode(bytes, DefaultEncoding::Utf8);
    assert!(
        out.contains("type,balloon"),
        "全内容がデコードされる: {out:?}"
    );
    assert!(
        out.contains("charset,UTF-8"),
        "宣言行も内容として復元される: {out:?}"
    );
}

/// R2.2（生き証人）: 宣言 Shift_JIS が既定 Utf8 に勝ち、文字化けしない。
///
/// SJIS でエンコードしたバイト列を Utf8 既定で decode しても、宣言 SJIS が優先され
/// 「かくかく」が正しく復元される（宣言が既定に勝つ・SJIS を UTF-8 と誤読しない）。
#[test]
fn declared_shift_jis_beats_utf8_default() {
    let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode("charset,Shift_JIS\r\nname,かくかく\r\n");
    let out = decode(&bytes, DefaultEncoding::Utf8);
    assert!(
        out.contains("name,かくかく"),
        "宣言 SJIS が既定 Utf8 に勝ち文字化けしない: {out:?}"
    );
}

/// R2.5: 未対応（解釈不能）ラベルは呼び出し側指定の既定へ寛容フォールバックする。
#[test]
fn unsupported_label_falls_back_to_default() {
    let bytes = b"charset,NOSUCH-ENC\nname,hi\n";
    let out = decode(bytes, DefaultEncoding::Utf8);
    assert!(
        out.contains("name,hi"),
        "未対応ラベルは既定 Utf8 でデコード継続: {out:?}"
    );
}

/// R2.4: 宣言なし・既定 Utf8 → UTF-8 としてデコードする。
#[test]
fn no_declaration_uses_utf8_default() {
    let bytes = "windowposition.x,266\nname,かきくけこ\n".as_bytes();
    let out = decode(bytes, DefaultEncoding::Utf8);
    assert!(
        out.contains("name,かきくけこ"),
        "宣言なしは既定 Utf8 でデコード: {out:?}"
    );
}

/// R2.4: 宣言なし・既定 Ansi（CP932）→ SJIS バイト列が正しく復元される。
#[test]
fn no_declaration_uses_ansi_default_as_shift_jis() {
    let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode("name,さしすせそ\r\n");
    let out = decode(&bytes, DefaultEncoding::Ansi);
    assert!(
        out.contains("name,さしすせそ"),
        "宣言なし・既定 Ansi は CP932 でデコード: {out:?}"
    );
}

/// R2.6: 宣言エンコードとして不正な並びを代替文字で吸収し、破棄も panic もしない。
///
/// 宣言 UTF-8 のあとに単独の不正 UTF-8 バイト（`0xFF`）を含める。プリスキャンは
/// 非 ASCII で走査を打ち切るが宣言 `charset,UTF-8` は先に検出されるため、最終
/// デコードは UTF-8 として全体を走査し、不正部を U+FFFD で置換して String を返す。
#[test]
fn malformed_bytes_absorbed_without_panic() {
    let bytes = b"charset,UTF-8\nname,\xff\xfe\n";
    let out = decode(bytes, DefaultEncoding::Utf8);
    // 破棄せず宣言行・前半内容は保持される。
    assert!(out.contains("charset,UTF-8"), "破棄せず内容を返す: {out:?}");
    assert!(out.contains("name,"), "不正部の前は保持される: {out:?}");
    // 不正並びは代替文字 U+FFFD で吸収される（panic せず String を返す）。
    assert!(
        out.contains('\u{FFFD}'),
        "不正バイトは U+FFFD で吸収される: {out:?}"
    );
}

/// R5.2: 先頭 UTF-8 BOM 付き `charset,UTF-8` を宣言検出し、結果に BOM を残さない。
#[test]
fn bom_is_tolerated_and_not_leaked() {
    let bytes = b"\xEF\xBB\xBFcharset,UTF-8\nname,x\n";
    let out = decode(bytes, DefaultEncoding::Utf8);
    assert!(
        out.contains("name,x"),
        "BOM 付きでも内容をデコード: {out:?}"
    );
    assert!(
        !out.starts_with('\u{FEFF}'),
        "encoding_rs が BOM を sniff し先頭に残さない: {out:?}"
    );
    assert!(out.contains("charset,UTF-8"), "宣言行が復元される: {out:?}");
}

/// R3.2: 同一（バイト列, 既定）に対し決定的（複数回で同一 String）。
#[test]
fn decode_is_deterministic() {
    let bytes = b"charset,UTF-8\nname,determinism\n";
    let a = decode(bytes, DefaultEncoding::Utf8);
    let b = decode(bytes, DefaultEncoding::Utf8);
    assert_eq!(a, b, "同一入力は決定的に同一結果");
}

/// R6.1/R5.3 相当: 空入力でも panic せず空文字列相当を返す。
#[test]
fn empty_input_does_not_panic() {
    let out = decode(b"", DefaultEncoding::Utf8);
    assert!(out.is_empty(), "空入力は空文字列: {out:?}");
}
