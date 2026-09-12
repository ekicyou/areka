//! `Charset`／`LabelError` の決定論テスト（タスク 1.1・要件 1.1/1.2/1.4/1.5/3.2/3.4/7.3/10.1〜10.3）。
//!
//! 窓も I/O も持たない純関数の入出力だけで成立する（x64 のみ・32bit 成果物不要）。
//! 参照値のバイト列は符号化器から導かず定数として持つ（design.md §Supporting References）。

use std::borrow::Cow;

use super::{Charset, LabelError};

/// 別名 5 綴りが同じ `Charset` へ解決し、正規名が `Shift_JIS` で一致する（要件 1.2）。
#[test]
fn shift_jis_aliases_resolve_to_the_same_charset() {
    let labels = [
        "shift_jis",
        "Shift-JIS",
        "sjis",
        "windows-31j",
        " SHIFT_JIS ",
    ];

    for label in labels {
        let cs = Charset::for_label(label).expect("Shift_JIS の別名は解決できる");
        assert_eq!(
            cs,
            Charset::SHIFT_JIS,
            "別名 {label:?} が同じ値へ解決しない"
        );
        assert_eq!(
            cs.name(),
            "Shift_JIS",
            "別名 {label:?} の正規名が一致しない"
        );
    }
}

/// 既定の 2 つ以外の文字コードも同じ経路で解決でき、実際にその文字コードで符号化される
/// （要件 1.4「既定は対応範囲ではない」＝Shift_JIS／UTF-8 だけを特別扱いしない）。
#[test]
fn charsets_beyond_the_two_constants_resolve_and_encode() {
    // 「あ」の参照値（design.md §Supporting References の定数）。
    let euc_jp = Charset::for_label("euc-jp").expect("EUC-JP は解決できる");
    assert_eq!(euc_jp.name(), "EUC-JP");
    assert_eq!(euc_jp.encode("あ").0.as_ref(), &[0xA4, 0xA2]);

    let iso_2022_jp = Charset::for_label("ISO-2022-JP").expect("ISO-2022-JP は解決できる");
    assert_eq!(iso_2022_jp.name(), "ISO-2022-JP");
    assert_eq!(
        iso_2022_jp.encode("あ").0.as_ref(),
        &[0x1B, 0x24, 0x42, 0x24, 0x22, 0x1B, 0x28, 0x42]
    );
}

/// 符号化の出力が要求と一致しないラベル（UTF-16 系・replacement 系）は
/// `NotEncodable`、Encoding Standard に無いラベルは `Unknown` で、理由が区別できる
/// （要件 1.5・10.3）。
#[test]
fn unusable_labels_are_rejected_with_distinguishable_reasons() {
    for label in ["UTF-16", "UTF-16LE", "replacement", "hz-gb-2312"] {
        assert_eq!(
            Charset::for_label(label),
            Err(LabelError::NotEncodable),
            "{label:?} は通信で使えない既知の限界として退けられるべき"
        );
    }

    assert_eq!(Charset::for_label("x-nope"), Err(LabelError::Unknown));
}

/// UTF-8 の符号化は入力の UTF-8 バイト列と同一で、複製も置換も起きない（要件 3.4）。
#[test]
fn utf8_encode_is_byte_identical_to_the_input() {
    let text = "ID: OnBoot\r\nReference0: あ\r\n";
    let (bytes, replaced) = Charset::UTF_8.encode(text);

    assert_eq!(bytes.as_ref(), text.as_bytes());
    assert_eq!(replaced, 0);
    assert!(
        matches!(bytes, Cow::Borrowed(_)),
        "UTF-8 では借用のまま返る（複製 0）"
    );
}

/// 表せない文字は 10 進の数値文字参照へ置換され、置換した文字数が返る（要件 10.1・3.2）。
#[test]
fn unmappable_char_becomes_a_decimal_numeric_character_reference() {
    let (bytes, replaced) = Charset::SHIFT_JIS.encode("\u{1F600}");

    assert_eq!(bytes.as_ref(), b"&#128512;");
    assert_eq!(replaced, 1);
}

/// 宣言された文字コードとして不正な並びは代替文字へ吸収され、置換の有無が返る（要件 10.2）。
#[test]
fn invalid_byte_sequence_decodes_to_the_replacement_char() {
    // 0x82 は Shift_JIS の 2 バイト文字の先行バイト。単独で終わるので不正な並び。
    let (text, had_errors) = Charset::SHIFT_JIS.decode(&[0x82]);

    assert!(
        text.contains('\u{FFFD}'),
        "代替文字へ吸収されていない: {text:?}"
    );
    assert!(had_errors);

    // 妥当な並びでは置換が起きない（恒真な緑にしない較正）。
    let (text, had_errors) = Charset::SHIFT_JIS.decode(&[0x82, 0xA0]);
    assert_eq!(text, "あ");
    assert!(!had_errors);
}

/// 復号は宣言された文字コードだけで行い、先頭の BOM で文字コードを切り替えない
/// （BOM は 1 文字として残る・要件 10.2 の経路と同じ寛容さ）。
#[test]
fn decode_does_not_let_a_bom_override_the_declared_charset() {
    let (text, had_errors) = Charset::UTF_8.decode(b"\xEF\xBB\xBFA");
    assert_eq!(text, "\u{FEFF}A");
    assert!(!had_errors);

    // UTF-8 の BOM を Shift_JIS として宣言された応答に混ぜても UTF-8 へ切り替わらない。
    let (text, _) = Charset::SHIFT_JIS.decode(b"\xEF\xBB\xBF\x82\xA0");
    assert!(text.ends_with('あ'));
    assert_ne!(text, "\u{FEFF}あ", "UTF-8 として読み直してはならない");
}

/// 正規名は `Charset` ヘッダに書く綴りで、定数と解決結果で一致する（要件 3.2）。
#[test]
fn canonical_names_match_the_header_spelling() {
    assert_eq!(Charset::UTF_8.name(), "UTF-8");
    assert_eq!(Charset::SHIFT_JIS.name(), "Shift_JIS");
    assert_eq!(
        Charset::for_label("utf8").map(Charset::name),
        Ok("UTF-8"),
        "別名からも同じ綴りが取れる"
    );
}
